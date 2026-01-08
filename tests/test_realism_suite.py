import unittest
import sys
import os
import shutil
import tempfile
from pathlib import Path
from unittest.mock import patch, MagicMock

# Add scripts to path
ROOT_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.append(os.path.join(ROOT_DIR, "scripts"))

# Import module
import run_realism_suite

class TestRealismSuite(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()
        self.topo_dir = Path(self.test_dir) / "topologies"
        self.traffic_dir = Path(self.test_dir) / "traffic"
        self.output_dir = Path(self.test_dir) / "output"
        
        self.topo_dir.mkdir()
        self.traffic_dir.mkdir()
        
        # Create dummy files
        (self.topo_dir / "test_topo.graphml").touch()
        (self.traffic_dir / "test_traffic.csv").touch()
        
    def tearDown(self):
        shutil.rmtree(self.test_dir)

    @patch("run_realism_suite.run_command")
    @patch("run_realism_suite.get_node_count")
    @patch("run_realism_suite.parse_telemetry")
    def test_matrix_execution(self, mock_parse, mock_count, mock_run):
        # Override globals
        run_realism_suite.TOPO_DIR = self.topo_dir
        run_realism_suite.TRAFFIC_DIR = self.traffic_dir
        run_realism_suite.OUTPUT_DIR = self.output_dir
        
        # Mock returns
        mock_count.return_value = 10
        mock_run.return_value = True
        mock_parse.return_value = 0.5
        
        # Run main
        run_realism_suite.main()
        
        # Verify calls
        # 1. Import topology
        self.assertTrue(any("import_topology.py" in str(call) for call in mock_run.call_args_list))
        # 2. Convert stimulus
        self.assertTrue(any("pcap_to_stimulus.py" in str(call) for call in mock_run.call_args_list))
        # 3. Run simulation
        self.assertTrue(any("cargo run" in str(call) for call in mock_run.call_args_list))
        
        # Verify results CSV
        self.assertTrue((self.output_dir / "matrix_results.csv").exists())

if __name__ == "__main__":
    unittest.main()
