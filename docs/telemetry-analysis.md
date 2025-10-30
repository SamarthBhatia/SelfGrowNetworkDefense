# Telemetry Analysis Guide

The runtime can write JSONL telemetry when invoked with `--telemetry telemetry.jsonl`. Use the helper script below to summarize outcomes across one or more runs.

## Quick Summary Script

```bash
python scripts/analyze_telemetry.py telemetry.jsonl
```

Options:

- `--limit <N>`: Read at most `N` records from each file (useful for large runs).
- Multiple file paths can be supplied to aggregate across experiments.

Sample output:

```
=== Telemetry Summary: telemetry.jsonl ===
Total events: 42
Replications: 18 | Lineage shifts: 9 | Signals: 15
Lineage transitions:
  - IntrusionDetection: 6
  - Healer: 3

=== Aggregate Summary ===
Replications: 18 | Lineage shifts: 9 | Signals: 15
Lineage transitions (aggregate):
  - IntrusionDetection: 6
  - Healer: 3
```

## Integrating With Experiment Workflows

1. Run the simulation with telemetry enabled:

   ```bash
   cargo run -- --config docs/examples/baseline-growth.yaml --telemetry runs/baseline.jsonl
   ```

2. Append additional stimuli during execution (optional):

   ```bash
   cargo run --bin stimulus -- runs/stimulus.jsonl activator 0.9 6
   ```

3. After the simulation, summarize telemetry:

   ```bash
   python scripts/analyze_telemetry.py runs/baseline.jsonl
   ```

For deeper analysis import the JSONL into Python/Pandas, Jupyter notebooks, or a Rust analytics pipeline. The summary script is intentionally lightweight and can be extended as the telemetry schema evolves.
