use morphogenetic_security::cellular::SecurityCell;
use morphogenetic_security::config;
use morphogenetic_security::signaling::Signal;
use morphogenetic_security::stimulus::StimulusSchedule;
use morphogenetic_security::telemetry::{InMemorySink, TelemetryPipeline, TelemetryEvent, TelemetrySink};
use morphogenetic_security::{MorphogeneticApp, ScenarioConfig};
use std::cmp::max;
use std::env;
use std::path::PathBuf;
use std::process;

fn main() {
    let runtime = resolve_runtime();
    let config = runtime.config;

    let cell_count = max(1, config.initial_cell_count);
    let mut cells = Vec::with_capacity(cell_count);
    for idx in 0..cell_count {
        let mut cell = SecurityCell::new(format!("seed-{idx}"));
        cell.genome.reproduction_threshold = config.threat_profile.spike_threshold;
        if config.cell_reproduction_rate > 0.0 {
            cell.genome.reproduction_energy_cost /= config.cell_reproduction_rate;
        }
        cells.push(cell);
    }

    let mut telemetry_pipeline = runtime
        .telemetry_path
        .as_ref()
        .map(|path| TelemetryPipeline::with_file(path))
        .transpose()
        .unwrap_or_else(|err| {
            eprintln!("Failed to initialize telemetry sink: {err}");
            process::exit(1);
        })
        .unwrap_or_else(|| TelemetryPipeline::new(InMemorySink::default(), None));

    telemetry_pipeline.record(
        std::time::SystemTime::now(),
        TelemetryEvent::Scenario {
            name: config.scenario_name.clone(),
        },
    );

    let mut app = MorphogeneticApp::new(cells, telemetry_pipeline, config.topology.clone());

    let mut stimulus_schedule = runtime
        .stimulus_path
        .as_ref()
        .map(|path| StimulusSchedule::load(path))
        .transpose()
        .unwrap_or_else(|err| {
            eprintln!("Failed to load stimulus schedule: {err}");
            process::exit(1);
        });

    let steps = max(1, config.simulation_steps);
    for step in 0..steps {
        let threat = config.threat_level_for_step(step);
        if threat >= config.threat_profile.spike_threshold {
            app.inject_signal(Signal {
                topic: "activator".to_string(),
                value: threat,
                source: None,
                target: None,
                attestation: None,
            });
        }

        if let Some(schedule) = stimulus_schedule.as_mut() {
            for command in schedule.take_for_step(step) {
                app.inject_signal(Signal {
                    topic: command.topic.clone(),
                    value: command.value,
                    source: None,
                    target: command.target.clone(),
                    attestation: None,
                });
            }
        }

        app.step(step, threat);
    }

    let events = app.telemetry().events();
    println!(
        "Scenario `{}` executed {} step(s); recorded {} telemetry event(s).",
        config.scenario_name,
        steps,
        events.len()
    );
}

struct RuntimeContext {
    config: ScenarioConfig,
    telemetry_path: Option<PathBuf>,
    stimulus_path: Option<PathBuf>,
}

fn resolve_runtime() -> RuntimeContext {
    match parse_cli() {
        Ok(context) => context,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    }
}

fn parse_cli() -> Result<RuntimeContext, String> {
    let mut args = env::args().skip(1);
    let mut config_path: Option<PathBuf> = None;
    let mut telemetry_path: Option<PathBuf> = None;
    let mut stimulus_path: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --config".to_string())?;
                config_path = Some(PathBuf::from(value));
            }
            "--telemetry" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --telemetry".to_string())?;
                telemetry_path = Some(PathBuf::from(value));
            }
            "--stimulus" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --stimulus".to_string())?;
                stimulus_path = Some(PathBuf::from(value));
            }
            _ if arg.starts_with("--") => {
                return Err(format!("Unknown argument `{arg}`"));
            }
            positional => {
                if config_path.is_none() {
                    config_path = Some(PathBuf::from(positional));
                } else {
                    return Err(format!("Unexpected positional argument `{positional}`"));
                }
            }
        }
    }

    let config = if let Some(path) = config_path {
        config::load_from_path(&path).map_err(|err| err.to_string())?
    } else {
        ScenarioConfig::default()
    };

    Ok(RuntimeContext {
        config,
        telemetry_path,
        stimulus_path,
    })
}