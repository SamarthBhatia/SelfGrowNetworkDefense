//! High-level orchestration for the morphogenetic security system.

use crate::cellular::{CellAction, CellEnvironment, PopulationStats, SecurityCell};
use crate::config::{TopologyConfig, TopologyStrategy};
use crate::signaling::{Signal, SignalBus};
use crate::telemetry::{TelemetryEvent, TelemetrySink};
use std::collections::HashMap;
use std::time::SystemTime;

#[allow(dead_code)]
pub struct MorphogeneticApp<TSink: TelemetrySink> {
    cells: Vec<SecurityCell>,
    signal_bus: SignalBus,
    telemetry: TSink,
    topology_config: TopologyConfig,
    neighbors: HashMap<String, Vec<String>>,
}

impl<TSink: TelemetrySink> MorphogeneticApp<TSink> {
    #[allow(dead_code)]
    pub fn new(
        cells: Vec<SecurityCell>,
        telemetry: TSink,
        topology_config: TopologyConfig,
    ) -> Self {
        let mut neighbors = HashMap::new();

        if matches!(topology_config.strategy, TopologyStrategy::Graph) {
            if let Some(links) = &topology_config.explicit_links {
                for link in links {
                    if link.len() >= 2 {
                        let u = &link[0];
                        let v = &link[1];
                        neighbors
                            .entry(u.clone())
                            .or_insert_with(Vec::new)
                            .push(v.clone());
                        neighbors
                            .entry(v.clone())
                            .or_insert_with(Vec::new)
                            .push(u.clone());
                    }
                }
            }
        }

        let mut app = Self {
            cells,
            telemetry,
            topology_config,
            signal_bus: SignalBus::default(),
            neighbors,
        };

        if matches!(app.topology_config.strategy, TopologyStrategy::Graph)
            && app.topology_config.explicit_links.is_none()
        {
            app.initialize_topology();
        }

        app
    }

    #[allow(dead_code)]
    fn initialize_topology(&mut self) {
        self.neighbors.clear();
        match self.topology_config.strategy {
            TopologyStrategy::Global => {
                // In Global mode, we don't strictly need neighbors map if we branch in step()
                // But for consistency/visualization we could populate it.
                // For now, leave empty and handle in step().
            }
            TopologyStrategy::Graph => {
                // Initialize a simple linear chain for now as a "basic graph"
                // or just leave disconnected until we have a better initialization logic.
                // Let's do a simple linear chain: 0-1-2-...
                if self.cells.is_empty() {
                    return;
                }

                for i in 0..self.cells.len() {
                    let current_id = self.cells[i].id.clone();
                    // Connect to previous
                    if i > 0 {
                        let prev_id = self.cells[i - 1].id.clone();
                        self.neighbors
                            .entry(current_id.clone())
                            .or_default()
                            .push(prev_id.clone());
                        self.neighbors
                            .entry(prev_id.clone())
                            .or_default()
                            .push(current_id.clone());

                        self.telemetry.record(
                            SystemTime::now(),
                            TelemetryEvent::LinkAdded {
                                source: current_id,
                                target: prev_id,
                            },
                        );
                    }
                }
            }
        }
    }

    fn calculate_topology_stats(&self) -> crate::telemetry::TopologyStats {
        if self.cells.is_empty() {
            return crate::telemetry::TopologyStats {
                avg_degree: 0.0,
                isolation_count: 0,
            };
        }

        let mut total_degree = 0;
        let mut isolation_count = 0;

        for cell in &self.cells {
            let degree = self.neighbors.get(&cell.id).map(|n| n.len()).unwrap_or(0);
            total_degree += degree;
            if degree == 0 {
                isolation_count += 1;
            }
        }

        crate::telemetry::TopologyStats {
            avg_degree: total_degree as f32 / self.cells.len() as f32,
            isolation_count,
        }
    }

