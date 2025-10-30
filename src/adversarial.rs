//! Adversarial attack evolution harness scaffolding.
//!
//! This module outlines the structures required to evolve adversarial attack
//! scenarios against the morphogenetic runtime. The intent is to support
//! population-based search where candidates are queued, executed, scored, and
//! either retired or mutated for additional pressure testing.

use csv::Reader;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

/// Configuration knobs for the evolution harness.
#[derive(Debug, Clone)]
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

/// Description of an attack scenario candidate scheduled for execution.
#[derive(Debug, Clone)]
pub struct AttackCandidate {
    /// Unique identifier for correlating outcomes and telemetry.
    pub id: String,
    /// Path to the scenario manifest or generator seed.
    pub scenario_ref: String,
    /// Generation index (0 for seed scenarios).
    pub generation: u32,
    /// Optional notes describing the mutation applied to derive the candidate.
    pub mutation_note: Option<String>,
}

/// Recorded outcome after executing a candidate against the runtime.
#[derive(Debug, Clone)]
pub struct AttackOutcome {
    /// Identifier linking back to [`AttackCandidate::id`].
    pub candidate_id: String,
    /// Generation that produced the candidate.
    pub generation: u32,
    /// Fitness score where higher values imply stronger adversarial pressure.
    pub fitness_score: f32,
    /// Whether the candidate forced a breach or critical degradation.
    pub breach_observed: bool,
    /// Free-form notes (e.g., telemetry pointers, anomaly details).
    pub notes: Option<String>,
}

/// Aggregated statistics derived from dashboard-ready telemetry exports.
#[derive(Debug, Clone)]
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
    pub recommended_mutation: Option<String>,
}

/// Errors emitted when processing harness analytics.
#[derive(Debug)]
pub enum HarnessError {
    Io(io::Error),
    Csv(csv::Error),
    Json(serde_json::Error),
    EmptyDataset,
}

impl std::fmt::Display for HarnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HarnessError::Io(err) => write!(f, "IO error: {err}"),
            HarnessError::Csv(err) => write!(f, "CSV parse error: {err}"),
            HarnessError::Json(err) => write!(f, "JSON parse error: {err}"),
            HarnessError::EmptyDataset => write!(f, "no rows found in telemetry metrics CSV"),
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
    }

    /// Evaluate a candidate by ingesting dashboard-ready metrics and enqueue follow-up mutations.
    pub fn evaluate_csv<P: AsRef<Path>>(
        &mut self,
        mut candidate: AttackCandidate,
        metrics_csv: P,
    ) -> Result<(AttackOutcome, Option<AttackCandidate>, HarnessAnalysis), HarnessError> {
        let analysis = analyze_metrics_csv(metrics_csv)?;
        let note = outcome_note_for_analysis(&analysis);
        let outcome = AttackOutcome {
            candidate_id: candidate.id.clone(),
            generation: candidate.generation,
            fitness_score: analysis.fitness_score,
            breach_observed: analysis.breach_observed,
            notes: Some(note),
        };
        self.record_outcome(outcome.clone());

        let next_candidate = analysis.recommended_mutation.as_ref().map(|mutation| {
            let next_generation = candidate.generation + 1;
            AttackCandidate {
                id: format!("{}-mut{}", candidate.id, next_generation),
                scenario_ref: candidate.scenario_ref.clone(),
                generation: next_generation,
                mutation_note: Some(mutation.clone()),
            }
        });

        if let Some(mutant) = &next_candidate {
            self.enqueue(mutant.clone());
        } else if self.config.retain_elite {
            // Retain the evaluated candidate for potential future mutations.
            candidate.mutation_note = Some("retained for future mutation".to_string());
            self.enqueue(candidate);
        }

        Ok((outcome, next_candidate, analysis))
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
}

/// Load metrics produced by `scripts/prepare_telemetry_dashboard.py` and derive harness guidance.
pub fn analyze_metrics_csv<P: AsRef<Path>>(path: P) -> Result<HarnessAnalysis, HarnessError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    analyze_metrics_from_reader(reader)
}

#[derive(Debug, Deserialize)]
struct MetricsRow {
    threat_score: f32,
    cell_count: u32,
    replications: u32,
    signals_total: u32,
    lineage_shifts_total: u32,
    stimulus_total: f32,
    signals_by_topic: String,
    lineage_shifts_by_lineage: String,
    stimulus_by_topic: String,
}

