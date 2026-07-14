# Cold-encode optimization campaign — gigatoken BPE encode, rounds 1–5

Five rounds of profile-driven optimization of the cold encode path
(single-threaded `encode_st` and 16-thread `encode_docs_ragged`), run as a
multi-agent campaign: analysis agents propose from traces, implementation
agents build each technique on its own branch, and a central harness measures
everything sequentially. Workload throughout: one cold 10 GB OWT pass per
process, GPT-2 tokenizer unless noted, Apple M4 Max (12P+4E), corpus in page
cache. Every variant in this report produced **bit-identical token counts**
to the reference; nothing was merged (or even measured) without passing the
identity gates in §2.

## 1. Executive summary

Final three-way showdown (same session, interleaved sequential A/B, fresh
process per sample):

| variant | MT mean (best) | ST mean | ST output mode |
|---|---|---|---|
| `encode-opt-main` (this campaign) | **7797 MB/s** (8021) | **1023 MB/s** | **materializing** all ~2.28 G tokens (~9.1 GB) into a flat buffer |
| PR #4 head | 5195 MB/s | 940 MB/s | counting only |
| original `encode-perf` baseline | 2826 MB/s | 605 MB/s | counting only |

- MT cold encode: **+50% over PR #4, +176% over the `encode-perf` baseline.**
- ST: the mainline *materializes* every token faster than the predecessors
  merely *count* them. Like-for-like materializing comparisons measured
  earlier put PR #4 at ~601–767 MB/s vs the mainline's 1023.
