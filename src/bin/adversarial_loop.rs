use morphogenetic_security::MorphogeneticApp;
use morphogenetic_security::adversarial::{
    AdversarialHarness, AttackCandidate, EvolutionConfig, ExecutionReport, HarnessError,
    StepMetrics,
};
use morphogenetic_security::cellular::SecurityCell;
use morphogenetic_security::config;
use morphogenetic_security::signaling::Signal;
use morphogenetic_security::stimulus::StimulusSchedule;
use morphogenetic_security::telemetry::TelemetryPipeline;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let mut harness = initialise_harness(&args)?;

    if !args.seeds.is_empty() {
        for seed in &args.seeds {
            harness.enqueue(AttackCandidate {
                id: seed.id.clone(),
                scenario_ref: seed.scenario.to_string_lossy().to_string(),
                stimulus_ref: args
                    .stimulus_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
                generation: 0,
                parent_id: None,
                mutation_note: seed.note.clone(),
            });
        }
        println!(
            "[info] Enqueued {} seed candidate(s); backlog now {}",
            args.seeds.len(),
            harness.backlog_len()
        );
    }

    if harness.backlog_len() == 0 {
        println!("[warn] Harness backlog is empty; nothing to execute.");
        persist_harness(&harness, &args.state_path)?;
        return Ok(());
    }

    let artifact_root = args.artifact_dir.clone();
    let stimulus_path = args.stimulus_path.clone();

    let evaluations = harness
        .run_generations(args.generations, move |candidate| {
            simulate_candidate(candidate, &artifact_root, stimulus_path.as_deref())
        })
        .map_err(|err| format!("Harness execution failed: {err}"))?;

    if evaluations.is_empty() {
        println!("[info] No candidates executed (limited by generations or backlog).");
    } else {
        for evaluation in &evaluations {
            let outcome = &evaluation.outcome;
            let analysis = &evaluation.analysis;
            println!(
                "candidate `{}` gen {} => fitness {:.3} | breach={}",
                outcome.candidate.id,
                outcome.candidate.generation,
                analysis.fitness_score,
                analysis.breach_observed
            );
            println!(
                "  steps={} avg_threat={:.2} replication={} signals={} stimulus={:.2}",
                analysis.statistics.step_count,
                analysis.statistics.avg_threat,
                analysis.statistics.total_replications,
                analysis.statistics.total_signals,
                analysis.statistics.total_stimulus
            );
            if let Some(mutation) = &analysis.recommended_mutation {
                println!("  recommended mutation: {mutation}");
            } else if harness.config().retain_elite {
                println!("  no mutation suggested; candidate retained for future iteration");
            } else {
                println!("  no mutation suggested");
            }
            if let Some(follow_up) = &evaluation.follow_up {
                println!(
                    "  queued follow-up `{}` (generation {})",
                    follow_up.id, follow_up.generation
                );
            }
            if let Some(metrics_path) = &evaluation.report.metrics_path {
                println!("  metrics captured at {}", metrics_path.display());
            }
            println!(
                "  backlog size after evaluation: {}",
                evaluation.backlog_len_after
            );
        }
    }

    println!(
        "[info] Harness backlog after loop: {} candidate(s)",
        harness.backlog_len()
    );

    persist_harness(&harness, &args.state_path)?;
    println!(
        "[info] Persisted harness state to {}",
        args.state_path.display()
    );

    Ok(())
}

fn initialise_harness(args: &CliArgs) -> Result<AdversarialHarness, String> {
    if args.state_path.exists() {
        AdversarialHarness::load_state(&args.state_path)
            .map_err(|err| {
                format!(
                    "Failed to load harness state `{}`: {err}",
                    args.state_path.display()
                )
            })
            .map(|harness| {
                if args.batch_size.is_some()
                    || args.max_generations.is_some()
                    || args.retain_elite.is_some()
                {
                    println!("[info] Existing harness loaded; configuration overrides ignored.");
                }
                harness
            })
    } else {
        let mut config = EvolutionConfig::default_smoke_test();
        if let Some(batch) = args.batch_size {
            config.batch_size = batch;
        }
        if let Some(max_gen) = args.max_generations {
            config.max_generations = max_gen;
        }
        if let Some(retain) = args.retain_elite {
            config.retain_elite = retain;
        }
        println!(
            "[info] Initialising new harness with batch_size={} max_generations={} retain_elite={}",
            config.batch_size, config.max_generations, config.retain_elite
        );
        Ok(AdversarialHarness::new(config))
    }
}

fn persist_harness(harness: &AdversarialHarness, path: &Path) -> Result<(), String> {
    harness
        .save_state(path)
        .map_err(|err| format!("Failed to persist harness state: {err}"))
}

