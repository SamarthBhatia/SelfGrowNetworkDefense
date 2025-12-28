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
    pub fn new(cells: Vec<SecurityCell>, telemetry: TSink, topology_config: TopologyConfig) -> Self {
        let mut app = Self {
            cells,
            signal_bus: SignalBus::default(),
            telemetry,
            topology_config,
            neighbors: HashMap::new(),
        };
        app.initialize_topology();
        app
    }

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
                        let prev_id = self.cells[i-1].id.clone();
                        self.neighbors.entry(current_id.clone()).or_default().push(prev_id.clone());
                        self.neighbors.entry(prev_id.clone()).or_default().push(current_id.clone());
                        
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
             return crate::telemetry::TopologyStats { avg_degree: 0.0, isolation_count: 0 };
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
        
        let global_signals: Option<Vec<Signal>> = if matches!(self.topology_config.strategy, TopologyStrategy::Global) {
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
                    signals_by_source.entry(source.clone()).or_default().push(signal);
                }
            }
        }

        let mut actions = Vec::with_capacity(self.cells.len());

        let global_neighbors: Vec<String> = if matches!(self.topology_config.strategy, TopologyStrategy::Global) {
            self.cells.iter().map(|c| c.id.clone()).collect()
        } else {
            Vec::new()
        };

        for (index, cell) in self.cells.iter_mut().enumerate() {
            let neighbor_signals: Vec<Signal> = if let Some(ref globals) = global_signals {
                globals.clone()
            } else {
                let mut cell_signals = Vec::new();
                // 1. Incorporate system signals (source == None)
                for signal in signals.iter().filter(|s| s.source.is_none()) {
                    if signal.target.as_ref().map_or(true, |t| t == &cell.id) {
                        cell_signals.push(signal.clone());
                    }
                }

                // 2. Incorporate neighbor signals
                if matches!(self.topology_config.strategy, TopologyStrategy::Global) {
                    // Global Mode: Iterate all signals, filtering blacklisted sources
                    for signal in signals.iter().filter(|s| s.source.is_some()) {
                        let source_id = signal.source.as_ref().unwrap();
                        if !cell.state.blacklist.contains(source_id) {
                             if signal.target.as_ref().map_or(true, |t| t == &cell.id) {
                                cell_signals.push(signal.clone());
                            }
                        }
                    }
                } else {
                    // Graph Mode: Only look at adjacency list neighbors
                    if let Some(neighbors) = self.neighbors.get(&cell.id) {
                        for neighbor_id in neighbors {
                            if let Some(neighbor_signals) = signals_by_source.get(neighbor_id) {
                                for signal in neighbor_signals {
                                    if signal.target.as_ref().map_or(true, |t| t == &cell.id) {
                                        cell_signals.push((*signal).clone());
                                    }
                                }
                            }
                        }
                    }
                }
                
                cell_signals
            };

            let detected_neighbors = if matches!(self.topology_config.strategy, TopologyStrategy::Global) {
                global_neighbors.iter()
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
        let dead_ids: Vec<String> = self.cells.iter()
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
        let population_stats = if step_index % 10 == 0 || cell_count < 100 {
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
                    self.neighbors.entry(parent_id.clone()).or_default().push(child_id.clone());
                    self.neighbors.entry(child_id.clone()).or_default().push(parent_id.clone());
                    
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
                if let Some(cell) = self.cells.get_mut(index) {
                    if !cell.state.blacklist.contains(&target_id) {
                        cell.state.blacklist.push(target_id.clone());
                    }
                }

                if matches!(self.topology_config.strategy, TopologyStrategy::Graph) {
                     // Remove forward link
                     if let Some(neighbors) = self.neighbors.get_mut(&cell_id) {
                         if let Some(pos) = neighbors.iter().position(|x| x == &target_id) {
                             neighbors.remove(pos);
                         }
                     }
                     // Remove backward link (undirected graph assumption for now, or just symmetric)
                     if let Some(neighbors) = self.neighbors.get_mut(&target_id) {
                         if let Some(pos) = neighbors.iter().position(|x| x == &cell_id) {
                             neighbors.remove(pos);
                         }
                     }
                     
                     self.telemetry.record(
                         SystemTime::now(),
                         TelemetryEvent::LinkRemoved {
                             source: cell_id,
                             target: target_id,
                         },
                     );
                } else if matches!(self.topology_config.strategy, TopologyStrategy::Global) {
                     // In Global mode, logical isolation is handled by the blacklist.
                     self.telemetry.record(
                         SystemTime::now(),
                         TelemetryEvent::LinkRemoved {
                             source: cell_id,
                             target: target_id,
                         },
                     );
                }
            }
            CellAction::Connect(target_id) => {
                 if matches!(self.topology_config.strategy, TopologyStrategy::Graph) {
                     let cell_id = self.cells[index].id.clone();
                     // Add forward link
                     self.neighbors.entry(cell_id.clone()).or_default().push(target_id.clone());
                     // Add backward link
                     self.neighbors.entry(target_id.clone()).or_default().push(cell_id.clone());
                     
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
        let emissions: Vec<&TelemetryEvent> = events.iter().filter(|e| matches!(e.event, TelemetryEvent::SignalEmitted { .. })).map(|e| &e.event).collect();

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

        assert!(b_emitted, "Cell B should have received signal from A and emitted response");
        assert!(!c_emitted, "Cell C should NOT have received signal from A directly");
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
        let emissions: Vec<&TelemetryEvent> = events.iter().filter(|e| matches!(e.event, TelemetryEvent::SignalEmitted { .. })).map(|e| &e.event).collect();

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
}
