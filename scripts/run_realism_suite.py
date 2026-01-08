#!/usr/bin/env python3
"""
Realism Expansion Suite Runner.

Scans `data/external/topologies` and `data/external/traffic` to execute
a cross-product simulation matrix (Every Topology x Every Dataset).

Generates a matrix of Defense Activation scores to prove "Burstiness Sensitivity"
across diverse environments.
"""

import os
import glob
import subprocess
import json
import yaml
import shutil
import sys
import pandas as pd
import numpy as np
from pathlib import Path

# Paths
BASE_DIR = Path(__file__).parent.parent
EXTERNAL_DIR = BASE_DIR / "data" / "external"
TOPO_DIR = EXTERNAL_DIR / "topologies"
TRAFFIC_DIR = EXTERNAL_DIR / "traffic"
OUTPUT_DIR = BASE_DIR / "target" / "realism_results"

def ensure_dirs():
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

def run_command(cmd, log_file=None):
    """Run a shell command and return stdout."""
    try:
        if log_file:
            with open(log_file, 'w') as f:
                subprocess.check_call(cmd, shell=True, stdout=f, stderr=subprocess.STDOUT)
        else:
            subprocess.check_call(cmd, shell=True)
        return True
    except subprocess.CalledProcessError as e:
        print(f"Error running command: {cmd}")
        return False

def get_node_count(graphml_path):
    import networkx as nx
    try:
        G = nx.read_graphml(graphml_path)
        return G.number_of_nodes()
    except:
        return 0

def parse_telemetry(log_file, num_nodes):
    """Extract mean defense activation from the run."""
    if not log_file.exists():
        return 0.0
        
    try:
        # We need to manually parse because 'event' is a struct
        cell_counts = []
        with open(log_file, 'r') as f:
            for line in f:
                try:
                    record = json.loads(line)
                    event = record.get('event', {})
                    if 'StepSummary' in event:
                        summary = event['StepSummary']
                        cell_counts.append(summary.get('cell_count', 0))
                except:
                    continue
        
        if cell_counts and num_nodes > 0:
            return np.mean(cell_counts) / num_nodes
    except Exception as e:
        print(f"Error parsing telemetry: {e}")
        pass
    return 0.0

def main():
    ensure_dirs()
    
    topologies = [f for f in sorted(list(TOPO_DIR.glob("*.graphml"))) if not f.name.startswith("._")]
    datasets = [f for f in sorted(list(TRAFFIC_DIR.glob("*.csv"))) if not f.name.startswith("._")]
    
    if not topologies:
        print(f"No topologies found in {TOPO_DIR}. Please add .graphml files.")
        return
    if not datasets:
        print(f"No traffic datasets found in {TRAFFIC_DIR}. Please add .csv files.")
        return

    print(f"Found {len(topologies)} topologies and {len(datasets)} datasets.")
    print("Starting execution matrix...")
    
    results = []

    for topo in topologies:
        topo_name = topo.stem
        node_count = get_node_count(topo)
        print(f"\n--- Topology: {topo_name} ({node_count} nodes) ---")
        
        # 1. Import Topology
        scenario_file = OUTPUT_DIR / f"{topo_name}_scenario.yaml"
        cmd = f"{sys.executable} scripts/importers/import_topology.py '{topo}' '{scenario_file}' --duration 300"
        if not run_command(cmd):
            continue
            
        for dataset in datasets:
            data_name = dataset.stem
            print(f"  > Dataset: {data_name}")
            
            # 2. Convert Stimulus (Spatial Hash Strategy)
            stimulus_file = OUTPUT_DIR / f"{data_name}_{node_count}_stimulus.jsonl"
            
            # Only generate if not exists or force (optimization)
            if not stimulus_file.exists():
                cmd = (f"{sys.executable} scripts/importers/pcap_to_stimulus.py '{dataset}' '{stimulus_file}' "
                       f"--nodes {node_count} --strategy hash --duration 300")
                if not run_command(cmd):
                    print("    Failed to convert stimulus.")
                    continue
            
            # 3. Run Simulation
            log_file = OUTPUT_DIR / f"{topo_name}_{data_name}.log"
            telemetry_file = OUTPUT_DIR / f"{topo_name}_{data_name}_telemetry.jsonl"
            
            print(f"    Running Simulation...")
            cmd = (f"cargo run --release --bin morphogenetic-security -- --config '{scenario_file}' "
                   f"--stimulus '{stimulus_file}' --telemetry '{telemetry_file}'")
            
            if run_command(cmd, log_file=log_file):
                # 4. Collect Stats
                score = parse_telemetry(telemetry_file, node_count) 
                print(f"    Result: Defense Density = {score:.4f}")
                results.append({
                    "Topology": topo_name,
                    "Dataset": data_name,
                    "Nodes": node_count,
                    "Defense_Density": score
                })
            else:
                print("    Simulation failed.")

    # Save Results
    if results:
        df_res = pd.DataFrame(results)
        csv_path = OUTPUT_DIR / "matrix_results.csv"
        df_res.to_csv(csv_path, index=False)
        print(f"\nMatrix completed. Results saved to {csv_path}")
        try:
            print(df_res.pivot(index='Topology', columns='Dataset', values='Defense_Density'))
        except Exception as e:
            print(f"Could not print pivot table: {e}")
    else:
        print("\nNo results collected.")

if __name__ == "__main__":
    main()