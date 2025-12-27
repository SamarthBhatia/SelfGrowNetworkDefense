# Swarm Immune Response

This document describes the Phase 3 features of the morphogenetic security system, focusing on distributed coordination, hardware-backed trust, and adaptive immunity.

## Overview

The Swarm Immune Response layer enables individual security cells to collaborate as a unified defense organism. It moves beyond local reaction–diffusion rules to implement higher-order behaviors like consensus-based quarantine and cross-generational immune memory.

## Core Mechanisms

### 1. Distributed Anomaly Detection
Cells in the `IntrusionDetection` lineage act as specialized sensors. They monitor the `effective_threat` level and compare it against their evolved `anomaly_sensitivity`. 
- **Trigger:** If threat exceeds sensitivity and is not being suppressed by inhibitors, the cell emits a `ReportAnomaly` action.
- **Verification:** Only detections with high confidence and valid hardware attestation are broadcast to the swarm.

### 2. Simulated TPM Attestation
To prevent "poisoning" attacks where a compromised node floods the network with false alarms, every cell is equipped with a simulated **Trusted Platform Module (TPM)**.
- **Attestation Token:** Contains `cell_id`, `timestamp`, and a `signature`.
- **Enforcement:** Swarm-level signals (like consensus votes) are ignored unless they carry a valid attestation token verified via `TPM::verify`.

### 3. Trust Scores & Dynamic Isolation
Cells maintain a local reputation map (`neighbor_trust`) for all detected peers.
- **Rewards:** Receiving validly attested signals increases trust.
- **Penalties:** Missing or invalid attestations (especially on consensus topics) result in severe trust penalties.
- **Auto-Isolation:** If a neighbor's trust score falls below the `min_trust_threshold` (part of the genome), the cell proactively triggers a `Disconnect` action to prune the untrusted link.

### 4. Swarm Consensus (Coordinated Quarantine)
Consensus allows the swarm to take aggressive action against a threat even before individual cells reach their stress limits.
- **Voting:** When a cell reports an anomaly, it effectively casts a vote (`consensus:topic`).
- **Trigger:** If a cell detects a sufficient weight of votes from trusted neighbors, it triggers a **Coordinated Quarantine**, disconnecting from the suspected high-threat neighbor.

### 5. Immune Memory & Adaptation
Cells that survive a threat encounter (or detect an anomaly) record the event in their `immune_memory`.
- **Hardenining:** Cells dynamically adjust their genome in response to recorded threats (e.g., reducing `stress_sensitivity` to `activator` signals).
- **Inheritance:** When a cell replicates, the child inherits the parent's `immune_memory` and adapted genome, implementing a form of "acquired immunity" that persists through the lineage.

## Telemetry Events

The following events are emitted to track swarm behavior:
- `AnomalyDetected`: Emitted when a cell identifies a potential breach.
- `VoteCast`: Tracks participation in the consensus mechanism.
- `LinkRemoved`: Often indicates a trust-based or consensus-based isolation.

## Trust Graph Visualization
Use `scripts/visualize_trust_graph.py` to generate Graphviz visualizations of the evolving trust network. Active voters are highlighted, showing the "warm" areas of the swarm actively coordinating defense.
