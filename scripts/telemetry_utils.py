#!/usr/bin/env python3
"""
Shared utilities for working with morphogenetic telemetry artifacts.

This module centralises common parsing logic so multiple analytics scripts
can build on consistent primitives without duplicating JSONL handling.
"""

from __future__ import annotations

import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Dict, Iterable, List, Tuple


StepMetrics = Dict[str, object]
PerStepTelemetry = Dict[int, StepMetrics]
OrderedEventStream = List[Tuple[int, dict]]
StimulusSummary = Dict[int, dict]


def load_telemetry_per_step(path: Path) -> Tuple[PerStepTelemetry, OrderedEventStream]:
    """
    Load telemetry JSONL and return both per-step summaries and the ordered events list.

    Each StepSummary event finalises the metrics accumulated since the previous
    summary. The returned dictionary maps step numbers to a metrics payload that
    downstream analytics scripts can extend.
    """
    if not path.exists():
        raise FileNotFoundError(f"Telemetry file not found: {path}")

    per_step: PerStepTelemetry = {}
    buffer: List[dict] = []
    ordered_events: OrderedEventStream = []

    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            if not line.strip():
                continue
            payload = json.loads(line)
            ts = int(payload["timestamp_ms"])
            event = payload["event"]
            ordered_events.append((ts, event))

            if "StepSummary" in event:
                summary = event["StepSummary"]
                step = int(summary["step"])
                stats = _digest_buffer(buffer)
                per_step[step] = {
                    "threat_score": float(summary["threat_score"]),
                    "cell_count": int(summary["cell_count"]),
                    "replications": stats["replications"],
                    "signals": stats["signals"],
                    "lineage_shifts": stats["lineage_shifts"],
                }
                buffer = []
            else:
                buffer.append(event)

    return per_step, ordered_events


def _digest_buffer(buffer: Iterable[dict]) -> dict:
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


def load_stimuli(path: Path | None) -> StimulusSummary:
    """
    Load optional stimulus JSONL artefacts keyed by simulation step.
    """
    if path is None:
        return {}

    if not path.exists():
        raise FileNotFoundError(f"Stimulus file not found: {path}")

    stimuli: StimulusSummary = defaultdict(lambda: {"total": 0.0, "topics": Counter()})  # type: ignore[assignment]

    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            if not line.strip():
                continue
            payload = json.loads(line)
            step = int(payload["step"])
            topic = payload["topic"]
            value = float(payload["value"])
            entry = stimuli[step]
            entry["total"] += value
            entry["topics"][topic] += value

    # Convert nested Counters into ordinary dicts for downstream JSON serialisation.
    return {
        step: {"total": data["total"], "topics": dict(data["topics"])}
        for step, data in stimuli.items()
    }
