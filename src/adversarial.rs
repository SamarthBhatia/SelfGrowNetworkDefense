//! Adversarial attack evolution harness with persistence and execution utilities.
//!
//! This module coordinates iterative attack scenario exploration, supports
//! multi-generation execution loops, and provides helpers for persisting harness
//! state between runs. It also exposes analytics utilities that convert
//! telemetry-derived per-step metrics into fitness scores and mutation guidance.

use csv::{Reader, WriterBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

use rand::seq::SliceRandom;
use rand::Rng;

use crate::config;
use crate::config::ConfigError;
use crate::stimulus::StimulusSchedule;
use std::collections::{BTreeMap, HashSet};

/// The strategy used for selecting parents for the next generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectionStrategy {
    /// Select parents using tournament selection with a given size.
    Tournament { size: usize },
    /// Select parents using roulette wheel selection.
    RouletteWheel,
}

/// The strategy used for crossover.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrossoverStrategy {
    /// Each stimulus command is chosen from one of the parents at random.
    Uniform,
}

/// The strategy used for mutation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MutationStrategy {
    /// A random mutation is chosen from a predefined set.
    Random,
}


/// Configuration knobs for the evolution harness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    /// Number of candidates to evaluate in a single iteration.
    pub batch_size: usize,
    /// Maximum generations to retain when archiving outcomes.
    pub max_generations: u32,
    /// Whether to requeue high-performing candidates for mutation.
    pub retain_elite: bool,
    /// The probability of performing crossover (0.0 to 1.0).
    pub crossover_rate: f32,
    /// The selection strategy to use for breeding new candidates.
    pub selection_strategy: SelectionStrategy,
    /// The crossover strategy to use for breeding new candidates.
    pub crossover_strategy: CrossoverStrategy,
    /// The mutation strategy to use for breeding new candidates.
    pub mutation_strategy: MutationStrategy,
}

impl EvolutionConfig {
    /// Construct an [`EvolutionConfig`] with sane defaults for quick experiments.
    pub fn default_smoke_test() -> Self {
        Self {
            batch_size: 3,
            max_generations: 10,
            retain_elite: true,
            crossover_rate: 0.7,
            selection_strategy: SelectionStrategy::Tournament { size: 3 },
            crossover_strategy: CrossoverStrategy::Uniform,
            mutation_strategy: MutationStrategy::Random,
        }
    }
}

/// A structured mutation to be applied to an attack candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Mutation {
    IncreaseStimulus { topic: String, factor: f32 },
    DecreaseStimulus { topic: String, factor: f32 },
    AddSpike { step: u32, intensity: f32 },
    ChangeEventTiming { event_index: usize, new_step: u32 },
}

/// Description of an attack scenario candidate scheduled for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackCandidate {
    /// Unique identifier for correlating outcomes and telemetry.
    pub id: String,
    /// Path to the scenario manifest or generator seed.
    pub scenario_ref: String,
    /// Optional path to a stimulus schedule associated with this candidate.
    pub stimulus_ref: Option<String>,
    /// Generation index (0 for seed scenarios).
    pub generation: u32,
    /// Optional identifier of the candidate that produced this mutation.
    pub parent_id: Option<String>,
    /// Optional mutation that produced this candidate.
    pub mutation: Option<Mutation>,
}

/// Recorded outcome after executing a candidate against the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackOutcome {
    /// Snapshot of the candidate that produced this outcome.
    pub candidate: AttackCandidate,
    /// Fitness score where higher values imply stronger adversarial pressure.
    pub fitness_score: f32,
    /// Whether the candidate forced a breach or critical degradation.
    pub breach_observed: bool,
    /// Free-form notes (e.g., telemetry pointers, anomaly details).
    pub notes: Option<String>,
    /// Aggregated run statistics derived from telemetry exports.
    pub statistics: RunStatistics,
}

/// Aggregated statistics derived from dashboard-ready telemetry exports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStatistics {
    pub step_count: usize,
    pub avg_threat: f32,
    pub max_threat: f32,
    pub avg_cell_count: f32,
    pub min_cell_count: usize,
    pub max_cell_count: usize,
    pub total_replications: u32,
    pub total_signals: u32,
    pub total_lineage_shifts: u32,
    pub total_stimulus: f32,
    pub signals_by_topic: HashMap<String, u32>,
    pub lineage_by_type: HashMap<String, u32>,
    pub stimuli_by_topic: HashMap<String, f32>,
}

/// Harness evaluation result combining statistics, fitness, and guidance.
#[derive(Debug, Clone)]
pub struct HarnessAnalysis {
    pub statistics: RunStatistics,
    pub fitness_score: f32,
    pub breach_observed: bool,
    pub recommended_mutation: Option<Mutation>,
}

/// Per-step telemetry summary used to build [`RunStatistics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepMetrics {
    pub step: u32,
    pub threat_score: f32,
    pub cell_count: u32,
    pub replications: u32,
    pub signals_total: u32,
    pub lineage_shifts_total: u32,
    pub stimulus_total: f32,
    pub signals_by_topic: HashMap<String, u32>,
    pub lineage_shifts_by_lineage: HashMap<String, u32>,
    pub stimulus_by_topic: HashMap<String, f32>,
}

/// Rolling archive snapshot used for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessState {
    pub config: EvolutionConfig,
    pub backlog: VecDeque<AttackCandidate>,
    pub archive: Vec<AttackOutcome>,
}

/// Execution artifacts captured while running a candidate.
#[derive(Debug, Clone)]
pub struct ExecutionReport {
    /// Per-step metrics summarising the telemetry for this run.
    pub steps: Vec<StepMetrics>,
    /// Optional path to persisted telemetry JSONL.
    pub telemetry_path: Option<PathBuf>,
    /// Optional path to persisted per-step metrics CSV.
    pub metrics_path: Option<PathBuf>,
    /// Optional path to the stimulus schedule used for the run.
    pub stimulus_path: Option<PathBuf>,
}

/// Result bundle returned for each evaluated candidate in a loop.
#[derive(Debug, Clone)]
pub struct EvaluatedCandidate {
    pub candidate: AttackCandidate,
    pub outcome: AttackOutcome,
    pub analysis: HarnessAnalysis,
    pub follow_up: Option<AttackCandidate>,
    pub report: ExecutionReport,
    pub backlog_len_after: usize,
}

/// Errors emitted when processing harness analytics or persistence.
#[derive(Debug)]
pub enum HarnessError {
    Io(io::Error),
    Csv(csv::Error),
    Json(serde_json::Error),
    EmptyDataset,
    Custom(String),
}

