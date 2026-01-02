#!/usr/bin/env python3
"""
Correlate telemetry StepSummary events with replication/signalling metrics and optional stimuli.

Usage:
    python scripts/telemetry_correlate.py telemetry.jsonl [--stimulus stimulus.jsonl]
"""

from __future__ import annotations

import argparse
from pathlib import Path

from telemetry_utils import load_stimuli, load_telemetry_per_step


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Correlate morphogenetic telemetry with stimuli.")
    parser.add_argument("telemetry", type=Path, help="Telemetry JSONL file produced via --telemetry.")
    parser.add_argument(
        "--stimulus",
        type=Path,
        help="Optional stimulus JSONL file used during the run.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    per_step, _, _ = load_telemetry_per_step(args.telemetry)
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
