#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="$ROOT_DIR/target"
TELEMETRY_FILE="$TARGET_DIR/ci_attack_telemetry.jsonl"
STIMULUS_FILE="$TARGET_DIR/ci_stimulus.jsonl"

mkdir -p "$TARGET_DIR"
rm -f "$TELEMETRY_FILE" "$STIMULUS_FILE"

# Seed a simple stimulus schedule
cargo run --quiet --bin stimulus -- "$STIMULUS_FILE" activator 0.9 3
cargo run --quiet --bin stimulus -- "$STIMULUS_FILE" inhibitor 0.4 5

# Execute the morphogenetic runtime against the intense-defense scenario
cargo run --quiet -- \
  --config "$ROOT_DIR/docs/examples/intense-defense.yaml" \
  --telemetry "$TELEMETRY_FILE" \
  --stimulus "$STIMULUS_FILE"

# Summaries for quick diagnostics
python3 "$ROOT_DIR/scripts/analyze_telemetry.py" "$TELEMETRY_FILE" --limit 50
python3 "$ROOT_DIR/scripts/telemetry_correlate.py" "$TELEMETRY_FILE" --stimulus "$STIMULUS_FILE"

printf "Telemetry artifacts saved to %s\n" "$TARGET_DIR"
