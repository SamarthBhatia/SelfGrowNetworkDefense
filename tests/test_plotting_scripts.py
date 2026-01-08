import unittest
import subprocess
import os
import tempfile
import json
import shutil
from pathlib import Path

ROOT_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

class TestPlottingScripts(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()
        self.telemetry_path = Path(self.test_dir) / "telemetry.jsonl"
        self.stimulus_path = Path(self.test_dir) / "stimulus.jsonl"
        
        # Create minimal valid telemetry
        with self.telemetry_path.open("w") as f:
            f.write(json.dumps({"timestamp_ms": 1000, "event": {"Scenario": {"name": "test"}}}) + "\n")
            for i in range(10):
                f.write(json.dumps({
                    "timestamp_ms": 1000 + i*1000,
                    "event": {
                        "StepSummary": {
                            "step": i,
                            "threat_score": i * 0.1,
                            "cell_count": 10,
                            "population_stats": None,
                            "topology_stats": None
                        }
                    }
                }) + "\n")
                
        # Create minimal stimulus
        with self.stimulus_path.open("w") as f:
            for i in range(10):
                f.write(json.dumps({"step": i, "topic": "activator", "value": 0.5, "duration": 1}) + "\n")

    def tearDown(self):
        shutil.rmtree(self.test_dir)

    def test_analyze_telemetry(self):
        script = os.path.join(ROOT_DIR, "scripts", "analyze_telemetry.py")
        cmd = ["python3", script, str(self.telemetry_path), "--limit", "5"]
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, f"analyze_telemetry failed: {result.stderr}")
        self.assertIn("Total events:", result.stdout)
        self.assertIn("Replications:", result.stdout)

    def test_telemetry_correlate(self):
        script = os.path.join(ROOT_DIR, "scripts", "telemetry_correlate.py")
        # Ensure we don't try to open a window (though this script doesn't plot)
        env = os.environ.copy()
        env["MPLBACKEND"] = "Agg" 
        
        cmd = ["python3", script, str(self.telemetry_path), "--stimulus", str(self.stimulus_path)]
        
        result = subprocess.run(cmd, env=env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, f"telemetry_correlate failed: {result.stderr}")
        # It prints correlation stats table
        self.assertIn("Step | Threat | Cells", result.stdout)

if __name__ == "__main__":
    unittest.main()
