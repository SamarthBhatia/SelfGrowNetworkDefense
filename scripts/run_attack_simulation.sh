#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="$ROOT_DIR/target"
TELEMETRY_FILE="$TARGET_DIR/ci_attack_telemetry.jsonl"
STIMULUS_FILE="$TARGET_DIR/ci_stimulus.jsonl"
DASHBOARD_CSV="$TARGET_DIR/ci_attack_steps.csv"
DASHBOARD_SPEC="$TARGET_DIR/ci_attack_spec.json"
HARNESS_JSON="$TARGET_DIR/ci_harness_outcome.json"
HARNESS_STATE="$TARGET_DIR/ci_harness_state.json"

mkdir -p "$TARGET_DIR"
rm -f "$TELEMETRY_FILE" "$STIMULUS_FILE" "$DASHBOARD_CSV" "$DASHBOARD_SPEC" "$HARNESS_JSON" "$HARNESS_STATE"

# Seed a simple stimulus schedule
cargo run --quiet --bin stimulus -- "$STIMULUS_FILE" activator 0.9 3
cargo run --quiet --bin stimulus -- "$STIMULUS_FILE" inhibitor 0.4 5

# Execute the morphogenetic runtime against the intense-defense scenario
cargo run --quiet --bin morphogenetic-security -- \
  --config "$ROOT_DIR/docs/examples/intense-defense.yaml" \
  --telemetry "$TELEMETRY_FILE" \
  --stimulus "$STIMULUS_FILE"

# Summaries for quick diagnostics
python3 "$ROOT_DIR/scripts/analyze_telemetry.py" "$TELEMETRY_FILE" --limit 50
python3 "$ROOT_DIR/scripts/telemetry_correlate.py" "$TELEMETRY_FILE" --stimulus "$STIMULUS_FILE"
python3 "$ROOT_DIR/scripts/prepare_telemetry_dashboard.py" "$TELEMETRY_FILE" \
  --stimulus "$STIMULUS_FILE" \
  --output "$DASHBOARD_CSV" \
  --vega-lite "$DASHBOARD_SPEC"

cargo run --quiet --bin adversarial_cycle -- \
  --candidate-id ci-seed \
  --scenario "$ROOT_DIR/docs/examples/intense-defense.yaml" \
  --generation 0 \
  --metrics "$DASHBOARD_CSV" \
  --stimulus "$STIMULUS_FILE" \
  --state "$HARNESS_STATE" \
  --emit-json "$HARNESS_JSON"

printf "Telemetry artifacts saved to %s\n" "$TARGET_DIR"
