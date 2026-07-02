//! Fast scalar pretokenizer for the GPT-2 (r50k_base) regex:
//! `'(?:[sdmt]|ll|ve|re)| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+`
//!
//! Uses SWAR (u64) for letter runs + arithmetic predicates. The hot path
//! (space + letters / bare letters) is fully inlined in `advance_pos`.
//!
//! `advance_pos` is a pure free function (`(bytes, pos) -> end`) rather than
//! a `&mut self` method: keeping the cursor in a register instead of writing
//! `self.pos` at every scan step shortens the per-token dependency chain and
//! is worth ~30% throughput. Non-ASCII characters are classified with one
//! packed-table load (`unicode::class_of`) on a hand-rolled UTF-8 decode.
//!
//! A windowed multi-cursor variant (finding guaranteed boundaries and running
//! 2-4 independent `advance_pos` chains interleaved, queueing token ends) was
//! benchmarked at 0.80-0.95x of this streaming version: the queue traffic and
//! interleaved branch history cost more than the extra ILP recovers.

use super::{
    decode_cp, is_ascii_ws, is_digit, is_letter, scan_digits_from, scan_letters_from,
    scan_other_from,
};
use crate::pretokenize::unicode::{self, CharClass};
use crate::pretokenize::Pretoken;

// -----------------------------------------------------------------------
// FastR50kPretokenizer
// -----------------------------------------------------------------------

pub struct FastR50kPretokenizer<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> FastR50kPretokenizer<'a> {
    #[inline]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    /// Resume iteration at a byte offset previously returned by [`Self::pos`].
    /// Used by the Python bindings, which re-borrow the underlying buffer on
    /// every `__next__` call.
    #[inline]
    pub fn with_pos(bytes: &'a [u8], pos: usize) -> Self {
        Self { bytes, pos }
    }

    /// Current position as a byte offset into the input.
    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }
}

impl<'a> Iterator for FastR50kPretokenizer<'a> {
    type Item = Pretoken<'a>;

    #[inline]
    fn next(&mut self) -> Option<Pretoken<'a>> {
        let start = self.pos;
        if start >= self.bytes.len() {
            return None;
        }
        let end = advance_pos(self.bytes, start);
        self.pos = end;
        Some(Pretoken(&self.bytes[start..end]))
    }
}

/// Advance past one token starting at `start`; returns the token's end.
/// `start` must be < `bytes.len()` and a valid token start.
/// Uses direct comparison chains instead of LUT + jump table to avoid
/// GOT indirection and improve branch prediction on common patterns.
///
/// Byte loads here look redundant but are effectively free: their addresses
/// depend only on `start`, so they issue in parallel under speculation. A
/// variant that did one u64 load and extracted bytes/scanned letters
/// in-register measured 0.84x — the shifts serialize after the load, while
/// independent L1 loads don't.
#[inline(always)]
fn advance_pos(bytes: &[u8], start: usize) -> usize {
    let len = bytes.len();
    let b0 = unsafe { *bytes.get_unchecked(start) };

    // Bare ASCII letter start (~5% of OWT tokens; most words carry a space)
    if is_letter(b0) {
        return scan_letters_from(bytes, start + 1);
    }

    // Hot path: space before content (~78% of tokens, ~75% space+letters)
    if b0 == b' ' {
        if start + 1 < len {
            let b1 = unsafe { *bytes.get_unchecked(start + 1) };
            if is_letter(b1) {
                return scan_letters_from(bytes, start + 2);
            }
            if is_digit(b1) {
                return scan_digits_from(bytes, start + 2);
            }
            if b1 >= 0x80 {
                let (cp, l) = unsafe { decode_cp(bytes, start + 1) };
                let p = start + 1 + l;
                return match unicode::class_of(cp) {
                    CharClass::Letter => scan_letters_from(bytes, p),
                    CharClass::Number => scan_digits_from(bytes, p),
                    CharClass::Whitespace => advance_ws(bytes, p, start),
                    CharClass::Other => scan_other_from(bytes, p),
                };
            }
            if is_ascii_ws(b1) {
                return advance_ws(bytes, start + 1, start);
            }
            return scan_other_from(bytes, start + 2);
        }
        return start + 1;
    }

    // Non-ASCII
    if b0 >= 0x80 {
        let (cp, l) = unsafe { decode_cp(bytes, start) };
        let p = start + l;
        return match unicode::class_of(cp) {
            CharClass::Letter => scan_letters_from(bytes, p),
            CharClass::Number => scan_digits_from(bytes, p),
            CharClass::Whitespace => advance_ws(bytes, p, start),
            CharClass::Other => scan_other_from(bytes, p),
        };
    }

    // Digit
    if is_digit(b0) {
        return scan_digits_from(bytes, start + 1);
    }

    // Apostrophe / contraction
    if b0 == b'\'' {
        match bytes.get(start + 1) {
            Some(b's' | b'd' | b'm' | b't') => return start + 2,
            Some(b'l') if bytes.get(start + 2) == Some(&b'l') => return start + 3,
            Some(b'v') if bytes.get(start + 2) == Some(&b'e') => return start + 3,
            Some(b'r') if bytes.get(start + 2) == Some(&b'e') => return start + 3,
            _ => return scan_other_from(bytes, start + 1),
        }
    }

    // Whitespace (tab, newline, etc.)
    if b0.wrapping_sub(9) < 5 {
        return advance_ws(bytes, start + 1, start);
    }

    // Other (punctuation, symbols)
    scan_other_from(bytes, start + 1)
}