fn analyze_metrics_from_reader<R: Read>(reader: R) -> Result<HarnessAnalysis, HarnessError> {
    let mut csv_reader = Reader::from_reader(reader);
    let mut step_count = 0usize;
    let mut threat_sum = 0.0f32;
    let mut max_threat = f32::MIN;
    let mut cell_sum = 0.0f32;
    let mut min_cell = u32::MAX;
    let mut max_cell = 0u32;
    let mut total_replications = 0u32;
    let mut total_signals = 0u32;
    let mut total_lineage_shifts = 0u32;
    let mut total_stimulus = 0.0f32;
    let mut signals_by_topic: HashMap<String, u32> = HashMap::new();
    let mut lineage_by_type: HashMap<String, u32> = HashMap::new();
    let mut stimuli_by_topic: HashMap<String, f32> = HashMap::new();

    for record in csv_reader.deserialize::<MetricsRow>() {
        let row = record?;
        step_count += 1;
        threat_sum += row.threat_score;
        max_threat = max_threat.max(row.threat_score);
        cell_sum += row.cell_count as f32;
        min_cell = min_cell.min(row.cell_count);
        max_cell = max_cell.max(row.cell_count);
        total_replications += row.replications;
        total_signals += row.signals_total;
        total_lineage_shifts += row.lineage_shifts_total;
        total_stimulus += row.stimulus_total;

        accumulate_counts(&mut signals_by_topic, &row.signals_by_topic)?;
        accumulate_counts(&mut lineage_by_type, &row.lineage_shifts_by_lineage)?;
        accumulate_stimuli(&mut stimuli_by_topic, &row.stimulus_by_topic)?;
    }

    if step_count == 0 {
        return Err(HarnessError::EmptyDataset);
    }

    let stats = RunStatistics {
        step_count,
        avg_threat: threat_sum / step_count as f32,
        max_threat,
        avg_cell_count: cell_sum / step_count as f32,
        min_cell_count: min_cell as usize,
        max_cell_count: max_cell as usize,
        total_replications,
        total_signals,
        total_lineage_shifts,
        total_stimulus,
        signals_by_topic,
        lineage_by_type,
        stimuli_by_topic,
    };

    let (fitness_score, breach_observed) = compute_fitness(&stats);
    let recommended_mutation = recommend_mutation(&stats, fitness_score, breach_observed);

    Ok(HarnessAnalysis {
        statistics: stats,
        fitness_score,
        breach_observed,
        recommended_mutation,
    })
}

fn accumulate_counts(
    accumulator: &mut HashMap<String, u32>,
    raw_json: &str,
) -> Result<(), HarnessError> {
    for (key, value) in parse_u32_map(raw_json)? {
        *accumulator.entry(key).or_insert(0) += value;
    }
    Ok(())
}

fn accumulate_stimuli(
    accumulator: &mut HashMap<String, f32>,
    raw_json: &str,
) -> Result<(), HarnessError> {
    for (key, value) in parse_f32_map(raw_json)? {
        *accumulator.entry(key).or_insert(0.0) += value;
    }
    Ok(())
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

    let fitness = 0.45 * threat_component
        + 0.25 * suppression_component
        + 0.2 * cell_loss_component
        + 0.1 * stimulus_component;
    let breach_observed = fitness > 0.7 || stats.max_threat > 1.1 || cell_loss_component > 0.4;
    (fitness, breach_observed)
}

