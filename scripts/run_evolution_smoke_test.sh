#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="$ROOT_DIR/target/evolution_smoke_test"
HARNESS_STATE="$TARGET_DIR/harness_state.json"
ARTIFACT_DIR="$TARGET_DIR/runs"

mkdir -p "$TARGET_DIR" "$ARTIFACT_DIR"
rm -f "$TARGET_DIR"/*.json "$ARTIFACT_DIR"/*

echo "[smoke-test] Initializing adversarial harness with a seed candidate..."

# Enqueue a seed candidate
cargo run --quiet --bin adversarial_loop -- \
  --state "$HARNESS_STATE" \
  --artifact-dir "$ARTIFACT_DIR" \
  --seed "initial=docs/examples/baseline-growth.yaml" \
  --generations 0 # Just enqueue, don't run generations yet

echo "[smoke-test] Running 2 generations of adversarial evolution..."

# Run a few generations of the adversarial loop
cargo run --quiet --bin adversarial_loop -- \
  --state "$HARNESS_STATE" \
  --artifact-dir "$ARTIFACT_DIR" \
  --generations 2

echo "[smoke-test] Verifying harness state and outcomes..."

# Assertions
if ! grep -q "backlog_len": 1" "$HARNESS_STATE"; then
    echo "Error: Expected backlog_len of 1 in harness_state.json after 2 generations."
    exit 1
fi

# Check for generated metrics (at least 2 runs: initial seed + subsequent generations)
num_outcomes=$(find "$ARTIFACT_DIR" -name "step_metrics.csv" | wc -l | tr -d ' ')
if [ "$num_outcomes" -lt 2 ]; then
    echo "Error: Expected at least 2 step_metrics.csv files, found $num_outcomes."
    exit 1
fi

echo "[smoke-test] Evolution smoke test passed successfully!"
