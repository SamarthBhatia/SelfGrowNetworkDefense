use morphogenetic_security::adversarial::{
    AdversarialHarness, AttackCandidate, EvolutionConfig, HarnessAnalysis,
};
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    if !args.metrics_path.exists() {
        return Err(format!(
            "Metrics CSV not found at `{}`",
            args.metrics_path.display()
        ));
    }

    let mut config = EvolutionConfig::default_smoke_test();
    if let Some(batch_size) = args.batch_size {
        config.batch_size = batch_size;
    }
    if let Some(max_generations) = args.max_generations {
        config.max_generations = max_generations;
    }
    if let Some(retain_elite) = args.retain_elite {
        config.retain_elite = retain_elite;
    }

    let mut harness = AdversarialHarness::new(config);
    let candidate = AttackCandidate {
        id: args.candidate_id.clone(),
        scenario_ref: args.scenario_ref.clone(),
        generation: args.generation,
        mutation_note: args.initial_note.clone(),
    };

    let (outcome, maybe_mutation, analysis) =
        harness
            .evaluate_csv(candidate, &args.metrics_path)
            .map_err(|err| format!("Harness evaluation failed: {err}"))?;
    let backlog_len = harness.backlog_len();

    print_summary(&analysis, &outcome, maybe_mutation.as_ref(), backlog_len);

    if let Some(path) = args.emit_json {
        write_json(&analysis, &outcome, maybe_mutation.as_ref(), path)?;
    }

    Ok(())
}

fn print_summary(
    analysis: &HarnessAnalysis,
    outcome: &morphogenetic_security::adversarial::AttackOutcome,
    maybe_mutation: Option<&AttackCandidate>,
    backlog_len: usize,
) {
    let stats = &analysis.statistics;
    println!("=== Harness Evaluation ===");
    println!(
        "Candidate `{}` generation {} => fitness {:.3} | breach={}",
        outcome.candidate_id, outcome.generation, analysis.fitness_score, analysis.breach_observed
    );
    println!(
        "Steps: {} | avg threat {:.2} (max {:.2}) | replications {} | signals {} | stimulus {:.2}",
        stats.step_count,
        stats.avg_threat,
        stats.max_threat,
        stats.total_replications,
        stats.total_signals,
        stats.total_stimulus
    );
    println!(
        "Cell count range: min {} -> max {} | avg {:.2}",
        stats.min_cell_count, stats.max_cell_count, stats.avg_cell_count
    );

    if let Some(mutation) = &analysis.recommended_mutation {
        println!("Recommended mutation: {mutation}");
    } else {
        println!("Recommended mutation: none (candidate retained)");
    }

    if let Some(next_candidate) = maybe_mutation {
        println!(
            "Queued follow-up candidate `{}` (generation {}) with note: {}",
            next_candidate.id,
            next_candidate.generation,
            next_candidate
                .mutation_note
                .as_deref()
                .unwrap_or("<unspecified>")
        );
    }
    println!("Harness backlog size after evaluation: {backlog_len}");
}

fn write_json(
    analysis: &HarnessAnalysis,
    outcome: &morphogenetic_security::adversarial::AttackOutcome,
    maybe_mutation: Option<&AttackCandidate>,
    path: PathBuf,
) -> Result<(), String> {
    let stats = &analysis.statistics;
    let payload = json!({
        "outcome": {
            "candidate_id": outcome.candidate_id,
            "generation": outcome.generation,
            "fitness_score": outcome.fitness_score,
            "breach_observed": outcome.breach_observed,
            "notes": outcome.notes,
        },
        "statistics": {
            "step_count": stats.step_count,
            "avg_threat": stats.avg_threat,
            "max_threat": stats.max_threat,
            "avg_cell_count": stats.avg_cell_count,
            "min_cell_count": stats.min_cell_count,
            "max_cell_count": stats.max_cell_count,
            "total_replications": stats.total_replications,
            "total_signals": stats.total_signals,
            "total_lineage_shifts": stats.total_lineage_shifts,
            "total_stimulus": stats.total_stimulus,
            "signals_by_topic": stats.signals_by_topic,
            "lineage_by_type": stats.lineage_by_type,
            "stimuli_by_topic": stats.stimuli_by_topic,
        },
        "recommended_mutation": analysis.recommended_mutation,
        "next_candidate": maybe_mutation.map(|candidate| {
            json!({
                "id": candidate.id,
                "scenario_ref": candidate.scenario_ref,
                "generation": candidate.generation,
                "mutation_note": candidate.mutation_note,
            })
        }),
    });

    let parent = path.parent().map(PathBuf::from);
    if let Some(parent_dir) = parent {
        if !parent_dir.as_path().exists() {
            fs::create_dir_all(&parent_dir)
                .map_err(|err| format!("Failed to create output directory: {err}"))?;
        }
    }

    fs::write(
        &path,
        serde_json::to_string_pretty(&payload).map_err(|err| err.to_string())?,
    )
    .map_err(|err| format!("Failed to write JSON output: {err}"))?;
    println!("Wrote harness evaluation JSON to {}", path.display());
    Ok(())
}

