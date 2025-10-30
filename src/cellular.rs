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
        // Placeholder logic; future iterations will implement reaction–diffusion dynamics.
        if environment.local_threat_score > self.reproduction_threshold {
            return CellAction::Replicate(format!("{}::child", self.id));
        }

        if self.state.stress_level > 0.5 {
            return CellAction::Differentiate(CellLineage::IntrusionDetection);
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
        let action = cell.tick(&env_with_threat(0.75));
        assert!(matches!(action, CellAction::Replicate(_)));
    }

    #[test]
    fn cell_differentiates_under_stress() {
        let mut cell = SecurityCell::new("beta");
        cell.state.stress_level = 0.6;
        let action = cell.tick(&env_with_threat(0.1));
        match action {
            CellAction::Differentiate(lineage) => {
                assert_eq!(lineage, CellLineage::IntrusionDetection);
            }
            other => panic!("expected differentiation, got {other:?}"),
        }
    }

    #[test]
    fn cell_idles_when_conditions_are_nominal() {
        let mut cell = SecurityCell::new("gamma");
        let action = cell.tick(&env_with_threat(0.1));
        assert!(matches!(action, CellAction::Idle));
    }
}
