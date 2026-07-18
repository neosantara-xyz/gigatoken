#!/usr/bin/env python3
"""Summarize an xctrace 'CPU Counters' (CPU Bottlenecks guided mode) trace.

Exports the MetricTable and RemarksByThread tables and aggregates them.
xctrace XML uses ref-compression: any element with id=N may later be
referenced as <tag ref="N"/>; refs must be resolved. Ratio metrics are
duration-weighted; count metrics are summed.

When the bench wrote a <trace>.phases.json sidecar (see profile.sh /
benches/common/mod.rs Phases), metrics are additionally aggregated per
measured phase — the trace's epoch start comes from the TOC's <start-date>,
so the sidecar's epoch-ns boundaries map directly onto row timestamps and
the encode passes are separable from corpus read / tokenizer load without
guessing a --window.

Usage: python3 pmu_summary.py TRACE.trace [--window START_S END_S] [--pcore-only]
"""

import argparse
import json
import os
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from collections import Counter, defaultdict
from datetime import datetime


def export_table(trace, schema, cache_dir):
    out = os.path.join(cache_dir, f"{os.path.basename(trace)}.{schema}.xml")
    if not os.path.exists(out):
        with open(out, "w") as f:
            subprocess.run(
                [
                    "xcrun", "xctrace", "export", "--input", trace, "--xpath",
                    f'/trace-toc/run[@number="1"]/data/table[@schema="{schema}"]',
                ],
                stdout=f,
                check=True,
            )
    return out


def trace_start_epoch_ns(trace, cache_dir):
    """Epoch ns of the recording start, from the trace TOC's <start-date>."""
    out = os.path.join(cache_dir, f"{os.path.basename(trace)}.toc.xml")
    if not os.path.exists(out):
        with open(out, "w") as f:
            subprocess.run(
                ["xcrun", "xctrace", "export", "--input", trace, "--toc"],
                stdout=f,
                check=True,
            )
    m = re.search(r"<start-date>([^<]+)</start-date>", open(out).read())
    if not m:
        return None
    return int(datetime.fromisoformat(m.group(1)).timestamp() * 1e9)


def load_phase_windows(trace, cache_dir):
    """[(start_ns_rel, end_ns_rel, group)] in trace time, from the bench's
    .phases.json sidecar; None when the sidecar or TOC start-date is absent.
    Per-pass encode phases collapse into 'encode cold' (pass 0) and
    'encode warm' (later passes)."""
    sidecar = re.sub(r"\.trace$", ".phases.json", trace)
    if sidecar == trace or not os.path.exists(sidecar):
        return None
    t0 = trace_start_epoch_ns(trace, cache_dir)
    if t0 is None:
        return None
    d = json.load(open(sidecar))
    windows = []
    for p in d["phases"]:
        m = re.match(r"encode pass (\d+)", p["name"])
        group = (
            ("encode cold" if m.group(1) == "0" else "encode warm")
            if m
            else p["name"]
        )
        windows.append((p["start_epoch_ns"] - t0, p["end_epoch_ns"] - t0, group))
    return windows, d.get("meta", {})


class RefResolver:
    """Resolve xctrace export id/ref compression."""

    def __init__(self):
        self.by_id = {}

    def resolve(self, el):
        rid = el.get("ref")
        if rid is not None:
            return self.by_id[rid]
        eid = el.get("id")
        if eid is not None:
            self.by_id[eid] = el
        return el


