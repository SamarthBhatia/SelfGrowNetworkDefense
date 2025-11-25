#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="$ROOT_DIR/target/lineage_evaluation"

mkdir -p "$TARGET_DIR"
rm -f "$TARGET_DIR"/*.jsonl "$TARGET_DIR"/*.csv "$TARGET_DIR"/*.json

echo "[eval] Preparing stimulus schedule for extreme-mutation scenario..."
STIMULUS_FILE="$TARGET_DIR/extreme_mutation_stimulus.jsonl"
cargo run --quiet --bin stimulus -- "$STIMULUS_FILE" activator 1.0 2
cargo run --quiet --bin stimulus -- "$STIMULUS_FILE" activator 1.0 4
cargo run --quiet --bin stimulus -- "$STIMULUS_FILE" activator 1.0 6
cargo run --quiet --bin stimulus -- "$STIMULUS_FILE" inhibitor 0.1 8
cargo run --quiet --bin stimulus -- "$STIMULUS_FILE" activator 1.0 10

run_scenario() {
  local label="$1"
  local scenario_path="$2"
  local use_stimulus="$3"
  local telemetry_file="$TARGET_DIR/${label}_telemetry.jsonl"
  local metrics_csv="$TARGET_DIR/${label}_step_metrics.csv"
  local summary_json="$TARGET_DIR/${label}_summary.json"
  local harness_json="$TARGET_DIR/${label}_outcome.json"

  echo "[eval] Running scenario '$label' ($scenario_path)..."
  if [[ "$use_stimulus" == "true" ]]; then
    cargo run --quiet --bin morphogenetic-security -- \
      --config "$scenario_path" \
      --telemetry "$telemetry_file" \
      --stimulus "$STIMULUS_FILE"
  else
    cargo run --quiet --bin morphogenetic-security -- \
      --config "$scenario_path" \
      --telemetry "$telemetry_file"
  fi

  echo "[eval] Summarising telemetry for '$label'..."
  local prepare_args=(
    python3 "$ROOT_DIR/scripts/prepare_telemetry_dashboard.py" "$telemetry_file"
    --output "$metrics_csv"
    --summary-json "$summary_json"
  )
  if [[ "$use_stimulus" == "true" ]]; then
    prepare_args+=(--stimulus "$STIMULUS_FILE")
  fi
  "${prepare_args[@]}"

  echo "[eval] Scoring adversarial pressure for '$label'..."
  local emit_args=(
    cargo run --quiet --bin adversarial_cycle --
    --candidate-id "eval-${label}"
    --scenario "$scenario_path"
    --generation 0
    --metrics "$metrics_csv"
    --state "$TARGET_DIR/harness_state.json"
    --emit-json "$harness_json"
  )
  if [[ "$use_stimulus" == "true" ]]; then
    emit_args+=(--stimulus "$STIMULUS_FILE")
  fi
  "${emit_args[@]}"
}

run_scenario "extreme_mutation" "$ROOT_DIR/docs/examples/high_mutation.yaml" "true"

echo "[eval] Evaluation artifacts ready under $TARGET_DIR"
echo "[eval] To analyze, inspect the 'lineage_pressure' in '$TARGET_DIR/extreme_mutation_summary.json'"
echo "[eval] You can now run the TUI with: cargo run --bin pitch_tui -- $TARGET_DIR"