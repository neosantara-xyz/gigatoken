#!/usr/bin/env python3
"""Interleaved A/B for the encode_doc MT bench (round-format output).

Same methodology as ab_bench.py: fresh process per sample, round-robin
interleave, token-identity gate across all variants, load-average guard.
Usage:
  mt_ab.py --variant main=path --variant new=path [--samples 7]
           [--encode-mb 10000] [--tokenizer-json PATH] [--json-out f.json]
"""

import argparse
import json
import os
import re
import statistics
import subprocess
import sys

ROUND_RE = re.compile(r"round 0: (\d+) tokens in ([0-9.]+)s — (\d+) MB/s")


def load_ok(max_per_cpu=0.25):
    load1 = os.getloadavg()[0]
    return load1 / (os.cpu_count() or 1) < max_per_cpu


def run_sample(binary, encode_mb, tokenizer_json):
    env = dict(os.environ)
    env["ENCODE_MB"] = str(encode_mb)
    if tokenizer_json:
        env["TOKENIZER_JSON"] = tokenizer_json
    proc = subprocess.run([binary], env=env, capture_output=True, text=True)
    m = ROUND_RE.search(proc.stderr)
    if not m:
        sys.exit(f"no round line from {binary}:\n{proc.stderr[-2000:]}")
    tokens, secs, _mbps = int(m.group(1)), float(m.group(2)), int(m.group(3))
    return tokens, encode_mb * 1e6 / secs / 1e6  # recompute MB/s at full precision


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--variant", action="append", required=True)
    ap.add_argument("--samples", type=int, default=7)
    ap.add_argument("--encode-mb", type=int, default=10000)
    ap.add_argument("--tokenizer-json", default=None)
    ap.add_argument("--json-out", default=None)
    args = ap.parse_args()

    variants = [v.split("=", 1) for v in args.variant]
    if not load_ok():
        sys.exit("machine under load; refusing to measure")

    results = {name: [] for name, _ in variants}
    tokens_seen = set()
    for rnd in range(args.samples):
        for name, path in variants:
            if not load_ok(0.6):
                sys.exit("load spiked mid-run; aborting (partial data unsaved)")
            tokens, mbps = run_sample(path, args.encode_mb, args.tokenizer_json)
            tokens_seen.add(tokens)
            results[name].append(mbps)
            print(f"round {rnd+1}/{args.samples}  {name:12s} {mbps:8.1f} MB/s"
                  f"  ({tokens} tokens)", flush=True)
    if len(tokens_seen) != 1:
        sys.exit(f"TOKEN MISMATCH across variants: {tokens_seen}")

    print(f"\ntokens: {tokens_seen.pop()}  (identical across all variants)")
    print(f"{'variant':16s}{'min':>9s}{'median':>9s}{'mean':>9s}{'stdev':>8s}")
    for name, _ in variants:
        r = results[name]
        print(f"{name:16s}{min(r):9.1f}{statistics.median(r):9.1f}"
              f"{statistics.mean(r):9.1f}{statistics.stdev(r):8.1f}")

    base_name, _ = variants[0]
    base = results[base_name]
    print(f"\npairwise (mean ratio, Welch t vs {base_name}):")
    for name, _ in variants[1:]:
        r = results[name]
        ratio = statistics.mean(r) / statistics.mean(base)
        se = (statistics.variance(r) / len(r)
              + statistics.variance(base) / len(base)) ** 0.5
        t = (statistics.mean(r) - statistics.mean(base)) / se if se else 0.0
        print(f"  {name} vs {base_name}: {ratio:.4f}x  (t={t:+.2f})")

    if args.json_out:
        json.dump(results, open(args.json_out, "w"), indent=1)
        print(f"raw samples -> {args.json_out}")


if __name__ == "__main__":
    main()