impl std::fmt::Display for HarnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HarnessError::Io(err) => write!(f, "IO error: {err}"),
            HarnessError::Csv(err) => write!(f, "CSV parse error: {err}"),
            HarnessError::Json(err) => write!(f, "JSON parse error: {err}"),
            HarnessError::EmptyDataset => write!(f, "no rows found in telemetry metrics"),
            HarnessError::Custom(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for HarnessError {}

impl From<io::Error> for HarnessError {
    fn from(value: io::Error) -> Self {
        HarnessError::Io(value)
    }
}

impl From<csv::Error> for HarnessError {
    fn from(value: csv::Error) -> Self {
        HarnessError::Csv(value)
    }
}

impl From<serde_json::Error> for HarnessError {
    fn from(value: serde_json::Error) -> Self {
        HarnessError::Json(value)
    }
}

impl From<ConfigError> for HarnessError {
    fn from(value: ConfigError) -> Self {
        HarnessError::Custom(format!("Config error: {value}"))
    }
}

/// Rolling archive of adversarial exploration.
#[derive(Debug)]
pub struct AdversarialHarness {
    config: EvolutionConfig,
    backlog: VecDeque<AttackCandidate>,
    archive: Vec<AttackOutcome>,
}

impl AdversarialHarness {
    /// Create a new harness with the provided [`EvolutionConfig`].
    pub fn new(config: EvolutionConfig) -> Self {
        Self {
            config,
            backlog: VecDeque::new(),
            archive: Vec::new(),
        }
    }

    /// Reconstruct a harness from persisted state.
    pub fn from_state(state: HarnessState) -> Self {
        Self {
            config: state.config,
            backlog: state.backlog,
            archive: state.archive,
        }
    }

    /// Persist the current harness snapshot to disk.
    pub fn save_state<P: AsRef<Path>>(&self, path: P) -> Result<(), HarnessError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &self.snapshot_state())?;
        Ok(())
    }

    /// Load a persisted harness state from disk.
    pub fn load_state<P: AsRef<Path>>(path: P) -> Result<Self, HarnessError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let state: HarnessState = serde_json::from_reader(reader)?;
        Ok(Self::from_state(state))
    }

    /// Current harness configuration.
    pub fn config(&self) -> &EvolutionConfig {
        &self.config
    }

    /// Number of pending candidates waiting to be executed.
    pub fn backlog_len(&self) -> usize {
        self.backlog.len()
    }

    /// Queue a new candidate for evaluation.
    pub fn enqueue(&mut self, candidate: AttackCandidate) {
        self.backlog.push_back(candidate);
    }

    /// Fetch the next batch of candidates constrained by `batch_size`.
    ///
    /// Returned candidates are removed from the backlog so the caller can run
    /// them immediately. Unavailable slots result in a smaller batch.
    pub fn next_batch(&mut self) -> Vec<AttackCandidate> {
        let mut batch = Vec::with_capacity(self.config.batch_size);
        for _ in 0..self.config.batch_size {
            if let Some(candidate) = self.backlog.pop_front() {
                batch.push(candidate);
            } else {
                break;
            }
        }
        batch
    }

    /// Persist an execution outcome for downstream analytics.
    pub fn record_outcome(&mut self, outcome: AttackOutcome) {
        self.archive.push(outcome);
        let limit = self.config.max_generations as usize;
        if limit == 0 {
            self.archive.clear();
        } else if self.archive.len() > limit {
            let overflow = self.archive.len() - limit;
            self.archive.drain(0..overflow);
        }
    }

    /// Evaluate a candidate by ingesting dashboard-ready metrics and enqueue follow-up mutations.
    pub fn evaluate_csv<P: AsRef<Path>>(
        &mut self,
        candidate: AttackCandidate,
        metrics_csv: P,
    ) -> Result<(AttackOutcome, Option<AttackCandidate>, HarnessAnalysis), HarnessError> {
        let file = File::open(metrics_csv)?;
        let reader = BufReader::new(file);
        let steps = load_step_metrics_from_csv(reader)?;
        self.evaluate_steps(candidate, steps)
    }

    /// Evaluate a candidate using precomputed per-step metrics.
    pub fn evaluate_steps(
        &mut self,
        candidate: AttackCandidate,
        steps: Vec<StepMetrics>,
    ) -> Result<(AttackOutcome, Option<AttackCandidate>, HarnessAnalysis), HarnessError> {
        let stats = build_statistics_from_steps(&steps)?;
        let analysis = analyze_run_statistics(stats);
        Ok(self.finalize_evaluation(candidate, analysis))
    }

    /// Execute multiple generations using a caller-provided executor.
    ///
    /// The executor is responsible for running the morphogenetic runtime and
    /// returning per-step metrics alongside any persisted artifacts.
    pub fn run_generations<F>(
        &mut self,
        generations: usize,
        artifact_root: &Path,
        mut executor: F,
    ) -> Result<Vec<EvaluatedCandidate>, HarnessError>
    where
        F: FnMut(&AttackCandidate) -> Result<ExecutionReport, HarnessError>,
    {
        let mut all_evaluations = Vec::new();

        for gen_idx in 0..generations {
            println!("[info] Starting generation {}/{}", gen_idx + 1, generations);

            // 1. Process all candidates currently in the backlog
            let mut current_generation_evaluations = Vec::new();
            let backlog_size = self.backlog.len();

            if backlog_size == 0 && gen_idx > 0 {
                println!("[warn] Backlog empty, no candidates to evaluate for this generation.");
                break;
            }

            let candidates_to_process: Vec<AttackCandidate> = (0..backlog_size)
                .filter_map(|_| self.backlog.pop_front())
                .collect();

            if candidates_to_process.is_empty() && gen_idx == 0 {
                println!("[warn] No seed candidates in backlog. Exiting.");
                break;
            }

            for candidate in candidates_to_process {
                let candidate_snapshot = candidate.clone();
                let report = executor(&candidate_snapshot)?;
                let stats = build_statistics_from_steps(&report.steps)?;
                let analysis = analyze_run_statistics(stats);
                let (outcome, follow_up, analysis) = self.finalize_evaluation(candidate, analysis);
                let backlog_len_after = self.backlog.len(); // This backlog length is for immediate follow-ups
                current_generation_evaluations.push(EvaluatedCandidate {
                    candidate: candidate_snapshot,
                    outcome,
                    analysis,
                    follow_up,
                    report,
                    backlog_len_after,
                });
            }
            all_evaluations.extend(current_generation_evaluations);

            // 2. Select parents and generate new candidates for the next generation
            if self.archive.is_empty() {
                println!("[warn] Archive empty, cannot select parents for next generation.");
                continue;
            }

            let num_new_candidates = self.config.batch_size; // Generate a new batch size worth of candidates
            let mut rng = rand::thread_rng();

            for _ in 0..num_new_candidates {
                let new_candidate = if rng.gen_range(0.0..1.0) < self.config.crossover_rate {
                    // Perform crossover
                    let parent1 = match self.config.selection_strategy {
                        SelectionStrategy::Tournament { size } => {
                            tournament_selection(&self.archive, size, &mut rng)
                        }
                        SelectionStrategy::RouletteWheel => roulette_wheel_selection(&self.archive, &mut rng),
                    }.map_err(|e| HarnessError::Custom(format!("Selection failed for parent 1: {}", e)))?;

                    let parent2 = match self.config.selection_strategy {
                        SelectionStrategy::Tournament { size } => {
                            tournament_selection(&self.archive, size, &mut rng)
                        }
                        SelectionStrategy::RouletteWheel => roulette_wheel_selection(&self.archive, &mut rng),
                    }.map_err(|e| HarnessError::Custom(format!("Selection failed for parent 2: {}", e)))?;

                    perform_crossover(
                        parent1,
                        parent2,
                        &mut rng,
                        artifact_root,
                        &self.config.crossover_strategy,
                    )?
                } else {
                    // Perform mutation
                    let parent_outcome = match self.config.selection_strategy {
                        SelectionStrategy::Tournament { size } => {
                            tournament_selection(&self.archive, size, &mut rng)
                        }
                        SelectionStrategy::RouletteWheel => roulette_wheel_selection(&self.archive, &mut rng),
                    }.map_err(|e| HarnessError::Custom(format!("Selection failed: {}", e)))?;

                    let new_candidate_id = format!("{}-gen{}-mut{}",
                        parent_outcome.candidate.id,
                        gen_idx + 1,
                        rng.gen_range(0..1000)
                    );
                    
                    let mutation = perform_mutation(
                        &self.config.mutation_strategy,
                        &parent_outcome.statistics,
                        parent_outcome.fitness_score,
                        parent_outcome.breach_observed,
                        &mut rng,
                    );

                    AttackCandidate {
                        id: new_candidate_id,
                        scenario_ref: parent_outcome.candidate.scenario_ref.clone(),
                        stimulus_ref: parent_outcome.candidate.stimulus_ref.clone(),
                        generation: gen_idx as u32 + 1,
                        parent_id: Some(parent_outcome.candidate.id.clone()),
                        mutation,
                    }
                };
                self.enqueue(new_candidate);
            }
            println!("[info] Enqueued {} new candidates for next generation.", num_new_candidates);
        }

        Ok(all_evaluations)
    }

    /// Most recent outcomes, truncated to the configured generation history.
    pub fn recent_outcomes(&self) -> Vec<&AttackOutcome> {
        let limit = self.config.max_generations as usize;
        self.archive
            .iter()
            .rev()
            .take(limit)
            .collect::<Vec<&AttackOutcome>>()
    }

    /// Requeue a candidate for additional mutations when elite retention is enabled.
    pub fn maybe_requeue(&mut self, candidate: AttackCandidate) {
        if self.config.retain_elite {
            self.backlog.push_back(candidate);
        }
    }

    fn finalize_evaluation(
        &mut self,
        candidate: AttackCandidate,
        analysis: HarnessAnalysis,
    ) -> (AttackOutcome, Option<AttackCandidate>, HarnessAnalysis) {
        let outcome_candidate = candidate.clone();
        let note = outcome_note_for_analysis(&analysis);

        let outcome = AttackOutcome {
            candidate: outcome_candidate.clone(),
            fitness_score: analysis.fitness_score,
            breach_observed: analysis.breach_observed,
            notes: Some(note),
            statistics: analysis.statistics.clone(),
        };
        self.record_outcome(outcome.clone());

        let recommended_mutation = analysis.recommended_mutation.clone();
        let next_candidate = recommended_mutation.map(|mutation| {
            let next_generation = candidate.generation + 1;
            AttackCandidate {
                id: format!("{}-mut{}", candidate.id, next_generation),
                scenario_ref: candidate.scenario_ref.clone(),
                stimulus_ref: candidate.stimulus_ref.clone(),
                generation: next_generation,
                parent_id: Some(candidate.id.clone()),
                mutation: Some(mutation),
            }
        });

        if let Some(mutant) = &next_candidate {
            self.enqueue(mutant.clone());
        } // Removed the else if self.config.retain_elite { ... } block here.

        (outcome, next_candidate, analysis)
    }

    fn snapshot_state(&self) -> HarnessState {
        HarnessState {
            config: self.config.clone(),
            backlog: self.backlog.clone(),
            archive: self.archive.clone(),
        }
    }
}

