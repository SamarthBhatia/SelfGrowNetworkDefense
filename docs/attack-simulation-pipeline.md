# Attack Simulation Pipeline

This repository ships with a basic smoke-test pipeline that exercises the morphogenetic runtime under a high-threat scenario. The goal is to extend it into a full adversarial harness as development progresses.

## Scripted Workflow

Run the helper script to reproduce the CI smoke test locally:

```bash
bash scripts/run_attack_simulation.sh
```

The script performs the following steps:

1. Generates a temporary stimulus schedule (`target/ci_stimulus.jsonl`) using the `stimulus` CLI, adding an activator burst and subsequent inhibitor pulse.
2. Executes the runtime against `docs/examples/intense-defense.yaml`, streaming telemetry to `target/ci_attack_telemetry.jsonl`.
3. Summarises the output via `scripts/analyze_telemetry.py` and `scripts/telemetry_correlate.py` for quick diagnostics.

## CI Integration

The GitHub Actions workflow (`.github/workflows/ci.yml`) runs the same script after formatting checks, clippy, and unit tests. This ensures every change compiles, passes linting, and survives a representative morphogenetic run.

## Extending the Pipeline

- **Adversarial traffic:** Replace the scripted `cargo run` invocation with a container that generates evolving attacks (e.g., Scapy or a custom Rust attacker). Capture telemetry for regression analysis.
- **Performance metrics:** Export CPU/memory data from the runtime to correlate resource pressure with morphogenetic reactions.
- **Parameterized scenarios:** Add matrix jobs in CI to run several YAML configurations (baseline, intense, sparse devices) and compare telemetry summaries.
- **Artifact publication:** Upload telemetry and stimulus JSONL files as workflow artifacts for deeper offline inspection.

As the system evolves, convert this smoke test into a full-fledged attack evolution harness that validates morphogenetic adaptation end-to-end.
