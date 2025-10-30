#!/usr/bin/env python3
"""
Telemetry summarizer and optional visualizer for morphogenetic JSONL output.

Usage:
    python scripts/analyze_telemetry.py telemetry.jsonl [--limit 100] [--plot]
"""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Iterable, List, Tuple


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Summarize morphogenetic telemetry JSONL output.")
    parser.add_argument("paths", nargs="+", type=Path, help="Telemetry JSONL file(s).")
    parser.add_argument(
        "--limit",
        type=int,
        default=None,
        help="Optional cap on number of lines read from each file.",
    )
    parser.add_argument(
        "--plot",
        action="store_true",
        help="Render cumulative event counts as a line chart (requires matplotlib).",
    )
    parser.add_argument(
        "--plot-output",
        type=Path,
        default=None,
        help="Optional PNG file to save when --plot is used.",
    )
    return parser.parse_args()


def load_records(path: Path, limit: int | None) -> Iterable[Tuple[int, dict]]:
    with path.open("r", encoding="utf-8") as handle:
        for idx, line in enumerate(handle):
            if limit is not None and idx >= limit:
                break
            if not line.strip():
                continue
            payload = json.loads(line)
            yield payload["timestamp_ms"], payload["event"]


def summarize(events: Iterable[Tuple[int, dict]]) -> dict:
    counts = Counter()
    lineage_changes = defaultdict(Counter)

    for _, event in events:
        if "CellReplicated" in event:
            counts["replications"] += 1
        elif "LineageShift" in event:
            counts["lineage_shifts"] += 1
            lineage = event["LineageShift"]["lineage"]
            lineage_changes[lineage]["count"] += 1
        elif "SignalEmitted" in event:
            counts["signals"] += 1

    return {
        "replications": counts["replications"],
        "lineage_shifts": counts["lineage_shifts"],
        "signals": counts["signals"],
        "lineage_counts": {lineage: data["count"] for lineage, data in lineage_changes.items()},
    }


def build_timeline(events: List[Tuple[int, dict]]) -> dict:
    cumulative = {"replications": 0, "lineage_shifts": 0, "signals": 0}
    timestamps = []
    replications = []
    lineage_shifts = []
    signals = []

    if not events:
        return {
            "timestamps": [],
            "replications": [],
            "lineage_shifts": [],
            "signals": [],
        }

    sorted_events = sorted(events, key=lambda item: item[0])
    first_ts = sorted_events[0][0]

    for timestamp, event in sorted_events:
        if "CellReplicated" in event:
            cumulative["replications"] += 1
        elif "LineageShift" in event:
            cumulative["lineage_shifts"] += 1
        elif "SignalEmitted" in event:
            cumulative["signals"] += 1

        timestamps.append((timestamp - first_ts) / 1000.0)
        replications.append(cumulative["replications"])
        lineage_shifts.append(cumulative["lineage_shifts"])
        signals.append(cumulative["signals"])

    return {
        "timestamps": timestamps,
        "replications": replications,
        "lineage_shifts": lineage_shifts,
        "signals": signals,
    }


def render_plot(data: dict, output_path: Path | None) -> None:
    try:
        import matplotlib.pyplot as plt  # type: ignore
    except ImportError:
        print("[warn] matplotlib not installed; install with `pip install matplotlib` to enable plotting.")
        return

    if not data["timestamps"]:
        print("[warn] No data available for plotting.")
        return

    plt.figure(figsize=(8, 4.5))
    plt.plot(data["timestamps"], data["replications"], label="Replications")
    plt.plot(data["timestamps"], data["lineage_shifts"], label="Lineage Shifts")
    plt.plot(data["timestamps"], data["signals"], label="Signals")
    plt.xlabel("Elapsed Time (seconds)")
    plt.ylabel("Cumulative Count")
    plt.title("Morphogenetic Telemetry Events Over Time")
    plt.legend()
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150)
        print(f"[info] Saved plot to {output_path}")
    else:
        plt.show()


def merge_plot_data(datasets: List[dict]) -> dict:
    if not datasets:
        return {
            "timestamps": [],
            "replications": [],
            "lineage_shifts": [],
            "signals": [],
        }

    longest = max(datasets, key=lambda d: len(d["timestamps"]))
    base = {
        "timestamps": longest["timestamps"],
        "replications": [0] * len(longest["timestamps"]),
        "lineage_shifts": [0] * len(longest["timestamps"]),
        "signals": [0] * len(longest["timestamps"]),
    }

    for data in datasets:
        for idx, value in enumerate(data["replications"]):
            base["replications"][idx] += value
        for idx, value in enumerate(data["lineage_shifts"]):
            base["lineage_shifts"][idx] += value
        for idx, value in enumerate(data["signals"]):
            base["signals"][idx] += value

    return base


def main() -> None:
    args = parse_args()
    overall_counts = Counter()
    lineage_totals = Counter()
    plot_data: List[dict] = []

    for path in args.paths:
        if not path.exists():
            print(f"[warn] Skipping missing file: {path}")
            continue
        events = list(load_records(path, args.limit))
        summary = summarize(events)
        print(f"=== Telemetry Summary: {path} ===")
        print(f"Total events: {len(events)}")
        print(
            f"Replications: {summary['replications']} | "
            f"Lineage shifts: {summary['lineage_shifts']} | "
            f"Signals: {summary['signals']}"
        )
        if summary["lineage_counts"]:
            print("Lineage transitions:")
            for lineage, count in summary["lineage_counts"].items():
                print(f"  - {lineage}: {count}")
        print()

        overall_counts.update(
            {
                "replications": summary["replications"],
                "lineage_shifts": summary["lineage_shifts"],
                "signals": summary["signals"],
            }
        )
        lineage_totals.update(summary["lineage_counts"])
        if args.plot:
            plot_data.append(build_timeline(events))

    if args.paths:
        print("=== Aggregate Summary ===")
        print(
            f"Replications: {overall_counts['replications']} | "
            f"Lineage shifts: {overall_counts['lineage_shifts']} | "
            f"Signals: {overall_counts['signals']}"
        )
        if lineage_totals:
            print("Lineage transitions (aggregate):")
            for lineage, count in lineage_totals.items():
                print(f"  - {lineage}: {count}")

    if args.plot and plot_data:
        merged = merge_plot_data(plot_data)
        render_plot(merged, args.plot_output)


if __name__ == "__main__":
    main()