/// Applies a candidate's mutation to its scenario and stimulus (if present),
/// writing the modified definitions to new files within the specified artifact
/// root directory.
///
/// Returns the paths to the mutated scenario file and the mutated stimulus file (if any).
pub fn apply_mutation_and_generate_files(
    candidate: &AttackCandidate,
    artifact_root: &Path,
) -> Result<(PathBuf, Option<PathBuf>), HarnessError> {
    // Determine the directory for this candidate's artifacts
    let candidate_dir = artifact_root
        .join(format!("gen{:03}", candidate.generation))
        .join(&candidate.id);
    fs::create_dir_all(&candidate_dir)?;

    // Load and mutate scenario
    let original_scenario_path = PathBuf::from(&candidate.scenario_ref);
    let mut scenario_config =
        config::load_from_path(&original_scenario_path).map_err(|e| {
            HarnessError::Custom(format!(
                "Failed to load scenario from {}: {}",
                original_scenario_path.display(),
                e
            ))
        })?;

    if let Some(mutation) = &candidate.mutation {
        scenario_config.apply_mutation(mutation);
    }

    let mutated_scenario_path = candidate_dir.join(format!("{}.yaml", candidate.id));
    scenario_config.save_to_path(&mutated_scenario_path)?;

    // Load and mutate stimulus, if present
    let mut mutated_stimulus_path: Option<PathBuf> = None;
    if let Some(stimulus_ref) = &candidate.stimulus_ref {
        let original_stimulus_path = PathBuf::from(stimulus_ref);
        let mut stimulus_schedule =
            StimulusSchedule::load(&original_stimulus_path).map_err(|e| {
                HarnessError::Custom(format!(
                    "Failed to load stimulus from {}: {}",
                    original_stimulus_path.display(),
                    e
                ))
            })?;

        if let Some(mutation) = &candidate.mutation {
            stimulus_schedule.apply_mutation(mutation);
        }

        let current_mutated_stimulus_path = candidate_dir.join(format!("{}.jsonl", candidate.id));
        stimulus_schedule.save_to_path(&current_mutated_stimulus_path)?;
        mutated_stimulus_path = Some(current_mutated_stimulus_path);
    }

    Ok((mutated_scenario_path, mutated_stimulus_path))
}

/// Selects a parent [`AttackOutcome`] using tournament selection.
///
/// `population`: The pool of [`AttackOutcome`]s to select from.
/// `tournament_size`: The number of candidates to randomly pick for the tournament.
/// `rng`: A mutable reference to a random number generator.
///
/// Returns the selected [`AttackOutcome`] (the fittest in the tournament).
pub fn tournament_selection<'a, R: Rng>(
    population: &'a [AttackOutcome],
    tournament_size: usize,
    rng: &mut R,
) -> Result<&'a AttackOutcome, String> {
    if population.is_empty() {
        return Err("Cannot perform tournament selection on an empty population".to_string());
    }
    if tournament_size == 0 {
        return Err("Tournament size cannot be zero".to_string());
    }

    let actual_tournament_size = std::cmp::min(tournament_size, population.len());
    let selected_candidates: Vec<&AttackOutcome> = population
        .choose_multiple(rng, actual_tournament_size)
        .collect();

    selected_candidates
        .into_iter()
        .max_by(|a, b| a.fitness_score.partial_cmp(&b.fitness_score).unwrap_or(std::cmp::Ordering::Equal))
        .ok_or_else(|| "Failed to select candidate from tournament".to_string())
}

/// Selects a parent [`AttackOutcome`] using roulette wheel selection.
///
/// `population`: The pool of [`AttackOutcome`]s to select from.
/// `rng`: A mutable reference to a random number generator.
///
/// Returns the selected [`AttackOutcome`].
pub fn roulette_wheel_selection<'a, R: Rng>(
    population: &'a [AttackOutcome],
    rng: &mut R,
) -> Result<&'a AttackOutcome, String> {
    if population.is_empty() {
        return Err("Cannot perform roulette wheel selection on an empty population".to_string());
    }

    let total_fitness: f32 = population.iter().map(|outcome| outcome.fitness_score).sum();

    if total_fitness <= 0.0 {
        // If total fitness is zero or negative, fall back to uniform random selection
        population.choose(rng).ok_or_else(|| "Failed to select candidate from population".to_string()).map(|outcome| outcome)
    } else {
        let mut pick = rng.gen_range(0.0..total_fitness);
        for outcome in population {
            if pick < outcome.fitness_score {
                return Ok(outcome);
            }
            pick -= outcome.fitness_score;
        }
        // Fallback in case of floating point inaccuracies or if no outcome is picked
        population.choose(rng).ok_or_else(|| "Failed to select candidate from population".to_string()).map(|outcome| outcome)
    }
}

/// Performs a uniform crossover between two stimulus schedules.
fn uniform_crossover_stimulus<R: Rng>(
    parent1: &StimulusSchedule,
    parent2: &StimulusSchedule,
    rng: &mut R,
) -> StimulusSchedule {
    let mut child_commands = BTreeMap::new();
    let all_steps: HashSet<u32> = parent1
        .commands
        .keys()
        .chain(parent2.commands.keys())
        .copied()
        .collect();

    for step in all_steps {
        let p1_commands = parent1.commands.get(&step);
        let p2_commands = parent2.commands.get(&step);

        let chosen_commands = match (p1_commands, p2_commands) {
            (Some(cmds1), Some(cmds2)) => {
                if rng.gen_bool(0.5) {
                    cmds1.clone()
                } else {
                    cmds2.clone()
                }
            }
            (Some(cmds1), None) => cmds1.clone(),
            (None, Some(cmds2)) => cmds2.clone(),
            (None, None) => continue,
        };
        child_commands.insert(step, chosen_commands);
    }

    StimulusSchedule::new(child_commands, None)
}

