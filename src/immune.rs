//! Swarm immune response and distributed anomaly detection logic.

use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};
use std::collections::HashMap;
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey, Signature};
use rand::rngs::OsRng;

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
    pub signature: Vec<u8>,
    pub valid: bool,
}

/// Simulated Trusted Platform Module (TPM).
/// Now uses true asymmetric cryptography.
#[derive(Serialize, Deserialize)]
pub struct TPM {
    pub cell_id: String,
    pub compromised: bool,
    // Private signing key (serialized bytes for storage/cloning)
    secret_bytes: Vec<u8>,
}

impl std::fmt::Debug for TPM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TPM")
            .field("cell_id", &self.cell_id)
            .field("compromised", &self.compromised)
            .field("secret_bytes", &"<REDACTED>")
            .finish()
    }
}

// Ensure TPM cannot be cloned to prevent key exfiltration/duplication
// (SecurityCell must also lose Clone derive)

// Registry stores only PUBLIC verification keys.
static PKI_REGISTRY: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();

fn get_pki() -> &'static Mutex<HashMap<String, Vec<u8>>> {
    PKI_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

impl TPM {
            pub fn new(cell_id: String) -> Self {
                let mut csprng = OsRng;
                let mut bytes = [0u8; 32];
                use rand::RngCore;
                csprng.fill_bytes(&mut bytes);
                let signing_key = SigningKey::from_bytes(&bytes);
                let verifying_key: VerifyingKey = signing_key.verifying_key();
                
                // Publish public key to PKI
                get_pki().lock().unwrap().insert(cell_id.clone(), verifying_key.to_bytes().to_vec());
                
                Self {
                    cell_id,
                    compromised: false,
                    secret_bytes: signing_key.to_bytes().to_vec(),
                }
            }
                pub fn attest(&self, step: u64, payload: &str) -> Option<Attestation> {
            if self.compromised {
                None
            } else {
                let payload_hash = format!("{:x}", md5::compute(payload));
                let message = format!("{}:{}", step, payload_hash);
                
                // Reconstruct signing key from stored bytes
                let signing_key = SigningKey::from_bytes(self.secret_bytes.as_slice().try_into().unwrap());
                let signature: Signature = signing_key.sign(message.as_bytes());
    
                Some(Attestation {
                    cell_id: self.cell_id.clone(),
                    step,
                    payload_hash,
                    signature: signature.to_bytes().to_vec(),
                    valid: true,
                })
            }
        }
    
        pub fn verify(attestation: &Attestation, current_step: u64, payload: &str) -> bool {
            if !attestation.valid {
                return false;
            }
            // Freshness check (allow 1 step delay)
            if attestation.step > current_step || (current_step - attestation.step) > 1 {
                return false; 
            }
            // Integrity check
            let expected_hash = format!("{:x}", md5::compute(payload));
            if attestation.payload_hash != expected_hash {
                return false; 
            }
            
            // Retrieve PUBLIC key from registry
            let pki = get_pki().lock().unwrap();
            if let Some(pub_bytes) = pki.get(&attestation.cell_id) {
                if let Ok(verifying_key) = VerifyingKey::from_bytes(pub_bytes.as_slice().try_into().unwrap()) {
                    let message = format!("{}:{}", attestation.step, expected_hash);
                    // Correctly handle Result from Signature::from_bytes? No, Signature::from_bytes returns Signature directly in recent versions?
                    // Wait, dalek 2.x Signature::from_bytes returns Signature.
                    // The error said: expected `Signature`, found `Result<_, _>`.
                    // Checking ed25519-dalek 2.x docs: from_bytes returns Signature. 
                    // Ah, maybe TryFrom was used implicitly? 
                    // Let's use `Signature::from_bytes` and wrap in Ok/Result handling properly.
                    // Or maybe the error meant I treated a Result as a Signature?
                    // Error: "expected `Signature`, found `Result<_, _>`".
                    // Ah, `Signature::from_bytes` DOES return a Signature, but maybe the surrounding code expected Result?
                    // Wait, error trace: `if let Ok(signature) = Signature::from_bytes(...)`
                    // If `from_bytes` returns `Signature`, then `if let Ok` fails because it's not a Result.
                    
                    let signature = Signature::from_bytes(attestation.signature.as_slice().try_into().unwrap());
                    return verifying_key.verify(message.as_bytes(), &signature).is_ok();
                }
            }
            false
        }
    }
    