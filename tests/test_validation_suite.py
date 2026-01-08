import unittest
import os
import subprocess
import shutil
from pathlib import Path

class TestValidationSuite(unittest.TestCase):
    def test_suite_smoke_run(self):
        root_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        script_path = os.path.join(root_dir, "scripts", "run_validation_suite.py")
        bin_path = os.path.join(root_dir, "target", "debug", "morphogenetic-security")
        
        if not os.path.exists(bin_path):
            self.skipTest("Debug binary not found, please run 'cargo build --bin morphogenetic-security'")

        # Environment overrides for fast execution
        env = os.environ.copy()
        env["VALIDATION_BIN"] = bin_path
        env["VALIDATION_RUNS"] = "1"
        
        # Ensure output directory exists to avoid script errors if it assumes it
        # The script creates TELEMETRY_DIR, but we check artifact generation
        
        cmd = ["python3", script_path]
        
        print(f"Running validation suite smoke test with BIN={bin_path}...")
        result = subprocess.run(cmd, env=env, capture_output=True, text=True)
        
        if result.returncode != 0:
            print("STDOUT:", result.stdout)
            print("STDERR:", result.stderr)
            
        self.assertEqual(result.returncode, 0, "Validation suite script failed")
        
        # Check if output artifact was created
        stats_file = os.path.join(root_dir, "docs", "images", "validation_stats.txt")
        self.assertTrue(os.path.exists(stats_file), "validation_stats.txt was not generated")
        
        with open(stats_file, 'r') as f:
            content = f.read()
            
        self.assertIn("Attack", content)
        self.assertIn("Zero-Pressure", content)

    def test_numerical_bands_logic(self):
        """
        Verifies that the validation logic correctly flags results outside 
        the documented thesis bounds (Attack > 80, Controls < 20).
        """
        # Mock data matching the report's success criteria
        good_stats = """Experiment       Mean Shifts  Std Shifts  Mean Reps  Std Reps
Attack                112.50       10.20      45.00      5.00
Zero-Pressure           2.10        0.50      80.00      2.00
Full-Shuffled           8.40        3.10      10.00      1.00
"""
        self._check_bands(good_stats, expect_success=True)

        # Mock data representing a regression (Attack too low)
        bad_attack_stats = """Experiment       Mean Shifts  Std Shifts  Mean Reps  Std Reps
Attack                 40.00       10.20      45.00      5.00
Zero-Pressure           2.10        0.50      80.00      2.00
"""
        self._check_bands(bad_attack_stats, expect_success=False)

        # Mock data representing a regression (Control too high)
        bad_control_stats = """Experiment       Mean Shifts  Std Shifts  Mean Reps  Std Reps
Attack                112.50       10.20      45.00      5.00
Full-Shuffled          50.00        3.10      10.00      1.00
"""
        self._check_bands(bad_control_stats, expect_success=False)

    def _check_bands(self, content, expect_success):
        """
        Helper to parse the table string and assert bounds.
        Bounds:
          - Attack 'Mean Shifts' > 80
          - Zero-Pressure/Shuffled 'Mean Shifts' < 20
        """
        lines = content.strip().split('\n')
        headers = lines[0].split()
        
        # Simple column index lookup
        try:
            name_idx = headers.index("Experiment")
            shift_idx = headers.index("Mean") # "Mean" from "Mean Shifts"
            # In the fixed width output, headers might be merged or split by spaces.
            # The pandas to_string output is usually:
            # Experiment  Mean Shifts  ...
            # But split() will see "Mean", "Shifts".
            # Let's rely on finding the line starting with the experiment name
        except ValueError:
            pass 

        failures = []
        
        for line in lines[1:]:
            parts = line.split()
            if not parts: continue
            name = parts[0]
            # Assuming 'Mean Shifts' is the 2nd column (index 1) in the split list
            # Experiment(0) Mean(1) Shifts(ignored/implicit in value?) 
            # Wait, pandas to_string:
            # Experiment       Mean Shifts ...
            # Value line:
            # Attack                112.50 ...
            # parts[0] = Attack, parts[1] = 112.50
            
            try:
                mean_shifts = float(parts[1])
            except ValueError:
                continue

            if name == "Attack":
                if mean_shifts < 80.0:
                    failures.append(f"Attack shifts {mean_shifts} too low (<80)")
            elif name in ["Zero-Pressure", "Full-Shuffled", "Block-Shuffled"]:
                if mean_shifts > 20.0:
                    failures.append(f"{name} shifts {mean_shifts} too high (>20)")
        
        if expect_success:
            self.assertEqual(failures, [], f"Expected success but found failures: {failures}")
        else:
            self.assertNotEqual(failures, [], "Expected failures but found none")

if __name__ == "__main__":
    unittest.main()
