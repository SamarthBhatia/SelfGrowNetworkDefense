//! Swarm immune response and distributed anomaly detection logic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A recorded threat event in a cell's local memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatEvent {
    pub step: u32,
    pub topic: String,
    pub magnitude: f32,
    pub confidence: f32,
}

/// Swarm-level consensus state for a specific threat.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SwarmConsensus {
    /// Mapping of threat topic to number of votes.
    pub votes: HashMap<String, u32>,
    /// Confirmed threats that have passed the consensus threshold.
    pub confirmed: Vec<String>,
}

/// Simulated cryptographic attestation token.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attestation {
    pub cell_id: String,
    pub timestamp: u64,
    pub signature: String,
    pub valid: bool,
}

/// Simulated Trusted Platform Module (TPM).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPM {
    pub cell_id: String,
    pub compromised: bool,
}

impl TPM {
    pub fn new(cell_id: String) -> Self {
        Self {
            cell_id,
            compromised: false,
        }
    }

    pub fn attest(&self, timestamp: u64) -> Option<Attestation> {
        if self.compromised {
            None
        } else {
            Some(Attestation {
                cell_id: self.cell_id.clone(),
                timestamp,
                signature: format!("sig_{}_{}", self.cell_id, timestamp),
                valid: true,
            })
        }
    }

    pub fn verify(attestation: &Attestation) -> bool {
        attestation.valid && attestation.signature.starts_with("sig_")
    }
}
