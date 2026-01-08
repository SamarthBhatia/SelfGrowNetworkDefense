# Realism Expansion Pack Data

This directory is the staging area for real-world topologies and traffic datasets.

## 1. Topologies (GraphML)
Place `.graphml` files in `topologies/`.
Recommended source: [Internet Topology Zoo](http://www.topology-zoo.org/dataset.html)

Examples to download:
- `Abilene.graphml`
- `Geant2012.graphml`
- `Sprint.graphml`

## 2. Traffic Datasets (CSV/PCAP)
Place `.csv` files in `traffic/`.
Supported formats:
- **UNSW IoT Botnet**: CSVs with no headers (auto-detected).
- **Generic CSV**: Must have columns for `timestamp` (or `ts`), `packets` (or `size`), and optional `label` and `src_ip`.

Recommended sources:
- [CICIoT2023](https://www.unb.ca/cic/datasets/iotdataset-2023.html)
- [IoT-23](https://www.stratosphereips.org/datasets-iot23)

## 3. Running the Suite
Once files are placed, run:
```bash
python3 scripts/run_realism_suite.py
```
This will:
1. Import all topologies into Scenario Configs.
2. Convert all traffic logs into Stimulus Schedules.
3. Run the Morphogenetic Simulation for every Topology x Traffic combination.
4. Generate a summary report of "Burstiness Sensitivity".
