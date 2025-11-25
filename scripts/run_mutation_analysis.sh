#!/bin/bash
set -e

# This script runs the adversarial evolution loop with different mutation strategies
# to analyze their impact.

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
PROJECT_ROOT="$SCRIPT_DIR/.."
ARTIFACT_DIR_BASE="$PROJECT_ROOT/target/mutation_analysis"
SEED_SCENARIO="$PROJECT_ROOT/docs/examples/baseline-growth.yaml"

MUTATION_STRATEGIES=("Random")

# Clean up previous runs
rm -rf "$ARTIFACT_DIR_BASE"
mkdir -p "$ARTIFACT_DIR_BASE"

for strategy in "${MUTATION_STRATEGIES[@]}"; do
    echo "[info] Running evolution with mutation strategy: $strategy"
    ARTIFACT_DIR="$ARTIFACT_DIR_BASE/$strategy"
    HARNESS_STATE="$ARTIFACT_DIR/harness_state.json"
    mkdir -p "$ARTIFACT_DIR"

    cargo run --bin adversarial_loop -- \
        --state "$HARNESS_STATE" \
        --artifact-dir "$ARTIFACT_DIR" \
        --seed "seed-0=$SEED_SCENARIO" \
        --generations 5 \
        --mutation-strategy "$strategy"
done

echo "[info] Mutation analysis runs complete."
for strategy in "${MUTATION_STRATEGIES[@]}"; do
    echo "  $strategy artifacts in: $ARTIFACT_DIR_BASE/$strategy"
done
