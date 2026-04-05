use rustc_hash::FxBuildHasher;
use std::collections::HashMap;

use crate::bpe::Tokenizer;
use crate::pretokenize::pretoken_fast::FastPretokenizer;
use crate::token::TokenId;

// ---------------------------------------------------------------------------
// High-throughput streaming encoder
//
// 32-byte cache-aligned probe slots. The packed prefix (8 bytes) is computed
// once per pretoken and used for: (a) deriving the hash via wymix, and
// (b) inline byte verification for pretokens <= 8 bytes. Longer pretokens
// verify the tail via an entries array.
//
// Other optimizations:
// - Raw pointer output writes (no Vec overhead per token)
// - Single-token results stored inline in the slot
// ---------------------------------------------------------------------------

/// Pack pretoken bytes into a u64 using overlapping reads.
#[inline(always)]
fn pack8(bytes: &[u8]) -> u64 {
    let len = bytes.len();
    let ptr = bytes.as_ptr();
    if len >= 8 {
        unsafe { (ptr as *const u64).read_unaligned() }
    } else if len >= 4 {
        let lo = unsafe { (ptr as *const u32).read_unaligned() } as u64;
        let hi = unsafe { ((ptr.add(len - 4)) as *const u32).read_unaligned() } as u64;
        lo | (hi << 32)
    } else if len >= 2 {
        let lo = unsafe { (ptr as *const u16).read_unaligned() } as u64;
        let hi = bytes[len - 1] as u64;
        lo | (hi << 16)
    } else {
        bytes[0] as u64
    }
}

/// Hash from packed prefix + length via 128-bit multiply (wymix).
#[inline(always)]
fn prefix_hash(packed: u64, len: u16) -> u64 {
    let r = (packed as u128).wrapping_mul((len as u64 ^ 0xe7037ed1a0b428db) as u128);
    ((r as u64) ^ (r >> 64) as u64) | 1
}

/// Full hash for long byte slices (> 8 bytes). Reads all bytes.
#[inline(always)]
fn full_hash(bytes: &[u8]) -> u64 {
    let len = bytes.len();
    let ptr = bytes.as_ptr();
    let mut h: u64 = len as u64 ^ 0xa0761d6478bd642f;
    let mut i = 0;
    while i + 8 <= len {
        let w = unsafe { (ptr.add(i) as *const u64).read_unaligned() };
        let r = (h ^ w) as u128 * 0xe7037ed1a0b428db_u128;
        h = (r as u64) ^ (r >> 64) as u64;
        i += 8;
    }
    if i < len {
        let tail = unsafe { (ptr.add(len - 8) as *const u64).read_unaligned() };
        let r = (h ^ tail) as u128 * 0x8a5cd789635d2dff_u128;
        h = (r as u64) ^ (r >> 64) as u64;
    }
    h | 1
}

#[derive(Copy, Clone)]
#[repr(C, align(32))]
struct Slot {
    prefix8: u64,    // 8
    fp: u64,         // 8: hash; 0 = empty
    tok_or_idx: u32, // 4
    entry_idx: u32,  // 4
    pt_len: u16,     // 2
    n_tok: u16,      // 2
    _pad: u32,       // 4
}

impl Slot {
    const EMPTY: Self = Slot { prefix8: 0, fp: 0, tok_or_idx: 0, entry_idx: 0, pt_len: 0, n_tok: 0, _pad: 0 };
}

#[derive(Copy, Clone)]
struct EntryPtr(*const u8);
unsafe impl Send for EntryPtr {}
unsafe impl Sync for EntryPtr {}

