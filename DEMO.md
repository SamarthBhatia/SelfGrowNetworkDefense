# Morphogenetic Security Demonstration Guide

This guide describes the complete validation suite for the Morphogenetic Security architecture, demonstrating its evolutionary adaptation to real-world botnet traffic and proving its logical soundness through rigorous controls.

## 1. Prerequisites
- **Rust Toolchain**: `cargo build --release` (must be compiled)
- **Python 3**: `pandas`, `matplotlib`, `networkx`, `scikit-learn` (optional for advanced metrics)

## 2. Experiments Overview
We run a comparative suite of 5 experiments against the **Abilene Network Topology** (11 nodes).

| Experiment | Stimulus Source | Purpose | Expected Outcome |
| :--- | :--- | :--- | :--- |
| **Attack** | Mirai Botnet Trace (UNSW-2018) | Test defense activation | High Shifts (~100+) |
| **Zero-Pressure** | Empty Trace | Sanity Check (Stability) | 0 Shifts |
| **Full-Shuffled** | Time-Randomized Benign Trace | Test Temporal Sensitivity | Low Shifts (<10%) |
| **Block-Shuffled** | Block-Randomized Attack Trace | Test Burst Sensitivity | High Shifts (Restored) |
| **Volume-Matched** | Amplified Benign Trace | Test Volume Sensitivity | High Shifts (Hypersensitivity) |

## 3. Running the Validation Suite
A master script automates the execution of all 5 experiments (5 runs each) and generates a statistical summary.

```bash
# Ensure release build is ready
cargo build --release

# Run the suite (approx. 2-5 minutes)
python3 scripts/run_validation_suite.py
```

**Output:**
- Console summary table (Mean ± Std Dev).
- Artifacts saved to `data/real_world_samples/stats_runs/`.
- Summary text file: `docs/images/validation_stats.txt`.

## 4. Visualizing Results
After running the suite (or a single run), you can generate the thesis visualizations:

### A. Network State (Topology)
Visualizes the spread of the `IntrusionDetection` lineage across the Abilene graph.

```bash
# Run single attack instance
cargo run --release --bin morphogenetic-security -- --config data/real_world_samples/abilene_scenario.yaml --stimulus data/real_world_samples/real_stimulus.jsonl --telemetry data/real_world_samples/validation_telemetry.jsonl

# Generate comparison plot (Start vs End)
python3 scripts/visualize_abilene_results.py data/real_world_samples/abilene_scenario.yaml data/real_world_samples/validation_telemetry.jsonl data/real_world_samples/viz_output
```
**Artifact:** `docs/images/abilene_comparison.png`

### B. Defense Correlation
Visualizes the system's reaction time and saturation relative to attack intensity.

```bash
python3 scripts/visualize_correlation.py data/real_world_samples/validation_telemetry.jsonl data/real_world_samples/real_stimulus.jsonl docs/images/defense_correlation.png
```
**Artifact:** `docs/images/defense_correlation.png`

### C. Stability Analysis
Visualizes the rate of adaptation to prove stability (no wild oscillations).

```bash
python3 scripts/analyze_stability.py data/real_world_samples/validation_telemetry.jsonl
```
**Artifacts:** `docs/images/adaptation_over_time.png`, `docs/images/shifts_histogram.png`

## 5. Key Findings
1.  **Temporal Intelligence**: The system ignores unstructured noise (Full-Shuffled) but reacts to coordinated bursts (Block-Shuffled).
2.  **Homeostasis**: The system is perfectly stable at rest (Zero-Pressure).
3.  **Hypersensitivity**: Current limitation—amplified benign traffic triggers defense, motivating future work in baseline normalization.