fn recommend_mutation(
    stats: &RunStatistics,
    fitness_score: f32,
    breach_observed: bool,
) -> Option<String> {
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

    if fitness_score < 0.4 {
        if activator <= inhibitor {
            Some("increase activator spike amplitude and damp inhibitor recovery".to_string())
        } else {
            Some(
                "inject cooperative decoys ahead of activator bursts to overwhelm defences"
                    .to_string(),
            )
        }
    } else if breach_observed {
        if stats.total_signals < stats.step_count as u32 {
            Some("extend breach window with sustained activator pulses post-impact".to_string())
        } else {
            Some(
                "tighten attack cadence: alternate activator and inhibitor surges faster"
                    .to_string(),
            )
        }
    } else if reproduction_rate > 0.6 {
        Some(
            "slow defensive replication by scheduling inhibitor spikes before activator peaks"
                .to_string(),
        )
    } else if inhibitor > activator && activator > 0.0 {
        Some(
            "rebalance stimuli by boosting activator intensity relative to inhibitor damping"
                .to_string(),
        )
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
        format!("{base}; next_mutation={mutation}")
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
            generation: 0,
            mutation_note: None,
        });

        harness.enqueue(AttackCandidate {
            id: "mut-2".into(),
            scenario_ref: "docs/examples/intense-defense.yaml".into(),
            generation: 1,
            mutation_note: Some("increased inhibitor spike".into()),
        });

        harness.enqueue(AttackCandidate {
            id: "mut-3".into(),
            scenario_ref: "docs/examples/rapid-probe.yaml".into(),
            generation: 1,
            mutation_note: Some("shortened tick window".into()),
        });

        let first_batch = harness.next_batch();
        assert_eq!(first_batch.len(), 2);
        assert_eq!(harness.backlog_len(), 1);

        let second_batch = harness.next_batch();
        assert_eq!(second_batch.len(), 1);
        assert_eq!(harness.backlog_len(), 0);
    }

    #[test]
    fn recent_outcomes_tracks_latest_entries() {
        let mut harness = AdversarialHarness::new(EvolutionConfig {
            batch_size: 1,
            max_generations: 2,
            retain_elite: true,
        });

        harness.record_outcome(AttackOutcome {
            candidate_id: "seed-1".into(),
            generation: 0,
            fitness_score: 0.4,
            breach_observed: false,
            notes: None,
        });

        harness.record_outcome(AttackOutcome {
            candidate_id: "mut-2".into(),
            generation: 1,
            fitness_score: 0.7,
            breach_observed: true,
            notes: Some("breached quorum guard".into()),
        });

        harness.record_outcome(AttackOutcome {
            candidate_id: "mut-3".into(),
            generation: 2,
            fitness_score: 0.5,
            breach_observed: false,
            notes: None,
        });

        let recent = harness.recent_outcomes();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].candidate_id, "mut-3");
        assert_eq!(recent[1].candidate_id, "mut-2");
    }

    #[test]
    fn analyze_metrics_produces_statistics_and_mutation() {
        let rows = vec![
            TestRow::new(
                0.45,
                3,
                1,
                &json!({ "activator": 2 }),
                &json!({}),
                &json!({ "activator": 0.6 }),
                2,
                0,
                0.6,
            ),
            TestRow::new(
                0.82,
                3,
                0,
                &json!({ "inhibitor": 1 }),
                &json!({ "Healer": 1 }),
                &json!({ "activator": 0.2, "inhibitor": 0.9 }),
                1,
                1,
                1.1,
            ),
            TestRow::new(
                1.05,
                2,
                0,
                &json!({ "activator": 2 }),
                &json!({}),
                &json!({ "activator": 0.4, "inhibitor": 1.0 }),
                2,
                0,
                1.4,
            ),
        ];
        let csv = serialize_rows(rows);
        let analysis = analyze_metrics_from_reader(Cursor::new(csv)).expect("analysis");
        assert_eq!(analysis.statistics.step_count, 3);
        assert!(analysis.fitness_score >= 0.0);
        assert!(analysis.statistics.total_signals > 0);
        // Ensure mutation guidance is surfaced for low fitness cases.
        assert!(analysis.recommended_mutation.is_some());
    }

    #[test]
    fn evaluate_csv_records_outcome_and_enqueues_mutation() {
        let rows = vec![
            TestRow::new(
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

        let temp_csv = tempfile::NamedTempFile::new().expect("temp file");
        std::fs::write(temp_csv.path(), csv).expect("write csv");

        let candidate = AttackCandidate {
            id: "seed-ci".into(),
            scenario_ref: "docs/examples/intense-defense.yaml".into(),
            generation: 0,
            mutation_note: None,
        };

        let (outcome, maybe_mutation, analysis) = harness
            .evaluate_csv(candidate, temp_csv.path())
            .expect("evaluation");
        assert!((0.0..=1.0).contains(&analysis.fitness_score));
        assert_eq!(outcome.candidate_id, "seed-ci");
        if let Some(mutant) = maybe_mutation {
            assert!(mutant.id.starts_with("seed-ci-mut"));
            assert_eq!(harness.backlog_len(), 1);
        }
    }

    #[derive(Serialize)]
    struct TestRow {
        threat_score: f32,
        cell_count: u32,
        replications: u32,
        signals_total: u32,
        lineage_shifts_total: u32,
        stimulus_total: f32,
        signals_by_topic: String,
        lineage_shifts_by_lineage: String,
        stimulus_by_topic: String,
    }

    impl TestRow {
        fn new(
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
            Self {
                threat_score,
                cell_count,
                replications,
                signals_total,
                lineage_shifts_total: lineage_total,
                stimulus_total,
                signals_by_topic: signals_map.to_string(),
                lineage_shifts_by_lineage: lineage_map.to_string(),
                stimulus_by_topic: stimulus_map.to_string(),
            }
        }
    }

    fn serialize_rows(rows: Vec<TestRow>) -> String {
        let mut writer = csv::Writer::from_writer(vec![]);
        for row in rows {
            writer.serialize(&row).expect("serialize row");
        }
        let bytes = writer.into_inner().expect("extract writer");
        String::from_utf8(bytes).expect("utf8")
    }
}
