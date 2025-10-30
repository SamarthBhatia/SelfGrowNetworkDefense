//! High-level orchestration for the morphogenetic security system.

use crate::cellular::{CellAction, CellEnvironment, SecurityCell};
use crate::signaling::{Signal, SignalBus};
use crate::telemetry::{TelemetryEvent, TelemetrySink};
use std::collections::HashMap;
use std::time::SystemTime;

#[allow(dead_code)]
pub struct MorphogeneticApp<TSink: TelemetrySink> {
    cells: Vec<SecurityCell>,
    signal_bus: SignalBus,
    telemetry: TSink,
}

impl<TSink: TelemetrySink> MorphogeneticApp<TSink> {
    #[allow(dead_code)]
    pub fn new(cells: Vec<SecurityCell>, telemetry: TSink) -> Self {
        Self {
            cells,
            signal_bus: SignalBus::default(),
            telemetry,
        }
    }

    #[allow(dead_code)]
    pub fn step(&mut self, step_index: u32, threat_score: f32) {
        let signals = self.signal_bus.drain();
        let mut neighbor_signals = HashMap::new();
        for signal in &signals {
            neighbor_signals
                .entry(signal.topic.clone())
                .and_modify(|value| {
                    *value = (*value + signal.value) * 0.5;
                })
                .or_insert(signal.value);
        }

        let mut actions = Vec::with_capacity(self.cells.len());

        for (index, cell) in self.cells.iter_mut().enumerate() {
            let environment = CellEnvironment {
                local_threat_score: threat_score,
                neighbor_signals: neighbor_signals.clone(),
            };
            let action = cell.tick(&environment);
            actions.push((index, action));
        }

        for (index, action) in actions {
            self.handle_action(index, action);
        }

        let cell_count = self.cells.len();
        self.telemetry.record(
            SystemTime::now(),
            TelemetryEvent::StepSummary {
                step: step_index,
                threat_score,
                cell_count,
            },
        );
    }

    fn handle_action(&mut self, index: usize, action: CellAction) {
        match action {
            CellAction::Idle => {}
            CellAction::Replicate(child_id) => {
                let child = SecurityCell::new(child_id.clone());
                let parent_id = self.cells[index].id.clone();
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
