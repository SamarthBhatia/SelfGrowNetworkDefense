//! Adversarial attack evolution harness scaffolding.
//!
//! This module outlines the structures required to evolve adversarial attack
//! scenarios against the morphogenetic runtime. The intent is to support
//! population-based search where candidates are queued, executed, scored, and
//! either retired or mutated for additional pressure testing.

use std::collections::VecDeque;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
