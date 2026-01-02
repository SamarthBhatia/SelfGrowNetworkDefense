#!/usr/bin/env python3
"""
Generate Volume-Matched Benign Control Trace.
Takes the benign segment of the stimulus and scales its intensity/frequency 
to match the peak volume of the Mirai attack trace, but preserves benign structure.
"""

import json
import argparse
import random
import sys

def main():
    if len(sys.argv) != 3:
        print("Usage: python3 scripts/generate_volume_matched_control.py <original_stimulus.jsonl> <output_control.jsonl>")
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2]

    print(f"Generating volume-matched control from {input_path}...")

    # 1. Analyze Original Trace to find Peak Intensity
    max_intensity = 0.0
    benign_records = []
    
    with open(input_path, 'r') as f:
        for line in f:
            if not line.strip(): continue
            try:
                rec = json.loads(line)
                val = rec.get("value", 0.0)
                if val > max_intensity:
                    max_intensity = val
                
                # Assume "benign" is low intensity in the original trace?
                # Actually, the original trace IS the Mirai trace.
                # We want to take the LOW intensity parts and boost them.
                
                # Heuristic: if value < 0.2, it's background noise.
                if val < 0.2:
                    benign_records.append(rec)
            except:
                continue
                
    if not benign_records:
        print("Error: No benign records found to amplify.")
        sys.exit(1)
        
    print(f"Found {len(benign_records)} benign records. Peak attack intensity was {max_intensity:.4f}")
    
    # 2. Generate Boosted Control
    # We will replay the benign records but multiply their intensity to match the attack peak.
    # And we'll repeat them to fill the time if needed.
    
    boost_factor = max_intensity / 0.05 # Assume avg benign is ~0.05
    if boost_factor < 1.0: boost_factor = 1.0
    
    print(f"Applying boost factor: {boost_factor:.2f}")
    
    with open(output_path, 'w') as f:
        # Generate 2000 steps worth of boosted benign traffic
        for i in range(2000):
            # Pick a random benign record template
            template = random.choice(benign_records)
            
            # Modulate: keep benign pattern (activator only, no 'threat' topic ideally? 
            # But real traffic has both if we mapped it that way.
            # Our pcap importer maps heavy traffic to 'activator' and malicious to 'threat'.
            # A volume-matched BENIGN control should have high 'activator' but NO 'threat' topic events? 
            # OR high 'activator' and high 'threat' but unstructured?
            
            # Definition: "Volume-matched benign" usually means high traffic volume (activator) 
            # but WITHOUT the specific 'threat' label signature if the detector uses it.
            # But our detector currently uses 'threat' signal directly from the importer if label is present.
            
            # IF the detector purely reacts to 'activator' (volume), this control tests that.
            # We will emit high 'activator' values.
            
            new_val = min(1.0, template["value"] * boost_factor)
            
            cmd = {
                "step": i,
                "topic": "activator", # Benign high volume
                "value": new_val,
                "duration": 1
            }
            f.write(json.dumps(cmd) + "\n")

    print(f"Wrote {output_path}")

if __name__ == "__main__":
    main()
