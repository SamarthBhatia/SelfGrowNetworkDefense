# Telemetry Analysis Guide

The runtime can write JSONL telemetry when invoked with `--telemetry telemetry.jsonl`. Use the helper script below to summarize outcomes across one or more runs.

## Quick Summary Script

```bash
python scripts/analyze_telemetry.py telemetry.jsonl
```

Options:

- `--limit <N>`: Read at most `N` records from each file (useful for large runs).
- Multiple file paths can be supplied to aggregate across experiments.
- `--plot`: Render a cumulative events chart (requires `pip install matplotlib`).
- `--plot-output plot.png`: Save the chart instead of opening a window.

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

To generate a plot and save it:

```bash
python scripts/analyze_telemetry.py runs/baseline.jsonl --plot --plot-output runs/baseline.png
```

For deeper analysis import the JSONL into Python/Pandas, Jupyter notebooks, or a Rust analytics pipeline. The summary script is intentionally lightweight and can be extended as the telemetry schema evolves.

## Step-Level Correlation

Use the correlation helper to align telemetry `StepSummary` events with replication counts and stimulus injections:

```bash
python scripts/telemetry_correlate.py runs/baseline.jsonl --stimulus runs/stimulus.jsonl
```

This prints per-step threat levels, cell counts, replication totals, signal emissions, and aggregated stimulus values—ideal for diagnosing how morphogenetic responses track external pressure.

## Dashboard Bootstrap

When you need richer dashboards or notebook-ready datasets, export structured step metrics and a Vega-Lite template in one pass:

```bash
python scripts/prepare_telemetry_dashboard.py runs/baseline.jsonl \
  --stimulus runs/stimulus.jsonl \
  --output dashboards/baseline_steps.csv \
  --vega-lite dashboards/baseline_spec.json
```

The CSV exposes one row per step with replication counts, dominant signal topics, stimulus totals, and more—drop it into Pandas, Polars, or spreadsheet tools. The optional Vega-Lite spec can be opened in [vega.github.io/editor](https://vega.github.io/editor/) to iterate on interactive visualisations without writing boilerplate.
