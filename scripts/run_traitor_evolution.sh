#!/bin/bash
# Phase 4 Validation: Traitor Cell Evolution
# Verifies that the system can evolve resistance to a persistent traitor.

set -e

STATE_FILE="target/traitor_evolution_state.json"
ARTIFACT_DIR="target/traitor_evolution_runs"
SCENARIO="docs/examples/traitor-cell.yaml"
STIMULUS="docs/examples/traitor-stimulus.jsonl"

rm -rf $ARTIFACT_DIR
rm -f $STATE_FILE

echo "[info] Starting Traitor Cell Evolution Validation..."

# Run 5 generations
cargo run --bin adversarial_loop -- \
    --state $STATE_FILE \
    --artifact-dir $ARTIFACT_DIR \
    --seed traitor_seed=$SCENARIO \
    --stimulus $STIMULUS \
    --generations 5 \
    --batch-size 2

echo "[info] Evolution complete. Analyzing results..."

# Check if fitness improves or genome drifts
python3 scripts/visualize_genome_drift.py --artifact-dir $ARTIFACT_DIR --output-dir target/traitor_drift_plots

echo "[info] Validation finished. Check target/traitor_drift_plots/genome_drift_trends.png"