/// Advance through whitespace. `scan_pos` is where to continue scanning,
/// `token_start` is where the token began (for the split-off-last-char logic).
#[inline(always)]
fn advance_ws(bytes: &[u8], scan_pos: usize, token_start: usize) -> usize {
    let len = bytes.len();
    let mut p = scan_pos;
    while p < len {
        let b = unsafe { *bytes.get_unchecked(p) };
        if is_ascii_ws(b) {
            p += 1;
        } else if b >= 0x80 {
            let (cp, l) = unsafe { decode_cp(bytes, p) };
            if unicode::class_of(cp) == CharClass::Whitespace {
                p += l;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    if p < len {
        let ws_bytes = p - token_start;
        if ws_bytes >= 2 {
            let mut last = p - 1;
            while last > token_start && unsafe { *bytes.get_unchecked(last) } & 0xC0 == 0x80 {
                last -= 1;
            }
            if last > token_start {
                return last;
            }
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_matches_state_machine_owt() {
        let data_dir = std::env::home_dir().unwrap().join("data");
        let all_bytes = std::fs::read(data_dir.join("owt_train.txt"))
            .expect("Could not read ~/data/owt_train.txt");
        let max = 5_000_000.min(all_bytes.len());
        let mut end = max;
        while end > 0 && std::str::from_utf8(&all_bytes[..end]).is_err() {
            end -= 1;
        }
        let input = &all_bytes[..end];

        let mut sm = crate::pretokenize::PretokenizerIter::new(input);
        let mut fast = FastR50kPretokenizer::new(input);
        let mut idx = 0usize;

        loop {
            match (sm.next(), fast.next()) {
                (Some(a), Some(b)) => {
                    assert_eq!(
                        a.0, b.0,
                        "Mismatch at token {idx}: sm={:?} fast={:?}",
                        String::from_utf8_lossy(a.0),
                        String::from_utf8_lossy(b.0),
                    );
                }
                (None, None) => break,
                (Some(a), None) => panic!("SM extra at {idx}: {:?}", String::from_utf8_lossy(a.0)),
                (None, Some(b)) => {
                    panic!("Fast extra at {idx}: {:?}", String::from_utf8_lossy(b.0))
                }
            }
            idx += 1;
        }
        eprintln!("All {idx} tokens match.");
    }

    fn load_owt(max_bytes: usize) -> Vec<u8> {
        let data_dir = std::env::home_dir().unwrap().join("data");
        let all_bytes = std::fs::read(data_dir.join("owt_train.txt"))
            .expect("Could not read ~/data/owt_train.txt");
        let mut end = max_bytes.min(all_bytes.len());
        while end > 0 && std::str::from_utf8(&all_bytes[..end]).is_err() {
            end -= 1;
        }
        all_bytes[..end].to_vec()
    }

    #[test]
    #[ignore]
    fn r50k_token_stats_owt() {
        let input = load_owt(10_000_000);
        let mut counts = [0usize; 8]; // letter, sp+letter, sp+digit, sp+other, digit, ws, apos, other/nonascii
        let mut letter_tokens = 0usize;
        let mut in_window = 0usize;
        let mut total = 0usize;
        let mut pos = 0usize;
        while pos < input.len() {
            let end = advance_pos(&input, pos);
            let b0 = input[pos];
            let idx = if is_letter(b0) {
                0
            } else if b0 == b' ' {
                match input.get(pos + 1) {
                    Some(&b) if is_letter(b) => 1,
                    Some(&b) if is_digit(b) => 2,
                    _ => 3,
                }
            } else if is_digit(b0) {
                4
            } else if is_ascii_ws(b0) {
                5
            } else if b0 == b'\'' {
                6
            } else {
                7
            };
            counts[idx] += 1;
            if idx == 0 || idx == 1 {
                letter_tokens += 1;
                if end - pos <= 8 {
                    in_window += 1;
                }
            }
            total += 1;
            pos = end;
        }
        let pct = |n: usize| 100.0 * n as f64 / total as f64;
        eprintln!("total tokens: {total}");
        for (name, n) in [
            "letter", "sp+letter", "sp+digit", "sp+other", "digit", "ws", "apos", "other/hi",
        ]
        .iter()
        .zip(counts)
        {
            eprintln!("{name:>10}: {n:>9} ({:.1}%)", pct(n));
        }
        eprintln!(
            "letter tokens resolved in 8-byte window: {:.1}%",
            100.0 * in_window as f64 / letter_tokens as f64
        );
        eprintln!(
            "avg token len: {:.2} bytes",
            input.len() as f64 / total as f64
        );
    }

    /// Best-of-5 throughput of `advance_pos` over `bytes`, in MB/s.
    fn measure(bytes: &[u8], label: &str) -> f64 {
        let mb = bytes.len() as f64 / 1e6;
        let (n, _) = drive(bytes, advance_pos);
        let mut best = f64::INFINITY;
        for _ in 0..5 {
            let t = std::time::Instant::now();
            std::hint::black_box(drive(bytes, advance_pos));
            best = best.min(t.elapsed().as_secs_f64());
        }
        let mbs = mb / best;
        // cycles/token estimate assumes ~4.5 GHz P-core
        let cpt = 4.5e9 * best / n as f64;
        eprintln!(
            "{label:>22}: {mbs:>5.0} MB/s | {:.2} B/token | ~{cpt:.1} cy/token",
            bytes.len() as f64 / n as f64
        );
        mbs
    }

    #[test]
    #[ignore]
    fn r50k_prediction_floor() {
        // Pure single-path floor: one token shape, perfectly predictable.
        let hello: Vec<u8> = b" hello".repeat(16_000_000 / 6);
        measure(&hello, "' hello' x N");

        // Same multiset of real OWT tokens, predictable vs shuffled order.
        // Space-prefixed tokens are adjacency-safe: concatenating them in any
        // order re-tokenizes to exactly the same tokens.
        let input = load_owt(100_000_000);
        let bag: Vec<&[u8]> = FastR50kPretokenizer::new(&input)
            .map(|t| t.0)
            .filter(|t| t[0] == b' ' && t.len() > 1)
            .collect();
        eprintln!("bag: {} space-prefixed tokens", bag.len());

        let mut sorted = bag.clone();
        sorted.sort_unstable();
        let predictable: Vec<u8> = sorted.concat();
        drop(sorted);

        let mut shuffled = bag;
        let mut state = 0x9E3779B97F4A7C15u64;
        let mut next = || {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            state.wrapping_mul(0x2545F4914F6CDD1D)
        };
        for i in (1..shuffled.len()).rev() {
            let j = (next() % (i as u64 + 1)) as usize;
            shuffled.swap(i, j);
        }
        let unpredictable: Vec<u8> = shuffled.concat();
        let n_bag = shuffled.len();
        drop(shuffled);

        assert_eq!(drive(&predictable, advance_pos).0, n_bag);
        assert_eq!(drive(&unpredictable, advance_pos).0, n_bag);

        measure(&predictable, "sorted (predictable)");
        measure(&unpredictable, "shuffled (same bag)");
        measure(&input, "natural OWT");
    }

    fn drive(bytes: &[u8], f: impl Fn(&[u8], usize) -> usize) -> (usize, u64) {
        let mut pos = 0usize;
        let mut n = 0usize;
        let mut acc = 0u64;
        while pos < bytes.len() {
            let end = f(bytes, pos);
            acc = acc.wrapping_add(end as u64);
            n += 1;
            pos = end;
        }
        (n, acc)
    }

    #[test]
    #[ignore]
    fn aa_r50k_advance_interleaved() {
        // A/A control: same implementation through two separately
        // monomorphized drivers, to gauge code-layout noise in this harness.
        let input = load_owt(100_000_000);
        let mb = input.len() as f64 / 1e6;
        let copy_a = advance_pos;
        let copy_b = |b: &[u8], s: usize| advance_pos(b, s);
        std::hint::black_box(drive(&input, copy_a));
        std::hint::black_box(drive(&input, copy_b));
        let (mut best_a, mut best_b) = (f64::INFINITY, f64::INFINITY);
        for round in 0..7 {
            let t = std::time::Instant::now();
            std::hint::black_box(drive(&input, copy_a));
            let da = t.elapsed().as_secs_f64();
            let t = std::time::Instant::now();
            std::hint::black_box(drive(&input, copy_b));
            let db = t.elapsed().as_secs_f64();
            best_a = best_a.min(da);
            best_b = best_b.min(db);
            eprintln!("round {round}: A {:.0} MB/s | A' {:.0} MB/s", mb / da, mb / db);
        }
        eprintln!(
            "best: A {:.0} MB/s | A' {:.0} MB/s | ratio {:.3}x",
            mb / best_a,
            mb / best_b,
            best_a / best_b
        );
    }

    /// Interleaved same-binary A/B harness (min-of-7): swap an experimental
    /// `advance_pos` candidate in below and run with `--ignored --nocapture`.
    /// Run `aa_r50k_advance_interleaved` first as the layout-noise control.
    /// (Isolated A/B of the packed class table vs the old chained ICU
    /// predicates measured 0.90x for the ICU version, i.e. the table alone
    /// is worth ~1.11x.)
    #[test]
    #[ignore]
    fn ab_r50k_advance_interleaved() {
        let input = load_owt(100_000_000);
        let mb = input.len() as f64 / 1e6;
        eprintln!("input: {mb:.1} MB");

        // Experimental candidate under test; placeholder = shipped impl.
        let experimental = |b: &[u8], s: usize| advance_pos(b, s);

        // Warmup + equivalence check
        let (n_a, acc_a) = drive(&input, advance_pos);
        let (n_b, acc_b) = drive(&input, experimental);
        assert_eq!((n_a, acc_a), (n_b, acc_b), "variants disagree");

        let mut best_a = f64::INFINITY;
        let mut best_b = f64::INFINITY;
        for round in 0..7 {
            let t = std::time::Instant::now();
            let r = drive(&input, advance_pos);
            let da = t.elapsed().as_secs_f64();
            std::hint::black_box(r);

            let t = std::time::Instant::now();
            let r = drive(&input, experimental);
            let db = t.elapsed().as_secs_f64();
            std::hint::black_box(r);

            best_a = best_a.min(da);
            best_b = best_b.min(db);
            eprintln!(
                "round {round}: shipped {:.0} MB/s | experimental {:.0} MB/s",
                mb / da,
                mb / db
            );
        }
        eprintln!(
            "best: shipped {:.0} MB/s | experimental {:.0} MB/s | ratio {:.3}x",
            mb / best_a,
            mb / best_b,
            best_a / best_b
        );
    }
}
