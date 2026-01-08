#!/usr/bin/env python3
"""
IoT Dataset (PCAP/CSV) to Stimulus Converter.

Usage:
    python3 scripts/importers/pcap_to_stimulus.py <csv_file> <output_jsonl> [options]

Options:
    --nodes <N>         Number of nodes in the topology. If > 0, distributes traffic.
    --strategy <S>      Distribution strategy: 'global' (default), 'random', 'hash'.
    --duration <D>      Max duration in seconds (optional).

Adapts to:
1. Headers present (generic CSV)
2. UNSW IoT Botnet Dataset (no headers, 35 columns)
"""

import sys
import json
import pandas as pd
import numpy as np
import argparse
import hashlib

def get_target_node(row, strategy, num_nodes, src_col=None):
    if strategy == 'global' or num_nodes <= 0:
        return None
    
    if strategy == 'random':
        return f"seed-{np.random.randint(0, num_nodes)}"
    
    if strategy == 'hash' and src_col:
        val = str(row[src_col])
        # Stable hash
        h = int(hashlib.sha256(val.encode('utf-8')).hexdigest(), 16)
        idx = h % num_nodes
        return f"seed-{idx}"
        
    # Fallback
    return f"seed-{np.random.randint(0, num_nodes)}"

def main():
    parser = argparse.ArgumentParser(description="Convert CSV traffic logs to simulation stimulus.")
    parser.add_argument("input_path", help="Path to input CSV file")
    parser.add_argument("output_path", help="Path to output JSONL file")
    parser.add_argument("--nodes", type=int, default=0, help="Number of nodes in topology (for spatial distribution)")
    parser.add_argument("--strategy", type=str, choices=['global', 'random', 'hash'], default='global', help="Targeting strategy")
    parser.add_argument("--duration", type=int, default=0, help="Limit processing to first N seconds")

    args = parser.parse_args()

    print(f"[import] Reading traffic log from {args.input_path}...")
    
    # First, try reading with headers
    try:
        df = pd.read_csv(args.input_path, sep=None, engine='python', nrows=5)
    except Exception as e:
        print(f"Error reading CSV: {e}")
        sys.exit(1)

    # Heuristic check for UNSW format (35 columns, first column is ID integer, second is float timestamp)
    is_unsw = False
    if len(df.columns) == 35:
        try:
            if df.iloc[0, 1] > 1000000000: # Epoch time > 2001
                 is_unsw = True
        except:
            pass

    if is_unsw:
        print("[import] Detected UNSW IoT Dataset format (no headers).")
        # 1: stime, 8: pkts, 33: category, 3: srcip, 6: dstip
        df = pd.read_csv(args.input_path, header=None)
        df.rename(columns={1: 'ts', 8: 'pkts', 33: 'label', 3: 'srcip', 6: 'dstip'}, inplace=True)
        keep_cols = ['ts', 'pkts', 'label']
        if args.strategy == 'hash':
            keep_cols.extend(['srcip', 'dstip'])
        df = df[keep_cols]
    else:
        # Re-read full file assuming headers
        df = pd.read_csv(args.input_path, sep=None, engine='python')
        # Normalize
        df.columns = [c.lower() for c in df.columns]

    # Map columns
    ts_col = next((c for c in ['ts', 'timestamp', 'stime', 'starttime'] if c in df.columns), None)
    if not ts_col:
        print("Error: Could not find timestamp column.")
        sys.exit(1)

    size_col = next((c for c in ['pkts', 'pkt', 'packets', 'tot_pkts', 'orig_pkts'] if c in df.columns), None)
    label_col = next((c for c in ['label', 'category', 'attack'] if c in df.columns), None)
    
    # For hash strategy, we need a source column
    src_col = next((c for c in ['srcip', 'saddr', 'src_ip', 'source'] if c in df.columns), None)

    print(f"[import] Columns - Time: {ts_col}, Size: {size_col}, Label: {label_col}, Src: {src_col}")

    # Normalize time
    df[ts_col] = pd.to_numeric(df[ts_col], errors='coerce')
    df = df.dropna(subset=[ts_col])
    start_time = df[ts_col].min()
    df['sim_step'] = ((df[ts_col] - start_time)).astype(int)
    
    if args.duration > 0:
        df = df[df['sim_step'] < args.duration]

    print(f"[import] Processing {len(df)} records into simulation steps...")

    # Assign targets if needed
    if args.strategy != 'global' and args.nodes > 0:
        print(f"[import] Distributing traffic across {args.nodes} nodes using {args.strategy} strategy...")
        # Vectorized assignment is hard for hash, so we stick to apply or pre-calc
        if args.strategy == 'random':
            df['target'] = np.random.randint(0, args.nodes, size=len(df))
            df['target'] = 'seed-' + df['target'].astype(str)
        elif args.strategy == 'hash' and src_col:
             # Fast hashing using pandas
             # We just map unique IPs to nodes
             unique_ips = df[src_col].unique()
             ip_map = {ip: f"seed-{int(hashlib.sha256(str(ip).encode()).hexdigest(), 16) % args.nodes}" for ip in unique_ips}
             df['target'] = df[src_col].map(ip_map)
        else:
             df['target'] = None
    else:
        df['target'] = None

    # Grouping
    group_cols = ['sim_step']
    if args.strategy != 'global' and args.nodes > 0:
        group_cols.append('target')

    grouped = df.groupby(group_cols)
    
    with open(args.output_path, 'w') as f:
        for name, group in grouped:
            # Unpack name
            if isinstance(name, tuple):
                step, target = name
            else:
                step, target = name, None
                
            if size_col:
                intensity = group[size_col].sum()
            else:
                intensity = len(group)
            
            # Normalize (heuristic)
            threat_val = min(1.0, intensity / 500.0) 
            
            if threat_val > 0.05:
                cmd = {
                    "step": int(step),
                    "topic": "activator",
                    "value": float(threat_val),
                    "duration": 1
                }
                if target:
                    cmd['target'] = target
                    
                f.write(json.dumps(cmd) + "\n")
                
                # Threat signal
                is_malicious = False
                if label_col:
                     # Check if any row in this group is malicious
                     if group[label_col].astype(str).str.contains('Normal', case=False, regex=False).all() == False:
                         is_malicious = True
                
                if is_malicious:
                     cmd_threat = {
                        "step": int(step),
                        "topic": "threat",
                        "value": float(threat_val),
                        "duration": 1
                    }
                     if target:
                        cmd_threat['target'] = target
                     f.write(json.dumps(cmd_threat) + "\n")

    print(f"[import] Wrote stimulus schedule to {args.output_path}.")

if __name__ == "__main__":
    main()
