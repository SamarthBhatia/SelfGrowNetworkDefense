#!/usr/bin/env python3
"""
Master script to run repeated experiments and gather statistical metrics.
Compares Attack vs. Controls (Zero, Shuffled, Block-Shuffled).
"""

import subprocess
import json
import numpy as np
import pandas as pd
import os
from pathlib import Path

# Paths
BIN = "target/release/morphogenetic-security"
CONFIG = "data/real_world_samples/abilene_scenario.yaml"
STIM_DIR = "data/real_world_samples/controls"
TELEMETRY_DIR = "data/real_world_samples/stats_runs"

# Experiments
EXPERIMENTS = {
    "Attack": "data/real_world_samples/real_stimulus.jsonl",
    "Zero-Pressure": "data/real_world_samples/zero_stimulus.jsonl",
    "Full-Shuffled": "data/real_world_samples/controls/shuffled_stimulus.jsonl",
    "Block-Shuffled": "data/real_world_samples/controls/block_shuffled_stimulus.jsonl",
    "Volume-Matched": "data/real_world_samples/controls/volume_matched_stimulus.jsonl"
}

NUM_RUNS = 5 # Reduced from 20 for time efficiency during demo setup, can be increased

def run_sim(stimulus, telemetry):
    cmd = [BIN, "--config", CONFIG, "--stimulus", stimulus, "--telemetry", telemetry]
    subprocess.run(cmd, capture_output=True)

def analyze(telemetry):
    shifts = 0
    replications = 0
    with open(telemetry, 'r') as f:
        for line in f:
            if "LineageShift" in line: shifts += 1
            if "CellReplicated" in line: replications += 1
    return shifts, replications

def main():
    Path(TELEMETRY_DIR).mkdir(parents=True, exist_ok=True)
    results = []

    for name, stim in EXPERIMENTS.items():
        print(f"Running Experiment: {name} ({NUM_RUNS} times)...")
        exp_shifts = []
        exp_reps = []
        
        for i in range(NUM_RUNS):
            tel = f"{TELEMETRY_DIR}/{name}_{i}.jsonl"
            run_sim(stim, tel)
            s, r = analyze(tel)
            exp_shifts.append(s)
            exp_reps.append(r)
            print(f"  Run {i+1}: Shifts={s}, Reps={r}", end='\r')
            # Clean up large telemetry files to save disk
            os.remove(tel)
        
        print(f"\n  Summary: Mean Shifts = {np.mean(exp_shifts):.2f} ± {np.std(exp_shifts):.2f}")
        results.append({
            "Experiment": name,
            "Mean Shifts": np.mean(exp_shifts),
            "Std Shifts": np.std(exp_shifts),
            "Mean Reps": np.mean(exp_reps),
            "Std Reps": np.std(exp_reps)
        })

    # Output table
    df = pd.DataFrame(results)
    print("\n" + "="*50)
    print("FINAL VALIDATION RESULTS")
    print("="*50)
    print(df.to_string(index=False))
    
    # Save to report (simple table)
    with open("docs/images/validation_stats.txt", "w") as f:
        f.write(df.to_string(index=False))
    print(f"\nSaved results to docs/images/validation_stats.txt")

if __name__ == "__main__":
    main()
