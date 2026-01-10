//! Swarm immune response and distributed anomaly detection logic.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

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
pub struct TPM {
    pub cell_id: String,
    pub compromised: bool,
    // Private signing key (serialized bytes for internal use only)
    secret_bytes: Vec<u8>,
}

impl Serialize for TPM {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("TPM", 3)?;
        state.serialize_field("cell_id", &self.cell_id)?;
        state.serialize_field("compromised", &self.compromised)?;

        // Obfuscate secret using cell_id as salt (simulation security)
        let salt = md5::compute(&self.cell_id).0;
        let encrypted: Vec<u8> = self
            .secret_bytes
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ salt[i % 16])
            .collect();

        state.serialize_field("secret_bytes", &encrypted)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for TPM {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct TPMDef {
            cell_id: String,
            compromised: bool,
            secret_bytes: Vec<u8>,
        }

        let def = TPMDef::deserialize(deserializer)?;

        // De-obfuscate
        let salt = md5::compute(&def.cell_id).0;
        let secret_bytes: Vec<u8> = def
            .secret_bytes
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ salt[i % 16])
            .collect();

        // Re-register public key in PKI
        if !def.compromised
            && !secret_bytes.is_empty()
            && let Ok(bytes) = secret_bytes.as_slice().try_into()
        {
            let signing_key = SigningKey::from_bytes(bytes);
            let verifying_key = signing_key.verifying_key();
            get_pki()
                .lock()
                .unwrap()
                .insert(def.cell_id.clone(), verifying_key.to_bytes().to_vec());
        }

        Ok(TPM {
            cell_id: def.cell_id,
            compromised: def.compromised,
            secret_bytes,
        })
    }
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
        get_pki()
            .lock()
            .unwrap()
            .insert(cell_id.clone(), verifying_key.to_bytes().to_vec());

        Self {
            cell_id,
            compromised: false,
            secret_bytes: signing_key.to_bytes().to_vec(),
        }
    }

    pub fn attest(&self, step: u64, payload: &str) -> Option<Attestation> {
        if self.compromised || self.secret_bytes.is_empty() {
            None
        } else {
            let mut hasher = Sha256::new();
            hasher.update(payload.as_bytes());
            let payload_hash = format!("{:x}", hasher.finalize());
            let message = format!("{}:{}", step, payload_hash);

            // Reconstruct signing key from stored bytes safely
            let signing_key_bytes: [u8; 32] = match self.secret_bytes.as_slice().try_into() {
                Ok(bytes) => bytes,
                Err(_) => return None,
            };
            let signing_key = SigningKey::from_bytes(&signing_key_bytes);
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
        let mut hasher = Sha256::new();
        hasher.update(payload.as_bytes());
        let expected_hash = format!("{:x}", hasher.finalize());
        if attestation.payload_hash != expected_hash {
            return false;
        }

        // Retrieve PUBLIC key from registry
        let pki = get_pki().lock().unwrap();
        if let Some(pub_bytes) = pki.get(&attestation.cell_id) {
            let verifying_key_bytes: [u8; 32] = match pub_bytes.as_slice().try_into() {
                Ok(bytes) => bytes,
                Err(_) => return false,
            };

            if let Ok(verifying_key) = VerifyingKey::from_bytes(&verifying_key_bytes) {
                let message = format!("{}:{}", attestation.step, expected_hash);
                let signature_bytes: [u8; 64] = match attestation.signature.as_slice().try_into() {
                    Ok(bytes) => bytes,
                    Err(_) => return false,
                };

                let signature = Signature::from_bytes(&signature_bytes);
                return verifying_key.verify(message.as_bytes(), &signature).is_ok();
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attestation_verification() {
        let tpm = TPM::new("valid_node".to_string());
        let step = 10;
        let payload = "consensus:activator:1.0:target";

        let attestation = tpm.attest(step, payload).expect("Attestation failed");

        // Valid verification
        assert!(TPM::verify(&attestation, step, payload));
        // Valid with 1 step delay
        assert!(TPM::verify(&attestation, step + 1, payload));
    }

    #[test]
    fn test_compromised_tpm() {
        let mut tpm = TPM::new("bad_node".to_string());
        tpm.compromised = true;

        assert!(tpm.attest(10, "payload").is_none());
    }

    #[test]
    fn test_replay_attack() {
        let tpm = TPM::new("replay_node".to_string());
        let step = 10;
        let payload = "consensus:activator:1.0:target";

        let attestation = tpm.attest(step, payload).unwrap();

        // Too late (2 steps later)
        assert!(!TPM::verify(&attestation, step + 2, payload));
        // Future step (impossible)
        assert!(!TPM::verify(&attestation, step - 1, payload));
    }

    #[test]
    fn test_tampered_payload() {
        let tpm = TPM::new("tamper_node".to_string());
        let step = 10;
        let payload = "original_payload";

        let attestation = tpm.attest(step, payload).unwrap();

        assert!(!TPM::verify(&attestation, step, "modified_payload"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let tpm = TPM::new("persist_node".to_string());
        let json = serde_json::to_string(&tpm).expect("Serialize");

        // Deserialize
        let loaded_tpm: TPM = serde_json::from_str(&json).expect("Deserialize");

        assert_eq!(tpm.cell_id, loaded_tpm.cell_id);
        assert_eq!(tpm.secret_bytes, loaded_tpm.secret_bytes);

        // Verify loaded TPM can attest and verify
        let att = loaded_tpm.attest(5, "data").unwrap();
        assert!(TPM::verify(&att, 5, "data"));

        // Verify PKI has the key (re-registered on deserialize)
        let pki = get_pki().lock().unwrap();
        assert!(pki.contains_key("persist_node"));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_fuzz_verification(
            step in 0u64..1000u64,
            current_step in 0u64..1000u64,
            payload in "\\PC*",
            bad_payload in "\\PC*",
            byte_mutation_idx in 0usize..64usize,
            byte_mutation_val in 0u8..255u8
        ) {
            let tpm = TPM::new("fuzz_node".to_string());
            let attestation = tpm.attest(step, &payload);

            if let Some(mut att) = attestation {
                // 1. Happy path: if steps align, should verify
                if current_step >= step && current_step <= step + 1 {
                    prop_assert!(TPM::verify(&att, current_step, &payload));
                } else {
                    prop_assert!(!TPM::verify(&att, current_step, &payload));
                }

                // 2. Tampered payload
                if payload != bad_payload {
                    prop_assert!(!TPM::verify(&att, step, &bad_payload));
                }

                // 3. Tampered signature
                if byte_mutation_idx < att.signature.len() {
                    let original_byte = att.signature[byte_mutation_idx];
                    if original_byte != byte_mutation_val {
                        att.signature[byte_mutation_idx] = byte_mutation_val;
                        prop_assert!(!TPM::verify(&att, step, &payload));
                    }
                }
            }
        }
    }
}
