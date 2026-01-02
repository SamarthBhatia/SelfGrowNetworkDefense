#!/usr/bin/env python3
"""
Simulated Playback of Morphogenetic Defense.
Reads telemetry and renders a live-updating grid of cell states.
"""

import json
import time
import sys
import os
from collections import defaultdict

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 scripts/playback_simulation.py <telemetry_jsonl> [delay_sec]")
        sys.exit(1)

    telemetry_path = sys.argv[1]
    delay = float(sys.argv[2]) if len(sys.argv) > 2 else 0.05

    if not os.path.exists(telemetry_path):
        print(f"Error: {telemetry_path} not found.")
        sys.exit(1)

    # State tracking
    cells = {} # cell_id -> {lineage, dead}
    
    # Emoji Map
    EMOJI = {
        "Stem": "🌱",
        "IntrusionDetection": "🛡️",
        "Firewall": "🧱",
        "Encryption": "🔑",
        "Healer": "🩹",
        "Dead": "💀",
        "System": "⚡"
    }

    print("\033[2J") # Clear screen

    with open(telemetry_path, 'r') as f:
        for line in f:
            try:
                record = json.loads(line)
            except:
                continue
                
            evt_wrapper = record.get("event", {})
            if not evt_wrapper: continue
            
            evt_type = list(evt_wrapper.keys())[0]
            data = evt_wrapper[evt_type]
            
            updated = False
            
            if evt_type == "Scenario":
                print(f"Scenario: {data['name']}")
                continue

            if evt_type == "CellReplicated":
                cells[data["child_id"]] = {"lineage": "Stem", "dead": False}
                updated = True
            elif evt_type == "LineageShift":
                if data["cell_id"] not in cells:
                    cells[data["cell_id"]] = {"lineage": "Stem", "dead": False}
                cells[data["cell_id"]]["lineage"] = data["lineage"]
                updated = True
            elif evt_type == "CellDied":
                if data["cell_id"] in cells:
                    # Remove from active view
                    del cells[data["cell_id"]]
                updated = True
            elif evt_type == "StepSummary":
                # Render Frame
                render_grid(cells, data, EMOJI)
                time.sleep(delay)
                updated = False

def render_grid(cells, summary, emoji_map):
    # Move cursor to top and clear screen
    # \033[H moves to 0,0; \033[J clears from cursor to end of screen
    print("\033[H\033[J", end="")
    
    step = summary.get("step", 0)
    threat = summary.get("threat_score", 0.0)
    count = len([c for c in cells.values() if not c.get("dead", False)])
    
    print(f"=== Morphogenetic Defense Simulation ===")
    print(f"Step: {step:04d} | Attack Intensity: {threat:.2f} | Active Swarm: {count:03d}")
    print("-" * 50)
    
    # Layout cells in a grid
    sorted_ids = sorted(cells.keys())
    
    cols = 10
    current_row = []
    
    for cid in sorted_ids:
        c = cells[cid]
        if c.get("dead"): continue
        
        icon = emoji_map.get(c["lineage"], emoji_map["Stem"])
        current_row.append(icon)
        
        if len(current_row) >= cols:
            print("  ".join(current_row))
            current_row = []
    
    if current_row:
        print("  ".join(current_row))
        
    print("-" * 50)
    print("Legend: 🌱 Stem (Hardware) | 🛡️ Intrusion Detection Active")
    print("        (Swarm size automatically adapts to threat level)")

if __name__ == "__main__":
    main()
