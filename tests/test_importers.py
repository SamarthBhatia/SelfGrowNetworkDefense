import unittest
import tempfile
import os
import subprocess
import json
import yaml
import shutil
import sys

class TestImporters(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()
        self.root_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        
    def tearDown(self):
        shutil.rmtree(self.test_dir)

    def test_pcap_to_stimulus_generic(self):
        csv_path = os.path.join(self.test_dir, "traffic.csv")
        output_path = os.path.join(self.test_dir, "stimulus.jsonl")
        
        with open(csv_path, 'w') as f:
            f.write("timestamp,packets,label,src_ip\n")
            f.write("1000,10,Normal,192.168.1.1\n")
            f.write("1001,600,Attack,192.168.1.2\n") # High intensity
            
        cmd = [
            sys.executable,
            os.path.join(self.root_dir, "scripts/importers/pcap_to_stimulus.py"),
            csv_path,
            output_path,
            "--nodes", "5",
            "--strategy", "hash"
        ]
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, f"Script failed: {result.stderr}")
        
        with open(output_path, 'r') as f:
            lines = f.readlines()
            
        self.assertTrue(len(lines) > 0)
        first_cmd = json.loads(lines[0])
        self.assertIn("step", first_cmd)
        self.assertIn("target", first_cmd)
        self.assertTrue(first_cmd["target"].startswith("seed-"))

    def test_import_topology_graphml(self):
        graphml_path = os.path.join(self.test_dir, "test.graphml")
        output_path = os.path.join(self.test_dir, "scenario.yaml")
        
        content = """<?xml version="1.0" encoding="UTF-8"?>
<graphml xmlns="http://graphml.graphdrawing.org/xmlns"  
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xsi:schemaLocation="http://graphml.graphdrawing.org/xmlns http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd">
  <graph id="G" edgedefault="undirected">
    <node id="n0"/>
    <node id="n1"/>
    <edge source="n0" target="n1"/>
  </graph>
</graphml>"""
        with open(graphml_path, 'w') as f:
            f.write(content)
            
        cmd = [
            sys.executable,
            os.path.join(self.root_dir, "scripts/importers/import_topology.py"),
            graphml_path,
            output_path,
            "--duration", "50"
        ]
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, f"Script failed: {result.stderr}")
        
        # Verify YAML
        with open(output_path, 'r') as f:
            config = yaml.safe_load(f)
            
        self.assertEqual(config["initial_cell_count"], 2)
        self.assertEqual(config["topology"]["strategy"], "Graph")
        self.assertEqual(len(config["topology"]["explicit_links"]), 1)
        self.assertEqual(config["simulation_steps"], 50)
        
        # Verify mapping JSON
        mapping_path = output_path.replace(".yaml", "_mapping.json")
        self.assertTrue(os.path.exists(mapping_path))
        with open(mapping_path, 'r') as f:
            mapping = json.load(f)
        # Should map seed-X to nX
        self.assertIn("seed-0", mapping)
        
if __name__ == "__main__":
    unittest.main()
