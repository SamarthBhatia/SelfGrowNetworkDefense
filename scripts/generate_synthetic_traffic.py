#!/usr/bin/env python3
"""
Generate synthetic traffic CSV for testing realism suite.
"""
import pandas as pd
import numpy as np
import os

OUTPUT_DIR = "data/external/traffic"
os.makedirs(OUTPUT_DIR, exist_ok=True)

def generate(name, duration=100, bursty=False):
    print(f"Generating {name}...")
    timestamps = []
    src_ips = []
    sizes = []
    labels = []
    
    start_time = 1600000000.0
    
    for t in range(duration):
        # Base traffic
        num_pkts = np.random.randint(10, 50)
        if bursty and t > 40 and t < 60:
            num_pkts += np.random.randint(500, 2000) # Burst
            
        for _ in range(num_pkts):
            timestamps.append(start_time + t + np.random.random())
            src_ips.append(f"192.168.1.{np.random.randint(1, 255)}")
            sizes.append(1)
            labels.append("Normal" if not bursty else ("Attack" if t > 40 and t < 60 else "Normal"))

    df = pd.DataFrame({
        "timestamp": timestamps,
        "src_ip": src_ips,
        "packets": sizes,
        "label": labels
    })
    
    df.to_csv(f"{OUTPUT_DIR}/{name}.csv", index=False)

if __name__ == "__main__":
    generate("synthetic_baseline")
    generate("synthetic_burst", bursty=True)