/// Performs crossover between two parent outcomes to produce a new child candidate.
///
/// `parent1`: The first parent [`AttackOutcome`].
/// `parent2`: The second parent [`AttackOutcome`].
/// `rng`: A mutable reference to a random number generator.
///
/// Returns a new child [`AttackCandidate`].
pub fn perform_crossover<R: Rng>(
    parent1: &AttackOutcome,
    parent2: &AttackOutcome,
    rng: &mut R,
    artifact_root: &Path,
    crossover_strategy: &CrossoverStrategy,
) -> Result<AttackCandidate, HarnessError> {
    let child_scenario_ref = parent1.candidate.scenario_ref.clone();
    let child_generation =
        std::cmp::max(parent1.candidate.generation, parent2.candidate.generation) + 1;
    let child_id = format!(
        "crossover-{}-{}-gen{}",
        parent1.candidate.id, parent2.candidate.id, child_generation
    );

    let child_stimulus_ref = match (&parent1.candidate.stimulus_ref, &parent2.candidate.stimulus_ref) {
        (Some(s1), Some(s2)) => {
            let p1_schedule = StimulusSchedule::load(s1)?;
            let p2_schedule = StimulusSchedule::load(s2)?;

            let child_schedule = match crossover_strategy {
                CrossoverStrategy::Uniform => {
                    uniform_crossover_stimulus(&p1_schedule, &p2_schedule, rng)
                }
            };

            let child_stimulus_path = artifact_root
                .join(format!("gen{:03}", child_generation))
                .join(&child_id)
                .join("stimulus.jsonl");
            fs::create_dir_all(child_stimulus_path.parent().unwrap())?;
            child_schedule.save_to_path(&child_stimulus_path)?;
            Some(child_stimulus_path.to_string_lossy().to_string())
        }
        (Some(s1), None) => Some(s1.clone()),
        (None, Some(s2)) => Some(s2.clone()),
        (None, None) => None,
    };

    // Choose mutation from one of the parents, or create a new one if parents have no mutation
    let mutation = if rng.gen_bool(0.5) {
        parent1.candidate.mutation.clone()
    } else {
        parent2.candidate.mutation.clone()
    }
    .or_else(|| {
        // Fallback to a random mutation if both parents have no mutation
        let topics = ["activator", "inhibitor", "reproducer"];
        let topic = topics.choose(rng).unwrap_or(&"activator").to_string();
        Some(Mutation::IncreaseStimulus {
            topic,
            factor: rng.gen_range(1.1..=1.5),
        })
    });

    Ok(AttackCandidate {
        id: child_id,
        scenario_ref: child_scenario_ref,
        stimulus_ref: child_stimulus_ref,
        generation: child_generation,
        parent_id: Some(format!("{},{}", parent1.candidate.id, parent2.candidate.id)),
        mutation,
    })
}


/// Load metrics produced by `scripts/prepare_telemetry_dashboard.py` and derive harness guidance.
pub fn analyze_metrics_csv<P: AsRef<Path>>(path: P) -> Result<HarnessAnalysis, HarnessError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let steps = load_step_metrics_from_csv(reader)?;
    let stats = build_statistics_from_steps(&steps)?;
    Ok(analyze_run_statistics(stats))
}

/// Persist per-step metrics as a CSV compatible with the analytics tooling.
pub fn write_step_metrics_csv<P: AsRef<Path>>(
    path: P,
    steps: &[StepMetrics],
) -> Result<(), HarnessError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut writer = WriterBuilder::new().has_headers(true).from_path(path)?;

    writer.write_record([
        "step",
        "threat_score",
        "cell_count",
        "replications",
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
    ])?;

    for step in steps {
        let signals_json = serde_json::to_string(&step.signals_by_topic)?;
        let lineage_json = serde_json::to_string(&step.lineage_shifts_by_lineage)?;
        let stimulus_json = serde_json::to_string(&step.stimulus_by_topic)?;
        let (top_signal_topic, top_signal_count) = top_u32(&step.signals_by_topic);
        let (top_lineage, top_lineage_count) = top_u32(&step.lineage_shifts_by_lineage);

        writer.write_record([
            step.step.to_string(),
            format!("{:.6}", step.threat_score),
            step.cell_count.to_string(),
            step.replications.to_string(),
            step.signals_total.to_string(),
            step.lineage_shifts_total.to_string(),
            format!("{:.6}", step.stimulus_total),
            top_signal_topic,
            top_signal_count.to_string(),
            top_lineage,
            top_lineage_count.to_string(),
            signals_json,
            lineage_json,
            stimulus_json,
        ])?;
    }

    writer.flush()?;
    Ok(())
}

fn load_step_metrics_from_csv<R: Read>(reader: R) -> Result<Vec<StepMetrics>, HarnessError> {
    let mut csv_reader = Reader::from_reader(reader);
    let mut steps = Vec::new();

    for record in csv_reader.deserialize::<RawMetricsRow>() {
        let row = record?;
        let signals_map = parse_u32_map(&row.signals_by_topic)?;
        let lineage_map = parse_u32_map(&row.lineage_shifts_by_lineage)?;
        let stimulus_map = parse_f32_map(&row.stimulus_by_topic)?;

        steps.push(StepMetrics {
            step: row.step,
            threat_score: row.threat_score,
            cell_count: row.cell_count,
            replications: row.replications,
            signals_total: row.signals_total,
            lineage_shifts_total: row.lineage_shifts_total,
            stimulus_total: row.stimulus_total,
            signals_by_topic: signals_map,
            lineage_shifts_by_lineage: lineage_map,
            stimulus_by_topic: stimulus_map,
        });
    }

    Ok(steps)
}

#[derive(Debug, Deserialize)]
struct RawMetricsRow {
    step: u32,
    threat_score: f32,
    cell_count: u32,
    replications: u32,
    signals_total: u32,
    lineage_shifts_total: u32,
    stimulus_total: f32,
    #[serde(rename = "top_signal_topic")]
    _top_signal_topic: String,
    #[serde(rename = "top_signal_count")]
    _top_signal_count: u32,
    #[serde(rename = "top_lineage")]
    _top_lineage: String,
    #[serde(rename = "top_lineage_count")]
    _top_lineage_count: u32,
    signals_by_topic: String,
    lineage_shifts_by_lineage: String,
    stimulus_by_topic: String,
}

fn build_statistics_from_steps(steps: &[StepMetrics]) -> Result<RunStatistics, HarnessError> {
    let mut accumulator = StatsAccumulator::default();
    for step in steps {
        accumulator.add_step(step);
    }
    accumulator.finish()
}

#[derive(Default)]
struct StatsAccumulator {
    step_count: usize,
    threat_sum: f32,
    max_threat: f32,
    cell_sum: f32,
    min_cell: Option<u32>,
    max_cell: u32,
    total_replications: u32,
    total_signals: u32,
    total_lineage_shifts: u32,
    total_stimulus: f32,
    signals_by_topic: HashMap<String, u32>,
    lineage_by_type: HashMap<String, u32>,
    stimuli_by_topic: HashMap<String, f32>,
}

impl StatsAccumulator {
    fn add_step(&mut self, step: &StepMetrics) {
        self.step_count += 1;
        self.threat_sum += step.threat_score;
        self.max_threat = if self.step_count == 1 {
            step.threat_score
        } else {
            self.max_threat.max(step.threat_score)
        };
        self.cell_sum += step.cell_count as f32;
        self.min_cell = Some(match self.min_cell {
            Some(current) => current.min(step.cell_count),
            None => step.cell_count,
        });
        self.max_cell = self.max_cell.max(step.cell_count);
        self.total_replications += step.replications;
        self.total_signals += step.signals_total;
        self.total_lineage_shifts += step.lineage_shifts_total;
        self.total_stimulus += step.stimulus_total;

        merge_u32_map(&mut self.signals_by_topic, &step.signals_by_topic);
        merge_u32_map(&mut self.lineage_by_type, &step.lineage_shifts_by_lineage);
        merge_f32_map(&mut self.stimuli_by_topic, &step.stimulus_by_topic);
    }

    fn finish(self) -> Result<RunStatistics, HarnessError> {
        if self.step_count == 0 {
            return Err(HarnessError::EmptyDataset);
        }

        let min_cell = self.min_cell.unwrap_or(0) as usize;
        let max_cell = self.max_cell as usize;

        Ok(RunStatistics {
            step_count: self.step_count,
            avg_threat: self.threat_sum / self.step_count as f32,
            max_threat: self.max_threat,
            avg_cell_count: self.cell_sum / self.step_count as f32,
            min_cell_count: min_cell,
            max_cell_count: max_cell,
            total_replications: self.total_replications,
            total_signals: self.total_signals,
            total_lineage_shifts: self.total_lineage_shifts,
            total_stimulus: self.total_stimulus,
            signals_by_topic: self.signals_by_topic,
            lineage_by_type: self.lineage_by_type,
            stimuli_by_topic: self.stimuli_by_topic,
        })
    }
}

