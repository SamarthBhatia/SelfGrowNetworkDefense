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
        # Sort and deduplicate
        df_stim = df_stim.sort_values("step")
        
        # Calculate Rolling Mean (Window = 50 steps) to smooth the noise
        # Re-index to ensure continuity
        min_step = df_stim["step"].min()
        max_step = df_stim["step"].max()
        full_idx = pd.RangeIndex(start=min_step, stop=max_step + 1, name="step")
        
        # Group by step first to handle multiple events per step
        df_grouped = df_stim.groupby("step")["intensity"].max()
        df_reindexed = df_grouped.reindex(full_idx, fill_value=0.0)
        
        df_stim = df_reindexed.rolling(window=50, center=True).mean().reset_index()
        df_stim.rename(columns={"index": "step", "intensity": "intensity_rolling"}, inplace=True)
    else:
        df_stim = pd.DataFrame({"step": [], "intensity_rolling": []})

    # 2. Load Telemetry (Lineage State)
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
    
    reaction_step = None
    saturation_step = None
    
    for rec in events:
        evt_wrapper = rec.get("event", {})
        if not evt_wrapper: continue
        evt_type = list(evt_wrapper.keys())[0]
        evt_data = evt_wrapper[evt_type]
        
        if evt_type == "StepSummary":
            current_step = evt_data["step"]
            
            # Record state at this step
            total_nodes = len(node_states) if len(node_states) > 0 else evt_data.get("cell_count", 0)
            if total_nodes > 0:
                defensive_count = sum(1 for s in node_states.values() if s == "IntrusionDetection")
                fraction = defensive_count / total_nodes
                history.append({"step": current_step, "fraction": fraction})
                
                # Check metrics
                if reaction_step is None and fraction >= 0.1:
                    reaction_step = current_step
                if saturation_step is None and fraction >= 0.9:
                    saturation_step = current_step
            
        elif evt_type == "LineageShift":
            cell_id = evt_data["cell_id"]
            root_id = cell_id.split("::")[0]
            node_states[root_id] = evt_data["lineage"]
            
        elif evt_type == "CellReplicated":
            cell_id = evt_data["cell_id"]
            root_id = cell_id.split("::")[0]
            if root_id not in node_states:
                node_states[root_id] = "Stem"

    df_state = pd.DataFrame(history)

    # 3. Plot
    fig, ax1 = plt.subplots(figsize=(10, 6))

    color_red = 'tab:red'
    ax1.set_xlabel('Simulation Step')
    ax1.set_ylabel('Attack Intensity (Rolling Mean)', color=color_red)
    
    if not df_stim.empty:
        # Plot as line, not fill, for clarity
        ax1.plot(df_stim['step'], df_stim['intensity_rolling'], color=color_red, linewidth=1.5, alpha=0.6, label='Attack Intensity')
        # Add very faint fill
        ax1.fill_between(df_stim['step'], 0, df_stim['intensity_rolling'], color=color_red, alpha=0.05)
    
    ax1.tick_params(axis='y', labelcolor=color_red)
    ax1.set_ylim(0, 1.2)

    ax2 = ax1.twinx()

    color_blue = 'tab:blue'
    ax2.set_ylabel('Fraction of Defensive Nodes', color=color_blue)
    if not df_state.empty:
        ax2.plot(df_state['step'], df_state['fraction'], color=color_blue, linewidth=2.5, label='Defensive Fraction')
    
    ax2.tick_params(axis='y', labelcolor=color_blue)
    ax2.set_ylim(0, 1.05)

    # Annotations
    if reaction_step:
        ax2.axvline(x=reaction_step, color='gray', linestyle='--', alpha=0.5)
        ax2.text(reaction_step, 0.5, f" Reaction\n t={reaction_step}", rotation=90, va='center', fontsize=9, color='gray')

    if saturation_step:
        ax2.axvline(x=saturation_step, color='green', linestyle='--', alpha=0.5)
        ax2.text(saturation_step, 0.5, f" Saturation\n t={saturation_step}", rotation=90, va='center', fontsize=9, color='green')

    plt.title("Correlation: Defense Adoption vs Attack Intensity")
    fig.tight_layout()
    
    plt.savefig(output_path, dpi=150)
    print(f"Saved correlation plot to {output_path}")

if __name__ == "__main__":
    main()