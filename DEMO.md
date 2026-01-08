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
3. **Hypersensitivity**: Current limitation—amplified benign traffic triggers defense, motivating future work in baseline normalization.

## 6. Realism Expansion Matrix (Tier 1 Validity)
To prove the architecture's generality beyond a single topology, we run a "Realism Matrix" crossing multiple topologies with diverse datasets.

### Supported Topologies
- **Abilene** (11 Nodes, US Research Backbone)
- **Geant2012** (40 Nodes, Pan-European Research Network)

### Supported Datasets
- **UNSW-NB15 / IoT Botnet** (Packet-level PCAP)
- **CICIoT2023** (Flow-based CSV, timestamps synthesized)

### Running the Matrix
This script downloads necessary topologies, imports traffic logs, and runs the full cross-product simulation (Topology x Dataset).

```bash
# Downloads Geant2012 and Abilene
bash scripts/download_topologies.sh

# Links or places CICIoT2023 CSVs in data/external/traffic/
# (Assuming you have sample CSVs linked)

# Runs the suite
python3 scripts/run_realism_suite.py
```

**Output:**
- Matrix CSV: `target/realism_results/matrix_results.csv`
- Demonstrates consistent defense activation (saturation) across 40-node and 11-node networks under massive botnet pressure.