pub fn encode_lines(lines: &[&[u8]], tokenizer: &Tokenizer) -> (Vec<TokenId>, Vec<usize>) {
    let merges = &tokenizer.merges;
    let remap = tokenizer.byte_remapping.as_ref().map(|br| br.mapping.as_slice());
    let b2t: [TokenId; 256] = {
        let mut t = [TokenId(0); 256];
        for i in 0..256 {
            t[i] = TokenId(match remap { Some(r) => r[i] as u32, None => i as u32 });
        }
        t
    };

    let total_bytes: usize = lines.iter().map(|l| l.len()).sum();
    let est_unique = (total_bytes / 300).max(4096);
    let cap = (est_unique * 2).next_power_of_two().max(4096);
    let mut mask = cap - 1;

    let mut slots: Vec<Slot> = vec![Slot::EMPTY; cap];
    let mut entries: Vec<EntryPtr> = Vec::with_capacity(est_unique / 8);
    let mut tok_store: Vec<TokenId> = Vec::with_capacity(est_unique * 2);
    let mut scratch: Vec<TokenId> = Vec::with_capacity(128);
    let mut n_entries = 0usize;

    let init_cap = (total_bytes * 3 / 10).max(1024);
    let mut output: Vec<TokenId> = Vec::with_capacity(init_cap);
    let mut out_base = output.as_mut_ptr();
    let mut out_len = 0usize;
    let mut out_cap = output.capacity();

    let mut boundaries: Vec<usize> = Vec::with_capacity(lines.len() + 1);
    boundaries.push(0);

    for &line in lines {
        if out_len + line.len() > out_cap {
            unsafe { output.set_len(out_len); }
            output.reserve(line.len() + out_cap / 4);
            out_base = output.as_mut_ptr();
            out_cap = output.capacity();
        }

        let mut pt = FastPretokenizer::new(line);
        while let Some((pretoken, packed)) = pt.next_with_pack8() {
            let bytes = pretoken.0;
            let blen = bytes.len();

            if blen == 1 {
                unsafe { *out_base.add(out_len) = b2t[bytes[0] as usize]; }
                out_len += 1;
                continue;
            }

            // packed is already computed by the pretokenizer
            let blen16 = blen as u16;
            let fp = if blen <= 8 {
                prefix_hash(packed, blen16)
            } else {
                full_hash(bytes)
            };

            let mut si = fp as usize & mask;

            loop {
                let slot = unsafe { slots.get_unchecked(si) };
                if slot.fp == 0 {
                    scratch.clear();
                    for &b in bytes { scratch.push(b2t[b as usize]); }
                    bpe_merge(&mut scratch, merges);
                    let nt = scratch.len();

                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            scratch.as_ptr(), out_base.add(out_len), nt,
                        );
                    }
                    out_len += nt;

                    let ei = entries.len() as u32;
                    if blen > 8 { entries.push(EntryPtr(bytes.as_ptr())); }

                    let new_slot = if nt == 1 {
                        Slot { prefix8: packed, fp, tok_or_idx: scratch[0].0, entry_idx: ei, pt_len: blen16, n_tok: 1, _pad: 0 }
                    } else {
                        let ts = tok_store.len() as u32;
                        tok_store.extend_from_slice(&scratch);
                        Slot { prefix8: packed, fp, tok_or_idx: ts, entry_idx: ei, pt_len: blen16, n_tok: nt as u16, _pad: 0 }
                    };
                    slots[si] = new_slot;
                    n_entries += 1;
                    if n_entries * 2 > slots.len() { grow(&mut slots, &mut mask); }
                    break;
                }
                if slot.fp == fp && slot.pt_len == blen16 && slot.prefix8 == packed {
                    let verified = blen <= 8 || {
                        let p = unsafe { entries.get_unchecked(slot.entry_idx as usize).0 };
                        let tail = unsafe { std::slice::from_raw_parts(p.add(8), blen - 8) };
                        tail == &bytes[8..]
                    };
                    if verified {
                        let n = slot.n_tok as usize;
                        if n == 1 {
                            unsafe { *out_base.add(out_len) = TokenId(slot.tok_or_idx); }
                            out_len += 1;
                        } else {
                            let s = slot.tok_or_idx as usize;
                            unsafe {
                                std::ptr::copy_nonoverlapping(
                                    tok_store.as_ptr().add(s), out_base.add(out_len), n,
                                );
                            }
                            out_len += n;
                        }
                        break;
                    }
                }
                si = (si + 1) & mask;
            }
        }
        boundaries.push(out_len);
    }
    unsafe { output.set_len(out_len); }

    (output, boundaries)
}