- Generalization beyond GPT-2 (MT, mainline vs PR #4): olmo3 7269 vs
  4921 MB/s (+48%); qwen2 6045 vs 4226 MB/s (+43%).
- 10 techniques merged across 4 completed rounds; 5 measured and rejected
  (kept on their branches with the numbers that killed them). Round 5
  (walker rethink, MT round 5) is in flight.
- A five-lens adversarial review of the full diff found **zero critical
  correctness bugs in shipped paths**; the confirmed findings (one API
  soundness regression, one BE-portability gap, one added-token edge case)
  were fixed in `1ffff3c` with hot-path codegen verified bit-identical.

## 2. Methodology

The campaign's rules, all of which earned their keep:

- **One technique, one branch, one worktree.** Every candidate lives on
  `opt/<name>` and was developed in an isolated worktree. Merges into
  `encode-opt-main` happen only after measurement; rejected branches are
  kept, not deleted — the negative results are part of the record.
- **Strictly sequential, interleaved A/B.** This bench varies ±8%
  run-to-run, so nothing runs in parallel, ever, and A/B comparisons
  interleave variant and control samples within one session (min-of-N /
  means over interleaved samples). One cold pass per process — no warm
  reuse inside a process.
- **Token-identity gates before any measurement.** Exact token counts at
  100 MB (`encode_st` 22,834,020; `encode_doc` 22,723,342) plus a 50 MB
  token-by-token differential against the reference encoder. A variant that
  fails identity is a bug, not a candidate.
- **Central measurement only.** Implementation agents were forbidden from
  benchmarking; all numbers come from one sequential harness on an
  otherwise-idle machine. This removes both machine contention and
  motivated measurement.
- **Profile → propose → build → measure → merge/reject** per round, with the
  next round's profile taken on the new mainline.

## 3. Profiling infrastructure

Documented in detail in `profiling/report.md`; the short version:

- **Parity bench profile**: `[profile.bench]` inherits `release` (fat LTO,
  identical codegen) + `debug = true`, packed dSYM. Verified 1.6% delta vs
  plain release on identical full passes — inside the noise band.
- **samply 0.13.1 at 4 kHz**, `--save-only` (unsymbolicated) with offline
  symbolication: `atos` against the dSYM with `-i` expands **inline frames
  with exact file:line at each sampled address** — the thing that makes
  attribution trustworthy under fat LTO + `#[inline(always)]`. System
  dylibs resolved from the presymbolicate sidecar. `analyze.py` produces
  phase/bucket splits, top-functions, hot lines, and flamegraph-format
  collapsed stacks; `analyze_mt.py` adds per-thread accounting keyed on
  `threadCPUDelta` (authoritative when 17 threads overload the sampler).
- **xctrace CPU Counters (PMU)**, "CPU Bottlenecks" mode, exported via
  XPath with id/ref decompression and duration weighting (`pmu_summary.py`)
  — unweighted export over-counts rare rows.
- Sanity checks each round: profile-total vs wall (99.5%), bucket
  reproducibility across independent traces (within ~1 pt), tops matching
  code expectations.

Traces and per-round analyses live under `profiling/traces/*_analysis/` and
`profiling/mt*_analysis/`; MT round findings in
`profiling/mt_round3_findings.md` and `profiling/mt4_analysis/mt_round4_findings.md`.

## 4. Round by round

All per-round percentages are interleaved A/B wins **against that round's
control** (the then-current `encode-opt-main`), MT unless marked ST.

### Round 1 — from the baseline ST profile

Baseline profile (`profiling/report.md`): 10.5 s encode, walker 49.4%,
miss-path merge 15.4%, driver 14.6%, cache probe 14.2%, key pack 5.6%;
PMU verdict: **25.1% of P-core issue bandwidth discarded to bad
speculation** (§5). Five proposals (`scratchpad/proposals/*.md`), five
branches:

| branch | verdict |
|---|---|
| `opt/probe-emit` — branchless probe/emit, 4-token inline entries, flat-buffer output, staged prefetch | **merged, +6.2% MT / +27.6% ST-materializing** |
| `opt/walker-twophase` — two-phase branchless span walker | **merged, +3.4% MT** |
| `opt/keypack` — arithmetic pack masks, buffered keyed fill, CRC32 key hash | **merged, +1.7%** |
| `opt/miss-path` — PairRankTable, vocab-seeded short cache, stack/NEON short merge | **merged, +1.3%** |
| `opt/mt-alloc` — pre-sized worker caches + streaming gather + LPT + Arc-shared tables | **−5.9%, rejected as a unit** |

The mt-alloc regression was bisected with env-var kill switches
(`1aa6210`): the streaming-gather committer thread was the culprit; the
other three components were salvaged as `opt/mt-salvage` (pre-size from
batch share, Arc-share merges/vocab/vocab_inv across forks, LPT descending
chunk order) and merged. probe-emit and miss-path were semantically
unified at merge (`160bdd0`).

### Round 2

- `opt/emit2` — register-resident cursors, parallel pair loads,
  value-select probe: **merged, +5.2% MT / +7.4% ST**.
- `opt/phaseb` — pointer-indexed emission loops, careful-pack suffix
  split, popcount-idiom dodge in `flatten_bits`: **−1.5%, rejected.**
- `opt/rank2` — packed rank slots, narrow NEON scan, fingerprint long map:
  **±0 standalone, held out.** Partially re-taken in round 4 when the
  round-3 profile showed `PairRankTable::rank` had grown in relative share.

### Round 3 — first MT trace

`mt_round3_findings.md` decomposed the 1462 ms MT window: 71.9% steady-state
encode, then three orchestration losses — 163 ms gather copy, **114 ms
serial free** of ~9.1 GB of chunk buffers on the main thread, and a
**104 ms straggler tail** caused by rayon's range splitting turning the LPT
big→small chunk order into a hint (a 78 MB head chunk observed *starting*
after other threads reached the tail).

- `opt/mt3` — fused parallel copy+drop gather, atomic-counter strict
  in-order LPT handout: **merged, +6.2% MT.** Round-4 trace confirmed both
  predictions: straggler spread 104 → 23 ms, gather+free 277 → 214 ms.
- `opt/walker3` — pinned `movemask64` addp tree as asm, table-based
  branchless phase-B key pack: **merged, +2.1% MT / +6.2% ST.**
- `opt/emit3` — two variants (L1 staging buffer + memcpy flush; direct
  stores + `prfm pstl1keep`): **−4..−7% both, rejected.** The existing
  emit loop's stores were already not the bottleneck; adding a staging hop
  or extra prefetch traffic only cost.

### Round 4 — the gather convoy

`mt_round4_findings.md` on the round-3 mainline (1336 ms window): the fused
gather ran at only **7.2/16 threads busy** — a classic vm-map
reader/writer convoy. First-touch zero-fill faults on the 9.1 GB flat
buffer take the vm-map read lock; the interleaved chunk-buffer munmaps
take the **write** lock (~0.7 ms × 307), each stalling every concurrently
faulting thread. 205 ms of actual teardown CPU was costing ~1.4 s of
blocked thread-time.

- `opt/mt4` — defer chunk-buffer teardown to a detached background task
  after `assemble_ragged` returns; Heaps-law worker table sizing
  (OWT-calibrated distinct(n) ≈ 3.45·n^0.62 ⇒ 2^21 slots instead of 2^22
  for a 10 GB/16 share, halving 2 GB of fork memset): **merged, +3.4% MT.**
- `opt/driver4` — AoS `SpanBatch` (one 32 B entry per pretoken instead of
  parallel arrays), unclamped prefetch-slack: **merged, +0.7% MT /
  +1.9% ST.**
- `opt/miss4` — packed rank slots, narrow NEON scan, memcpy emits, fused
  miss→insert slot (the surviving parts of rank2 plus new work):
  **merged, +0.9% ST.**
- `opt/walker4` — dual-nibble TBL classify, cmtst asm pin, ctpop-idiom
  dodge: **±0, rejected despite cutting 113 → 88 instructions per 64 B
  batch.** The walker is not retiring-limited (§5); fewer instructions on
  the same dependency/mispredict structure bought nothing. This is the
  round's most instructive negative result.

### Round 5 — in flight

The round-5 ST trace (`traces/samply_round5_analysis/`) shows the walker
bucket at 40.5% of encode (from 49.4% at baseline) with the remaining time
concentrated in `fill_spans_two_phase` and the driver's output stores.
A walker rethink (`opt/walker5`) and MT round 5 (`opt/mt5`) are open;
**results pending** — nothing from round 5 is in the numbers above.

## 5. The microarchitectural story

**ST: the encoder was branch-misprediction-bound, not memory-bound.**
Baseline PMU decomposition (10.4 s encode window, 4.0 GHz, 41.8 G cycles):
54.9% retiring, **25.1% discarded (mispredicted-path)**, 11.4% backend
(dependent loads), 7.4% frontend. The discarded quarter concentrated in
per-pretoken data-dependent branch ladders: the walker's `rem != 0`
bit-walk and segment-refill exits, `pack_pretoken_key`'s length/page
branches with a dependent `PACK_MASK[n]` table load, and the probe/emit
hit/spill/miss triad. The merged fixes attack exactly these:

- **Walker**: two-phase structure — phase A extracts all span boundaries
  of a 64 B block branchlessly into a buffer, phase B consumes them with a
  table-based branchless key pack (walker-twophase, walker3).
- **Key pack / hash**: arithmetic (shift-based) masks replace the
  dependent table load; CRC32 hardware hash (keypack).
- **Probe/emit**: 4-token inline cache entries emitted unconditionally
  into a flat output buffer with branchless length advance; paired probe
  compares; staged prefetch (probe-emit, emit2); the rare spill path
  deferred off the hot loop.
- **Miss path**: hashbrown pair-rank probes replaced by a flat
  `PairRankTable`; vocab-seeded short cache so every short vocab word hits
  on first touch; stack-array + NEON merge for short pretokens (miss-path,
  miss4). This is disproportionately an MT win: 16 cold per-worker caches
  each pay the miss path until warm.

The negative results trace the same boundary from the other side: emit3
(store staging) and walker4 (instruction-count golf) both optimized
dimensions the core had slack in, and phaseb's restructuring perturbed
layout for a net loss. On this core, only removing mispredicts or
shortening dependent-load chains moved the number.

**MT: orchestration, not encode, was the recoverable gap.** The
steady-state encode scales; the campaign's MT wins came from the phase
structure around it, diagnosed one trace at a time: serial-prefix
allocation → pre-size + Arc-share (R1 salvage); scheduler-order LPT
violation → atomic in-order handout (R3); serial 9 GB teardown → fused
parallel copy+drop (R3) → which exposed the vm-map read/write convoy →
deferred teardown off the timed window (R4); oversized worker tables →
Heaps-law sizing (R4). MT-specific overhead beyond steady-state encode
went from ~25% of the round-3 window to a straggler spread of one tail
chunk (23 ms) and a gather that pays copy+faults only.

## 6. Adversarial review round

After round 4, five parallel review agents audited the full diff
(`49ff4cb..ff363be`, ~2.3 k insertions) through five lenses: unsafe/memory,
token identity, concurrency, edge cases + cross-round invariant
interactions, portability/API (reports in `scratchpad/review/*.md`).

**Zero critical correctness bugs in shipped paths.** Confirmed findings,
all fixed in `1ffff3c`:

1. **SpanBatch/PretokenSpans soundness (MAJOR, API boundary)** — safe
   external code could implement the safe `PretokenSpans` trait and drive
   `SpanBatch::span`'s `from_raw_parts` with arbitrary `(ptr, len)`. Fixed:
   `unsafe trait` with a documented fill contract, `BatchEntry` fields and
   `SpanBatch::entries` now `pub(crate)`, per-impl SAFETY notes.
2. **Big-endian compile guard** — key packing and emit-loop token-lane
   stores assume LE; BE targets previously got silent wrong tokens, now a
   `compile_error!`.
3. **`add_special_token` duplicate-content seed divergence** — an added
   token whose content duplicates an existing short vocab entry left the
   parent's seeded cache on the old ID while forks reseeded to the new one.
   Fixed with insert-not-insert-if-absent seed sync + fork-side re-apply;
   regression test added (verified to fail pre-fix).
4. **Proto-mutation-after-fork invariant** documented on `WorkerPool` and
   the three mutators (latent, Rust-API-only).
5. Doc corrections (stale consumer list, `GIGATOK_NO_LPT`, aarch64-linux
   `+crc` note).

Post-fix verification: full release test suite clean (59+58 tests), 100 MB
identity counts exact, and the hot-path functions' generated asm
**bit-identical** to pre-fix — the soundness fixes cost zero cycles.

## 7. What's left

- **The walker plateau.** After two rounds of de-branching the walker is
  still ~40% of ST encode (49.4% → 40.5%), and round-4's
  instruction-golf null result says the remaining cost is structural
  (per-span dependent chain + residual boundary mispredicts), not
  instruction count. Round 5's rethink targets the structure itself;
  pending.
- **MT per-worker steady state.** Encode CPU is ~14.7 s MT vs ~11 s ST for
  the same input: E-core cycles plus 16 cold caches re-deriving the Zipf
  head 16× (Σ per-worker distinct ≈ 16 M vs 5.5 M ST). Inherent to
  sharded caches; a shared warm-head structure is the open idea, with the
  usual coherence-traffic risk.
- **x86-64 / EPYC porting.** The aarch64 asm pins (`movemask64` addp tree,
  walker3) and NEON paths all have portable fallbacks, so x86 builds run —
  but an EPYC target currently gets round-1-era codegen on those paths;
  the AVX2 equivalents are unwritten. The CRC32 key hash needs
  `-C target-feature=+crc` on aarch64-linux (documented). The MT gather
  numbers are macOS-specific (16 KB pages, no THP): Linux THP will change
  the fault/convoy arithmetic and the deferred-drop win should be
  re-measured there.
- **Round 5** (walker rethink, MT round 5) is in flight on `opt/walker5` /
  `opt/mt5`.

## 8. Branch inventory

| branch | disposition | measured delta (vs round control) |
|---|---|---|
| `opt/probe-emit` | merged R1 | +6.2% MT / +27.6% ST-materializing |
| `opt/walker-twophase` | merged R1 | +3.4% MT |
| `opt/keypack` | merged R1 | +1.7% |
| `opt/miss-path` | merged R1 (`160bdd0`) | +1.3% |
| `opt/mt-alloc` | rejected R1 (kept) | −5.9% as a unit; bisected to streaming-gather committer thread |
| `opt/mt-salvage` | merged R1 | salvage of mt-alloc: pre-size + Arc-share + LPT |
| `opt/emit2` | merged R2 | +5.2% MT / +7.4% ST |
| `opt/phaseb` | rejected R2 (kept) | −1.5% |
| `opt/rank2` | held R2 (kept) | ±0 standalone; parts re-taken in miss4 |
| `opt/mt3` | merged R3 | +6.2% MT |
| `opt/walker3` | merged R3 | +2.1% MT / +6.2% ST |
| `opt/emit3` | rejected R3 (kept) | −4..−7% (both variants) |
| `opt/mt4` | merged R4 | +3.4% MT |
| `opt/driver4` | merged R4 | +0.7% MT / +1.9% ST |
| `opt/miss4` | merged R4 | +0.9% ST |
| `opt/walker4` | rejected R4 (kept) | ±0 despite 113→88 inst/batch |
| `opt/walker5`, `opt/mt5` | round 5, in flight | pending |
| review fixes | `1ffff3c` on mainline | hot-path asm bit-identical |
