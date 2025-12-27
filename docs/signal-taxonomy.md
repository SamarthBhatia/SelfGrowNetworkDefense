# Signal Taxonomy & Best Practices

Morphogenetic security agents coordinate through lightweight signal topics. Use the guidelines below when crafting scenarios or external stimulus schedules.

## Core Topics

- `activator`: Amplifies perceived threat in a region. Automatically emitted when the threat profile exceeds `spike_threshold`. Use external activator stimuli to simulate sudden attack bursts or red-team actions.
- `inhibitor`: Dampens threat perception and encourages healing or energy recovery. Schedule inhibitor pulses to emulate remediation teams or fail-safes.
- `cooperative`: Encourages specialization toward encryption or coordination tasks. Useful when devices must harden communications after an incident.
- `consensus:<topic>`: Swarm-level voting signals. Automatically emitted when a cell detects an anomaly. These signals require valid TPM attestation to influence neighbor behavior.

## Design Principles

1. **Locality**: In `Graph` topology mode, signals only propagate to immediate neighbors. Prefer small magnitudes (≤ 1.0) and targeted steps to avoid destabilizing the reaction–diffusion dynamics.
2. **Complementarity**: Pair activator pulses with follow-up inhibitor or cooperative signals to observe regenerative behavior.
3. **Observability**: Record stimulus schedules alongside telemetry so analysts can correlate external perturbations with cellular responses.
4. **Routing**: All cell-emitted signals now include a `source` field containing the cell's unique identifier. System-level stimuli (via CLI or config) have no source and are typically treated as broadcast or global perturbations depending on the runtime handler.

## Authoring Stimulus Files

Use the JSONL stimulus format consumed via `--stimulus`:

```json
{ "step": 12, "topic": "inhibitor", "value": 0.45 }
```

Append entries with:

```bash
cargo run --bin stimulus -- runs/stimulus.jsonl inhibitor 0.45 12
```

Keep values within `[0.0, 1.5]` for stability; experiment cautiously outside this range.
