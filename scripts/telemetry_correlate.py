#!/usr/bin/env python3
"""
Correlate telemetry StepSummary events with replication/signalling metrics and optional stimuli.

Usage:
    python scripts/telemetry_correlate.py telemetry.jsonl [--stimulus stimulus.jsonl]
"""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Dict, List, Tuple


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Correlate morphogenetic telemetry with stimuli.")
    parser.add_argument("telemetry", type=Path, help="Telemetry JSONL file produced via --telemetry.")
    parser.add_argument(
        "--stimulus",
        type=Path,
        help="Optional stimulus JSONL file used during the run.",
    )
    return parser.parse_args()


def load_telemetry(path: Path) -> Tuple[Dict[int, dict], List[Tuple[int, dict]]]:
    if not path.exists():
        raise FileNotFoundError(f"Telemetry file not found: {path}")

    per_step: Dict[int, dict] = {}
    buffer: List[dict] = []
    ordered_events: List[Tuple[int, dict]] = []

    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            if not line.strip():
                continue
            payload = json.loads(line)
            ts = payload["timestamp_ms"]
            event = payload["event"]
            ordered_events.append((ts, event))

            if "StepSummary" in event:
                summary = event["StepSummary"]
                step = summary["step"]
                stats = digest_buffer(buffer)
                per_step[step] = {
                    "threat_score": summary["threat_score"],
                    "cell_count": summary["cell_count"],
                    "replications": stats["replications"],
                    "signals": stats["signals"],
                    "lineage_shifts": stats["lineage_shifts"],
                }
                buffer = []
            else:
                buffer.append(event)

    return per_step, ordered_events


def digest_buffer(buffer: List[dict]) -> dict:
    replications = 0
    signals = Counter()
    lineage_shifts = Counter()

    for event in buffer:
        if "CellReplicated" in event:
            replications += 1
        elif "SignalEmitted" in event:
            topic = event["SignalEmitted"]["topic"]
            signals[topic] += 1
        elif "LineageShift" in event:
            lineage = event["LineageShift"]["lineage"]
            lineage_shifts[lineage] += 1

    return {
        "replications": replications,
        "signals": dict(signals),
        "lineage_shifts": dict(lineage_shifts),
    }


def load_stimuli(path: Path) -> Dict[int, dict]:
    stimuli: Dict[int, dict] = defaultdict(lambda: {"total": 0.0, "topics": Counter()})
    if not path:
        return {}
    if not path.exists():
        raise FileNotFoundError(f"Stimulus file not found: {path}")

    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            if not line.strip():
                continue
            payload = json.loads(line)
            step = payload["step"]
            topic = payload["topic"]
            value = float(payload["value"])
            entry = stimuli[step]
            entry["total"] += value
            entry["topics"][topic] += value

    return stimuli


def main() -> None:
    args = parse_args()
    per_step, _ = load_telemetry(args.telemetry)
    stimuli = load_stimuli(args.stimulus) if args.stimulus else {}

    if not per_step:
        print("[info] No StepSummary events found in telemetry; nothing to report.")
        return

    print("Step | Threat | Cells | Replications | Signals | Lineage Shifts | Stimuli")
    print("-----|--------|-------|--------------|---------|----------------|--------")

    for step in sorted(per_step):
        metrics = per_step[step]
        signal_summary = format_counter(metrics["signals"])
        lineage_summary = format_counter(metrics["lineage_shifts"])
        stim_entry = stimuli.get(step)
        if stim_entry:
            stim_summary = f"total={stim_entry['total']:.2f} ({format_counter(stim_entry['topics'])})"
        else:
            stim_summary = "none"

        print(
            f"{step:>4} | "
            f"{metrics['threat_score']:.2f} | "
            f"{metrics['cell_count']:>5} | "
            f"{metrics['replications']:>12} | "
            f"{signal_summary:<9} | "
            f"{lineage_summary:<14} | "
            f"{stim_summary}"
        )


def format_counter(counter: dict) -> str:
    if not counter:
        return "-"
    parts = [f"{key}:{value}" for key, value in counter.items()]
    return ", ".join(parts)


if __name__ == "__main__":
    main()