fn analyze_run_statistics(stats: RunStatistics) -> HarnessAnalysis {
    let (fitness_score, breach_observed) = compute_fitness(&stats);
    let recommended_mutation = recommend_mutation(&stats, fitness_score, breach_observed);
    HarnessAnalysis {
        statistics: stats,
        fitness_score,
        breach_observed,
        recommended_mutation,
    }
}

fn merge_u32_map(target: &mut HashMap<String, u32>, source: &HashMap<String, u32>) {
    for (key, value) in source {
        *target.entry(key.clone()).or_insert(0) += *value;
    }
}

fn merge_f32_map(target: &mut HashMap<String, f32>, source: &HashMap<String, f32>) {
    for (key, value) in source {
        *target.entry(key.clone()).or_insert(0.0) += *value;
    }
}

fn top_u32(map: &HashMap<String, u32>) -> (String, u32) {
    map.iter()
        .max_by_key(|entry| entry.1)
        .map(|(key, value)| (key.clone(), *value))
        .unwrap_or_else(|| (String::new(), 0))
}

fn parse_u32_map(raw_json: &str) -> Result<HashMap<String, u32>, HarnessError> {
    if raw_json.trim().is_empty() {
        return Ok(HashMap::new());
    }
    let value: Value = serde_json::from_str(raw_json)?;
    match value {
        Value::Object(map) => {
            let mut result = HashMap::new();
            for (key, val) in map {
                let count = match val {
                    Value::Number(number) => number.as_u64().unwrap_or(0) as u32,
                    Value::String(s) => s.parse::<u32>().unwrap_or(0),
                    _ => 0,
                };
                if count > 0 {
                    result.insert(key, count);
                }
            }
            Ok(result)
        }
        _ => Ok(HashMap::new()),
    }
}

fn parse_f32_map(raw_json: &str) -> Result<HashMap<String, f32>, HarnessError> {
    if raw_json.trim().is_empty() {
        return Ok(HashMap::new());
    }
    let value: Value = serde_json::from_str(raw_json)?;
    match value {
        Value::Object(map) => {
            let mut result = HashMap::new();
            for (key, val) in map {
                let magnitude = match val {
                    Value::Number(number) => number.as_f64().unwrap_or(0.0) as f32,
                    Value::String(s) => s.parse::<f32>().unwrap_or(0.0),
                    _ => 0.0,
                };
                if magnitude.abs() > f32::EPSILON {
                    result.insert(key, magnitude);
                }
            }
            Ok(result)
        }
        _ => Ok(HashMap::new()),
    }
}

fn compute_fitness(stats: &RunStatistics) -> (f32, bool) {
    let threat_component = (stats.avg_threat / 1.5).clamp(0.0, 1.0);
    let reproduction_rate =
        stats.total_replications as f32 / (stats.step_count as f32 + f32::EPSILON);
    let suppression_component = (1.0 - reproduction_rate.min(1.0)).max(0.0);
    let cell_loss_component = if stats.max_cell_count > 0 {
        ((stats.max_cell_count.saturating_sub(stats.min_cell_count)) as f32
            / stats.max_cell_count as f32)
            .clamp(0.0, 1.0)
    } else {
        0.0
    };
    let stimulus_component =
        (stats.total_stimulus / ((stats.step_count as f32).max(1.0) * 1.5)).clamp(0.0, 1.0);
    let lineage_component = compute_lineage_component(stats);

    let fitness = 0.35 * threat_component
        + 0.2 * suppression_component
        + 0.15 * cell_loss_component
        + 0.2 * lineage_component
        + 0.1 * stimulus_component;
    let breach_observed = fitness > 0.65
        || stats.max_threat > 1.1
        || cell_loss_component > 0.45
        || lineage_component > 0.8;
    (fitness, breach_observed)
}

fn perform_mutation<R: Rng>(
    mutation_strategy: &MutationStrategy,
    stats: &RunStatistics,
    fitness_score: f32,
    breach_observed: bool,
    rng: &mut R,
) -> Option<Mutation> {
    match mutation_strategy {
        MutationStrategy::Random => {
            recommend_mutation(stats, fitness_score, breach_observed).or_else(|| {
                let topics = ["activator", "inhibitor", "reproducer"];
                let topic = topics.choose(rng).unwrap_or(&"activator").to_string();
                Some(Mutation::IncreaseStimulus {
                    topic,
                    factor: rng.gen_range(1.1..=1.5),
                })
            })
        }
    }
}


fn compute_lineage_component(stats: &RunStatistics) -> f32 {
    if stats.step_count == 0 {
        return 0.0;
    }

    let pressure = stats.total_lineage_shifts as f32 / (stats.step_count as f32 + f32::EPSILON);
    let normalised_pressure = (pressure / 0.6).clamp(0.0, 1.0);

    let dominant_shift = stats.lineage_by_type.values().copied().max().unwrap_or(0) as f32;
    let focus_ratio = if stats.total_lineage_shifts == 0 {
        0.0
    } else {
        (dominant_shift / stats.total_lineage_shifts as f32).clamp(0.0, 1.0)
    };

    0.6 * normalised_pressure + 0.4 * focus_ratio
}

fn recommend_mutation(
    stats: &RunStatistics,
    fitness_score: f32,
    breach_observed: bool,
) -> Option<Mutation> {
    let activator = stats
        .stimuli_by_topic
        .get("activator")
        .copied()
        .unwrap_or(0.0);
    let inhibitor = stats
        .stimuli_by_topic
        .get("inhibitor")
        .copied()
        .unwrap_or(0.0);
    let reproduction_rate =
        stats.total_replications as f32 / (stats.step_count as f32 + f32::EPSILON);
    let lineage_pressure =
        stats.total_lineage_shifts as f32 / (stats.step_count as f32 + f32::EPSILON);
    let dominant_lineage_entry = stats.lineage_by_type.iter().max_by_key(|(_, count)| *count);
    let dominant_ratio = if stats.total_lineage_shifts == 0 {
        0.0
    } else {
        dominant_lineage_entry
            .map(|(_, count)| *count as f32 / stats.total_lineage_shifts as f32)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0)
    };

    if fitness_score < 0.4 {
        if activator <= inhibitor {
            Some(Mutation::IncreaseStimulus {
                topic: "activator".to_string(),
                factor: 1.2,
            })
        } else {
            Some(Mutation::IncreaseStimulus {
                topic: "inhibitor".to_string(),
                factor: 1.2,
            })
        }
    } else if breach_observed {
        if stats.total_signals < stats.step_count as u32 {
            Some(Mutation::AddSpike {
                step: stats.step_count as u32 / 2,
                intensity: 0.5,
            })
        } else {
            Some(Mutation::DecreaseStimulus {
                topic: "inhibitor".to_string(),
                factor: 0.8,
            })
        }
    } else if lineage_pressure < 0.2 {
        Some(Mutation::IncreaseStimulus {
            topic: "activator".to_string(),
            factor: 1.5,
        })
    } else if dominant_ratio < 0.5 && stats.total_lineage_shifts > 3 {
        if let Some((lineage, _)) = dominant_lineage_entry {
            Some(Mutation::IncreaseStimulus {
                topic: lineage.clone(),
                factor: 1.5,
            })
        } else {
            None
        }
    } else if reproduction_rate > 0.6 {
        Some(Mutation::IncreaseStimulus {
            topic: "inhibitor".to_string(),
            factor: 1.5,
        })
    } else if inhibitor > activator && activator > 0.0 {
        Some(Mutation::IncreaseStimulus {
            topic: "activator".to_string(),
            factor: 1.2,
        })
    } else {
        None
    }
}

