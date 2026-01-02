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
import networkx as nx
import argparse

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

    # Convert node labels to clean string IDs (e.g., node_0, node_1)
    # Mapping original IDs to simulation IDs if needed, but for now we just need counts and edges.
    # Actually, we need explicit names to map the edges correctly in the config.
    
    mapping = {old_id: f"node_{i}" for i, old_id in enumerate(G.nodes())}
    G = nx.relabel_nodes(G, mapping)
    
    num_nodes = G.number_of_nodes()
    num_edges = G.number_of_edges()
    
    print(f"[import] Graph stats: {num_nodes} nodes, {num_edges} edges.")

    # Convert edges to a list of [source, target] pairs
    edges = [[u, v] for u, v in G.edges()]

    # Construct the ScenarioConfig dictionary
    # We follow the schema from src/config.rs
    config = {
        "scenario_name": f"Imported Topology: {input_path}",
        "description": "Auto-generated from Internet Topology Zoo GraphML.",
        "simulation_steps": 100,
        "initial_cell_count": num_nodes,
        "topology": {
            "strategy": "Graph",
            # We need to add support for this field in src/config.rs:
            "explicit_links": edges
        },
        "threat_profile": {
            "background_threat_level": 0.1,
            "spike_probability": 0.05,
            "spike_intensity_min": 0.4,
            "spike_intensity_max": 0.8,
            "spike_duration": 5,
            "spike_threshold": 0.8  # Updated default hardening
        },
        "cell_reproduction_rate": 0.0, # Static topology usually implies fixed hardware? Or allow growth?
                                       # Let's default to 0.0 (static hardware) for topology imports.
        "cell_death_rate": 0.0
    }

    print(f"[import] Writing scenario to {output_path}...")
    with open(output_path, 'w') as f:
        yaml.dump(config, f, sort_keys=False)

    print("[import] Done.")

if __name__ == "__main__":
    main()
