use morphogenetic_security::cellular::{CellAction, CellEnvironment, SecurityCell};
use morphogenetic_security::signaling::{Signal, SignalBus};

#[test]
fn test_security_cell_tick_does_not_short_circuit_on_consensus_spam() {
    let mut cell = SecurityCell::new("test_cell");
    let signal = Signal {
        topic: "consensus:activator".to_string(),
        value: 1.0,
        source: Some("malicious_peer".to_string()),
        target: None,
        attestation: None, // Unauthenticated!
    };

    let env = CellEnvironment {
        step: 10,
        local_threat_score: 0.1,
        neighbor_signals: vec![signal],
        detected_neighbors: vec!["malicious_peer".to_string()],
    };

    let action = cell.tick(&env);

    // We expect Idle (trust was penalized but no early return/short-circuit)
    assert!(
        matches!(action, CellAction::Idle),
        "Expected Idle, got {:?}",
        action
    );
}

#[test]
fn test_signal_bus_purge_removes_all_signals_from_source() {
    let mut bus = SignalBus::default();

    // Targeted signal to victim
    bus.publish(Signal {
        topic: "ping".to_string(),
        value: 1.0,
        source: Some("spammer".to_string()),
        target: Some("victim".to_string()),
        attestation: None,
    });

    // Broadcast signal
    bus.publish(Signal {
        topic: "broadcast_spam".to_string(),
        value: 1.0,
        source: Some("spammer".to_string()),
        target: None,
        attestation: None,
    });

    // Targeted signal to someone else
    bus.publish(Signal {
        topic: "ping".to_string(),
        value: 1.0,
        source: Some("spammer".to_string()),
        target: Some("third_party".to_string()),
        attestation: None,
    });

    // Another signal (safe)
    bus.publish(Signal {
        topic: "safe".to_string(),
        value: 1.0,
        source: Some("good_guy".to_string()),
        target: None,
        attestation: None,
    });

    // Purge ALL signals from spammer
    bus.purge_from("spammer", "victim");

    let remaining = bus.drain();

    let from_spammer = remaining
        .iter()
        .filter(|s| s.source.as_deref() == Some("spammer"))
        .count();
    let has_safe = remaining.iter().any(|s| s.topic == "safe");

    assert_eq!(
        from_spammer, 0,
        "ALL signals from spammer should be removed"
    );
    assert!(has_safe, "Safe signal should be kept");
}