    #[allow(dead_code)]
    pub fn step(&mut self, step_index: u32, threat_score: f32) {
        let signals = self.signal_bus.drain();

        let global_signals: Option<Vec<Signal>> =
            if matches!(self.topology_config.strategy, TopologyStrategy::Global) {
                // In Global, everyone sees everything.
                Some(signals.clone())
            } else {
                None
            };

        // For Graph mode, index signals by source
        let mut signals_by_source: HashMap<String, Vec<&Signal>> = HashMap::new();
        if matches!(self.topology_config.strategy, TopologyStrategy::Graph) {
            for signal in &signals {
                if let Some(ref source) = signal.source {
                    signals_by_source
                        .entry(source.clone())
                        .or_default()
                        .push(signal);
                }
            }
        }

        let mut actions = Vec::with_capacity(self.cells.len());

        let global_neighbors: Vec<String> =
            if matches!(self.topology_config.strategy, TopologyStrategy::Global) {
                self.cells.iter().map(|c| c.id.clone()).collect()
            } else {
                Vec::new()
            };

        for (index, cell) in self.cells.iter_mut().enumerate() {
            let neighbor_signals: Vec<Signal> = if let Some(ref globals) = global_signals {
                // In Global mode, we must still filter out signals from blacklisted sources per cell
                globals
                    .iter()
                    .filter(|s| {
                        if let Some(source) = &s.source {
                            !cell.state.blacklist.contains(source)
                        } else {
                            true // Allow system signals
                        }
                    })
                    .cloned()
                    .collect()
            } else {
                let mut cell_signals = Vec::new();
                // 1. Incorporate system signals (source == None)
                for signal in signals.iter().filter(|s| s.source.is_none()) {
                    if signal.target.as_ref().is_none_or(|t| t == &cell.id) {
                        cell_signals.push(signal.clone());
                    }
                }

                // 2. Incorporate neighbor signals
                if matches!(self.topology_config.strategy, TopologyStrategy::Global) {
                    // Global Mode: Iterate all signals, filtering blacklisted sources
                    for signal in signals.iter().filter(|s| s.source.is_some()) {
                        let source_id = signal.source.as_ref().unwrap();
                        if !cell.state.blacklist.contains(source_id)
                            && (signal.topic.starts_with("consensus:")
                                || signal.target.as_ref().is_none_or(|t| t == &cell.id))
                        {
                            cell_signals.push(signal.clone());
                        }
                    }
                } else {
                    // Graph Mode: Only look at adjacency list neighbors
                    if let Some(neighbors) = self.neighbors.get(&cell.id) {
                        for neighbor_id in neighbors {
                            // Extra check: ignore if blacklisted (redundant if link removed, but safe)
                            if cell.state.blacklist.contains(neighbor_id) {
                                continue;
                            }
                            if let Some(neighbor_signals) = signals_by_source.get(neighbor_id) {
                                for signal in neighbor_signals {
                                    if signal.topic.starts_with("consensus:")
                                        || signal.target.as_ref().is_none_or(|t| t == &cell.id)
                                    {
                                        cell_signals.push((*signal).clone());
                                    }
                                }
                            }
                        }
                    }
                }

                cell_signals
            };

            let detected_neighbors =
                if matches!(self.topology_config.strategy, TopologyStrategy::Global) {
                    global_neighbors
                        .iter()
                        .filter(|id| *id != &cell.id && !cell.state.blacklist.contains(id))
                        .cloned()
                        .collect()
                } else {
                    self.neighbors.get(&cell.id).cloned().unwrap_or_default()
                };

            let environment = CellEnvironment {
                step: step_index,
                local_threat_score: threat_score,
                neighbor_signals,
                detected_neighbors,
            };
            let action = cell.tick(&environment);
            actions.push((index, action));
        }

        for (index, action) in actions {
            self.handle_action(index, action);
        }

        // Remove dead cells
        let dead_ids: Vec<String> = self
            .cells
            .iter()
            .filter(|c| c.state.dead)
            .map(|c| c.id.clone())
            .collect();

        if !dead_ids.is_empty() {
            self.cells.retain(|c| !c.state.dead);

            if matches!(self.topology_config.strategy, TopologyStrategy::Graph) {
                for dead_id in dead_ids {
                    if let Some(neighbors) = self.neighbors.remove(&dead_id) {
                        for neighbor in neighbors {
                            self.telemetry.record(
                                SystemTime::now(),
                                TelemetryEvent::LinkRemoved {
                                    source: dead_id.clone(),
                                    target: neighbor,
                                },
                            );
                        }
                    }
                    for (neighbor_id, neighbors) in self.neighbors.iter_mut() {
                        if let Some(pos) = neighbors.iter().position(|x| x == &dead_id) {
                            neighbors.remove(pos);
                            // We already logged the link removal from the dead cell's perspective.
                            // Since it's an undirected graph (effectively), one event is enough to signify the break?
                            // Or should we log both directions? Let's stick to one event per "logical link" break if possible,
                            // but logging both is safer for reconstruction.
                            // Actually, let's just rely on the first loop to catch the explicit connections.
                            // But wait, if A is neighbor of B, B is neighbor of A.
                            // Removing A from B's list is the other half.
                            // Let's log it for completeness so the graph reconstruction is robust.
                            self.telemetry.record(
                                SystemTime::now(),
                                TelemetryEvent::LinkRemoved {
                                    source: neighbor_id.clone(),
                                    target: dead_id.clone(),
                                },
                            );
                        }
                    }
                }
            }
        }

        let cell_count = self.cells.len();
        let population_stats = if step_index.is_multiple_of(10) || cell_count < 500 {
            Some(PopulationStats::from_cells(&self.cells))
        } else {
            None
        };

        let topology_stats = Some(self.calculate_topology_stats());

        self.telemetry.record(
            SystemTime::now(),
            TelemetryEvent::StepSummary {
                step: step_index,
                threat_score,
                cell_count,
                population_stats,
                topology_stats,
            },
        );
    }

