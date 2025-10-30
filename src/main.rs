use morphogenetic_security::cellular::SecurityCell;
use morphogenetic_security::config;
use morphogenetic_security::telemetry::InMemorySink;
use morphogenetic_security::{ConfigError, MorphogeneticApp, ScenarioConfig};
use std::cmp::max;
use std::env;
use std::process;

fn main() {
    let config = resolve_config();
    let cell_count = max(1, config.initial_cell_count);
    let mut cells = Vec::with_capacity(cell_count);
    for idx in 0..cell_count {
        let mut cell = SecurityCell::new(format!("seed-{idx}"));
        cell.reproduction_threshold = config.threat_profile.spike_threshold;
        cells.push(cell);
    }

    let telemetry = InMemorySink::default();
    let mut app = MorphogeneticApp::new(cells, telemetry);

    let steps = max(1, config.simulation_steps);
    for _ in 0..steps {
        app.step(config.threat_profile.background_threat);
    }

    let events = app.telemetry().events();
    println!(
        "Scenario `{}` executed {} step(s); recorded {} telemetry event(s).",
        config.scenario_name,
        steps,
        events.len()
    );
}

fn resolve_config() -> ScenarioConfig {
    match load_config_from_cli() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Failed to load scenario configuration: {err}");
            process::exit(1);
        }
    }
}

fn load_config_from_cli() -> Result<ScenarioConfig, ConfigError> {
    let mut args = env::args().skip(1);
    if let Some(path) = args.next() {
        config::load_from_path(path)
    } else {
        Ok(ScenarioConfig::default())
    }
}
