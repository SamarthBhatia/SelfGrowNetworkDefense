#!/usr/bin/env python3
"""
Visualize the evolution of the cellular topology from telemetry data.
Generates Graphviz DOT files for each step or significant event.
"""

import json
import argparse
from pathlib import Path
import sys

def parse_telemetry(telemetry_path):
    """Parses JSONL telemetry file and yields events."""
    with open(telemetry_path, 'r') as f:
        for line in f:
            if not line.strip():
                continue
            try:
                record = json.loads(line)
                yield record
            except json.JSONDecodeError as e:
                print(f"Skipping invalid JSON line: {e}", file=sys.stderr)

def generate_dot(nodes, edges, step, output_dir):
    """Generates a DOT file for the current graph state."""
    filename = output_dir / f"topology_step_{step:04d}.dot"
    with open(filename, 'w') as f:
        f.write("graph MorphogeneticTopology {\n")
        f.write("    node [shape=circle, style=filled, color=lightblue];\n")
        f.write("    overlap=false;\n")
        f.write("    splines=true;\n")
        
        for node in nodes:
            # Color dead cells red if we tracked state, but for now just existence
            f.write(f'    "{node}";\n')
            
        for source, target in edges:
            # Undirected graph for visualization simplicity
            f.write(f'    "{source}" -- "{target}";\n')
            
        f.write("}\n")
    # print(f"Generated {filename}")

def main():
    parser = argparse.ArgumentParser(description="Visualize topology evolution.")
    parser.add_argument("telemetry_path", type=Path, help="Path to telemetry.jsonl")
    parser.add_argument("--output-dir", type=Path, default=Path("topology_viz"), help="Directory to save DOT files")
    args = parser.parse_args()

    if not args.telemetry_path.exists():
        print(f"Error: {args.telemetry_path} does not exist.")
        sys.exit(1)

    args.output_dir.mkdir(parents=True, exist_ok=True)

    nodes = set()
    edges = set() # Set of tuples (min_id, max_id) to avoid duplicates in undirected view
    current_step = 0
    
    # Initialize with seed nodes if not explicitly logged (though new logic logs links)
    # We rely on LinkAdded/Removed and CellReplicated/Died events.

    print(f"Processing {args.telemetry_path}...")
    
    event_count = 0
    
    for record in parse_telemetry(args.telemetry_path):
        event_wrapper = record.get("event", {})
        
        # Flatten the event structure if it's nested like {"CellReplicated": {...}}
        event_type = list(event_wrapper.keys())[0] if event_wrapper else None
        if not event_type:
            continue
            
        data = event_wrapper[event_type]
        
        updated = False
        
        if event_type == "CellReplicated":
            # Add child node
            nodes.add(data["child_id"])
            nodes.add(data["cell_id"]) # Ensure parent exists
            # Note: LinkAdded should follow this event if using Graph topology
            updated = True
            
        elif event_type == "CellDied":
            # Mark dead or remove? Let's remove for topology view
            if data["cell_id"] in nodes:
                nodes.remove(data["cell_id"])
                # Remove connected edges
                edges = {e for e in edges if data["cell_id"] not in e}
                updated = True
                
        elif event_type == "LinkAdded":
            src = data["source"]
            tgt = data["target"]
            nodes.add(src)
            nodes.add(tgt)
            edge = tuple(sorted((src, tgt)))
            edges.add(edge)
            updated = True
            
        elif event_type == "LinkRemoved":
            src = data["source"]
            tgt = data["target"]
            edge = tuple(sorted((src, tgt)))
            if edge in edges:
                edges.remove(edge)
            updated = True
            
        elif event_type == "StepSummary":
            step = data["step"]
            if step > current_step:
                # Snapshot the state at the end of the previous step
                generate_dot(nodes, edges, current_step, args.output_dir)
                current_step = step
                updated = False # Don't gen twice
        
        if updated:
            event_count += 1

    # Generate final state
    generate_dot(nodes, edges, current_step, args.output_dir)
    print(f"Processed {event_count} topology events. Output saved to {args.output_dir}")
    print(f"To visualize, use: dot -Tpng {args.output_dir}/topology_step_XXXX.dot -o output.png")

if __name__ == "__main__":
    main()