    fn handle_action(&mut self, index: usize, action: CellAction) {
        match action {
            CellAction::Idle => {}
            CellAction::Replicate(child_id) => {
                if self.cells.len() >= 100 {
                    return; // Cap population at 100
                }
                let mut child = SecurityCell::new(child_id.clone());
                // Inherit genome and immune memory from parent
                child.genome = self.cells[index].genome.clone();
                child.state.immune_memory = self.cells[index].state.immune_memory.clone();
                // Child starts with fresh trust map to avoid inheriting bias/stale data?
                // Or should it inherit "reputation data"?
                // Let's inherit it for now, assuming "gossip" is passed down.
                child.state.neighbor_trust = self.cells[index].state.neighbor_trust.clone();
                child.genome.mutate();

                let parent_id = self.cells[index].id.clone();

                if matches!(self.topology_config.strategy, TopologyStrategy::Graph) {
                    self.neighbors
                        .entry(parent_id.clone())
                        .or_default()
                        .push(child_id.clone());
                    self.neighbors
                        .entry(child_id.clone())
                        .or_default()
                        .push(parent_id.clone());

                    self.telemetry.record(
                        SystemTime::now(),
                        TelemetryEvent::LinkAdded {
                            source: parent_id.clone(),
                            target: child_id.clone(),
                        },
                    );
                }

                self.telemetry.record(
                    SystemTime::now(),
                    TelemetryEvent::CellReplicated {
                        cell_id: parent_id,
                        child_id,
                    },
                );
                self.cells.push(child);
            }
            CellAction::Differentiate(lineage) => {
                if let Some(cell) = self.cells.get_mut(index) {
                    cell.state.lineage = lineage.clone();
                }
                self.telemetry.record(
                    SystemTime::now(),
                    TelemetryEvent::LineageShift {
                        cell_id: self.cells[index].id.clone(),
                        lineage: format!("{lineage:?}"),
                    },
                );
            }
            CellAction::EmitSignal(topic, value) => {
                let cell_id = self.cells[index].id.clone();
                self.signal_bus.publish(Signal {
                    topic: topic.clone(),
                    value,
                    source: Some(cell_id.clone()),
                    target: None, // Broadcast by default
                    attestation: None,
                });
                self.telemetry.record(
                    SystemTime::now(),
                    TelemetryEvent::SignalEmitted {
                        cell_id,
                        topic,
                        value,
                    },
                );
            }
            CellAction::Die => {
                if let Some(cell) = self.cells.get_mut(index) {
                    cell.state.dead = true;
                    self.telemetry.record(
                        SystemTime::now(),
                        TelemetryEvent::CellDied {
                            cell_id: cell.id.clone(),
                        },
                    );
                }
            }
            CellAction::Disconnect(target_id) => {
                let cell_id = self.cells[index].id.clone();
                // Add to local blacklist regardless of topology strategy
                if let Some(cell) = self.cells.get_mut(index)
                    && !cell.state.blacklist.contains(&target_id)
                {
                    cell.state.blacklist.push(target_id.clone());
                }

                if matches!(self.topology_config.strategy, TopologyStrategy::Graph) {
                    // Remove forward link
                    if let Some(neighbors) = self.neighbors.get_mut(&cell_id)
                        && let Some(pos) = neighbors.iter().position(|x| x == &target_id)
                    {
                        neighbors.remove(pos);
                    }
                    // Remove backward link (undirected graph assumption for now, or just symmetric)
                    if let Some(neighbors) = self.neighbors.get_mut(&target_id)
                        && let Some(pos) = neighbors.iter().position(|x| x == &cell_id)
                    {
                        neighbors.remove(pos);
                    }

                    self.telemetry.record(
                        SystemTime::now(),
                        TelemetryEvent::LinkRemoved {
                            source: cell_id.clone(),
                            target: target_id.clone(),
                        },
                    );

                    // Immediate Mute: Purge pending signals from the disconnected target
                    // destined for this cell to prevent "final burst" attacks.
                    // Access inner queue via drain/retain (SignalBus logic)
                    // Wait, SignalBus doesn't expose queue directly?
                    // I need to add a purge method to SignalBus.
                    // Or access if public? `queue` is private in `signaling.rs`.
                    // Let's add `purge_from` to SignalBus.
                    self.signal_bus.purge_from(&target_id, &cell_id);
                } else if matches!(self.topology_config.strategy, TopologyStrategy::Global) {
                    // In Global mode, logical isolation is handled by the blacklist.
                    self.telemetry.record(
                        SystemTime::now(),
                        TelemetryEvent::PeerQuarantined { cell_id, target_id },
                    );
                }
            }
            CellAction::Connect(target_id) => {
                if matches!(self.topology_config.strategy, TopologyStrategy::Graph) {
                    let cell_id = self.cells[index].id.clone();
                    // Add forward link
                    self.neighbors
                        .entry(cell_id.clone())
                        .or_default()
                        .push(target_id.clone());
                    // Add backward link
                    self.neighbors
                        .entry(target_id.clone())
                        .or_default()
                        .push(cell_id.clone());

                    self.telemetry.record(
                        SystemTime::now(),
                        TelemetryEvent::LinkAdded {
                            source: cell_id,
                            target: target_id,
                        },
                    );
                }
            }
            CellAction::ReportAnomaly(topic, confidence, target, attestation) => {
                let cell_id = self.cells[index].id.clone();
                self.telemetry.record(
                    SystemTime::now(),
                    TelemetryEvent::AnomalyDetected {
                        cell_id: cell_id.clone(),
                        topic: topic.clone(),
                        confidence,
                    },
                );
                // Also publish a 'consensus' signal to neighbors
                self.signal_bus.publish(Signal {
                    topic: format!("consensus:{}", topic),
                    value: confidence,
                    source: Some(cell_id.clone()),
                    target,
                    attestation,
                });
                self.telemetry.record(
                    SystemTime::now(),
                    TelemetryEvent::VoteCast {
                        cell_id,
                        target_topic: topic,
                    },
                );
            }
        }
    }

