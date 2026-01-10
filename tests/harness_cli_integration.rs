use std::fs;
use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn test_adversarial_cycle_cli() {
    // adversarial_cycle takes:
    // <candidate_json> <metrics_csv> <output_json> <harness_state_json>
    // It evaluates the candidate based on metrics and updates the state.

    let candidate_json = NamedTempFile::new().unwrap();
    let metrics_csv = NamedTempFile::new().unwrap();
    let output_json = NamedTempFile::new().unwrap();
    let state_json = NamedTempFile::new().unwrap();

    // Setup candidate
    let candidate_data = r#"{ 
        "id": "test-cand",
        "scenario_ref": "docs/examples/baseline-growth.yaml",
        "generation": 0,
        "mutation": null
    }"#;
    fs::write(candidate_json.path(), candidate_data).expect("write candidate");

    // Setup metrics (dummy data)
    let mut wtr = csv::Writer::from_path(metrics_csv.path()).expect("create writer");
    wtr.write_record(&[
        "step",
        "threat_score",
        "cell_count",
        "replications",
        "deaths",
        "signals_total",
        "lineage_shifts_total",
        "stimulus_total",
        "top_signal_topic",
        "top_signal_count",
        "top_lineage",
        "top_lineage_count",
        "signals_by_topic",
        "lineage_shifts_by_lineage",
        "stimulus_by_topic",
        "population_stats",
        "topology_stats",
    ])
    .expect("write header");

    wtr.write_record(&[
        "0",
        "0.5",
        "10",
        "1",
        "0",
        "5",
        "0",
        "0.0",
        "activator",
        "5",
        "stem",
        "1",
        r#"{"activator":5}"#, // Raw string for JSON
        "{}",
        "{}",
        "",
        "",
    ])
    .expect("write row");
    wtr.flush().expect("flush");

    // Setup initial state
    let state_data = r#"{ 
        "config": {
            "batch_size": 1,
            "max_generations": 5,
            "retain_elite": true,
            "crossover_rate": 0.0,
            "selection_strategy": {"Tournament": {"size": 3}},
            "crossover_strategy": "Uniform",
            "mutation_strategy": "Random"
        },
        "backlog": [],
        "archive": []
    }"#;
    fs::write(state_json.path(), state_data).expect("write state");

    let bin_path = env!("CARGO_BIN_EXE_adversarial_cycle");
    let status = Command::new(bin_path)
        .arg("--candidate-id")
        .arg("test-cand")
        .arg("--scenario")
        .arg("docs/examples/baseline-growth.yaml") // Scenario ref
        .arg("--metrics")
        .arg(metrics_csv.path())
        .arg("--emit-json")
        .arg(output_json.path())
        .arg("--state")
        .arg(state_json.path())
        .status()
        .expect("failed to run binary");

    assert!(status.success());

    // Verify output
    let output_content = fs::read_to_string(output_json.path()).expect("read output");
    assert!(output_content.contains("fitness_score"));
    assert!(output_content.contains("breach_observed"));

    // Verify state updated (archived the outcome)
    let state_content = fs::read_to_string(state_json.path()).expect("read state");
    assert!(state_content.contains("test-cand"));
    // Since we provided 1 row of metrics, fitnes score should be calculated.
}
