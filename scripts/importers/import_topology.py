#!/usr/bin/env python3
"""
Topology Zoo Importer for Morphogenetic Security Scenarios.

Usage:
    python3 scripts/importers/import_topology.py <graphml_file> <output_yaml>

Dependencies:
    pip install networkx pyyaml
"""

import sys
import yaml
import json
import networkx as nx
import argparse
import os

def main():
    if len(sys.argv) != 3:
        print("Usage: python3 import_topology.py <graphml_file> <output_yaml>")
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2]

    print(f"[import] Reading GraphML from {input_path}...")
    try:
        # read_graphml handles standard GraphML.
        G = nx.read_graphml(input_path)
    except Exception as e:
        print(f"Error reading GraphML: {e}")
        sys.exit(1)

    # Create mapping: old_id -> new_id
    # We save this to a sidecar JSON file for visualization
    mapping = {old_id: f"node_{i}" for i, old_id in enumerate(G.nodes())}
    
    # Save mapping
    mapping_path = output_path.replace(".yaml", "_mapping.json")
    with open(mapping_path, 'w') as f:
        # Invert mapping for viz lookup: node_0 -> "New York"
        # The internal simulation uses "seed-0" for "node_0".
        # So we want seed-0 -> "New York".
        
        # NOTE: Simulator Main Loop:
        # SecurityCell::new(format!("seed-{idx}"));
        # So node_0 corresponds to seed-0.
        
        viz_mapping = {f"seed-{i}": old_id for i, old_id in enumerate(G.nodes())}
        json.dump(viz_mapping, f, indent=2)
    print(f"[import] Saved node mapping to {mapping_path}")

    G = nx.relabel_nodes(G, mapping)
    
    num_nodes = G.number_of_nodes()
    num_edges = G.number_of_edges()
    
    print(f"[import] Graph stats: {num_nodes} nodes, {num_edges} edges.")

    # Convert edges to a list of [source, target] pairs
    edges = [[u, v] for u, v in G.edges()]

    # Construct the ScenarioConfig dictionary
    config = {
        "scenario_name": f"Imported Topology: {input_path}",
        "description": "Auto-generated from Internet Topology Zoo GraphML.",
        "simulation_steps": 100,
        "initial_cell_count": num_nodes,
        "topology": {
            "strategy": "Graph",
            "explicit_links": edges
        },
        "threat_profile": {
            "background_threat_level": 0.1,
            "spike_probability": 0.05,
            "spike_intensity_min": 0.4,
            "spike_intensity_max": 0.8,
            "spike_duration": 5,
            "spike_threshold": 0.8
        },
        "cell_reproduction_rate": 0.0,
        "cell_death_rate": 0.0
    }

    print(f"[import] Writing scenario to {output_path}...")
    with open(output_path, 'w') as f:
        yaml.dump(config, f, sort_keys=False)

    print("[import] Done.")

if __name__ == "__main__":
    main()