#!/bin/bash
set -e

# This script runs the adversarial evolution loop with and without adaptive mutation
# to analyze the impact of the adaptive mutation strategy.

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
PROJECT_ROOT="$SCRIPT_DIR/.."
ARTIFACT_DIR_BASE="$PROJECT_ROOT/target/adaptive_mutation_analysis"
SEED_SCENARIO="$PROJECT_ROOT/docs/examples/baseline-growth.yaml"

ADAPTIVE_MUTATION_STRATEGIES=("true" "false")

# Clean up previous runs
rm -rf "$ARTIFACT_DIR_BASE"
mkdir -p "$ARTIFACT_DIR_BASE"

for strategy in "${ADAPTIVE_MUTATION_STRATEGIES[@]}"; do
    echo "[info] Running evolution with adaptive_mutation: $strategy"
    ARTIFACT_DIR="$ARTIFACT_DIR_BASE/$strategy"
    HARNESS_STATE="$ARTIFACT_DIR/harness_state.json"
    mkdir -p "$ARTIFACT_DIR"

    ADAPTIVE_MUTATION_FLAG="$strategy" cargo run --bin adversarial_loop -- \
        --state "$HARNESS_STATE" \
        --artifact-dir "$ARTIFACT_DIR" \
        --seed "seed-0=$SEED_SCENARIO" \
        --generations 10
done

echo "[info] Adaptive mutation analysis runs complete."
for strategy in "${ADAPTIVE_MUTATION_STRATEGIES[@]}"; do
    echo "  $strategy artifacts in: $ARTIFACT_DIR_BASE/$strategy"
done
