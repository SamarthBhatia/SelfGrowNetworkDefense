use morphogenetic_security::cellular::{CellEnvironment, SecurityCell};
use morphogenetic_security::immune::TPM;
use morphogenetic_security::signaling::Signal;

#[test]
fn test_chaos_compromised_tpm_fails_attestation() {
    let mut cell = SecurityCell::new("compromised_node");

    // 1. Valid state
    let att = cell.tpm.attest(0, "test").expect("should attest");
    assert!(TPM::verify(&att, 0, "test"));

    // 2. Compromise
    cell.tpm.compromised = true;

    // 3. Attempt attestation
    let result = cell.tpm.attest(1, "test");
    assert!(result.is_none(), "Compromised TPM should not attest");
}

#[test]
fn test_chaos_malformed_signature_rejected() {
    let mut cell = SecurityCell::new("victim");
    let attacker_id = "attacker";
    // Register attacker in PKI implicitly by creating it (even if we don't use the object)
    let _attacker = SecurityCell::new(attacker_id);

    // Create a VALID attestation for a DIFFERENT payload
    let tpm = TPM::new(attacker_id.to_string()); // Re-creates key, overwrites PKI
    let valid_att = tpm.attest(0, "valid_payload").unwrap();

    // Inject signal with VALID attestation but DIFFERENT content
    // The cell receives "malicious_payload", checks signature against it.
    // Signature corresponds to "valid_payload". Verify should fail.

    let signal = Signal {
        topic: "consensus:activator".to_string(),
        value: 1.0,
        source: Some(attacker_id.to_string()),
        target: Some("victim".to_string()),
        attestation: Some(valid_att), // Mismatch!
    };

    let env = CellEnvironment {
        step: 0,
        local_threat_score: 0.0,
        neighbor_signals: vec![signal],
        detected_neighbors: vec![attacker_id.to_string()],
    };

    let _action = cell.tick(&env);

    // If verified, it would count vote. If vote > 1.5, disconnect.
    // Here vote is 1.0. It doesn't disconnect anyway unless multiple.
    // But trust should be penalized because verification failed.

    let trust = cell
        .state
        .neighbor_trust
        .get(attacker_id)
        .cloned()
        .unwrap_or(0.5);
    // Default 0.5. Penalty 0.2. Should be 0.3.
    // If verification PASSED, reward 0.05 -> 0.55.

    assert!(
        trust < 0.5,
        "Trust should be penalized for malformed signature"
    );
}
