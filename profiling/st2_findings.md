
## Round 3 (July 17, 2026 session) — merged onto CURRENT main (post-parquet/hf-cache)

Base recovery: st2/combined (rounds 1-2) merged cleanly onto current main as
`st2-r3`; recovered value re-measured 1.0678x..1.0704x vs current main.

New per-phase profiling infra (phase sidecars from the bench, per-phase PMU,
project rollup — see profiling-quality branch) aimed this round; PMU worked
again this session (cold: Useful 0.53, Discarded 0.21, Processing 0.22).

Deliveries (all identity-exact, tests green; 7-sample idle interleaved at 10GB):
- **R3c PRETOKEN_CHUNK 256→512: +1.07% (t=+2.2) — MERGED.**
  CHUNK=128 −4.5%; prefetch D=8 −0.6%; D=16 is the static max (slack assert).
- **R3d miss-path: n=2/3 fast paths + wave-2 rank prefetch: +0.83% (t=+1.7)
  — MERGED** (composes with CH512; combined +1.7% over base).
  Two definitive negatives documented in-tree: local-minima parallel merge is
  NOT order-equivalent on GPT-2 (counterexamples "ooot", "omma" — a lower
  merge two positions away undercuts an existing local minimum); neighbor-rank
  value speculation +25..140% ns/miss (post-R2α a chain level is only ~6ns).
- **R3a walker phase-A 2-batch ILP: 0.9911x — REJECTED** (branch kept).
  Fifth restructure loss; asm confirmed adjacent branch-free chains, price
  (q-spills, +18 vec ops unconditional movemasks, pair-discard branch) beat
  the overlap. The phase-A chain-latency thesis dies here: even pure chain
  interleaving with verbatim per-batch semantics loses.
- **R3b long-miss (32-lane mid-NEON class + heap rank_of mirror): 0.9975x —
  REJECTED as neutral** (branch kept; target was only ~2.8% of encode).
  Census: 84% of long misses are ≤24 symbols; heap tail owns 29% of rank
  calls via pop re-probes.

Final (7-sample idle interleaved, token-identical 2,279,617,884):
| binary | ST 10GB mean | vs main |
|---|---|---|
| main (current) | 1062.9 MB/s | — |
| st2-r3 base (rounds 1-2 recovered) | 1137.8 | 1.0704x (t=+13.5) |
| **r3final (+CH512+R3d)** | **1157.1** | **1.0886x (t=+19.1)** |

MT encode_doc (1 doc, 16 threads, 10GB, fresh pool, 7-sample interleaved):
- GPT-2: main 8984 → r3final 9568 MB/s = **1.0650x (t=+15.4)**
- Olmo3: main 7913 → r3final 8414 MB/s = **1.0633x (t=+21.6)**

Process note: a contended screen (user jobs running) inverted three verdicts
vs the idle rerun (r3a/r3b/D8 looked like wins, CH512 at +4.4%); only the
idle 7-sample run is the record. Load-guard every measurement.
