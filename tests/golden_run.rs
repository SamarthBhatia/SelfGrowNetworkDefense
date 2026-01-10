use morphogenetic_security::MorphogeneticApp;
use morphogenetic_security::cellular::SecurityCell;
use morphogenetic_security::config;
use morphogenetic_security::telemetry::{InMemorySink, TelemetryEvent};

#[test]
fn test_golden_run_miniature_scenario() {
    let yaml = r#"
scenario_name: golden-run-test
initial_cell_count: 2
simulation_steps: 3
threat_profile:
  background_threat: 0.2
spikes:
  - step: 1
    intensity: 0.5
    duration: 1
topology:
  strategy: Global
"#;

    let config = config::load_from_reader(yaml.as_bytes()).expect("config parse");

    let mut cells = Vec::new();
    for i in 0..config.initial_cell_count {
        cells.push(SecurityCell::new(format!("golden-{i}")));
    }

    let telemetry = InMemorySink::default();
    let mut app = MorphogeneticApp::new(cells, telemetry, config.topology.clone());

    // Step 0: Threat 0.2
    app.step(0, config.threat_level_for_step(0));

    // Step 1: Threat 0.2 + 0.5 = 0.7
    app.step(1, config.threat_level_for_step(1));

    // Step 2: Threat 0.2
    app.step(2, config.threat_level_for_step(2));

    let events = app.telemetry().events();

    // Verify StepSummary events
    let summaries: Vec<&TelemetryEvent> = events
        .iter()
        .filter_map(|s| {
            if let TelemetryEvent::StepSummary { .. } = &s.event {
                Some(&s.event)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(summaries.len(), 3);

    if let TelemetryEvent::StepSummary { threat_score, .. } = summaries[1] {
        assert!((threat_score - 0.7).abs() < f32::EPSILON);
    } else {
        panic!("Missing summary for step 1");
    }

    // Verify at least some events happened (e.g. signal emissions due to spike)
    // 0.7 threat is > default emission threshold (0.6)
    let emissions = events
        .iter()
        .any(|s| matches!(s.event, TelemetryEvent::SignalEmitted { .. }));
    assert!(emissions, "Expected signal emissions during spike");

    // Snapshot check (counts)
    // We expect 3 StepSummaries + some Signals.
    // Total count should be stable-ish.
    assert!(events.len() >= 3);
}
