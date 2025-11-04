#!/usr/bin/env python3
"""
Prepare per-step telemetry datasets and optional Vega-Lite specs for dashboards.

The script now emits lineage-aware fitness annotations alongside the traditional
step metrics so dashboards can surface breach heuristics directly.

Example:
    python scripts/prepare_telemetry_dashboard.py docs/runs/demo.jsonl \
        --stimulus docs/runs/demo_stimulus.jsonl \
        --output dashboards/demo_steps.csv \
        --vega-lite dashboards/demo_spec.json
"""

from __future__ import annotations

import argparse
import collections
import csv
import json
from pathlib import Path
from typing import Dict, Iterable, List, Tuple

from telemetry_utils import load_stimuli, load_telemetry_per_step


DEFAULT_SPEC_METRICS = [
    "threat_score",
    "cell_count",
    "replications",
    "signals_total",
    "lineage_shifts_total",
    "stimulus_total",
    "lineage_pressure",
    "lineage_component",
    "fitness_score",
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

    rows, stats = build_step_rows(per_step, stimuli)
    annotations = derive_harness_annotations(stats)
    apply_annotations(rows, annotations)

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


def build_step_rows(per_step: dict, stimuli: dict) -> Tuple[List[dict], Dict[str, object]]:
    rows: List[dict] = []
    aggregates = _initialise_aggregates()

    for step in sorted(per_step):
        metrics = per_step[step]
        signal_counts = metrics["signals"]
        lineage_counts = metrics["lineage_shifts"]
        signal_total = _sum_values(signal_counts)
        lineage_total = _sum_values(lineage_counts)
        top_signal_topic, top_signal_count = _top_item(signal_counts)
        top_lineage, top_lineage_count = _top_item(lineage_counts)
        stim_entry = stimuli.get(step, {"total": 0.0, "topics": {}})

        _accumulate_aggregates(aggregates, metrics, stim_entry)

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

    stats = _finalise_aggregates(aggregates)
    return rows, stats


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
        "lineage_pressure",
        "lineage_focus_ratio",
        "lineage_component",
        "fitness_score",
        "breach_observed",
        "recommended_mutation",
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
    if not rows:
        raise ValueError("Cannot build a Vega-Lite spec without rows.")

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
                {"field": "fitness_score", "type": "quantitative"},
                {"field": "breach_observed", "type": "nominal"},
                {"field": "recommended_mutation", "type": "nominal"},
            ],
        },
        "resolve": {"scale": {"y": "independent"}},
    }


def derive_harness_annotations(stats: Dict[str, object]) -> Dict[str, object]:
    if not stats or stats["step_count"] == 0:
        return {
            "lineage_pressure": 0.0,
            "lineage_focus_ratio": 0.0,
            "lineage_component": 0.0,
            "fitness_score": 0.0,
            "breach_observed": False,
            "recommended_mutation": "",
        }

    lineage_pressure = _compute_lineage_pressure(stats)
    focus_ratio = _compute_lineage_focus_ratio(stats)
    lineage_component = _clamp(0.6 * lineage_pressure + 0.4 * focus_ratio)
    suppression_component = _compute_suppression_component(stats)
    cell_loss_component = _compute_cell_loss_component(stats)
    stimulus_component = _compute_stimulus_component(stats)

    fitness = (
        0.35 * _clamp(stats["avg_threat"] / 1.5)
        + 0.2 * suppression_component
        + 0.15 * cell_loss_component
        + 0.2 * lineage_component
        + 0.1 * stimulus_component
    )

    breach_observed = (
        fitness > 0.65
        or stats["max_threat"] > 1.1
        or cell_loss_component > 0.45
        or lineage_component > 0.8
    )

    mutation = _recommend_mutation(stats, fitness, breach_observed) or ""

    return {
        "lineage_pressure": lineage_pressure,
        "lineage_focus_ratio": focus_ratio,
        "lineage_component": lineage_component,
        "fitness_score": fitness,
        "breach_observed": breach_observed,
        "recommended_mutation": mutation,
    }


def apply_annotations(rows: List[dict], annotations: Dict[str, object]) -> None:
    for row in rows:
        row.update(
            {
                "lineage_pressure": round(float(annotations["lineage_pressure"]), 6),
                "lineage_focus_ratio": round(float(annotations["lineage_focus_ratio"]), 6),
                "lineage_component": round(float(annotations["lineage_component"]), 6),
                "fitness_score": round(float(annotations["fitness_score"]), 6),
                "breach_observed": bool(annotations["breach_observed"]),
                "recommended_mutation": str(annotations["recommended_mutation"]),
            }
        )


def _initialise_aggregates() -> Dict[str, object]:
    return {
        "step_count": 0,
        "threat_sum": 0.0,
        "max_threat": 0.0,
        "cell_sum": 0.0,
        "min_cell": None,
        "max_cell": 0,
        "total_replications": 0,
        "total_signals": 0,
        "total_lineage_shifts": 0,
        "total_stimulus": 0.0,
        "signals_by_topic": collections.Counter(),
        "lineage_by_type": collections.Counter(),
        "stimuli_by_topic": collections.defaultdict(float),  # type: ignore[arg-type]
    }