fn simulate_candidate(
    candidate: &AttackCandidate,
    artifact_root: &Path,
    default_stimulus: Option<&Path>,
) -> Result<ExecutionReport, HarnessError> {
    let run_dir = artifact_root
        .join(format!("gen{:03}", candidate.generation))
        .join(&candidate.id);
    fs::create_dir_all(&run_dir)?;

    let telemetry_path = run_dir.join("telemetry.jsonl");
    let metrics_path = run_dir.join("step_metrics.csv");

    let candidate_stimulus = candidate.stimulus_ref.as_ref().map(PathBuf::from);
    let applied_stimulus =
        candidate_stimulus.or_else(|| default_stimulus.map(|path| path.to_path_buf()));

    let (mut schedule, persisted_stimulus) = if let Some(path) = applied_stimulus {
        if !path.exists() {
            return Err(HarnessError::Custom(format!(
                "Stimulus schedule `{}` not found",
                path.display()
            )));
        }
        let destination = run_dir.join("stimulus.jsonl");
        fs::copy(&path, &destination)?;
        (
            Some(StimulusSchedule::load(&path).map_err(HarnessError::Io)?),
            Some(destination),
        )
    } else {
        (None, None)
    };

    let scenario_path = PathBuf::from(&candidate.scenario_ref);
    let config = config::load_from_path(&scenario_path).map_err(|err| {
        HarnessError::Custom(format!(
            "Failed to load scenario `{}`: {err}",
            candidate.scenario_ref
        ))
    })?;

    let telemetry = TelemetryPipeline::with_file(&telemetry_path).map_err(HarnessError::Io)?;

    let cell_count = std::cmp::max(1, config.initial_cell_count);
    let mut cells = Vec::with_capacity(cell_count);
    for idx in 0..cell_count {
        let mut cell = SecurityCell::new(format!("seed-{idx}"));
        cell.reproduction_threshold = config.threat_profile.spike_threshold;
        cells.push(cell);
    }

    let mut app = MorphogeneticApp::new(cells, telemetry);
    let steps = std::cmp::max(1, config.simulation_steps);
    let mut per_step: Vec<StepMetrics> = Vec::with_capacity(steps as usize);
    let mut stimulus_ledger: HashMap<u32, HashMap<String, f32>> = HashMap::new();

    for step in 0..steps {
        let threat = config.threat_level_for_step(step);
        if threat >= config.threat_profile.spike_threshold {
            app.inject_signal(Signal {
                topic: "activator".to_string(),
                value: threat,
            });
        }

        if let Some(schedule) = schedule.as_mut() {
            let commands = schedule.take_for_step(step);
            if !commands.is_empty() {
                let entry = stimulus_ledger.entry(step).or_default();
                for command in commands {
                    app.inject_signal(Signal {
                        topic: command.topic.clone(),
                        value: command.value,
                    });
                    *entry.entry(command.topic).or_insert(0.0) += command.value;
                }
            }
        }

        let before = app.telemetry().events().len();
        app.step(step, threat);
        let events = app.telemetry().events();
        let new_events = &events[before..];

        let mut replications = 0u32;
        let mut signals_by_topic: HashMap<String, u32> = HashMap::new();
        let mut lineage_by_lineage: HashMap<String, u32> = HashMap::new();
        let mut summary_threat: Option<f32> = None;
        let mut summary_cells: Option<u32> = None;

        for snapshot in new_events {
            match &snapshot.event {
                morphogenetic_security::telemetry::TelemetryEvent::CellReplicated { .. } => {
                    replications += 1;
                }
                morphogenetic_security::telemetry::TelemetryEvent::SignalEmitted {
                    topic, ..
                } => {
                    *signals_by_topic.entry(topic.clone()).or_insert(0) += 1;
                }
                morphogenetic_security::telemetry::TelemetryEvent::LineageShift {
                    lineage, ..
                } => {
                    *lineage_by_lineage.entry(lineage.clone()).or_insert(0) += 1;
                }
                morphogenetic_security::telemetry::TelemetryEvent::StepSummary {
                    threat_score,
                    cell_count,
                    ..
                } => {
                    summary_threat = Some(*threat_score);
                    summary_cells = Some(*cell_count as u32);
                }
            }
        }

        let threat_score = summary_threat.ok_or_else(|| {
            HarnessError::Custom(format!(
                "Step summary missing for candidate `{}` step {}",
                candidate.id, step
            ))
        })?;
        let cell_count = summary_cells.unwrap_or(0);
        let stimulus_by_topic = stimulus_ledger.remove(&step).unwrap_or_default();
        let stimulus_total = stimulus_by_topic.values().copied().sum();
        let signals_total = signals_by_topic.values().copied().sum();
        let lineage_total = lineage_by_lineage.values().copied().sum();

        per_step.push(StepMetrics {
            step,
            threat_score,
            cell_count,
            replications,
            signals_total,
            lineage_shifts_total: lineage_total,
            stimulus_total,
            signals_by_topic,
            lineage_shifts_by_lineage: lineage_by_lineage,
            stimulus_by_topic,
        });
    }

    morphogenetic_security::adversarial::write_step_metrics_csv(&metrics_path, &per_step)?;

    Ok(ExecutionReport {
        steps: per_step,
        telemetry_path: Some(telemetry_path),
        metrics_path: Some(metrics_path),
        stimulus_path: persisted_stimulus,
    })
}

