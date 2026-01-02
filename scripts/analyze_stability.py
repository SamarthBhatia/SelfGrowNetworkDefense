#!/usr/bin/env python3
"""
Analyze telemetry to detect oscillation vs adaptation.
Plots:
1. Replications & Lineage Shifts over time (per 100 steps)
2. Histogram of Lineage Shifts per Node
"""

import json
import argparse
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import sys
from pathlib import Path

def main():
    parser = argparse.ArgumentParser(description="Analyze stability metrics.")
    parser.add_argument("telemetry_path", type=str, help="Path to telemetry.jsonl")
    parser.add_argument("--output-dir", type=str, default="data/real_world_samples/viz_output")
    args = parser.parse_args()

    Path(args.output_dir).mkdir(parents=True, exist_ok=True)

    print(f"Analyzing {args.telemetry_path}...")
    
    events = []
    
    with open(args.telemetry_path, 'r') as f:
        for line in f:
            try:
                rec = json.loads(line)
            except:
                continue
            
            # Extract timestamp/step approximation if needed, 
            # but usually StepSummary gives us the clock.
            # We will use 'StepSummary' to map wall-time to steps if needed, 
            # or just assume the 'step' field in summary.
            
            # Since individual events like LineageShift don't strictly have a step field in the JSON wrapper 
            # (they are just events in the stream), we need to infer step from the nearest StepSummary 
            # or the global order. 
            # The simulator emits StepSummary at the END of a step.
            
            # Strategy: Read sequentially, update current_step when StepSummary seen.
            events.append(rec)

    df_rows = []
    current_step = 0
    
    shifts_per_node = {} # cell_id -> count
    
    for rec in events:
        evt_wrapper = rec.get("event", {})
        if not evt_wrapper: continue
        evt_type = list(evt_wrapper.keys())[0]
        evt_data = evt_wrapper[evt_type]
        
        if evt_type == "StepSummary":
            current_step = evt_data["step"]
        
        elif evt_type == "LineageShift":
            cell_id = evt_data["cell_id"]
            root_id = cell_id.split("::")[0]
            shifts_per_node[root_id] = shifts_per_node.get(root_id, 0) + 1
            df_rows.append({
                "step": current_step,
                "type": "LineageShift",
                "count": 1
            })
            
        elif evt_type == "CellReplicated":
             df_rows.append({
                "step": current_step,
                "type": "Replication",
                "count": 1
            })

    if not df_rows:
        print("No relevant events found.")
        sys.exit(0)
        
    df = pd.DataFrame(df_rows)
    
    # 1. Events over Time (binned)
    df['bin'] = (df['step'] // 100) * 100
    binned = df.groupby(['bin', 'type'])['count'].sum().unstack(fill_value=0)
    
    plt.figure(figsize=(10, 6))
    if 'LineageShift' in binned.columns:
        plt.plot(binned.index, binned['LineageShift'], label='Lineage Shifts', marker='o')
    if 'Replication' in binned.columns:
        plt.plot(binned.index, binned['Replication'], label='Replications', marker='x', linestyle='--')
        
    plt.title("Adaptation Events over Time (per 100 steps)")
    plt.xlabel("Simulation Step")
    plt.ylabel("Event Count")
    plt.legend()
    plt.grid(True, alpha=0.3)
    
    outfile_time = f"{args.output_dir}/adaptation_over_time.png"
    plt.savefig(outfile_time)
    print(f"Saved time series to {outfile_time}")
    
    # 2. Histogram of Shifts per Node
    plt.figure(figsize=(10, 6))
    counts = list(shifts_per_node.values())
    if counts:
        plt.hist(counts, bins=range(0, max(counts)+2), align='left', rwidth=0.8, color='purple')
        plt.title("Histogram of Lineage Shifts per Node")
        plt.xlabel("Number of Shifts")
        plt.ylabel("Count of Nodes")
        plt.xticks(range(0, max(counts)+1))
        plt.grid(axis='y', alpha=0.3)
        
        outfile_hist = f"{args.output_dir}/shifts_histogram.png"
        plt.savefig(outfile_hist)
        print(f"Saved histogram to {outfile_hist}")
        
        # Stats
        print(f"Mean shifts per node: {np.mean(counts):.2f}")
        print(f"Max shifts per node: {np.max(counts)}")
    else:
        print("No lineage shifts recorded.")

if __name__ == "__main__":
    main()
