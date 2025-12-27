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
    pub step: u64,
    pub payload_hash: String,
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

    pub fn attest(&self, step: u64, payload: &str) -> Option<Attestation> {
        if self.compromised {
            None
        } else {
            // Simulated signature: sig_{cell_id}_{step}_{payload_hash}
            // In real TPM, this would be RSA/ECC sign(hash(step + payload))
            let payload_hash = format!("{:x}", md5::compute(payload));
            let signature = format!("sig_{}_{}_{}", self.cell_id, step, payload_hash);
            Some(Attestation {
                cell_id: self.cell_id.clone(),
                step,
                payload_hash,
                signature,
                valid: true,
            })
        }
    }

    pub fn verify(attestation: &Attestation, current_step: u64, payload: &str) -> bool {
        if !attestation.valid {
            return false;
        }
        // Relaxed freshness check: allow signals from current step or immediate past step
        // (Signals are often processed one step after emission due to bus drain timing)
        if attestation.step > current_step || (current_step - attestation.step) > 1 {
            return false; 
        }
        let expected_hash = format!("{:x}", md5::compute(payload));
        if attestation.payload_hash != expected_hash {
            return false; // Integrity check
        }
        let expected_sig = format!("sig_{}_{}_{}", attestation.cell_id, attestation.step, attestation.payload_hash);
        attestation.signature == expected_sig
    }
}