fn parse_seed(raw: &str) -> Result<SeedCandidate, String> {
    let (id, scenario) = raw
        .split_once('=')
        .or_else(|| raw.split_once(':'))
        .ok_or_else(|| "Seed must be formatted as <id>=<scenario_path>".to_string())?;

    if id.trim().is_empty() {
        return Err("Seed identifier cannot be empty".to_string());
    }

    let scenario_path = PathBuf::from(scenario);
    Ok(SeedCandidate {
        id: id.to_string(),
        scenario: scenario_path,
        note: None,
    })
}

fn parse_args() -> Result<CliArgs, String> {
    let mut args = env::args().skip(1).peekable();
    if matches!(args.peek(), Some(flag) if flag == "--help" || flag == "-h") {
        print_usage();
        process::exit(0);
    }

    let mut state_path: Option<PathBuf> = None;
    let mut generations: usize = 1;
    let mut artifact_dir: PathBuf = PathBuf::from("target/adversarial_runs");
    let mut batch_size: Option<usize> = None;
    let mut max_generations: Option<u32> = None;
    let mut retain_elite: Option<bool> = None;
    let mut seeds: Vec<SeedCandidate> = Vec::new();
    let mut stimulus_path: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--state" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --state".to_string())?;
                state_path = Some(PathBuf::from(value));
            }
            "--generations" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --generations".to_string())?;
                generations = value
                    .parse::<usize>()
                    .map_err(|_| "Generations must be a positive integer".to_string())?;
            }
            "--artifact-dir" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --artifact-dir".to_string())?;
                artifact_dir = PathBuf::from(value);
            }
            "--batch-size" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --batch-size".to_string())?;
                batch_size = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| "Batch size must be a positive integer".to_string())?,
                );
            }
            "--max-generations" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --max-generations".to_string())?;
                max_generations = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| "Max generations must be a positive integer".to_string())?,
                );
            }
            "--retain-elite" => {
                retain_elite = Some(true);
            }
            "--no-retain-elite" => {
                retain_elite = Some(false);
            }
            "--seed" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --seed".to_string())?;
                seeds.push(parse_seed(&value)?);
            }
            "--stimulus" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --stimulus".to_string())?;
                stimulus_path = Some(PathBuf::from(value));
            }
            unknown => {
                return Err(format!("Unknown argument `{unknown}`"));
            }
        }
    }

    let state_path =
        state_path.ok_or_else(|| "Missing required argument --state <path>".to_string())?;

    Ok(CliArgs {
        state_path,
        generations,
        artifact_dir,
        batch_size,
        max_generations,
        retain_elite,
        seeds,
        stimulus_path,
    })
}

fn print_usage() {
    println!(
        "Usage: cargo run --bin adversarial_loop -- --state <state.json> [options]

Options:
  --generations <n>        Number of loop iterations to execute (default: 1)
  --artifact-dir <path>    Directory for telemetry and metrics outputs (default: target/adversarial_runs)
  --batch-size <n>         Override batch size when creating a new harness
  --max-generations <n>    Override archival depth when creating a new harness
  --retain-elite           Retain elite candidates for future mutation (new harness only)
  --no-retain-elite        Disable elite retention (new harness only)
  --seed <id>=<scenario>   Enqueue a seed scenario (can repeat)
  --stimulus <path>        Stimulus schedule JSONL applied to each run
  --help                   Show this message"
    );
}

struct CliArgs {
    state_path: PathBuf,
    generations: usize,
    artifact_dir: PathBuf,
    batch_size: Option<usize>,
    max_generations: Option<u32>,
    retain_elite: Option<bool>,
    seeds: Vec<SeedCandidate>,
    stimulus_path: Option<PathBuf>,
}

struct SeedCandidate {
    id: String,
    scenario: PathBuf,
    note: Option<String>,
}
