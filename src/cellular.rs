//! Cellular automaton primitives for morphogenetic security nodes.

use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CellEnvironment {
    pub local_threat_score: f32,
    pub neighbor_signals: HashMap<String, f32>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellLineage {
    Stem,
    Firewall,
    IntrusionDetection,
    Encryption,
    Healer,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CellState {
    pub lineage: CellLineage,
    pub energy: f32,
    pub stress_level: f32,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SecurityCell {
    pub id: String,
    pub state: CellState,
    pub reproduction_threshold: f32,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum CellAction {
    Idle,
    Replicate(String),
    Differentiate(CellLineage),
    EmitSignal(String, f32),
}

impl SecurityCell {
    #[allow(dead_code)]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            state: CellState {
                lineage: CellLineage::Stem,
                energy: 1.0,
                stress_level: 0.0,
            },
            reproduction_threshold: 0.75,
        }
    }

    #[allow(dead_code)]
    pub fn tick(&mut self, environment: &CellEnvironment) -> CellAction {
        let activator = environment
            .neighbor_signals
            .get("activator")
            .copied()
            .unwrap_or(0.0);
        let inhibitor = environment
            .neighbor_signals
            .get("inhibitor")
            .copied()
            .unwrap_or(0.0);
        let cooperative = environment
            .neighbor_signals
            .get("cooperative")
            .copied()
            .unwrap_or(0.0);

        let effective_threat =
            (environment.local_threat_score + activator - inhibitor * 0.35).max(0.0);

        self.state.stress_level =
            (self.state.stress_level * 0.45 + effective_threat * 0.7).clamp(0.0, 1.0);
        self.state.energy =
            (self.state.energy + 0.08 - effective_threat * 0.25 - inhibitor * 0.1).clamp(0.0, 1.5);

        if effective_threat >= self.reproduction_threshold && self.state.energy >= 0.6 {
            self.state.energy = (self.state.energy - 0.3).max(0.0);
            return CellAction::Replicate(format!("{}::child", self.id));
        }

        if self.state.stress_level >= 0.75
            && !matches!(self.state.lineage, CellLineage::IntrusionDetection)
        {
            return CellAction::Differentiate(CellLineage::IntrusionDetection);
        }

        if inhibitor >= 0.6
            && self.state.stress_level <= 0.3
            && !matches!(self.state.lineage, CellLineage::Healer)
        {
            return CellAction::Differentiate(CellLineage::Healer);
        }

        if cooperative >= 0.5
            && self.state.energy >= 0.9
            && !matches!(self.state.lineage, CellLineage::Encryption)
        {
            return CellAction::Differentiate(CellLineage::Encryption);
        }

        if effective_threat >= 0.4 {
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
            local_threat_score: threat,
            neighbor_signals: HashMap::new(),
        }
    }

    #[test]
    fn cell_replicates_when_threat_exceeds_threshold() {
        let mut cell = SecurityCell::new("alpha");
        cell.reproduction_threshold = 0.5;
        let action = cell.tick(&env_with_threat(0.85));
        assert!(matches!(action, CellAction::Replicate(_)));
        assert!(cell.state.energy < 1.0);
    }

    #[test]
    fn cell_differentiates_under_stress() {
        let mut cell = SecurityCell::new("beta");
        cell.state.stress_level = 0.82;
        cell.reproduction_threshold = 1.0;
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
        let mut signals = HashMap::new();
        signals.insert("inhibitor".to_string(), 0.65);
        let environment = CellEnvironment {
            local_threat_score: 0.05,
            neighbor_signals: signals,
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
        let mut signals = HashMap::new();
        signals.insert("activator".to_string(), 0.1);
        let environment = CellEnvironment {
            local_threat_score: 0.45,
            neighbor_signals: signals,
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
}
