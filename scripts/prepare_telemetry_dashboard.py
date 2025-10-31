#!/usr/bin/env python3
"""
Prepare per-step telemetry datasets and optional Vega-Lite specs for dashboards.

Example:
    python scripts/prepare_telemetry_dashboard.py docs/runs/demo.jsonl \
        --stimulus docs/runs/demo_stimulus.jsonl \
        --output dashboards/demo_steps.csv \
        --vega-lite dashboards/demo_spec.json
"""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from typing import Iterable, List

from telemetry_utils import load_stimuli, load_telemetry_per_step


DEFAULT_SPEC_METRICS = [
    "threat_score",
    "cell_count",
    "replications",
    "signals_total",
    "lineage_shifts_total",
    "stimulus_total",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate CSV datasets and optional Vega-Lite specs from telemetry JSONL."
    )
    parser.add_argument("telemetry", type=Path, help="Telemetry JSONL file produced via --telemetry.")
    parser.add_argument(
        "--stimulus",
        type=Path,
        default=None,
        help="Optional stimulus JSONL file recorded during the run.",
    )
    parser.add_argument(
        "--output",
        required=True,
        type=Path,
        help="Destination CSV file capturing per-step metrics.",
    )
    parser.add_argument(
        "--vega-lite",
        dest="vega_lite",
        type=Path,
        default=None,
        help="Optional Vega-Lite JSON spec to write for quick dashboard bootstrapping.",
    )
    parser.add_argument(
        "--metrics",
        nargs="+",
        default=DEFAULT_SPEC_METRICS,
        help="Metric columns to include in the Vega-Lite fold transform.",
    )
    parser.add_argument(
        "--lineage-output",
        type=Path,
        default=None,
        help="Optional long-form CSV capturing lineage counts per step.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    per_step, _ = load_telemetry_per_step(args.telemetry)
    stimuli = load_stimuli(args.stimulus)

    if not per_step:
        print("[info] No StepSummary events found; nothing to export.")
        return

    rows = build_step_rows(per_step, stimuli)
    write_csv(rows, args.output)
    print(f"[info] Wrote {len(rows)} step rows to {args.output}")

    if args.lineage_output:
        lineage_rows = build_lineage_rows(rows)
        write_lineage_csv(lineage_rows, args.lineage_output)
        print(f"[info] Wrote {len(lineage_rows)} lineage rows to {args.lineage_output}")

    if args.vega_lite:
        spec = build_vega_spec(rows, args.metrics)
        args.vega_lite.parent.mkdir(parents=True, exist_ok=True)
        args.vega_lite.write_text(json.dumps(spec, indent=2))
        print(f"[info] Wrote Vega-Lite spec to {args.vega_lite}")


def build_step_rows(per_step: dict, stimuli: dict) -> List[dict]:
    rows: List[dict] = []

    for step in sorted(per_step):
        metrics = per_step[step]
        signal_counts = metrics["signals"]
        lineage_counts = metrics["lineage_shifts"]
        signal_total = _sum_values(signal_counts)
        lineage_total = _sum_values(lineage_counts)
        top_signal_topic, top_signal_count = _top_item(signal_counts)
        top_lineage, top_lineage_count = _top_item(lineage_counts)
        stim_entry = stimuli.get(step, {"total": 0.0, "topics": {}})

        rows.append(
            {
                "step": int(step),
                "threat_score": float(metrics["threat_score"]),
                "cell_count": int(metrics["cell_count"]),
                "replications": int(metrics["replications"]),
                "signals_total": signal_total,
                "lineage_shifts_total": lineage_total,
                "stimulus_total": float(stim_entry.get("total", 0.0)),
                "top_signal_topic": top_signal_topic or "",
                "top_signal_count": top_signal_count,
                "top_lineage": top_lineage or "",
                "top_lineage_count": top_lineage_count,
                "signals_by_topic": json.dumps(signal_counts, sort_keys=True),
                "lineage_shifts_by_lineage": json.dumps(lineage_counts, sort_keys=True),
                "stimulus_by_topic": json.dumps(stim_entry.get("topics", {}), sort_keys=True),
            }
        )

    return rows


def write_csv(rows: Iterable[dict], destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    rows_list = list(rows)
    if not rows_list:
        destination.write_text("")
        return

    fieldnames = [
        "step",
        "threat_score",
        "cell_count",
        "replications",
        "signals_total",
        "lineage_shifts_total",
        "stimulus_total",
        "top_signal_topic",
        "top_signal_count",
        "top_lineage",
        "top_lineage_count",
        "signals_by_topic",
        "lineage_shifts_by_lineage",
        "stimulus_by_topic",
    ]

    with destination.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows_list:
            writer.writerow(row)


def build_lineage_rows(rows: List[dict]) -> List[dict]:
    lineage_rows: List[dict] = []
    for row in rows:
        payload = json.loads(row["lineage_shifts_by_lineage"])
        for lineage, count in payload.items():
            lineage_rows.append(
                {
                    "step": int(row["step"]),
                    "lineage": str(lineage),
                    "count": int(count),
                }
            )
    return lineage_rows


def write_lineage_csv(rows: List[dict], destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    with destination.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=["step", "lineage", "count"])
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def build_vega_spec(rows: List[dict], metrics: Iterable[str]) -> dict:
    valid_metrics = [metric for metric in metrics if metric in rows[0]]
    if not valid_metrics:
        raise ValueError("No valid metrics provided for Vega-Lite spec generation.")

    return {
        "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
        "description": "Morphogenetic telemetry step metrics (auto-generated).",
        "data": {"values": rows},
        "transform": [
            {"fold": valid_metrics, "as": ["metric", "value"]},
        ],
        "mark": {"type": "line", "interpolate": "monotone"},
        "encoding": {
            "x": {"field": "step", "type": "quantitative"},
            "y": {"field": "value", "type": "quantitative"},
            "color": {"field": "metric", "type": "nominal"},
            "tooltip": [
                {"field": "step", "type": "quantitative"},
                {"field": "metric", "type": "nominal"},
                {"field": "value", "type": "quantitative"},
            ],
        },
        "resolve": {"scale": {"y": "independent"}},
    }


def _sum_values(payload: dict) -> int:
    return int(sum(int(value) for value in payload.values()))


def _top_item(payload: dict) -> tuple[str | None, int]:
    if not payload:
        return None, 0
    key, value = max(payload.items(), key=lambda item: item[1])
    return str(key), int(value)


if __name__ == "__main__":
    main()
