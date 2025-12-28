#!/bin/bash
# Phase 4 Validation: Hostile Environment Evolution
# Verifies that the system evolves when under high, distributed pressure.

set -e

STATE_FILE="target/hostile_evolution_state.json"
ARTIFACT_DIR="target/hostile_evolution_runs"
SCENARIO="docs/examples/distributed-pressure.yaml"
STIMULUS="docs/examples/coordinated-attack.jsonl"

rm -rf $ARTIFACT_DIR
rm -f $STATE_FILE

echo "[info] Starting Hostile Environment Evolution Validation..."

# Run 3 generations, batch 2
cargo run --release --bin adversarial_loop -- \
    --state $STATE_FILE \
    --artifact-dir $ARTIFACT_DIR \
    --seed hostile_seed=$SCENARIO \
    --stimulus $STIMULUS \
    --generations 3 \
    --batch-size 2

echo "[info] Evolution complete. Analyzing results..."

# Check if fitness improves or genome drifts
python3 scripts/visualize_genome_drift.py --artifact-dir $ARTIFACT_DIR --output-dir target/hostile_drift_plots

echo "[info] Validation finished. Check target/hostile_drift_plots/genome_drift_trends.png"
