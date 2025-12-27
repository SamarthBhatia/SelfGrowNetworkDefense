//! Swarm immune response and distributed anomaly detection logic.

use serde::{Deserialize, Serialize};

/// A recorded threat event in a cell's local memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatEvent {
    pub step: u32,
    pub topic: String,
    pub magnitude: f32,
    pub confidence: f32,
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
    // Secret key for signing (simulated)
    secret: String,
}

impl TPM {
    pub fn new(cell_id: String) -> Self {
        // Deterministic secret generation for simulation (acts as a mock PKI)
        let secret = format!("{:x}", md5::compute(format!("root_secret_{}", cell_id)));
        Self {
            cell_id,
            compromised: false,
            secret,
        }
    }

    pub fn attest(&self, step: u64, payload: &str) -> Option<Attestation> {
        if self.compromised {
            None
        } else {
            let payload_hash = format!("{:x}", md5::compute(payload));
            // Signature depends on secret, step, and payload hash
            let signature = format!("{:x}", md5::compute(format!("{}_{}_{}", self.secret, step, payload_hash)));
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
        if attestation.step > current_step || (current_step - attestation.step) > 1 {
            return false; 
        }
        let expected_hash = format!("{:x}", md5::compute(payload));
        if attestation.payload_hash != expected_hash {
            return false; // Integrity check
        }
        
        // Re-derive secret (Simulating PKI lookup)
        let secret = format!("{:x}", md5::compute(format!("root_secret_{}", attestation.cell_id)));
        let expected_sig = format!("{:x}", md5::compute(format!("{}_{}_{}", secret, attestation.step, expected_hash)));
        
        attestation.signature == expected_sig
    }
}
