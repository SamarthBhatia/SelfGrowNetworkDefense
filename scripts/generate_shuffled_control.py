#!/usr/bin/env python3
"""
Generate Time-Shuffled Benign Control Trace.
Keeps the exact benign records but randomizes their timestamps
to destroy temporal structure.
"""

import json
import argparse
import random
import sys

def main():
    if len(sys.argv) != 3:
        print("Usage: python3 scripts/generate_shuffled_control.py <original_stimulus.jsonl> <output_control.jsonl>")
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2]

    print(f"Generating time-shuffled control from {input_path}...")

    records = []
    
    with open(input_path, 'r') as f:
        for line in f:
            if not line.strip(): continue
            try:
                rec = json.loads(line)
                # Keep only benign records for this control?
                # No, if we want to test if temporal structure matters for the ATTACK, 
                # we should shuffle the ATTACK trace.
                # If we shuffle the benign trace, we just get shuffled noise.
                
                # The prompt said: "Keep the same benign records but shuffle timestamps".
                # But also "If defense activation drops...". 
                # If the original benign trace triggered defense (as my previous small control did), 
                # then shuffling it checks if that trigger was due to temporal bursts or just aggregate volume.
                
                # Let's shuffle the whole file to be safe.
                records.append(rec)
            except:
                continue

    # Assign random steps between 0 and 2000
    for rec in records:
        rec["step"] = random.randint(0, 1999)
        
    # Sort by new steps for clean simulation
    records.sort(key=lambda x: x["step"])
    
    with open(output_path, 'w') as f:
        for rec in records:
            f.write(json.dumps(rec) + "\n")

    print(f"Wrote {len(records)} shuffled records to {output_path}")

if __name__ == "__main__":
    main()
