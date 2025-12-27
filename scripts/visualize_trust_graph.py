#!/usr/bin/env python3
"""
Visualizes the Trust Graph from simulation telemetry.
Generates a Graphviz DOT file showing trusted connections between cells.
"""

import json
import sys
import argparse
from typing import Dict, Set

def parse_telemetry(file_path: str) -> Dict[int, Dict[str, any]]:
    """
    Parses telemetry to reconstruct trusted links and active voters at each step.
    Returns a dict mapping step -> {links: set, voters: set}.
    """
    history = {}
    current_links = set()
    current_voters = set()
    step = 0

    try:
        with open(file_path, "r") as f:
            for line in f:
                if not line.strip():
                    continue
                try:
                    record = json.loads(line)
                    event = record.get("event", {})
                    
                    if "StepSummary" in event:
                        step = event["StepSummary"]["step"]
                        # Snapshot the current state for this step
                        history[step] = {
                            "links": current_links.copy(),
                            "voters": current_voters.copy()
                        }
                        # Clear voters for next step? Or keep them if they are still "active"?
                        # For simplicity, let's treat them as per-step highlights.
                        current_voters.clear()
                    
                    elif "LinkAdded" in event:
                        src = event["LinkAdded"]["source"]
                        tgt = event["LinkAdded"]["target"]
                        current_links.add(tuple(sorted([src, tgt])))
                    
                    elif "LinkRemoved" in event:
                        src = event["LinkRemoved"]["source"]
                        tgt = event["LinkRemoved"]["target"]
                        link = tuple(sorted([src, tgt]))
                        if link in current_links:
                            current_links.remove(link)
                    
                    elif "VoteCast" in event:
                        voter = event["VoteCast"]["cell_id"]
                        current_voters.add(voter)
                            
                except json.JSONDecodeError:
                    continue
    except FileNotFoundError:
        print(f"Error: File {file_path} not found.")
        sys.exit(1)

    return history

def generate_dot(state: Dict[str, any], step: int) -> str:
    """Generates a DOT graph definition for a set of links and voters."""
    dot = [f'digraph TrustGraph_Step{step} {{']
    dot.append('  rankdir=LR;')
    dot.append('  node [shape=circle, style=filled, fillcolor=lightblue, fontname="Helvetica"];')
    dot.append('  edge [color=gray70, penwidth=1.5];')
    
    links = state["links"]
    voters = state["voters"]
    
    # Identify all nodes
    nodes = set()
    for src, tgt in links:
        nodes.add(src)
        nodes.add(tgt)
    for v in voters:
        nodes.add(v)
        
    # Define nodes
    for node in sorted(list(nodes)):
        fill = "orange" if node in voters else "lightblue"
        label = f"{node} (VOTER)" if node in voters else node
        dot.append(f'  "{node}" [fillcolor={fill}, label="{label}"];')
    
    # Define edges
    for src, tgt in sorted(list(links)):
        color = "red" if src in voters and tgt in voters else "gray70"
        dot.append(f'  "{src}" -> "{tgt}" [color={color}, dir=none];')
        
    dot.append('}')
    return "\n".join(dot)

def main():
    parser = argparse.ArgumentParser(description="Visualize Trust Graph from telemetry.")
    parser.add_argument("telemetry_file", help="Path to telemetry.jsonl file")
    parser.add_argument("--step", type=int, help="Specific step to visualize (defaults to last step)")
    parser.add_argument("--output", help="Output DOT file path (default: stdout)")
    
    args = parser.parse_args()
    
    history = parse_telemetry(args.telemetry_file)
    
    if not history:
        print("No graph history found.")
        sys.exit(1)
        
    target_step = args.step if args.step is not None else max(history.keys())
    
    if target_step not in history:
        # Fallback to closest previous step
        available_steps = sorted(history.keys())
        target_step = next((s for s in reversed(available_steps) if s <= target_step), available_steps[-1])
        print(f"Step {args.step} not found, showing step {target_step}")

    dot_content = generate_dot(history[target_step], target_step)
    
    if args.output:
        with open(args.output, "w") as f:
            f.write(dot_content)
        print(f"Graph written to {args.output}")
    else:
        print(dot_content)

if __name__ == "__main__":
    main()
