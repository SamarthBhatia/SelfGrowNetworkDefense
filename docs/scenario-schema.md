# Scenario Configuration Schema

The runtime consumes YAML manifests that describe morphogenetic defense experiments. This document summarizes the supported fields and provides working examples.

## Root Fields

### `scenario_name` (string, optional)
Human-friendly label used in logs and telemetry. Defaults to `baseline`.

### `initial_cell_count` (integer, optional)
Number of seed cells created before the first step. Must be ≥ 1. Defaults to `1`.

### `simulation_steps` (integer, optional)
Total number of iterations to execute. Must be ≥ 1. Defaults to `1`.

### `threat_profile` (object, optional)
Controls background threat pressure and reproduction thresholds.

- `background_threat` (float, default `0.1`): Baseline threat level evaluated each tick.
- `spike_threshold` (float, default `0.8`): When the composite threat equals or exceeds this value, an activator spike is injected automatically.

### `spikes` (array, optional)
Predefined threat spikes applied on specific steps. Each element is an object:

- `step` (integer): Step index (0-based).
- `intensity` (float): Threat to add on that step before evaluating automata.
- `duration` (integer, default `1`): Number of steps the spike persists.

### `topology` (object, optional)
Defines how cells communicate.

- `strategy` (string, default `Global`): Signaling routing logic. Supported values:
    - `Global`: All signals are broadcast to all cells (soup model).
    - `Graph`: Signals travel only between neighbors (parent-child or explicit connections).

## Example: Graph-based Topology

```yaml
scenario_name: graph-defense
topology:
  strategy: Graph
initial_cell_count: 5
simulation_steps: 10
```

## Example: Baseline Growth

```yaml
scenario_name: baseline-growth
initial_cell_count: 2
simulation_steps: 10
threat_profile:
  background_threat: 0.15
  spike_threshold: 0.6
spikes:
  - step: 3
    intensity: 0.35
  - step: 7
    intensity: 0.45
```

## Example: Intense Defensive Posture

```yaml
scenario_name: intense-defense
initial_cell_count: 4
simulation_steps: 20
threat_profile:
  background_threat: 0.3
  spike_threshold: 0.75
spikes:
  - step: 2
    intensity: 0.4
  - step: 5
    intensity: 0.55
  - step: 9
    intensity: 0.6
```

## Stimulus Schedule Integration

The runtime can ingest additional signals from a JSONL stimulus file via the `--stimulus` flag. Each line must be a JSON object matching:

```json
{ "step": 4, "topic": "inhibitor", "value": 0.5 }
```

To append entries programmatically use:

```bash
cargo run --bin stimulus -- stimulus.jsonl activator 0.9 6
```

At runtime the scheduler injects all commands whose `step` matches the current iteration.

## Telemetry Capture

Enable JSONL telemetry logging with `--telemetry telemetry.jsonl`. Each line has the shape:

```json
{ "timestamp_ms": 1730246400000, "event": { "CellReplicated": { "cell_id": "seed-0", "child_id": "seed-0::child" } } }
```

Use the analytics tooling described in `docs/telemetry-analysis.md` to summarize outputs.
