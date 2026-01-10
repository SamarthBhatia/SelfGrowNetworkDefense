use morphogenetic_security::MorphogeneticApp;
use morphogenetic_security::cellular::{CellAction, CellEnvironment, SecurityCell};
use morphogenetic_security::config::{TopologyConfig, TopologyStrategy};
use morphogenetic_security::immune::TPM;
use morphogenetic_security::signaling::{Signal, SignalBus};
use morphogenetic_security::telemetry::{InMemorySink, TelemetryEvent};

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

#[test]
fn test_authenticated_consensus_flood_triggers_isolation() {
    // Unique IDs to avoid collision with other tests using global PKI
    let id_a = "Flood_A";
    let id_b = "Flood_B";
    let id_c = "Flood_C";
    let id_d = "Flood_D";

    let cell_a = SecurityCell::new(id_a);
    let cell_b = SecurityCell::new(id_b);
    let cell_c = SecurityCell::new(id_c);
    let cell_d = SecurityCell::new(id_d);

    let cells = vec![cell_a, cell_b, cell_c, cell_d];
    let topology = TopologyConfig {
        strategy: TopologyStrategy::Graph,
        explicit_links: Some(vec![
            vec![id_a.into(), id_b.into()],
            vec![id_a.into(), id_c.into()],
            vec![id_a.into(), id_d.into()], // D is the target
        ]),
    };

    let mut app = MorphogeneticApp::new(cells, InMemorySink::default(), topology);

    // Step 0: B and C broadcast authenticated consensus votes against D
    // We must ensure the TPM keys match what's in the registry.
    // Since we just called SecurityCell::new, the registry has the keys from those cells.
    // We need to sign with THOSE keys.
    // But we don't have access to the private keys inside SecurityCell (private field).
    // So we must OVERWRITE the registry with our own TPMs that we control,
    // effectively simulating that B and C are the ones we hold.

    let tpm_b = TPM::new(id_b.into()); // Overwrites Flood_B in registry
    let payload_b = format!("consensus:activator:1.0:{}", id_d);
    let att_b = tpm_b.attest(0, &payload_b).unwrap();

    let tpm_c = TPM::new(id_c.into()); // Overwrites Flood_C in registry
    let payload_c = format!("consensus:activator:1.0:{}", id_d);
    let att_c = tpm_c.attest(0, &payload_c).unwrap();

    app.inject_signal(Signal {
        topic: "consensus:activator".into(),
        value: 1.0,
        source: Some(id_b.into()),
        target: Some(id_d.into()),
        attestation: Some(att_b),
    });

    app.inject_signal(Signal {
        topic: "consensus:activator".into(),
        value: 1.0,
        source: Some(id_c.into()),
        target: Some(id_d.into()),
        attestation: Some(att_c),
    });

    app.step(0, 0.0);

    // Check telemetry for A disconnecting from D
    let events = app.telemetry().events();
    let disconnected = events.iter().any(|e| {
        matches!(&e.event, TelemetryEvent::LinkRemoved { source, target }
        if (source == id_a && target == id_d) || (source == id_d && target == id_a))
    });

    assert!(
        disconnected,
        "A should have disconnected from D due to consensus votes"
    );
}

#[test]
fn test_quarantined_node_ignored_next_step() {
    let id_a = "Quarantine_A";
    let id_b = "Quarantine_B";

    let mut cell_a = SecurityCell::new(id_a);
    cell_a.genome.min_trust_threshold = 0.2;
    cell_a.genome.trust_penalty = 0.2;

    let cell_b = SecurityCell::new(id_b);

    let cells = vec![cell_a, cell_b];
    let topology = TopologyConfig {
        strategy: TopologyStrategy::Graph,
        explicit_links: Some(vec![vec![id_a.into(), id_b.into()]]),
    };

    let mut app = MorphogeneticApp::new(cells, InMemorySink::default(), topology);

    // Step 0: B sends 2 unauthenticated signals to A
    app.inject_signal(Signal {
        topic: "consensus:activator".into(),
        value: 1.0,
        source: Some(id_b.into()),
        target: None,
        attestation: None,
    });
    app.inject_signal(Signal {
        topic: "consensus:activator".into(),
        value: 1.0,
        source: Some(id_b.into()),
        target: None,
        attestation: None,
    });

    app.step(0, 0.0);

    // Verify disconnect happened
    {
        let events = app.telemetry().events();
        let disconnected = events.iter().any(|e| {
            matches!(&e.event, TelemetryEvent::LinkRemoved { source, target }
            if (source == id_a && target == id_b) || (source == id_b && target == id_a))
        });
        assert!(disconnected, "A should disconnect from B in step 0");
    }

    // Step 1: B tries to send a valid signal to A
    app.inject_signal(Signal {
        topic: "activator".into(),
        value: 1.0,
        source: Some(id_b.into()),
        target: Some(id_a.into()),
        attestation: None,
    });

    app.step(1, 0.0);

    let events = app.telemetry().events();
    let a_emitted_step_1 = events.iter().any(|snapshot| {
        if let TelemetryEvent::SignalEmitted { cell_id, .. } = &snapshot.event {
            cell_id == id_a && snapshot.timestamp > std::time::SystemTime::UNIX_EPOCH
        } else {
            false
        }
    });

    assert!(
        !a_emitted_step_1,
        "A should not emit signal if it ignored B"
    );
}