def _accumulate_aggregates(aggregates: Dict[str, object], metrics: dict, stim_entry: dict) -> None:
    aggregates["step_count"] = int(aggregates["step_count"]) + 1
    aggregates["threat_sum"] += float(metrics["threat_score"])
    aggregates["max_threat"] = max(float(aggregates["max_threat"]), float(metrics["threat_score"]))
    aggregates["cell_sum"] += float(metrics["cell_count"])

    min_cell = aggregates["min_cell"]
    cell_count = int(metrics["cell_count"])
    if min_cell is None:
        aggregates["min_cell"] = cell_count
    else:
        aggregates["min_cell"] = min(int(min_cell), cell_count)
    aggregates["max_cell"] = max(int(aggregates["max_cell"]), cell_count)

    aggregates["total_replications"] += int(metrics["replications"])

    signal_counts = metrics["signals"]
    lineage_counts = metrics["lineage_shifts"]
    aggregates["total_signals"] += _sum_values(signal_counts)
    aggregates["total_lineage_shifts"] += _sum_values(lineage_counts)
    aggregates["signals_by_topic"].update(signal_counts)
    aggregates["lineage_by_type"].update(lineage_counts)

    total_stimulus = float(stim_entry.get("total", 0.0))
    aggregates["total_stimulus"] += total_stimulus
    for topic, value in stim_entry.get("topics", {}).items():
        aggregates["stimuli_by_topic"][topic] += float(value)


def _finalise_aggregates(aggregates: Dict[str, object]) -> Dict[str, object]:
    step_count = int(aggregates["step_count"])
    stats = {
        "step_count": step_count,
        "avg_threat": aggregates["threat_sum"] / max(1, step_count),
        "max_threat": float(aggregates["max_threat"]),
        "avg_cell_count": aggregates["cell_sum"] / max(1, step_count),
        "min_cell_count": int(aggregates["min_cell"] or 0),
        "max_cell_count": int(aggregates["max_cell"]),
        "total_replications": int(aggregates["total_replications"]),
        "total_signals": int(aggregates["total_signals"]),
        "total_lineage_shifts": int(aggregates["total_lineage_shifts"]),
        "total_stimulus": float(aggregates["total_stimulus"]),
        "signals_by_topic": dict(aggregates["signals_by_topic"]),
        "lineage_by_type": dict(aggregates["lineage_by_type"]),
        "stimuli_by_topic": dict(aggregates["stimuli_by_topic"]),
    }
    return stats


def _compute_lineage_pressure(stats: Dict[str, object]) -> float:
    step_count = max(1, int(stats["step_count"]))
    total_lineage_shifts = float(stats["total_lineage_shifts"])
    pressure = total_lineage_shifts / step_count
    return _clamp(pressure / 0.6)


def _compute_lineage_focus_ratio(stats: Dict[str, object]) -> float:
    total = int(stats["total_lineage_shifts"])
    if total == 0:
        return 0.0
    dominant = max((int(count) for count in stats["lineage_by_type"].values()), default=0)
    return _clamp(dominant / total)


def _compute_suppression_component(stats: Dict[str, object]) -> float:
    step_count = max(1.0, float(stats["step_count"]))
    reproduction_rate = float(stats["total_replications"]) / step_count
    return max(0.0, 1.0 - min(reproduction_rate, 1.0))


def _compute_cell_loss_component(stats: Dict[str, object]) -> float:
    max_cell = int(stats["max_cell_count"])
    if max_cell == 0:
        return 0.0
    deficit = max_cell - int(stats["min_cell_count"])
    return _clamp(deficit / max_cell)


def _compute_stimulus_component(stats: Dict[str, object]) -> float:
    step_count = max(1.0, float(stats["step_count"]))
    return _clamp(float(stats["total_stimulus"]) / (step_count * 1.5))


def _recommend_mutation(stats: Dict[str, object], fitness: float, breach: bool) -> str | None:
    stimuli = stats["stimuli_by_topic"]
    step_count = max(1.0, float(stats["step_count"]))
    reproductions = float(stats["total_replications"]) / step_count
    lineage_pressure = float(stats["total_lineage_shifts"]) / step_count

    dominant_lineage = None
    dominant_count = 0
    for lineage, count in stats["lineage_by_type"].items():
        count_int = int(count)
        if count_int > dominant_count:
            dominant_lineage = lineage
            dominant_count = count_int

    total_lineage = int(stats["total_lineage_shifts"])
    dominant_ratio = 0.0 if total_lineage == 0 else dominant_count / total_lineage

    activator = float(stimuli.get("activator", 0.0))
    inhibitor = float(stimuli.get("inhibitor", 0.0))

    if fitness < 0.4:
        if activator <= inhibitor:
            return "increase activator spike amplitude and damp inhibitor recovery"
        return "inject cooperative decoys ahead of activator bursts to overwhelm defences"
    if breach:
        if int(stats["total_signals"]) < int(stats["step_count"]):
            return "extend breach window with sustained activator pulses post-impact"
        return "tighten attack cadence: alternate activator and inhibitor surges faster"
    if lineage_pressure < 0.2:
        return "escalate lineage churn by targeting secondary cell lineages with staggered activator bursts"
    if dominant_ratio < 0.5 and total_lineage > 3:
        if dominant_lineage:
            return f"focus mutation pressure on the {dominant_lineage} lineage to consolidate takeovers"
        return "focus mutation pressure on the highest-yield lineage to consolidate takeovers"
    if reproductions > 0.6:
        return "slow defensive replication by scheduling inhibitor spikes before activator peaks"
    if inhibitor > activator and activator > 0.0:
        return "rebalance stimuli by boosting activator intensity relative to inhibitor damping"
    return None


def _sum_values(payload: dict) -> int:
    return int(sum(int(value) for value in payload.values()))


def _top_item(payload: dict) -> Tuple[str | None, int]:
    if not payload:
        return None, 0
    item = max(payload.items(), key=lambda entry: entry[1])
    return item[0], int(item[1])


def _clamp(value: float, lower: float = 0.0, upper: float = 1.0) -> float:
    return max(lower, min(upper, value))


if __name__ == "__main__":
    main()
