#!/usr/bin/env python3
"""
Generate Block-Shuffled Control Trace.
Breaks the trace into blocks, shuffles them, but preserves 
local burstiness within each block.
"""

import json
import argparse
import random
import sys
from pathlib import Path

def main():
    if len(sys.argv) != 4:
        print("Usage: python3 scripts/generate_block_shuffled_control.py <original_stimulus.jsonl> <output_control.jsonl> <block_size_steps>")
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2]
    block_size = int(sys.argv[3])

    print(f"Generating block-shuffled control from {input_path} (block size: {block_size} steps)...")

    # Group records by step
    steps = {}
    with open(input_path, 'r') as f:
        for line in f:
            if not line.strip(): continue
            try:
                rec = json.loads(line)
                s = rec["step"]
                if s not in steps:
                    steps[s] = []
                steps[s].append(rec)
            except:
                continue

    if not steps:
        print("Error: No records found.")
        sys.exit(1)

    max_step = max(steps.keys())
    
    # Create blocks
    blocks = []
    for b_start in range(0, max_step + 1, block_size):
        block_content = []
        for s in range(b_start, min(b_start + block_size, max_step + 1)):
            if s in steps:
                # Store relative steps within block
                for rec in steps[s]:
                    block_content.append((s - b_start, rec))
        if block_content:
            blocks.append(block_content)

    # Shuffle blocks
    random.shuffle(blocks)
    
    # Reassemble with new global steps
    shuffled_records = []
    current_global_start = 0
    for block in blocks:
        for rel_step, rec in block:
            new_rec = rec.copy()
            new_rec["step"] = current_global_start + rel_step
            shuffled_records.append(new_rec)
        current_global_start += block_size

    # Sort just in case of overlaps or for simulation order
    shuffled_records.sort(key=lambda x: x["step"])

    with open(output_path, 'w') as f:
        for rec in shuffled_records:
            f.write(json.dumps(rec) + "\n")

    print(f"Wrote {len(shuffled_records)} records in {len(blocks)} blocks to {output_path}")

if __name__ == "__main__":
    main()
