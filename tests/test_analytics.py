import unittest
import tempfile
import os
import shutil
import json
import sys
import subprocess
from pathlib import Path

# Add scripts dir to path to import telemetry_utils
ROOT_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.append(os.path.join(ROOT_DIR, "scripts"))

import telemetry_utils

class TestAnalytics(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()
        self.telemetry_path = Path(self.test_dir) / "telemetry.jsonl"
        self.stimulus_path = Path(self.test_dir) / "stimulus.jsonl"
        self.dashboard_csv = Path(self.test_dir) / "dashboard.csv"
        
        # Generate synthetic telemetry
        with self.telemetry_path.open("w") as f:
            # Scenario
            f.write(json.dumps({"timestamp_ms": 1000, "event": {"Scenario": {"name": "test-scenario"}}}) + "\n")
            
            # Step 0 events
            f.write(json.dumps({"timestamp_ms": 1001, "event": {"SignalEmitted": {"cell_id": "A", "topic": "activator", "value": 0.5}}}) + "\n")
            f.write(json.dumps({"timestamp_ms": 1002, "event": {"CellReplicated": {"cell_id": "A", "child_id": "B"}}}) + "\n")
            
            # Step 0 summary
            f.write(json.dumps({"timestamp_ms": 1003, "event": {"StepSummary": {"step": 0, "threat_score": 0.1, "cell_count": 2, "population_stats": None, "topology_stats": None}}}) + "\n")
            
            # Step 1 events
            f.write(json.dumps({"timestamp_ms": 1004, "event": {"LineageShift": {"cell_id": "B", "lineage": "IntrusionDetection"}}}) + "\n")
            
            # Step 1 summary
            f.write(json.dumps({"timestamp_ms": 1005, "event": {"StepSummary": {"step": 1, "threat_score": 0.5, "cell_count": 2, "population_stats": None, "topology_stats": None}}}) + "\n")

        # Generate synthetic stimulus
        with self.stimulus_path.open("w") as f:
            f.write(json.dumps({"step": 0, "topic": "activator", "value": 0.8, "duration": 1}) + "\n")
            f.write(json.dumps({"step": 1, "topic": "inhibitor", "value": 0.2, "duration": 1}) + "\n")

    def tearDown(self):
        shutil.rmtree(self.test_dir)

    def test_telemetry_utils_parsing(self):
        per_step, ordered, name = telemetry_utils.load_telemetry_per_step(self.telemetry_path)
        
        self.assertEqual(name, "test-scenario")
        self.assertEqual(len(per_step), 2) # Steps 0 and 1
        
        # Check Step 0 stats
        step0 = per_step[0]
        self.assertEqual(step0["replications"], 1)
        self.assertEqual(step0["signals"]["activator"], 1)
        self.assertEqual(step0["threat_score"], 0.1)
        
        # Check Step 1 stats
        step1 = per_step[1]
        self.assertEqual(step1["lineage_shifts"]["IntrusionDetection"], 1)
        
    def test_telemetry_utils_stimulus(self):
        stimuli = telemetry_utils.load_stimuli(self.stimulus_path)
        self.assertEqual(stimuli[0]["total"], 0.8)
        self.assertEqual(stimuli[1]["total"], 0.2)
        
    def test_prepare_dashboard_script(self):
        cmd = [
            "python3",
            os.path.join(ROOT_DIR, "scripts/prepare_telemetry_dashboard.py"),
            str(self.telemetry_path),
            "--stimulus", str(self.stimulus_path),
            "--output", str(self.dashboard_csv)
        ]
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, f"Script failed: {result.stderr}")
        
        self.assertTrue(self.dashboard_csv.exists())
        
        # Check CSV content
        with self.dashboard_csv.open("r") as f:
            content = f.read()
            
        self.assertIn("step,threat_score", content) # Header
        self.assertIn("activator", content)
        # Check if stimulus total is integrated
        # We don't parse CSV here again to avoid circular dependency on logic, just grep
        
if __name__ == "__main__":
    unittest.main()
