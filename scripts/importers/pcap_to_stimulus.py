#!/usr/bin/env python3
"""
IoT Dataset (PCAP/CSV) to Stimulus Converter.

Usage:
    python3 scripts/importers/pcap_to_stimulus.py <csv_file> <output_jsonl>

Adapts to:
1. Headers present (generic CSV)
2. UNSW IoT Botnet Dataset (no headers, 35 columns)
"""

import sys
import json
import pandas as pd
import numpy as np
import argparse

def main():
    if len(sys.argv) != 3:
        print("Usage: python3 pcap_to_stimulus.py <csv_file> <output_jsonl>")
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2]

    print(f"[import] Reading traffic log from {input_path}...")
    
    # First, try reading with headers
    try:
        df = pd.read_csv(input_path, sep=None, engine='python', nrows=5)
    except Exception as e:
        print(f"Error reading CSV: {e}")
        sys.exit(1)

    # Heuristic check for UNSW format (35 columns, first column is ID integer, second is float timestamp)
    is_unsw = False
    if len(df.columns) == 35:
        # Check if first col is roughly incremental integers and second is timestamp-like
        try:
            if df.iloc[0, 1] > 1000000000: # Epoch time > 2001
                 is_unsw = True
        except:
            pass

    if is_unsw:
        print("[import] Detected UNSW IoT Dataset format (no headers).")
        # Re-read with header=None and names
        # We only care about timestamp, pkts, and label
        # 1: stime, 8: pkts, 33: category
        df = pd.read_csv(input_path, header=None)
        df.rename(columns={1: 'ts', 8: 'pkts', 33: 'label'}, inplace=True)
        # Keep only relevant columns to save memory
        df = df[['ts', 'pkts', 'label']]
    else:
        # Re-read full file assuming headers
        df = pd.read_csv(input_path, sep=None, engine='python')
        # Normalize
        df.columns = [c.lower() for c in df.columns]

    # Map columns to standard names
    ts_col = None
    for candidate in ['ts', 'timestamp', 'stime', 'starttime']:
        if candidate in df.columns:
            ts_col = candidate
            break
            
    if not ts_col:
        print("Error: Could not find timestamp column.")
        sys.exit(1)

    size_col = None
    for candidate in ['pkts', 'pkt', 'packets', 'tot_pkts', 'orig_pkts']:
        if candidate in df.columns:
            size_col = candidate
            break

    label_col = None
    for candidate in ['label', 'category', 'attack']:
        if candidate in df.columns:
            label_col = candidate
            break

    print(f"[import] Using timestamp: {ts_col}, intensity: {size_col}, label: {label_col}")

    # Normalize time
    df[ts_col] = pd.to_numeric(df[ts_col], errors='coerce')
    df = df.dropna(subset=[ts_col])
    start_time = df[ts_col].min()
    df['sim_step'] = ((df[ts_col] - start_time)).astype(int)

    print(f"[import] Processing {len(df)} records into simulation steps...")
    
    grouped = df.groupby('sim_step')
    
    with open(output_path, 'w') as f:
        for step, group in grouped:
            if size_col:
                intensity = group[size_col].sum()
            else:
                intensity = len(group)
            
            # Normalize (heuristic)
            threat_val = min(1.0, intensity / 500.0) # Lower threshold for this dataset
            
            if threat_val > 0.05:
                cmd = {
                    "step": int(step),
                    "topic": "activator",
                    "value": float(threat_val),
                    "duration": 1
                }
                f.write(json.dumps(cmd) + "\n")
                
                # Threat signal
                is_malicious = False
                if label_col:
                     # Check if any row in this second is malicious
                     # UNSW uses 'Normal' vs Attack names
                     if group[label_col].str.contains('Normal', case=False, regex=False).all() == False:
                         is_malicious = True
                
                if is_malicious:
                     cmd_threat = {
                        "step": int(step),
                        "topic": "threat",
                        "value": float(threat_val),
                        "duration": 1
                    }
                     f.write(json.dumps(cmd_threat) + "\n")

    print(f"[import] Wrote stimulus schedule to {output_path}.")

if __name__ == "__main__":
    main()