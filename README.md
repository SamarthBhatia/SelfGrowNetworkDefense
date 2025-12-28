# Morphogenetic Cybersecurity

> **Self-Evolving Defense Architecture Inspired by Biological Morphogenesis**

This repository contains the reference implementation of a **Morphogenetic Security System**: a decentralized, cellular cybersecurity architecture that grows, adapts, and evolves in response to adversarial pressure.

Unlike traditional static defense systems, this architecture uses **biological primitives** (reaction-diffusion systems, genetic regulatory networks, and swarm immunity) to autonomously detect, isolate, and neutralize threats.

---

## 🧬 Core Capabilities

### 1. Cellular Defense Kernel (Phase 1)
- **Reaction-Diffusion Logic:** Individual nodes ("cells") make decisions based on local chemical gradients (signals like `activator`, `inhibitor`, `threat`).
- **Autonomous Regulation:** Cells manage their own energy, stress, and replication cycles without a central controller.

### 2. Emergent Topology (Phase 2)
- **Adaptive Networking:** The system dynamically rewires itself. Stressed cells isolate themselves, while healthy clusters form dense protective networks.
- **Genetic Regulation:** Every cell parameter (sensitivity, replication threshold) is controlled by a `CellGenome` that can mutate and drift.

### 3. Swarm Immune Response (Phase 3)
- **Hardware-Backed Trust:** Nodes use simulated **TPM Attestation** to cryptographically sign anomaly reports, preventing "liar" nodes from poisoning the network.
- **Consensus Quarantine:** The swarm votes to disconnect malicious peers based on reputation scores.
- **Acquired Immunity:** Survivor cells retain "immune memory" of attack patterns, passing this resilience to their offspring.

### 4. Evolutionary Adaptation (Phase 4)
- **Adversarial Evolution:** An integrated genetic algorithm (`adversarial_loop`) continuously breeds attack scenarios against the defense system.
- **Darwinian Hardening:** The system has been proven to evolve tougher "phenotypes" (e.g., +20% replication threshold, -8% stress sensitivity) under persistent hostile pressure.

---

## 🚀 Getting Started

### Prerequisites
- **Rust** (latest stable)
- **Python 3.10+** (for visualization scripts)
- **Graphviz** (optional, for topology plotting)

### Build
```bash
car go build --release
```

### Quick Demo: The "Pitch"
Run a pre-packaged demonstration showing the difference between a baseline scenario and a high-threat "viral outbreak":

```bash
scripts/pitch_demo.sh
car go run --bin pitch_tui -- target/pitch_demo
```

---

## 🛠️ Usage Guide

### 1. Running Single Simulations
Execute the runtime with a specific scenario configuration:

```bash
car go run --release -- \
  --config docs/examples/distributed-pressure.yaml \
  --telemetry viral_telemetry.jsonl
```

### 2. Adversarial Evolution (The "Evo-Devo" Loop)
To train the system against an evolving adversary:

```bash
# Run 10 generations of evolution against a hostile environment
./scripts/run_hostile_evolution.sh
```

This will:
1.  Spawn a population of attack candidates.
2.  Run simulations in parallel.
3.  Calculate fitness based on breach success.
4.  Mutate/Crossover the strongest attacks.
5.  Generate genetic drift plots in `target/hostile_drift_plots/`.

### 3. Analyzing Results
We provide Python scripts to visualize the system's behavior:

- **Genome Drift:** `scripts/visualize_genome_drift.py`
- **Topology Graph:** `scripts/visualize_topology.py`
- **Trust Network:** `scripts/visualize_trust_graph.py`

---

## 📂 Project Structure

| Module | Description |
| :--- | :--- |
| `src/cellular.rs` | The biological state machine (Genome, Energy, Lineage). |
| `src/immune.rs` | Cryptographic trust, TPM simulation, and consensus logic. |
| `src/orchestration.rs` | Topology management and signal propagation bus. |
| `src/adversarial.rs` | The genetic algorithm harness for evolution. |
| `docs/examples/` | Scenario configurations (`.yaml`) and attack profiles. |

---

## 🔬 Scientific Validation

The system has been validated against "Traitor" and "Hostile Environment" scenarios. Key findings from Phase 4:

- **Resilience:** Under sustained pressure, the population autonomously evolved a **15-20% higher replication threshold**, prioritizing survival over growth.
- **Sensitivity:** Cells evolved **8% lower stress sensitivity**, reducing false positives from background noise.
- **Stability:** The "hardened" genome has been baked into the default configuration (`src/cellular.rs`) as of `v1.0`.

See [Phase 4 Report](docs/phase4-report.md) for detailed data.

---

**Status:** Completed (Phases 1–4 Delivered)
**License:** MIT