fn outcome_note_for_analysis(analysis: &HarnessAnalysis) -> String {
    let stats = &analysis.statistics;
    let base = format!(
        "avg_threat={:.2}, replications_total={}, signals_total={}",
        stats.avg_threat, stats.total_replications, stats.total_signals
    );
    if let Some(mutation) = &analysis.recommended_mutation {
        format!("{base}; next_mutation={:?}", mutation)
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stimulus::StimulusCommand;
    use serde::Serialize;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::io::Cursor;
    use tempfile::{tempdir, NamedTempFile};

    #[test]
    fn retain_elite_requeues_elite_candidates() {
        let mut harness = AdversarialHarness::new(EvolutionConfig {
            batch_size: 1,
            max_generations: 5,
            retain_elite: true,
            crossover_rate: 0.7,
            selection_strategy: SelectionStrategy::Tournament { size: 3 },
            crossover_strategy: CrossoverStrategy::Uniform,
            mutation_strategy: MutationStrategy::Random,
        });

        harness.enqueue(AttackCandidate {
            id: "elite-seed-1".into(),
            scenario_ref: "docs/examples/a.yaml".into(),
            stimulus_ref: None,
            generation: 0,
            parent_id: None,
            mutation: None,
        });

        let artifact_dir = tempdir().expect("failed to create temp dir");
        let evaluations = harness
            .run_generations(1, artifact_dir.path(), |_candidate| {
                // Simulate an elite candidate (high fitness, no mutation recommended)
                let steps = vec![StepMetrics {
                    step: 1, // Changed step to 1 to influence lineage_pressure calculation
                    threat_score: 1.0, // Increased threat to make fitness_score > 0.4
                    cell_count: 10,
                    replications: 0, // Set replications to 0 to make reproduction_rate 0
                    signals_total: 0,
                    lineage_shifts_total: 1, // Set lineage_shifts_total to 1 to make lineage_pressure >= 0.2
                    stimulus_total: 0.0,
                    signals_by_topic: HashMap::new(),
                    lineage_shifts_by_lineage: HashMap::new(),
                    stimulus_by_topic: HashMap::new(), // Make stimulus_by_topic empty
                }];
                Ok(ExecutionReport {
                    steps,
                    telemetry_path: None,
                    metrics_path: None,
                    stimulus_path: None,
                })
            })
            .expect("elite candidate evaluation");

        assert_eq!(evaluations.len(), 1);
        assert_eq!(harness.archive.len(), 1);
        // The backlog should now contain `batch_size` new candidates generated by the selection process
        assert_eq!(harness.backlog_len(), harness.config.batch_size);
    }

    #[test]
    fn next_batch_respects_batch_size() {
        let mut harness = AdversarialHarness::new(EvolutionConfig {
            batch_size: 2,
            max_generations: 5,
            retain_elite: false,
            crossover_rate: 0.7,
            selection_strategy: SelectionStrategy::Tournament { size: 3 },
            crossover_strategy: CrossoverStrategy::Uniform,
            mutation_strategy: MutationStrategy::Random,
        });

        harness.enqueue(AttackCandidate {
            id: "seed-1".into(),
            scenario_ref: "docs/examples/baseline-growth.yaml".into(),
            stimulus_ref: None,
            generation: 0,
            parent_id: None,
            mutation: None,
        });

        harness.enqueue(AttackCandidate {
            id: "mut-2".into(),
            scenario_ref: "docs/examples/intense-defense.yaml".into(),
            stimulus_ref: None,
            generation: 1,
            parent_id: Some("seed-1".into()),
            mutation: Some(Mutation::IncreaseStimulus {
                topic: "inhibitor".to_string(),
                factor: 1.5,
            }),
        });

        harness.enqueue(AttackCandidate {
            id: "mut-3".into(),
            scenario_ref: "docs/examples/rapid-probe.yaml".into(),
            stimulus_ref: None,
            generation: 1,
            parent_id: Some("seed-1".into()),
            mutation: Some(Mutation::AddSpike {
                step: 10,
                intensity: 0.8,
            }),
        });

        let first_batch = harness.next_batch();
        assert_eq!(first_batch.len(), 2);
        assert_eq!(harness.backlog_len(), 1);

        let second_batch = harness.next_batch();
        assert_eq!(second_batch.len(), 1);
        assert_eq!(harness.backlog_len(), 0);
    }

    #[test]
    fn analyze_metrics_from_csv_stream() {
        let rows = vec![
            TestRow::new(
                0,
                0.42,
                5,
                2,
                &json!({ "activator": 3, "inhibitor": 1 }),
                &json!({ "stem": 2 }),
                &json!({ "activator": 0.6 }),
                4,
                2,
                0.6,
            ),
            TestRow::new(
                1,
                0.85,
                4,
                1,
                &json!({ "activator": 2 }),
                &json!({ "adaptive": 1 }),
                &json!({ "activator": 0.4, "inhibitor": 1.0 }),
                2,
                1,
                1.4,
            ),
        ];
        let csv = serialize_rows(rows);
        let analysis = analyze_metrics_csv_from_reader(Cursor::new(csv)).expect("analysis");
        assert_eq!(analysis.statistics.step_count, 2);
        assert!(analysis.fitness_score >= 0.0);
        assert!(analysis.statistics.total_signals > 0);
        assert!(analysis.recommended_mutation.is_some());
    }

    #[test]
    fn evaluate_csv_records_outcome_and_enqueues_mutation() {
        let rows = vec![
            TestRow::new(
                0,
                0.30,
                4,
                3,
                &json!({ "activator": 1 }),
                &json!({}),
                &json!({ "activator": 0.2 }),
                1,
                0,
                0.2,
            ),
            TestRow::new(
                1,
                0.28,
                5,
                2,
                &json!({ "activator": 1 }),
                &json!({}),
                &json!({ "activator": 0.2 }),
                1,
                0,
                0.2,
            ),
        ];
        let csv = serialize_rows(rows);

        let mut harness = AdversarialHarness::new(EvolutionConfig {
            batch_size: 1,
            max_generations: 3,
            retain_elite: false,
            crossover_rate: 0.7,
            selection_strategy: SelectionStrategy::Tournament { size: 3 },
            crossover_strategy: CrossoverStrategy::Uniform,
            mutation_strategy: MutationStrategy::Random,
        });

        let temp_csv = NamedTempFile::new().expect("temp file");
        std::fs::write(temp_csv.path(), csv).expect("write csv");

        let candidate = AttackCandidate {
            id: "seed-ci".into(),
            scenario_ref: "docs/examples/intense-defense.yaml".into(),
            stimulus_ref: Some("docs/examples/ci-stimulus.jsonl".into()),
            generation: 0,
            parent_id: None,
            mutation: None,
        };

        let (outcome, maybe_mutation, analysis) = harness
            .evaluate_csv(candidate, temp_csv.path())
            .expect("evaluation");
        assert!((0.0..=1.0).contains(&analysis.fitness_score));
        assert_eq!(outcome.candidate.id, "seed-ci");
        assert_eq!(outcome.statistics.step_count, 2);
        if let Some(mutant) = maybe_mutation {
            assert!(mutant.id.starts_with("seed-ci-mut"));
            assert_eq!(
                mutant.stimulus_ref.as_deref(),
                Some("docs/examples/ci-stimulus.jsonl")
            );
            assert_eq!(harness.backlog_len(), 1);
        }
    }

    #[test]
    fn harness_state_roundtrip_persists_archive() {
        let mut harness = AdversarialHarness::new(EvolutionConfig {
            batch_size: 1,
            max_generations: 4,
            retain_elite: true,
            crossover_rate: 0.7,
            selection_strategy: SelectionStrategy::Tournament { size: 3 },
            crossover_strategy: CrossoverStrategy::Uniform,
            mutation_strategy: MutationStrategy::Random,
        });

        let candidate = AttackCandidate {
            id: "state-seed".into(),
            scenario_ref: "docs/examples/intense-defense.yaml".into(),
            stimulus_ref: None,
            generation: 0,
            parent_id: None,
            mutation: Some(Mutation::AddSpike {
                step: 0,
                intensity: 0.0,
            }),
        };

        let steps = vec![StepMetrics {
            step: 0,
            threat_score: 0.5,
            cell_count: 4,
            replications: 1,
            signals_total: 1,
            lineage_shifts_total: 0,
            stimulus_total: 0.4,
            signals_by_topic: HashMap::from([("activator".into(), 1)]),
            lineage_shifts_by_lineage: HashMap::new(),
            stimulus_by_topic: HashMap::from([("activator".into(), 0.4)]),
        }];

        harness
            .evaluate_steps(candidate, steps)
            .expect("evaluation succeeds");

        let tmp = NamedTempFile::new().expect("temp file");
        harness.save_state(tmp.path()).expect("save state");

        let loaded = AdversarialHarness::load_state(tmp.path()).expect("load state");
        assert_eq!(loaded.config().batch_size, 1);
        assert_eq!(loaded.archive.len(), 1);
        assert_eq!(loaded.archive[0].candidate.id, "state-seed");
        assert!(loaded.archive[0].statistics.total_signals >= 1);
    }

    #[test]
    fn run_generations_executes_batches() {
        let mut harness = AdversarialHarness::new(EvolutionConfig {
            batch_size: 1,
            max_generations: 5,
            retain_elite: false,
            crossover_rate: 0.7,
            selection_strategy: SelectionStrategy::Tournament { size: 3 },
            crossover_strategy: CrossoverStrategy::Uniform,
            mutation_strategy: MutationStrategy::Random,
        });

        harness.enqueue(AttackCandidate {
            id: "loop-seed-1".into(),
            scenario_ref: "docs/examples/a.yaml".into(),
            stimulus_ref: None,
            generation: 0,
            parent_id: None,
            mutation: None,
        });
        harness.enqueue(AttackCandidate {
            id: "loop-seed-2".into(),
            scenario_ref: "docs/examples/b.yaml".into(),
            stimulus_ref: None,
            generation: 0,
            parent_id: None,
            mutation: None,
        });

        let artifact_dir = tempdir().expect("failed to create temp dir");
        let evaluations = harness
            .run_generations(2, artifact_dir.path(), |candidate| {
                let base_threat = 0.3 + candidate.generation as f32 * 0.1;
                let steps = vec![StepMetrics {
                    step: candidate.generation,
                    threat_score: base_threat,
                    cell_count: 4,
                    replications: 1,
                    signals_total: 0,
                    lineage_shifts_total: 0,
                    stimulus_total: 0.0,
                    signals_by_topic: HashMap::new(),
                    lineage_shifts_by_lineage: HashMap::new(),
                    stimulus_by_topic: HashMap::new(),
                }];
                Ok(ExecutionReport {
                    steps,
                    telemetry_path: None,
                    metrics_path: None,
                    stimulus_path: None,
                })
            })
            .expect("loop execution");

        assert_eq!(evaluations.len(), 5); // 2 initial + 3 mutants from gen 2
        assert_eq!(harness.archive.len(), 5); // 2 initial + 3 from gen 2
        assert_eq!(harness.backlog_len(), 4); // New candidates for gen 3
    }

    #[test]
    fn archive_prunes_to_configured_limit() {
        let mut harness = AdversarialHarness::new(EvolutionConfig {
            batch_size: 1,
            max_generations: 2,
            retain_elite: false,
            crossover_rate: 0.7,
            selection_strategy: SelectionStrategy::Tournament { size: 3 },
            crossover_strategy: CrossoverStrategy::Uniform,
            mutation_strategy: MutationStrategy::Random,
        });

        let template_stats = RunStatistics {
            step_count: 1,
            avg_threat: 0.1,
            max_threat: 0.2,
            avg_cell_count: 1.0,
            min_cell_count: 1,
            max_cell_count: 1,
            total_replications: 0,
            total_signals: 0,
            total_lineage_shifts: 0,
            total_stimulus: 0.0,
            signals_by_topic: HashMap::new(),
            lineage_by_type: HashMap::new(),
            stimuli_by_topic: HashMap::new(),
        };

        for idx in 0..3 {
            let outcome = AttackOutcome {
                candidate: AttackCandidate {
                    id: format!("cand-{idx}"),
                    scenario_ref: "docs/examples/demo.yaml".into(),
                    stimulus_ref: None,
                    generation: idx,
                    parent_id: None,
                    mutation: None,
                },
                fitness_score: idx as f32,
                breach_observed: false,
                notes: None,
                statistics: template_stats.clone(),
            };
            harness.record_outcome(outcome);
        }

        assert_eq!(harness.archive.len(), 2);
        assert_eq!(harness.archive[0].candidate.id, "cand-1");
        assert_eq!(harness.archive[1].candidate.id, "cand-2");
    }

    #[test]
    fn archive_clears_when_limit_zero() {
        let mut harness = AdversarialHarness::new(EvolutionConfig {
            batch_size: 1,
            max_generations: 0,
            retain_elite: false,
            crossover_rate: 0.7,
            selection_strategy: SelectionStrategy::Tournament { size: 3 },
            crossover_strategy: CrossoverStrategy::Uniform,
            mutation_strategy: MutationStrategy::Random,
        });

        let stats = RunStatistics {
            step_count: 1,
            avg_threat: 0.1,
            max_threat: 0.2,
            avg_cell_count: 1.0,
            min_cell_count: 1,
            max_cell_count: 1,
            total_replications: 0,
            total_signals: 0,
            total_lineage_shifts: 0,
            total_stimulus: 0.0,
            signals_by_topic: HashMap::new(),
            lineage_by_type: HashMap::new(),
            stimuli_by_topic: HashMap::new(),
        };

        let outcome = AttackOutcome {
            candidate: AttackCandidate {
                id: "zero-limit".into(),
                scenario_ref: "docs/examples/demo.yaml".into(),
                stimulus_ref: None,
                generation: 0,
                parent_id: None,
                mutation: None,
            },
            fitness_score: 0.5,
            breach_observed: false,
            notes: None,
            statistics: stats,
        };

        harness.record_outcome(outcome);
        assert!(harness.archive.is_empty());
    }

    #[test]
    fn lineage_component_boosts_fitness() {
        let base_stats = RunStatistics {
            step_count: 20,
            avg_threat: 0.35,
            max_threat: 0.9,
            avg_cell_count: 10.0,
            min_cell_count: 9,
            max_cell_count: 12,
            total_replications: 8,
            total_signals: 24,
            total_lineage_shifts: 0,
            total_stimulus: 0.6,
            signals_by_topic: HashMap::from([("activator".into(), 20)]),
            lineage_by_type: HashMap::new(),
            stimuli_by_topic: HashMap::from([("activator".into(), 0.6)]),
        };
        let (baseline_fitness, baseline_breach) = compute_fitness(&base_stats);
        assert!(baseline_fitness > 0.0);
        assert!(!baseline_breach);

        let mut elevated_stats = base_stats.clone();
        elevated_stats.total_lineage_shifts = 18;
        elevated_stats
            .lineage_by_type
            .insert("AdaptiveProbe".into(), 8);
        elevated_stats
            .lineage_by_type
            .insert("IntrusionDetection".into(), 10);

        let (elevated_fitness, elevated_breach) = compute_fitness(&elevated_stats);
        assert!(
            elevated_fitness > baseline_fitness + 0.1,
            "expected {elevated_fitness} to significantly exceed {baseline_fitness}"
        );
        assert!(elevated_breach);
    }

    #[test]
    fn recommendation_targets_lineage_churn_gap() {
        let stats = RunStatistics {
            step_count: 20,
            avg_threat: 0.6,
            max_threat: 1.0,
            avg_cell_count: 11.0,
            min_cell_count: 8,
            max_cell_count: 12,
            total_replications: 9,
            total_signals: 12,
            total_lineage_shifts: 2,
            total_stimulus: 0.0,
            signals_by_topic: HashMap::new(),
            lineage_by_type: HashMap::from([("IntrusionDetection".into(), 2)]),
            stimuli_by_topic: HashMap::new(),
        };

        let (fitness, breach) = compute_fitness(&stats);
        assert!(fitness >= 0.4);
        assert!(!breach);

        let suggestion =
            recommend_mutation(&stats, fitness, breach).expect("expected lineage churn guidance");
        assert!(
            match suggestion {
                Mutation::IncreaseStimulus { ref topic, .. } => topic == "activator",
                _ => false,
            },
            "expected suggestion to be IncreaseStimulus for activator: {suggestion:?}"
        );
    }

    #[test]
    fn recommendation_focuses_dominant_lineage_when_diffuse() {
        let stats = RunStatistics {
            step_count: 20,
            avg_threat: 0.55,
            max_threat: 1.0,
            avg_cell_count: 12.0,
            min_cell_count: 9,
            max_cell_count: 14,
            total_replications: 6,
            total_signals: 18,
            total_lineage_shifts: 8,
            total_stimulus: 0.5,
            signals_by_topic: HashMap::new(),
            lineage_by_type: HashMap::from([
                ("IntrusionDetection".into(), 3),
                ("AdaptiveProbe".into(), 3),
                ("Recon".into(), 2),
            ]),
            stimuli_by_topic: HashMap::new(),
        };

        let (fitness, breach) = compute_fitness(&stats);
        assert!(fitness > 0.4);
        assert!(!breach);

        let suggestion =
            recommend_mutation(&stats, fitness, breach).expect("expected dominant lineage focus");
        assert!(
            matches!(suggestion,  Mutation::IncreaseStimulus { .. }),
            "expected focus guidance: {suggestion:?}"
        );
        if let Mutation::IncreaseStimulus { topic, .. } = suggestion {
            let lineages = ["IntrusionDetection", "AdaptiveProbe", "Recon"];
                    assert!(
                        lineages.iter().any(|lineage| *topic == **lineage),
                        "expected suggestion to reference a known lineage: {topic}"
                    );        }
    }

    #[test]
    fn perform_crossover_creates_child() {
        let parent1_stimulus_file = NamedTempFile::new().unwrap();
        let parent2_stimulus_file = NamedTempFile::new().unwrap();

        let parent1_stimulus_path = parent1_stimulus_file.path().to_str().unwrap().to_string();
        let parent2_stimulus_path = parent2_stimulus_file.path().to_str().unwrap().to_string();

        let parent1_outcome = AttackOutcome {
            candidate: AttackCandidate {
                id: "parent1".to_string(),
                scenario_ref: "scenario1.yaml".to_string(),
                stimulus_ref: Some(parent1_stimulus_path),
                generation: 1,
                parent_id: None,
                mutation: Some(Mutation::AddSpike {
                    step: 10,
                    intensity: 0.5,
                }),
            },
            fitness_score: 0.7,
            breach_observed: true,
            notes: None,
            statistics: RunStatistics {
                step_count: 1,
                avg_threat: 0.1,
                max_threat: 0.2,
                avg_cell_count: 1.0,
                min_cell_count: 1,
                max_cell_count: 1,
                total_replications: 0,
                total_signals: 0,
                total_lineage_shifts: 0,
                total_stimulus: 0.0,
                signals_by_topic: HashMap::new(),
                lineage_by_type: HashMap::new(),
                stimuli_by_topic: HashMap::new(),
            },
        };

        let parent2_outcome = AttackOutcome {
            candidate: AttackCandidate {
                id: "parent2".to_string(),
                scenario_ref: "scenario2.yaml".to_string(),
                stimulus_ref: Some(parent2_stimulus_path),
                generation: 2,
                parent_id: None,
                mutation: Some(Mutation::IncreaseStimulus {
                    topic: "activator".to_string(),
                    factor: 1.2,
                }),
            },
            fitness_score: 0.8,
            breach_observed: true,
            notes: None,
            statistics: RunStatistics {
                step_count: 1,
                avg_threat: 0.1,
                max_threat: 0.2,
                avg_cell_count: 1.0,
                min_cell_count: 1,
                max_cell_count: 1,
                total_replications: 0,
                total_signals: 0,
                total_lineage_shifts: 0,
                total_stimulus: 0.0,
                signals_by_topic: HashMap::new(),
                lineage_by_type: HashMap::new(),
                stimuli_by_topic: HashMap::new(),
            },
        };

        let mut rng = rand::thread_rng();
        let artifact_dir = tempdir().expect("failed to create temp dir");
        let child = perform_crossover(
            &parent1_outcome,
            &parent2_outcome,
            &mut rng,
            artifact_dir.path(),
            &CrossoverStrategy::Uniform,
        )
        .expect("crossover failed");

        assert_eq!(child.scenario_ref, "scenario1.yaml");
        assert!(child.stimulus_ref.is_some());
        assert_eq!(child.generation, 3);
        assert!(child.id.starts_with("crossover-parent1-parent2-gen3"));
        assert!(child.mutation.is_some());
    }

    #[test]
    fn test_uniform_crossover_stimulus() {
        let mut parent1_commands = BTreeMap::new();
        parent1_commands.insert(
            0,
            vec![StimulusCommand {
                step: 0,
                topic: "a".to_string(),
                value: 1.0,
            }],
        );
        let parent1 = StimulusSchedule::new(parent1_commands, None);

        let mut parent2_commands = BTreeMap::new();
        parent2_commands.insert(
            0,
            vec![StimulusCommand {
                step: 0,
                topic: "b".to_string(),
                value: 2.0,
            }],
        );
        let parent2 = StimulusSchedule::new(parent2_commands, None);

        let mut rng = rand::thread_rng();
        let child = uniform_crossover_stimulus(&parent1, &parent2, &mut rng);

        assert_eq!(child.commands.len(), 1);
        let child_command = &child.commands.get(&0).unwrap()[0];
        assert!(child_command.topic == "a" || child_command.topic == "b");
    }

    #[test]
    fn test_perform_mutation() {
        let stats = RunStatistics {
            step_count: 1,
            avg_threat: 0.1,
            max_threat: 0.2,
            avg_cell_count: 1.0,
            min_cell_count: 1,
            max_cell_count: 1,
            total_replications: 0,
            total_signals: 0,
            total_lineage_shifts: 0,
            total_stimulus: 0.0,
            signals_by_topic: HashMap::new(),
            lineage_by_type: HashMap::new(),
            stimuli_by_topic: HashMap::new(),
        };
        let mut rng = rand::thread_rng();
        let mutation = perform_mutation(
            &MutationStrategy::Random,
            &stats,
            0.5,
            false,
            &mut rng,
        );
        assert!(mutation.is_some());
    }

    #[derive(Serialize)]
    struct TestRow {
        step: u32,
        threat_score: f32,
        cell_count: u32,
        replications: u32,
        signals_total: u32,
        lineage_shifts_total: u32,
        stimulus_total: f32,
        top_signal_topic: String,
        top_signal_count: u32,
        top_lineage: String,
        top_lineage_count: u32,
        signals_by_topic: String,
        lineage_shifts_by_lineage: String,
        stimulus_by_topic: String,
    }

    impl TestRow {
        fn new(
            step: u32,
            threat_score: f32,
            cell_count: u32,
            replications: u32,
            signals_map: &serde_json::Value,
            lineage_map: &serde_json::Value,
            stimulus_map: &serde_json::Value,
            signals_total: u32,
            lineage_total: u32,
            stimulus_total: f32,
        ) -> Self {
            let (signal_topic, signal_count) = top_from_value(signals_map, "");
            let (lineage_topic, lineage_count) = top_from_value(lineage_map, "");
            Self {
                step,
                threat_score,
                cell_count,
                replications,
                signals_total,
                lineage_shifts_total: lineage_total,
                stimulus_total,
                top_signal_topic: signal_topic,
                top_signal_count: signal_count,
                top_lineage: lineage_topic,
                top_lineage_count: lineage_count,
                signals_by_topic: signals_map.to_string(),
                lineage_shifts_by_lineage: lineage_map.to_string(),
                stimulus_by_topic: stimulus_map.to_string(),
            }
        }
    }

    fn top_from_value(value: &serde_json::Value, default: &str) -> (String, u32) {
        match value {
            serde_json::Value::Object(map) => map
                .iter()
                .max_by(|a, b| a.1.as_u64().cmp(&b.1.as_u64()))
                .map(|(k, v)| (k.clone(), v.as_u64().unwrap_or(0) as u32))
                .unwrap_or_else(|| (default.to_string(), 0)),
            _ => (default.to_string(), 0),
        }
    }

    fn serialize_rows(rows: Vec<TestRow>) -> Vec<u8> {
        let mut writer = csv::Writer::from_writer(vec![]);
        for row in rows {
            writer.serialize(&row).expect("serialize row");
        }
        writer.into_inner().expect("extract writer")
    }

    fn analyze_metrics_csv_from_reader<R: Read>(
        reader: R,
    ) -> Result<HarnessAnalysis, HarnessError> {
        let steps = load_step_metrics_from_csv(reader)?;
        let stats = build_statistics_from_steps(&steps)?;
        Ok(analyze_run_statistics(stats))
    }
}