#!/usr/bin/env bash
set -e

# Define the root directory for the analysis artifacts
ARTIFACT_ROOT="target/targeted_mutation_analysis"
rm -rf "$ARTIFACT_ROOT"
mkdir -p "$ARTIFACT_ROOT"

# --- Run with Random Mutation ---
RANDOM_RUN_DIR="$ARTIFACT_ROOT/random"
mkdir -p "$RANDOM_RUN_DIR"
echo "Running with Random mutation strategy..."
cargo run --bin adversarial_loop -- \
    --state "$RANDOM_RUN_DIR/harness_state.json" \
    --generations 5 \
    --artifact-dir "$RANDOM_RUN_DIR" \
    --mutation-strategy Random \
    --seed "baseline=docs/examples/baseline-growth.yaml" \
    --seed "intense=docs/examples/intense-defense.yaml"

# --- Run with Targeted Mutation ---
TARGETED_RUN_DIR="$ARTIFACT_ROOT/targeted"
mkdir -p "$TARGETED_RUN_DIR"
echo "Running with Targeted mutation strategy..."
cargo run --bin adversarial_loop -- \
    --state "$TARGETED_RUN_DIR/harness_state.json" \
    --generations 5 \
    --artifact-dir "$TARGETED_RUN_DIR" \
    --mutation-strategy Targeted \
    --seed "baseline=docs/examples/baseline-growth.yaml" \
    --seed "intense=docs/examples/intense-defense.yaml"

echo "Targeted mutation analysis runs complete."
echo "Artifacts are in $ARTIFACT_ROOT"
