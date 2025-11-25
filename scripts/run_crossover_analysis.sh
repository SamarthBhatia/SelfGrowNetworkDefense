#!/bin/bash
set -e

# This script runs the adversarial evolution loop twice: once with no crossover
# and once with a high crossover rate, to analyze the impact of the crossover strategy.

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
PROJECT_ROOT="$SCRIPT_DIR/.."
HARNESS_STATE_NO_CROSSOVER="$PROJECT_ROOT/target/crossover_analysis/no_crossover/harness_state.json"
HARNESS_STATE_HIGH_CROSSOVER="$PROJECT_ROOT/target/crossover_analysis/high_crossover/harness_state.json"
ARTIFACT_DIR_NO_CROSSOVER="$PROJECT_ROOT/target/crossover_analysis/no_crossover"
ARTIFACT_DIR_HIGH_CROSSOVER="$PROJECT_ROOT/target/crossover_analysis/high_crossover"
SEED_SCENARIO="$PROJECT_ROOT/docs/examples/baseline-growth.yaml"

# Clean up previous runs
rm -rf "$ARTIFACT_DIR_NO_CROSSOVER"
rm -rf "$ARTIFACT_DIR_HIGH_CROSSOVER"
mkdir -p "$ARTIFACT_DIR_NO_CROSSOVER"
mkdir -p "$ARTIFACT_DIR_HIGH_CROSSOVER"

# Run with no crossover
echo "[info] Running evolution with no crossover..."
cargo run --bin adversarial_loop -- \
    --state "$HARNESS_STATE_NO_CROSSOVER" \
    --artifact-dir "$ARTIFACT_DIR_NO_CROSSOVER" \
    --seed "seed-0=$SEED_SCENARIO" \
    --generations 5 \
    --crossover-rate 0.0

# Run with high crossover
echo "[info] Running evolution with high crossover..."
cargo run --bin adversarial_loop -- \
    --state "$HARNESS_STATE_HIGH_CROSSOVER" \
    --artifact-dir "$ARTIFACT_DIR_HIGH_CROSSOVER" \
    --seed "seed-0=$SEED_SCENARIO" \
    --generations 5 \
    --crossover-rate 0.8

echo "[info] Crossover analysis runs complete."
echo "  No crossover artifacts in: $ARTIFACT_DIR_NO_CROSSOVER"
echo "  High crossover artifacts in: $ARTIFACT_DIR_HIGH_CROSSOVER"
