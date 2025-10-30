#!/usr/bin/env python3
"""
Quick summary tool for JSONL telemetry produced by the morphogenetic runtime.

Usage:
    python scripts/analyze_telemetry.py telemetry.jsonl [--limit 100]
"""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Iterable, Tuple


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Summarize morphogenetic telemetry JSONL output.")
    parser.add_argument("paths", nargs="+", type=Path, help="Telemetry JSONL file(s).")
    parser.add_argument(
        "--limit",
        type=int,
        default=None,
        help="Optional cap on number of lines read from each file.",
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


def main() -> None:
    args = parse_args()
    overall_counts = Counter()
    lineage_totals = Counter()

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


if __name__ == "__main__":
    main()