/// Encode all lines without materializing the output buffer.
/// Returns (result_ids, tok_store, n_tokens_per_result, line_pt_ends).
/// The consumer can iterate: for each result_id, look up n_tokens and tok_store offset.
/// This avoids the ~40% of time spent writing the output buffer.
pub fn encode_lines_lazy(lines: &[&[u8]], tokenizer: &Tokenizer) -> usize {
    let merges = &tokenizer.merges;
    let remap = tokenizer.byte_remapping.as_ref().map(|br| br.mapping.as_slice());
    let b2t: [TokenId; 256] = {
        let mut t = [TokenId(0); 256];
        for i in 0..256 {
            t[i] = TokenId(match remap { Some(r) => r[i] as u32, None => i as u32 });
        }
        t
    };

    let total_bytes: usize = lines.iter().map(|l| l.len()).sum();
    let est_unique = (total_bytes / 300).max(4096);
    let cap = (est_unique * 2).next_power_of_two().max(4096);
    let mut mask = cap - 1;

    let mut slots: Vec<Slot> = vec![Slot::EMPTY; cap];
    let mut entries: Vec<EntryPtr> = Vec::with_capacity(est_unique / 8);
    let mut tok_store: Vec<TokenId> = Vec::with_capacity(est_unique * 2);
    let mut scratch: Vec<TokenId> = Vec::with_capacity(128);
    let mut n_entries = 0usize;
    let mut total_tokens = 0usize;

    for &line in lines {
        let mut pt = FastPretokenizer::new(line);
        while let Some((pretoken, packed)) = pt.next_with_pack8() {
            let bytes = pretoken.0;
            let blen = bytes.len();

            if blen == 1 {
                total_tokens += 1;
                continue;
            }

            let blen16 = blen as u16;
            let fp = if blen <= 8 {
                prefix_hash(packed, blen16)
            } else {
                full_hash(bytes)
            };

            let mut si = fp as usize & mask;

            loop {
                let slot = unsafe { slots.get_unchecked(si) };
                if slot.fp == 0 {
                    scratch.clear();
                    for &b in bytes { scratch.push(b2t[b as usize]); }
                    bpe_merge(&mut scratch, merges);
                    let nt = scratch.len();
                    total_tokens += nt;

                    let ei = entries.len() as u32;
                    if blen > 8 { entries.push(EntryPtr(bytes.as_ptr())); }

                    let new_slot = if nt == 1 {
                        Slot { prefix8: packed, fp, tok_or_idx: scratch[0].0, entry_idx: ei, pt_len: blen16, n_tok: 1, _pad: 0 }
                    } else {
                        let ts = tok_store.len() as u32;
                        tok_store.extend_from_slice(&scratch);
                        Slot { prefix8: packed, fp, tok_or_idx: ts, entry_idx: ei, pt_len: blen16, n_tok: nt as u16, _pad: 0 }
                    };
                    slots[si] = new_slot;
                    n_entries += 1;
                    if n_entries * 2 > slots.len() { grow(&mut slots, &mut mask); }
                    break;
                }
                if slot.fp == fp && slot.pt_len == blen16 && slot.prefix8 == packed {
                    let verified = blen <= 8 || {
                        let p = unsafe { entries.get_unchecked(slot.entry_idx as usize).0 };
                        let tail = unsafe { std::slice::from_raw_parts(p.add(8), blen - 8) };
                        tail == &bytes[8..]
                    };
                    if verified {
                        total_tokens += slot.n_tok as usize;
                        break;
                    }
                }
                si = (si + 1) & mask;
            }
        }
    }

    total_tokens
}

fn grow(slots: &mut Vec<Slot>, mask: &mut usize) {
    let new_cap = slots.len() * 2;
    let new_mask = new_cap - 1;
    let mut new_slots = vec![Slot::EMPTY; new_cap];
    for &s in slots.iter() {
        if s.fp == 0 { continue; }
        let mut i = s.fp as usize & new_mask;
        while new_slots[i].fp != 0 { i = (i + 1) & new_mask; }
        new_slots[i] = s;
    }
    *slots = new_slots;
    *mask = new_mask;
}

#[inline]
fn bpe_merge(symbols: &mut Vec<TokenId>, merges: &HashMap<(TokenId, TokenId), TokenId, FxBuildHasher>) {
    let mut len = symbols.len();
    if len < 2 { return; }
    loop {
        let mut best_rank = u32::MAX;
        let mut best_pos = 0;
        for i in 0..len - 1 {
            if let Some(&m) = merges.get(&(symbols[i], symbols[i + 1])) {
                if m.0 < best_rank { best_rank = m.0; best_pos = i; }
            }
        }
        if best_rank == u32::MAX { break; }
        symbols[best_pos] = TokenId(best_rank);
        symbols.remove(best_pos + 1);
        len -= 1;
    }
}
