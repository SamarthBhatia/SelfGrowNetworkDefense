#!/usr/bin/env python3
"""
Visualize the spread of IntrusionDetection lineage across the Abilene topology.
Generates an animation or series of images showing the network state over time.

Usage:
    python3 scripts/visualize_abilene_results.py <config_yaml> <telemetry_jsonl> <output_dir>
"""

import yaml
import json
import networkx as nx
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
import sys
import os
from pathlib import Path

def main():
    if len(sys.argv) != 4:
        print("Usage: python3 scripts/visualize_abilene_results.py <config_yaml> <telemetry_jsonl> <output_dir>")
        sys.exit(1)

    config_path = sys.argv[1]
    telemetry_path = sys.argv[2]
    output_dir = Path(sys.argv[3])
    output_dir.mkdir(parents=True, exist_ok=True)

    # Load Mapping if exists
    mapping_path = config_path.replace(".yaml", "_mapping.json")
    label_map = {}
    if os.path.exists(mapping_path):
        with open(mapping_path, 'r') as f:
            label_map = json.load(f)
            print(f"Loaded node labels: {len(label_map)}")

    # 1. Load Topology from Config
    with open(config_path, 'r') as f:
        config = yaml.safe_load(f)
        
    G = nx.Graph()
    raw_links = config.get('topology', {}).get('explicit_links', [])
    
    def map_id(yaml_id):
        # "node_5" -> "seed-5"
        if yaml_id.startswith("node_"):
            return yaml_id.replace("node_", "seed-")
        return yaml_id

    for u, v in raw_links:
        u_prime = map_id(u)
        v_prime = map_id(v)
        G.add_edge(u_prime, v_prime)
        
    # Apply real labels
    if label_map:
        # We keep G nodes as seed-X for internal logic but will use labels for drawing
        pass

    # Layout
    pos = nx.spring_layout(G, seed=42, k=1.5) # Consistent layout, spread out

    # 2. Process Telemetry
    lineage_color_map = {
        'Stem': 'lightgray',
        'IntrusionDetection': '#ff6b6b', # Red
        'AdaptiveProbe': '#4ecdc4', # Teal
        'ResilientCore': '#45b7d1' # Blue
    }
    
    # State tracking
    node_states = {node: 'Stem' for node in G.nodes()}
    node_replications = {node: 0 for node in G.nodes()}
    
    snapshots = []
    
    print(f"Processing telemetry from {telemetry_path}...")
    
    with open(telemetry_path, 'r') as f:
        for line in f:
            try:
                rec = json.loads(line)
            except:
                continue
            
            evt_wrapper = rec.get("event", {})
            if not evt_wrapper: continue
            
            evt_type = list(evt_wrapper.keys())[0]
            evt_data = evt_wrapper[evt_type]
            
            if evt_type == "LineageShift":
                cell_id = evt_data["cell_id"]
                root_id = cell_id.split("::")[0]
                if root_id in node_states:
                    node_states[root_id] = evt_data["lineage"]
            
            elif evt_type == "CellReplicated":
                cell_id = evt_data["cell_id"]
                root_id = cell_id.split("::")[0]
                if root_id in node_replications:
                    node_replications[root_id] += 1
            
            elif evt_type == "StepSummary":
                step = evt_data["step"]
                # Snapshot at t=0 and t=final (or near final)
                if step == 0:
                    snapshots.append((0, node_states.copy(), node_replications.copy()))
                elif step % 2000 == 0: # Or just catch the last one
                     snapshots.append((step, node_states.copy(), node_replications.copy()))

    # Add final state if not exactly on modulo
    snapshots.append(("Final", node_states.copy(), node_replications.copy()))
    
    # Render Comparison Plot (Start vs End)
    fig, axes = plt.subplots(1, 2, figsize=(16, 8))
    
    times_to_plot = [snapshots[0], snapshots[-1]]
    titles = ["Initial State (t=0)", f"Final State (t={times_to_plot[1][0]})"]
    
    for ax, (time, states, reps), title in zip(axes, times_to_plot, titles):
        colors = [lineage_color_map.get(states[n], 'lightgray') for n in G.nodes()]
        
        # Labels: "Name\n(Reps: N)"
        labels = {}
        for n in G.nodes():
            real_name = label_map.get(n, n)
            labels[n] = f"{real_name}\n(Reps: {reps[n]})"
            
        nx.draw_networkx_nodes(G, pos, ax=ax, node_color=colors, node_size=1500, edgecolors='black')
        nx.draw_networkx_edges(G, pos, ax=ax, width=2, alpha=0.6)
        nx.draw_networkx_labels(G, pos, labels, ax=ax, font_size=8, font_weight='bold')
        
        ax.set_title(title, fontsize=14)
        ax.axis('off')

    # Legend
    patches = [mpatches.Patch(color=c, label=l) for l, c in lineage_color_map.items()]
    fig.legend(handles=patches, loc='lower center', ncol=4, fontsize=12)
    
    plt.tight_layout()
    plt.subplots_adjust(bottom=0.15)
    
    outfile = output_dir / "abilene_comparison.png"
    plt.savefig(outfile, dpi=150)
    print(f"Saved comparison plot to {outfile}")
    
    # Also save the "After" plot alone for detail
    plt.figure(figsize=(10, 8))
    time, states, reps = snapshots[-1]
    colors = [lineage_color_map.get(states[n], 'lightgray') for n in G.nodes()]
    labels = {n: f"{label_map.get(n, n)}\n(Reps: {reps[n]})" for n in G.nodes()}
    
    nx.draw_networkx_nodes(G, pos, node_color=colors, node_size=1500, edgecolors='black')
    nx.draw_networkx_edges(G, pos, width=2, alpha=0.6)
    nx.draw_networkx_labels(G, pos, labels, font_size=9, font_weight='bold')
    
    plt.title(f"Abilene Network Defense State (t={time})\nLineage Distribution & Reproduction Counts", fontsize=14)
    plt.axis('off')
    
    # Add legend
    plt.legend(handles=patches, loc='upper left')
    
    outfile_single = output_dir / "abilene_final_state.png"
    plt.savefig(outfile_single, dpi=150)
    print(f"Saved detailed final state to {outfile_single}")

if __name__ == "__main__":
    main()