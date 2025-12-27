//! Cellular automaton primitives for morphogenetic security nodes.
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::immune::{Attestation, ThreatEvent, TPM};
use crate::signaling::Signal;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellEnvironment {
    pub step: u32,
    pub local_threat_score: f32,
    pub neighbor_signals: Vec<Signal>,
    pub detected_neighbors: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellLineage {
    Stem,
    Firewall,
    IntrusionDetection,
    Encryption,
    Healer,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellState {
    pub lineage: CellLineage,
    pub energy: f32,
    pub stress_level: f32,
    pub dead: bool,
    #[serde(default)]
    pub immune_memory: Vec<ThreatEvent>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellGenome {
    pub threat_inhibitor_factor: f32,
    pub stress_decay: f32,
    pub stress_sensitivity: f32,
    pub energy_recharge: f32,
    pub energy_threat_drain: f32,
    pub energy_inhibitor_drain: f32,
    pub reproduction_threshold: f32,
    pub reproduction_energy_cost: f32,
    pub reproduction_energy_min: f32,
    pub stress_differentiation_threshold: f32,
    pub healer_inhibitor_threshold: f32,
    pub healer_stress_limit: f32,
    pub encryption_cooperative_threshold: f32,
    pub encryption_energy_min: f32,
    pub signal_emission_threshold: f32,
    pub connection_cost: f32,
    pub isolation_threshold: f32,
    pub anomaly_sensitivity: f32,
}

impl Default for CellGenome {
    fn default() -> Self {
        Self {
            threat_inhibitor_factor: 0.35,
            stress_decay: 0.45,
            stress_sensitivity: 0.7,
            energy_recharge: 0.08,
            energy_threat_drain: 0.25,
            energy_inhibitor_drain: 0.1,
            reproduction_threshold: 0.75,
            reproduction_energy_cost: 0.3,
            reproduction_energy_min: 0.6,
            stress_differentiation_threshold: 0.75,
            healer_inhibitor_threshold: 0.6,
            healer_stress_limit: 0.3,
            encryption_cooperative_threshold: 0.5,
            encryption_energy_min: 0.9,
            signal_emission_threshold: 0.4,
            connection_cost: 0.1,
            isolation_threshold: 0.85,
            anomaly_sensitivity: 0.5,
        }
    }
}

impl CellGenome {
    #[allow(dead_code)]
    pub fn mutate(&mut self) {
        let mut rng = rand::thread_rng();
        let rate = 0.05; // 5% chance per gene
        let strength = 0.1; // +/- 0.1 change

        let mut mutate_field = |field: &mut f32| {
            if rng.gen_bool(rate) {
                *field += rng.gen_range(-strength..=strength);
                *field = field.max(0.01); // Keep positive
            }
        };

        mutate_field(&mut self.threat_inhibitor_factor);
        mutate_field(&mut self.stress_decay);
        mutate_field(&mut self.stress_sensitivity);
        mutate_field(&mut self.energy_recharge);
        mutate_field(&mut self.energy_threat_drain);
        mutate_field(&mut self.energy_inhibitor_drain);
        mutate_field(&mut self.reproduction_threshold);
        mutate_field(&mut self.reproduction_energy_cost);
        mutate_field(&mut self.reproduction_energy_min);
        mutate_field(&mut self.stress_differentiation_threshold);
        mutate_field(&mut self.healer_inhibitor_threshold);
        mutate_field(&mut self.healer_stress_limit);
        mutate_field(&mut self.encryption_cooperative_threshold);
        mutate_field(&mut self.encryption_energy_min);
        mutate_field(&mut self.signal_emission_threshold);
        mutate_field(&mut self.connection_cost);
        mutate_field(&mut self.isolation_threshold);
        mutate_field(&mut self.anomaly_sensitivity);
    }

    #[allow(dead_code)]
    pub fn apply_immune_memory(&self, memory: &[ThreatEvent]) -> Self {
        let mut adapted = self.clone();
        for event in memory {
            if event.topic == "activator" {
                // Harden against activator by decreasing sensitivity
                adapted.stress_sensitivity *= (1.0 - 0.05 * event.confidence).max(0.5);
                // And increasing inhibitor effectiveness
                adapted.threat_inhibitor_factor *= (1.0 + 0.05 * event.confidence).min(2.0);
            }
        }
        adapted
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationStats {
    pub avg_reproduction_threshold: f32,
    pub avg_stress_sensitivity: f32,
    pub avg_energy_recharge: f32,
    pub avg_threat_inhibitor_factor: f32,
    // Add other key stats as needed, keeping it concise for now
}

impl PopulationStats {
    pub fn from_cells(cells: &[SecurityCell]) -> Self {
        if cells.is_empty() {
            return Self {
                avg_reproduction_threshold: 0.0,
                avg_stress_sensitivity: 0.0,
                avg_energy_recharge: 0.0,
                avg_threat_inhibitor_factor: 0.0,
            };
        }

        let count = cells.len() as f32;
        let mut sum_repro = 0.0;
        let mut sum_stress = 0.0;
        let mut sum_energy = 0.0;
        let mut sum_inhib = 0.0;

        for cell in cells {
            sum_repro += cell.genome.reproduction_threshold;
            sum_stress += cell.genome.stress_sensitivity;
            sum_energy += cell.genome.energy_recharge;
            sum_inhib += cell.genome.threat_inhibitor_factor;
        }

        Self {
            avg_reproduction_threshold: sum_repro / count,
            avg_stress_sensitivity: sum_stress / count,
            avg_energy_recharge: sum_energy / count,
            avg_threat_inhibitor_factor: sum_inhib / count,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SecurityCell {
    pub id: String,
    pub state: CellState,
    pub genome: CellGenome,
    pub tpm: TPM,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum CellAction {
    Idle,
    Replicate(String),
    Differentiate(CellLineage),
    EmitSignal(String, f32),
    Die,
    Connect(String),
    Disconnect(String),
    ReportAnomaly(String, f32, Option<Attestation>),
}

impl SecurityCell {
    #[allow(dead_code)]
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            state: CellState {
                lineage: CellLineage::Stem,
                energy: 1.0,
                stress_level: 0.0,
                dead: false,
                immune_memory: Vec::new(),
            },
            genome: CellGenome::default(),
            tpm: TPM::new(id),
        }
    }

    #[allow(dead_code)]
    pub fn tick(&mut self, environment: &CellEnvironment) -> CellAction {
        let mut activator = 0.0;
        let mut inhibitor = 0.0;
        let mut cooperative = 0.0;
        let mut consensus_votes = 0.0;

        for signal in &environment.neighbor_signals {
            match signal.topic.as_str() {
                "activator" => activator += signal.value,
                "inhibitor" => inhibitor += signal.value,
                "cooperative" => cooperative += signal.value,
                topic if topic.starts_with("consensus:") => {
                    // Only count vote if attestation is valid
                    if let Some(attestation) = &signal.attestation {
                        if TPM::verify(attestation) {
                            consensus_votes += signal.value;
                        }
                    }
                }
                _ => {}
            }
        }

        let effective_threat = (environment.local_threat_score + activator
            - inhibitor * self.genome.threat_inhibitor_factor)
            .max(0.0);

        self.state.stress_level = (self.state.stress_level * self.genome.stress_decay
            + effective_threat * self.genome.stress_sensitivity)
            .clamp(0.0, 1.0);
        self.state.energy = (self.state.energy + self.genome.energy_recharge
            - effective_threat * self.genome.energy_threat_drain
            - inhibitor * self.genome.energy_inhibitor_drain)
            .clamp(0.0, 1.5);

        if self.state.energy <= 0.01 {
            return CellAction::Die;
        }

        // 1. Coordinated Quarantine: Disconnect if neighbors reach consensus on high threat
        if consensus_votes > 1.5 && !environment.detected_neighbors.is_empty() {
             if let Some(target) = environment.detected_neighbors.first() {
                 return CellAction::Disconnect(target.clone());
             }
        }

        // 2. Individual Isolation
        if self.state.stress_level > self.genome.isolation_threshold && !environment.detected_neighbors.is_empty() {
             if let Some(target) = environment.detected_neighbors.first() {
                 return CellAction::Disconnect(target.clone());
             }
        }

        // Anomaly Detection (IntrusionDetection lineage only)
        if matches!(self.state.lineage, CellLineage::IntrusionDetection) 
           && effective_threat > self.genome.anomaly_sensitivity 
           && inhibitor < 0.2 // Not being suppressed
        {
            // Record in memory if not already there recently
            if !self.state.immune_memory.iter().any(|e| e.topic == "activator" && e.step > 0) {
                self.state.immune_memory.push(ThreatEvent {
                    step: environment.step, // Use actual step
                    topic: "activator".to_string(),
                    magnitude: effective_threat,
                    confidence: 0.8,
                });
            }
            let attestation = self.tpm.attest(environment.step as u64);
            return CellAction::ReportAnomaly("activator".to_string(), effective_threat, attestation);
        }

        if effective_threat >= self.genome.reproduction_threshold
            && self.state.energy >= self.genome.reproduction_energy_min
        {
            self.state.energy = (self.state.energy - self.genome.reproduction_energy_cost).max(0.0);
            return CellAction::Replicate(format!("{}::child", self.id));
        }

        if self.state.stress_level >= self.genome.stress_differentiation_threshold
            && !matches!(self.state.lineage, CellLineage::IntrusionDetection)
        {
            return CellAction::Differentiate(CellLineage::IntrusionDetection);
        }

        if inhibitor >= self.genome.healer_inhibitor_threshold
            && self.state.stress_level <= self.genome.healer_stress_limit
            && !matches!(self.state.lineage, CellLineage::Healer)
        {
            return CellAction::Differentiate(CellLineage::Healer);
        }

        if cooperative >= self.genome.encryption_cooperative_threshold
            && self.state.energy >= self.genome.encryption_energy_min
            && !matches!(self.state.lineage, CellLineage::Encryption)
        {
            return CellAction::Differentiate(CellLineage::Encryption);
        }

        if effective_threat >= self.genome.signal_emission_threshold {
            return CellAction::EmitSignal("activator".to_string(), effective_threat);
        }

        CellAction::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_with_threat(threat: f32) -> CellEnvironment {
        CellEnvironment {
            step: 0,
            local_threat_score: threat,
            neighbor_signals: Vec::new(),
            detected_neighbors: Vec::new(),
        }
    }

    #[test]
    fn cell_replicates_when_threat_exceeds_threshold() {
        let mut cell = SecurityCell::new("alpha");
        cell.genome.reproduction_threshold = 0.5;
        let action = cell.tick(&env_with_threat(0.85));
        assert!(matches!(action, CellAction::Replicate(_)));
        assert!(cell.state.energy < 1.0);
    }

    #[test]
    fn cell_differentiates_under_stress() {
        let mut cell = SecurityCell::new("beta");
        cell.state.stress_level = 0.82;
        cell.genome.reproduction_threshold = 1.0;
        let action = cell.tick(&env_with_threat(0.6));
        match action {
            CellAction::Differentiate(lineage) => {
                assert_eq!(lineage, CellLineage::IntrusionDetection);
            }
            other => panic!("expected differentiation, got {other:?}"),
        }
    }

    #[test]
    fn cell_transitions_to_healer_with_inhibitor_support() {
        let mut cell = SecurityCell::new("gamma");
        cell.state.stress_level = 0.2;
        let mut signals = Vec::new();
        signals.push(Signal {
             topic: "inhibitor".to_string(),
             value: 0.65,
             source: None,
             target: None,
             attestation: None,
        });
        let environment = CellEnvironment {
            step: 0,
            local_threat_score: 0.05,
            neighbor_signals: signals,
            detected_neighbors: Vec::new(),
        };
        let action = cell.tick(&environment);
        match action {
            CellAction::Differentiate(lineage) => {
                assert_eq!(lineage, CellLineage::Healer);
            }
            other => panic!("expected healer differentiation, got {other:?}"),
        }
    }

    #[test]
    fn cell_emits_signal_on_moderate_threat() {
        let mut cell = SecurityCell::new("delta");
        let mut signals = Vec::new();
        signals.push(Signal {
             topic: "activator".to_string(),
             value: 0.1,
             source: None,
             target: None,
             attestation: None,
        });
        let environment = CellEnvironment {
            step: 0,
            local_threat_score: 0.45,
            neighbor_signals: signals,
            detected_neighbors: Vec::new(),
        };
        let action = cell.tick(&environment);
        match action {
            CellAction::EmitSignal(topic, value) => {
                assert_eq!(topic, "activator");
                assert!(value >= 0.4);
            }
            other => panic!("expected signal emission, got {other:?}"),
        }
    }

    #[test]
    fn cell_idles_when_conditions_are_nominal() {
        let mut cell = SecurityCell::new("epsilon");
        let action = cell.tick(&env_with_threat(0.1));
        assert!(matches!(action, CellAction::Idle));
    }

    #[test]
    fn cell_disconnects_under_extreme_stress() {
        let mut cell = SecurityCell::new("zeta");
        cell.state.stress_level = 0.95;
        cell.genome.isolation_threshold = 0.9;
        
        let mut environment = env_with_threat(0.8);
        environment.detected_neighbors.push("bad_neighbor".to_string());
        
        let action = cell.tick(&environment);
        match action {
            CellAction::Disconnect(target) => {
                assert_eq!(target, "bad_neighbor");
            },
            other => panic!("expected disconnect, got {other:?}"),
        }
    }

    #[test]
    fn test_swarm_coordinated_quarantine() {
        let mut cell = SecurityCell::new("iota");
        cell.state.stress_level = 0.1; // Low stress
        cell.genome.isolation_threshold = 0.9;
        
        let mut environment = env_with_threat(0.0);
        environment.detected_neighbors.push("neighbor_1".to_string());
        
        // Mock TPM for neighbor to generate valid attestation
        let neighbor_tpm = TPM::new("neighbor_1".to_string());
        let attestation = neighbor_tpm.attest(0).unwrap();

        // Two neighbors reporting consensus
        environment.neighbor_signals.push(Signal {
             topic: "consensus:activator".to_string(),
             value: 2.0,
             source: Some("neighbor_1".to_string()),
             target: None,
             attestation: Some(attestation),
        });
        
        let action = cell.tick(&environment);
        match action {
            CellAction::Disconnect(target) => {
                assert_eq!(target, "neighbor_1");
            },
            other => panic!("expected coordinated disconnect, got {other:?}"),
        }
    }

    #[test]
    fn test_anomaly_detection_report() {
        let mut cell = SecurityCell::new("kappa");
        cell.state.lineage = CellLineage::IntrusionDetection;
        cell.genome.anomaly_sensitivity = 0.4;
        
        let mut signals = Vec::new();
        signals.push(Signal {
             topic: "inhibitor".to_string(),
             value: 0.1,
             source: None,
             target: None,
             attestation: None,
        });

        let environment = CellEnvironment {
            step: 5,
            local_threat_score: 0.6,
            neighbor_signals: signals,
            detected_neighbors: Vec::new(),
        };
        
        let action = cell.tick(&environment);
        match action {
            CellAction::ReportAnomaly(topic, confidence, attestation) => {
                assert_eq!(topic, "activator");
                assert!(confidence >= 0.5);
                assert!(attestation.is_some());
                assert!(attestation.unwrap().timestamp == 5);
            },
            other => panic!("expected anomaly report, got {other:?}"),
        }
    }
}
