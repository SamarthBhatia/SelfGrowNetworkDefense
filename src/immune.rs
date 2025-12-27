//! Swarm immune response and distributed anomaly detection logic.

use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};
use rand::Rng;
use std::collections::HashMap;

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
    // Secret key for signing (simulated, private)
    secret: String,
}

/// Mock Public Key Infrastructure to verify signatures without exposing secrets.
pub struct PKI;

static MOCK_REGISTRY: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn get_registry() -> &'static Mutex<HashMap<String, String>> {
    MOCK_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

impl TPM {
    pub fn new(cell_id: String) -> Self {
        // Generate a random non-derivable secret
        let mut rng = rand::thread_rng();
        let salt: u64 = rng.r#gen();
        let secret = format!("{:x}", md5::compute(format!("{}{}", cell_id, salt)));
        
        // Register it in the "hardware" registry
        get_registry().lock().unwrap().insert(cell_id.clone(), secret.clone());
        
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
            // Sign with private secret
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
        if attestation.step > current_step || (current_step - attestation.step) > 1 {
            return false; 
        }
        let expected_hash = format!("{:x}", md5::compute(payload));
        if attestation.payload_hash != expected_hash {
            return false; 
        }
        
        // Retrieve secret from secure registry (simulating PKI verification)
        let registry = get_registry().lock().unwrap();
        if let Some(secret) = registry.get(&attestation.cell_id) {
            let expected_sig = format!("{:x}", md5::compute(format!("{}_{}_{}", secret, attestation.step, expected_hash)));
            attestation.signature == expected_sig
        } else {
            false // Unknown device
        }
    }
}