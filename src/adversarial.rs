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

/// Configuration knobs for the evolution harness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    /// Number of candidates to evaluate in a single iteration.
    pub batch_size: usize,
    /// Maximum generations to retain when archiving outcomes.
    pub max_generations: u32,
    /// Whether to requeue high-performing candidates for mutation.
    pub retain_elite: bool,
}

impl EvolutionConfig {
    /// Construct an [`EvolutionConfig`] with sane defaults for quick experiments.
    pub fn default_smoke_test() -> Self {
        Self {
            batch_size: 3,
            max_generations: 10,
            retain_elite: true,
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
        mut executor: F,
    ) -> Result<Vec<EvaluatedCandidate>, HarnessError>
    where
        F: FnMut(&AttackCandidate) -> Result<ExecutionReport, HarnessError>,
    {
        let mut evaluations = Vec::new();

        for _ in 0..generations {
            if self.backlog.is_empty() {
                break;
            }

            let batch = self.next_batch();
            if batch.is_empty() {
                break;
            }

            for candidate in batch {
                let candidate_snapshot = candidate.clone();
                let report = executor(&candidate_snapshot)?;
                let stats = build_statistics_from_steps(&report.steps)?;
                let analysis = analyze_run_statistics(stats);
                let (outcome, follow_up, analysis) = self.finalize_evaluation(candidate, analysis);
                let backlog_len_after = self.backlog.len();
                evaluations.push(EvaluatedCandidate {
                    candidate: candidate_snapshot,
                    outcome,
                    analysis,
                    follow_up,
                    report,
                    backlog_len_after,
                });
            }
        }

        Ok(evaluations)
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
        } else if self.config.retain_elite {
            let mut retained = candidate;
            retained.mutation = None;
            self.enqueue(retained);
        }

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
    use serde::Serialize;
    use serde_json::json;
    use std::io::Cursor;
    use tempfile::NamedTempFile;

    #[test]
    fn next_batch_respects_batch_size() {
        let mut harness = AdversarialHarness::new(EvolutionConfig {
            batch_size: 2,
            max_generations: 5,
            retain_elite: false,
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

        let evaluations = harness
            .run_generations(2, |candidate| {
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

        assert_eq!(evaluations.len(), 2);
        assert_eq!(harness.archive.len(), 2);
    }

    #[test]
    fn archive_prunes_to_configured_limit() {
        let mut harness = AdversarialHarness::new(EvolutionConfig {
            batch_size: 1,
            max_generations: 2,
            retain_elite: false,
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