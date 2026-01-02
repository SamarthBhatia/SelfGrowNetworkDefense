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
import sys
import os
from pathlib import Path

def load_topology(config_path):
    with open(config_path, 'r') as f:
        config = yaml.safe_load(f)
    
    G = nx.Graph()
    # Explicit links
    links = config.get('topology', {}).get('explicit_links', [])
    for link in links:
        G.add_edge(link[0], link[1])
    
    # Ensure all nodes from 0 to N-1 exist if not covered by links
    count = config.get('initial_cell_count', 0)
    for i in range(count):
        node_id = f"node_{i}" # Assuming default naming convention in main.rs isn't used for explicit? 
                              # Wait, main.rs uses `seed-{idx}`.
                              # The explicit links in yaml use `node_X`.
                              # We need to check how main.rs names cells.
        # Actually, let's check main.rs: `SecurityCell::new(format!("seed-{idx}"));`
        # BUT `config.explicit_links` uses whatever strings are in the YAML.
        # If the YAML uses `node_0` but main creates `seed-0`, the Graph strategy in `cellular.rs` or `orchestration.rs`
        # needs to map them.
        # Let's check `src/main.rs` and `src/cellular.rs` logic later.
        # For now, let's assume the naming in telemetry matches the config or we might have a disconnect.
        pass

    return G

def main():
    if len(sys.argv) != 4:
        print("Usage: python3 scripts/visualize_abilene_results.py <config_yaml> <telemetry_jsonl> <output_dir>")
        sys.exit(1)

    config_path = sys.argv[1]
    telemetry_path = sys.argv[2]
    output_dir = Path(sys.argv[3])
    output_dir.mkdir(parents=True, exist_ok=True)

    # 1. Load Topology
    # Note: We need to verify node naming.
    # The importer generated `node_X`.
    # `main.rs` generates `seed-X`.
    # If they don't match, the topology might not have been effectively applied or the visualization will mismatch.
    # Let's inspect telemetry to see node IDs.
    
    # Pre-scan telemetry to get node IDs
    node_ids = set()
    lineage_map = {} # cell_id -> lineage
    
    with open(telemetry_path, 'r') as f:
        for line in f:
            if "LineageShift" in line:
                data = json.loads(line)
                evt = data["event"]["LineageShift"]
                node_ids.add(evt["cell_id"])
                lineage_map[evt["cell_id"]] = "Stem" # Default start
            elif "CellReplicated" in line:
                 data = json.loads(line)
                 evt = data["event"]["CellReplicated"]
                 node_ids.add(evt["child_id"])
                 node_ids.add(evt["cell_id"])

    print(f"Found {len(node_ids)} active nodes in telemetry: {list(node_ids)[:5]}...")

    # Load static topology structure from YAML
    # We will map `node_X` from YAML to `seed-X` from main.rs if strictly numeric correspondence exists.
    # YAML: node_0, node_1...
    # Main: seed-0, seed-1...
    
    with open(config_path, 'r') as f:
        config = yaml.safe_load(f)
        
    G = nx.Graph()
    raw_links = config.get('topology', {}).get('explicit_links', [])
    
    # Mapper function
    def map_id(yaml_id):
        # "node_5" -> "seed-5"
        if yaml_id.startswith("node_"):
            return yaml_id.replace("node_", "seed-")
        return yaml_id

    for u, v in raw_links:
        G.add_edge(map_id(u), map_id(v))
        
    # Layout
    pos = nx.spring_layout(G, seed=42) # Consistent layout

    # 2. Process Telemetry and Snapshot
    # We'll take snapshots every N steps or on significant changes
    
    current_step = 0
    # Reset lineage state
    # All nodes start as 'Stem' (Gray)
    node_colors = {node: 'lightgray' for node in G.nodes()}
    
    lineage_color_map = {
        'Stem': 'lightgray',
        'IntrusionDetection': '#ff6b6b', # Red
        'AdaptiveProbe': '#4ecdc4', # Teal
        'ResilientCore': '#45b7d1' # Blue
    }
    
    snapshots = 0
    
    # We want to show cumulative state
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
            
            # Update step
            # Some events don't have step directly, usually we track StepSummary or assume sequential
            # But the recorder puts timestamp. We'll rely on StepSummary for frame boundaries?
            # Or just update state and periodically dump.
            
            if evt_type == "LineageShift":
                cell_id = evt_data["cell_id"]
                # Map child cells to root physical node (e.g., seed-3::child -> seed-3)
                root_id = cell_id.split("::")[0]
                
                lineage = evt_data["lineage"]
                
                # Priority: IntrusionDetection > AdaptiveProbe > ResilientCore > Stem
                current_color = node_colors.get(root_id, 'lightgray')
                new_color = lineage_color_map.get(lineage, 'lightgray')
                
                # Simple latch logic: once red, stay red (to show infection/protection spread)
                # Or just overwrite. Let's overwrite but prioritize Red.
                if new_color == '#ff6b6b': # Red
                     node_colors[root_id] = new_color
                elif current_color != '#ff6b6b': # Only update if not already Red
                     node_colors[root_id] = new_color
            
            elif evt_type == "StepSummary":
                step = evt_data["step"]
                # Save frame every 100 steps
                if step % 100 == 0:
                    plt.figure(figsize=(10, 8))
                    colors = [node_colors.get(n, 'lightgray') for n in G.nodes()]
                    
                    nx.draw(G, pos, 
                            node_color=colors, 
                            with_labels=True, 
                            node_size=800,
                            font_size=8,
                            font_weight='bold')
                            
                    plt.title(f"Step {step}: Lineage Distribution")
                    plt.savefig(output_dir / f"frame_{step:04d}.png")
                    plt.close()
                    snapshots += 1
                    print(f"Saved frame for step {step}", end='\r')

    # Final frame
    plt.figure(figsize=(10, 8))
    colors = [node_colors.get(n, 'lightgray') for n in G.nodes()]
    nx.draw(G, pos, node_color=colors, with_labels=True, node_size=800)
    plt.title(f"Final State: Lineage Distribution")
    plt.savefig(output_dir / "frame_final.png")
    plt.close()
    
    print(f"\nGenerated {snapshots + 1} frames in {output_dir}")

if __name__ == "__main__":
    main()