def parse_metric_table(path, window=None, pcore_only=False, windows=None):
    rr = RefResolver()
    # (metric_name, is_ratio) -> [sum_weighted_value, sum_weight, sum_value]
    agg = defaultdict(lambda: [0.0, 0.0, 0.0])
    # phase group -> same aggregation, rows assigned by midpoint
    agg_by_group = defaultdict(lambda: defaultdict(lambda: [0.0, 0.0, 0.0]))
    core_types = Counter()
    for _ev, el in ET.iterparse(path, events=("end",)):
        if el.tag != "row":
            continue
        kids = list(el)
        # row layout: start-time, duration, string(pmi-event),
        # string(metric-name), thread, process, fixed-decimal, core,
        # boolean(is-ratio), [markdown-text]
        try:
            start = rr.resolve(kids[0])
            dur = rr.resolve(kids[1])
            pmi = rr.resolve(kids[2])
            name_el = rr.resolve(kids[3])
            val_el = rr.resolve(kids[6])
            core_el = rr.resolve(kids[7])
            ratio_el = rr.resolve(kids[8])
            # register any remaining ids (markdown-text etc.)
            for k in kids[9:]:
                rr.resolve(k)
            for sub in el.iter():
                if sub is not el:
                    rr.resolve(sub)
        except (IndexError, KeyError):
            el.clear()
            continue
        t_ns = int(start.text or start.get("fmt", "0").replace(",", "") or 0)
        d_ns = int(dur.text or 0)
        name = name_el.get("fmt") or (name_el.text or "?")
        val = float(val_el.text or 0.0)
        is_ratio = (ratio_el.text or "0") == "1"
        core_fmt = core_el.get("fmt") or ""
        if window and not (window[0] * 1e9 <= t_ns <= window[1] * 1e9):
            el.clear()
            continue
        if pcore_only and "E Core" in core_fmt:
            el.clear()
            continue
        core_types[core_fmt.split("(")[-1].rstrip(")")] += 1
        a = agg[(name, is_ratio)]
        a[0] += val * d_ns
        a[1] += d_ns
        a[2] += val
        if windows:
            mid = t_ns + d_ns / 2
            group = "<outside phases>"
            for s, e, gname in windows:
                if s <= mid < e:
                    group = gname
                    break
            ga = agg_by_group[group][(name, is_ratio)]
            ga[0] += val * d_ns
            ga[1] += d_ns
            ga[2] += val
        el.clear()
    return agg, core_types, agg_by_group


def parse_remarks(path):
    rr = RefResolver()
    remarks = Counter()
    for _ev, el in ET.iterparse(path, events=("end",)):
        if el.tag != "row":
            continue
        name, synopsis = None, None
        for sub in el.iter():
            if sub is el:
                continue
            r = rr.resolve(sub)
            if r.tag == "recount-remark-name" and name is None:
                s = r.find("string")
                name = (s.get("fmt") if s is not None else None) or r.get("fmt")
            # synopsis: a bare string child of row (not inside other elements)
        kids = list(el)
        if len(kids) >= 8:
            syn = rr.resolve(kids[7])
            if syn.tag == "string":
                synopsis = syn.get("fmt") or syn.text
        if name:
            remarks[(name, synopsis or "")] += 1
        el.clear()
    return remarks


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("trace")
    ap.add_argument("--window", nargs=2, type=float, default=None,
                    help="restrict to [start end] seconds of trace time")
    ap.add_argument("--pcore-only", action="store_true")
    ap.add_argument("--cache-dir", default=None)
    args = ap.parse_args()

    cache = args.cache_dir or os.path.dirname(os.path.abspath(args.trace))
    mt = export_table(args.trace, "MetricTable", cache)
    rm = export_table(args.trace, "RemarksByThread", cache)

    phase_windows, bench_meta = load_phase_windows(args.trace, cache) or (None, {})
    agg, cores, agg_by_group = parse_metric_table(
        mt, args.window, args.pcore_only, windows=phase_windows
    )

    def print_agg(agg, indent="   "):
        total_dur = max((a[1] for a in agg.values()), default=0)
        print(f"{indent}covered thread-time: {total_dur/1e9:.2f} s")
        for (name, is_ratio), (wsum, dsum, vsum) in sorted(agg.items()):
            if is_ratio:
                print(f"{indent}{name}: {wsum/dsum:.4f} (duration-weighted mean)")
            elif dsum:
                print(f"{indent}{name}: total {vsum:,.0f}  rate {vsum/(dsum/1e9):,.0f}/s")
            else:
                print(f"{indent}{name}: {vsum:,.0f}")

    print(f"== Metric aggregation ({args.trace}) ==")
    for k, v in bench_meta.items():
        print(f"   {k}: {v}")
    if args.window:
        print(f"   window: {args.window[0]}..{args.window[1]} s")
    print(f"   core-type sample counts: {dict(cores)}")
    print_agg(agg)

    if phase_windows:
        seen = []
        for _s, _e, g in phase_windows:
            if g not in seen:
                seen.append(g)
        for g in seen + [
            g for g in agg_by_group if g not in seen
        ]:
            if g not in agg_by_group:
                continue
            print(f"\n== Phase: {g} ==")
            print_agg(agg_by_group[g])
    else:
        print(
            "\n(no .phases.json sidecar next to the trace — per-phase metrics "
            "unavailable; record via profile.sh to get them)"
        )

    remarks = parse_remarks(rm)
    print("\n== Instruments remarks (bottleneck analysis) ==")
    for (name, syn), n in remarks.most_common(15):
        print(f"   [{n:5d}x] {name}: {syn}")


if __name__ == "__main__":
    main()
