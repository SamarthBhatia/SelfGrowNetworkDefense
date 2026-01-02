#!/usr/bin/env python3
"""
Visualize evolution of IntrusionDetection fraction vs Attack Intensity.
Overlays two time series to show correlation (or lack thereof).

Usage:
    python3 scripts/visualize_correlation.py <telemetry_jsonl> <stimulus_jsonl> <output_png>
"""

import json
import argparse
import pandas as pd
import matplotlib.pyplot as plt
import sys

def main():
    if len(sys.argv) != 4:
        print("Usage: python3 scripts/visualize_correlation.py <telemetry_jsonl> <stimulus_jsonl> <output_png>")
        sys.exit(1)

    telemetry_path = sys.argv[1]
    stimulus_path = sys.argv[2]
    output_path = sys.argv[3]

    print(f"Correlating {telemetry_path} with {stimulus_path}...")

    # 1. Load Stimulus (Attack Intensity)
    stimulus_data = []
    with open(stimulus_path, 'r') as f:
        for line in f:
            if not line.strip(): continue
            try:
                rec = json.loads(line)
                # We care about 'threat' or 'activator' value
                if rec.get("topic") in ["threat", "activator"]:
                    stimulus_data.append({
                        "step": rec["step"],
                        "intensity": rec["value"]
                    })
            except:
                continue
    
    df_stim = pd.DataFrame(stimulus_data)
    if not df_stim.empty:
        # Aggregate per step (max intensity)
        df_stim = df_stim.groupby("step")["intensity"].max().reset_index()
    else:
        df_stim = pd.DataFrame({"step": [], "intensity": []})

    # 2. Load Telemetry (Lineage State)
    # We need to track the fraction of nodes in IntrusionDetection over time.
    # We can infer state from LineageShift events.
    
    events = []
    with open(telemetry_path, 'r') as f:
        for line in f:
            try:
                rec = json.loads(line)
                events.append(rec)
            except:
                continue

    # Replay simulation state
    current_step = 0
    node_states = {} # cell_id -> lineage
    history = []
    
    # Pre-scan for total nodes to normalize fraction? 
    # Or just track unique nodes seen.
    # Better: Assume total nodes = max unique cells seen?
    # Or use StepSummary cell_count.
    
    step_summaries = {}
    
    for rec in events:
        evt_wrapper = rec.get("event", {})
        if not evt_wrapper: continue
        evt_type = list(evt_wrapper.keys())[0]
        evt_data = evt_wrapper[evt_type]
        
        if evt_type == "StepSummary":
            current_step = evt_data["step"]
            step_summaries[current_step] = evt_data.get("cell_count", 0)
            
            # Record state at this step
            total_nodes = len(node_states) if len(node_states) > 0 else evt_data.get("cell_count", 0)
            if total_nodes > 0:
                defensive_count = sum(1 for s in node_states.values() if s == "IntrusionDetection")
                fraction = defensive_count / total_nodes
                history.append({"step": current_step, "fraction": fraction})
            
        elif evt_type == "LineageShift":
            cell_id = evt_data["cell_id"]
            root_id = cell_id.split("::")[0]
            node_states[root_id] = evt_data["lineage"]
            
        elif evt_type == "CellReplicated":
            # Just tracking existence for now
            cell_id = evt_data["cell_id"]
            root_id = cell_id.split("::")[0]
            if root_id not in node_states:
                node_states[root_id] = "Stem"

    df_state = pd.DataFrame(history)

    # 3. Plot
    fig, ax1 = plt.subplots(figsize=(10, 6))

    color = 'tab:red'
    ax1.set_xlabel('Simulation Step')
    ax1.set_ylabel('Attack Intensity', color=color)
    if not df_stim.empty:
        ax1.plot(df_stim['step'], df_stim['intensity'], color=color, alpha=0.3, label='Attack Intensity')
        ax1.fill_between(df_stim['step'], 0, df_stim['intensity'], color=color, alpha=0.1)
    ax1.tick_params(axis='y', labelcolor=color)
    ax1.set_ylim(0, 1.2)

    ax2 = ax1.twinx()  # instantiate a second axes that shares the same x-axis

    color = 'tab:blue'
    ax2.set_ylabel('Fraction of Defensive Nodes', color=color)  # we already handled the x-label with ax1
    if not df_state.empty:
        ax2.plot(df_state['step'], df_state['fraction'], color=color, linewidth=2, label='Defensive Fraction')
    ax2.tick_params(axis='y', labelcolor=color)
    ax2.set_ylim(0, 1.05)

    plt.title("Correlation: Defense Adoption vs Attack Intensity")
    fig.tight_layout()  # otherwise the right y-label is slightly clipped
    
    plt.savefig(output_path)
    print(f"Saved correlation plot to {output_path}")

if __name__ == "__main__":
    main()