fn parse_args() -> Result<CliArgs, String> {
    let mut args = env::args().skip(1).peekable();
    if matches!(args.peek(), Some(flag) if flag == "--help" || flag == "-h") {
        print_usage();
        process::exit(0);
    }

    let mut candidate_id: Option<String> = None;
    let mut scenario_ref: Option<String> = None;
    let mut generation: u32 = 0;
    let mut metrics_path: Option<PathBuf> = None;
    let mut initial_note: Option<String> = None;
    let mut batch_size: Option<usize> = None;
    let mut max_generations: Option<u32> = None;
    let mut retain_elite: Option<bool> = None;
    let mut emit_json: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--candidate-id" => {
                candidate_id = Some(
                    args.next()
                        .ok_or_else(|| "Missing value for --candidate-id".to_string())?,
                );
            }
            "--scenario" => {
                scenario_ref = Some(
                    args.next()
                        .ok_or_else(|| "Missing value for --scenario".to_string())?,
                );
            }
            "--generation" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --generation".to_string())?;
                generation = value
                    .parse::<u32>()
                    .map_err(|_| "Generation must be a non-negative integer".to_string())?;
            }
            "--metrics" => {
                metrics_path = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "Missing value for --metrics".to_string())?,
                ));
            }
            "--note" => {
                initial_note = Some(
                    args.next()
                        .ok_or_else(|| "Missing value for --note".to_string())?,
                );
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
            "--retain-elite" => retain_elite = Some(true),
            "--no-retain-elite" => retain_elite = Some(false),
            "--emit-json" => {
                emit_json =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        "Missing value for --emit-json".to_string()
                    })?));
            }
            unknown => {
                return Err(format!("Unknown argument `{unknown}`"));
            }
        }
    }

    let candidate_id =
        candidate_id.ok_or_else(|| "Missing required argument --candidate-id".to_string())?;
    let scenario_ref =
        scenario_ref.ok_or_else(|| "Missing required argument --scenario".to_string())?;
    let metrics_path =
        metrics_path.ok_or_else(|| "Missing required argument --metrics".to_string())?;

    Ok(CliArgs {
        candidate_id,
        scenario_ref,
        generation,
        metrics_path,
        initial_note,
        batch_size,
        max_generations,
        retain_elite,
        emit_json,
    })
}

fn print_usage() {
    println!(
        "Usage: cargo run --bin adversarial_cycle -- --candidate-id <ID> --scenario <path> --metrics <csv> [options]

Options:
  --generation <n>         Generation index for the evaluated candidate (default: 0)
  --note <text>            Optional note describing the evaluated candidate
  --batch-size <n>         Override harness batch size (default: 3)
  --max-generations <n>    Override harness archival depth (default: 10)
  --retain-elite           Retain high performers for future mutation (default: true)
  --no-retain-elite        Disable elite retention
  --emit-json <path>       Persist evaluation output as JSON
  --help                   Show this message"
    );
}

struct CliArgs {
    candidate_id: String,
    scenario_ref: String,
    generation: u32,
    metrics_path: PathBuf,
    initial_note: Option<String>,
    batch_size: Option<usize>,
    max_generations: Option<u32>,
    retain_elite: Option<bool>,
    emit_json: Option<PathBuf>,
}