    #[allow(dead_code)]
    pub fn telemetry(&self) -> &TSink {
        &self.telemetry
    }

    #[allow(dead_code)]
    pub fn telemetry_mut(&mut self) -> &mut TSink {
        &mut self.telemetry
    }

    #[allow(dead_code)]
    pub fn inject_signal(&mut self, signal: Signal) {
        self.signal_bus.publish(signal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::InMemorySink;

    #[test]
    fn test_graph_topology_isolation() {
        // Setup 3 cells: A, B, C
        // Linear topology: A <-> B <-> C
        // A signal from A should reach B but not C directly.

        let mut cells = Vec::new();
        let mut cell_a = SecurityCell::new("A");
        cell_a.genome.signal_emission_threshold = 0.4; // Low threshold
        let mut cell_b = SecurityCell::new("B");
        cell_b.genome.signal_emission_threshold = 0.4;
        let mut cell_c = SecurityCell::new("C");
        cell_c.genome.signal_emission_threshold = 0.4;

        cells.push(cell_a);
        cells.push(cell_b);
        cells.push(cell_c);

        let topology_config = TopologyConfig {
            strategy: TopologyStrategy::Graph,
            explicit_links: Some(vec![
                vec!["A".to_string(), "B".to_string()],
                vec!["B".to_string(), "C".to_string()],
            ]),
        };

        let telemetry = InMemorySink::default();
        let mut app = MorphogeneticApp::new(cells, telemetry, topology_config);

        // Inject signal "from A" (spoofed source)
        // Topic 'activator' increases effective threat.
        app.inject_signal(Signal {
            topic: "activator".to_string(),
            value: 0.5,
            source: Some("A".to_string()),
            target: None,
            attestation: None,
        });

        // Step 1
        // B is neighbor of A, should receive 0.5. Effective threat = 0.5 >= 0.4. Should emit signal.
        // C is NOT neighbor of A, should receive 0.0. Effective threat = 0.0 < 0.4. Should Idle.
        // A is source, but it also receives its own signal? No, we didn't implement self-loop explicitly,
        // but `neighbors` map doesn't include self unless added. My init logic doesn't add self.
        // So A receives 0.0 (unless B emits).

        app.step(0, 0.0);

        let events = app.telemetry().events();

        // Find SignalEmitted events
        let emissions: Vec<&TelemetryEvent> = events
            .iter()
            .filter(|e| matches!(e.event, TelemetryEvent::SignalEmitted { .. }))
            .map(|e| &e.event)
            .collect();

        // Check B emitted
        let b_emitted = emissions.iter().any(|e| match e {
            TelemetryEvent::SignalEmitted { cell_id, .. } => cell_id == "B",
            _ => false,
        });

        // Check C emitted
        let c_emitted = emissions.iter().any(|e| match e {
            TelemetryEvent::SignalEmitted { cell_id, .. } => cell_id == "C",
            _ => false,
        });

        assert!(
            b_emitted,
            "Cell B should have received signal from A and emitted response"
        );
        assert!(
            !c_emitted,
            "Cell C should NOT have received signal from A directly"
        );
    }

    #[test]
    fn test_global_topology_broadcast() {
        // Setup 3 cells: A, B, C
        // Global topology
        // Signal from A should reach B and C.

        let mut cells = Vec::new();
        let mut cell_a = SecurityCell::new("A");
        cell_a.genome.signal_emission_threshold = 0.4;
        let mut cell_b = SecurityCell::new("B");
        cell_b.genome.signal_emission_threshold = 0.4;
        let mut cell_c = SecurityCell::new("C");
        cell_c.genome.signal_emission_threshold = 0.4;

        cells.push(cell_a);
        cells.push(cell_b);
        cells.push(cell_c);

        let topology_config = TopologyConfig {
            strategy: TopologyStrategy::Global,
            explicit_links: None,
        };

        let telemetry = InMemorySink::default();
        let mut app = MorphogeneticApp::new(cells, telemetry, topology_config);

        app.inject_signal(Signal {
            topic: "activator".to_string(),
            value: 0.5,
            source: Some("A".to_string()), // Source shouldn't matter for Global, but we provide it
            target: None,
            attestation: None,
        });

        app.step(0, 0.0);

        let events = app.telemetry().events();
        let emissions: Vec<&TelemetryEvent> = events
            .iter()
            .filter(|e| matches!(e.event, TelemetryEvent::SignalEmitted { .. }))
            .map(|e| &e.event)
            .collect();

        let b_emitted = emissions.iter().any(|e| match e {
            TelemetryEvent::SignalEmitted { cell_id, .. } => cell_id == "B",
            _ => false,
        });

        let c_emitted = emissions.iter().any(|e| match e {
            TelemetryEvent::SignalEmitted { cell_id, .. } => cell_id == "C",
            _ => false,
        });

        assert!(b_emitted, "Cell B should have received global signal");
        assert!(c_emitted, "Cell C should have received global signal");
    }

    #[test]
    fn test_blacklist_behavior() {
        // Setup: A -> B. A is blacklisted by B.
        let cell_a = SecurityCell::new("A");
        let mut cell_b = SecurityCell::new("B");
        cell_b.genome.signal_emission_threshold = 0.4;
        cell_b.state.blacklist.push("A".to_string());

        let cells = vec![cell_a, cell_b];
        let topology = TopologyConfig {
            strategy: TopologyStrategy::Graph,
            explicit_links: Some(vec![vec!["A".into(), "B".into()]]),
        };
        let mut app = MorphogeneticApp::new(cells, InMemorySink::default(), topology);

        // A emits signal
        app.inject_signal(Signal {
            topic: "activator".into(),
            value: 0.9,
            source: Some("A".into()),
            target: None,
            attestation: None,
        });

        app.step(0, 0.0);

        // B should NOT have emitted signal because it ignored A
        let events = app.telemetry().events();
        let b_emitted = events.iter().any(
            |e| matches!(&e.event, TelemetryEvent::SignalEmitted { cell_id, .. } if cell_id == "B"),
        );
        assert!(!b_emitted, "B should ignore signal from blacklisted A");
    }

    #[test]
    fn test_dead_cell_pruning_and_telemetry() {
        let cell_a = SecurityCell::new("A");
        let cell_b = SecurityCell::new("B");
        let cells = vec![cell_a, cell_b];
        let topology = TopologyConfig {
            strategy: TopologyStrategy::Graph,
            explicit_links: Some(vec![vec!["A".into(), "B".into()]]),
        };
        let mut app = MorphogeneticApp::new(cells, InMemorySink::default(), topology);

        // Kill A manually
        app.cells[0].state.dead = true;

        app.step(0, 0.0);

        // Verify A is gone
        assert_eq!(app.cells.len(), 1);
        assert_eq!(app.cells[0].id, "B");

        // Verify neighbor maps updated
        assert!(app.neighbors.get("A").is_none());
        let b_neighbors = app.neighbors.get("B").unwrap();
        assert!(b_neighbors.is_empty());

        // Verify telemetry
        let events = app.telemetry().events();
        let link_removed = events.iter().any(|e| {
            matches!(&e.event, TelemetryEvent::LinkRemoved { source, target }
            if (source == "A" && target == "B") || (source == "B" && target == "A"))
        });
        assert!(link_removed, "LinkRemoved event should be recorded");

        let _cell_died = events
            .iter()
            .any(|e| matches!(&e.event, TelemetryEvent::CellDied { cell_id } if cell_id == "A"));
        // CellDied is recorded during handle_action only if Die action returned?
        // Ah, step() loop: "Remove dead cells". But TelemetryEvent::CellDied is recorded in handle_action(CellAction::Die).
        // If I manually set .state.dead = true, handle_action isn't called for Die action.
        // Wait, step() loop does:
        // actions.push((index, action)); ... then handle_action.
        // Then "Remove dead cells".
        // If I manually set dead=true BEFORE step, A might still tick?
        // step() iterates `self.cells.iter_mut()`.
        // A is dead. Does tick() handle dead cells?
        // `SecurityCell::tick`: if self.state.energy <= 0.01 -> return CellAction::Die.
        // It doesn't check `dead` flag explicitly at start of tick.
        // But logic: `if self.state.energy <= 0.01 { return CellAction::Die; }`

        // So manually setting dead=true might not trigger CellAction::Die unless energy is low.
        // But the pruning logic uses `c.state.dead`.
        // So the cell IS pruned.
        // But TelemetryEvent::CellDied might NOT be emitted if the cell didn't return Die action.

        // This is a subtle behavior. If cell dies "naturally" (energy low), it emits action -> recorded.
        // If cell is killed externally (manually in test), it might not be recorded as event unless we do it.
        // But the pruning happens regardless.
        // The prompt asks to "Guarantee dead cells are pruned".
        // And "Ensure link add/remove telemetry fires".

        // The LinkRemoved events happen in the pruning block:
        // `self.telemetry.record(..., LinkRemoved ...)`
        // So LinkRemoved IS verified above.

        // I won't assert CellDied here because I bypassed the natural death mechanism.
    }

    #[test]
    fn test_graph_topology_auto_initialization() {
        let cells = vec![SecurityCell::new("A"), SecurityCell::new("B")];
        let topology = TopologyConfig {
            strategy: TopologyStrategy::Graph,
            explicit_links: None,
        };
        let app = MorphogeneticApp::<InMemorySink>::new(cells, InMemorySink::default(), topology);

        // neighbors should have A-B link due to initialize_topology's linear chain fallback
        assert!(app.neighbors.contains_key("A"));
        assert!(app.neighbors.get("A").unwrap().contains(&"B".to_string()));
    }
}
