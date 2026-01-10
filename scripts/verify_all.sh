#!/usr/bin/env bash
set -e

# verify_all.sh
# Runs the complete verification suite (Rust, Python, Integration) in one command.

echo "========================================"
echo " Morphogenetic Security Verification"
echo "========================================"

echo "[1/4] Rust Unit & Integration Tests..."
cargo test

echo "[2/4] Building Binaries..."
# Ensure binaries used by Python harnesses are built
cargo build --quiet --bin morphogenetic-security
cargo build --quiet --bin stimulus
cargo build --quiet --bin adversarial_loop

echo "[3/4] Python Test Suites..."
# Validation Suite (requires built binary)
echo "   - Validation Suite..."
python3 tests/test_validation_suite.py

# Realism Suite (verifies topology/importer logic)
echo "   - Realism Suite..."
python3 tests/test_realism_suite.py

# Plotting & Analytics
echo "   - Analytics & Plotting..."
python3 tests/test_analytics.py
python3 tests/test_plotting_scripts.py
python3 tests/test_importers.py

echo "[4/4] Smoke Tests (End-to-End)..."
echo "   - Pitch Demo Logic..."
bash tests/smoke_test_pitch_demo.sh
echo "   - Attack Simulation..."
bash scripts/run_attack_simulation.sh
echo "   - Evolutionary Loop..."
bash scripts/run_evolution_smoke_test.sh

echo "========================================"
echo " VERIFICATION COMPLETE: ALL SYSTEMS GO"
echo "========================================"
