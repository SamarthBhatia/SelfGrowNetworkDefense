# Adversarial Attack Evolution Harness

The harness coordinates iterative attack scenario generation so we can pressure-test the morphogenetic runtime under evolving conditions. This document captures the initial design intents landed in `src/adversarial.rs`.

## Goals
- Maintain a backlog of attack candidates derived from seed scenarios and mutation rules.
- Execute candidates in configurable batches, ingest telemetry, and score effectiveness.
- Retain outcome history for analytics dashboards and elite candidate recycling.
- Provide a scaffolding layer that future orchestration or CI workers can embed.

## High-Level Flow
1. **Seeding**: CLI tooling or orchestration code enqueues `AttackCandidate`s referencing scenario manifests or generator seeds.
2. **Batch Execution**: `AdversarialHarness::next_batch` surfaces a slice of candidates sized by `EvolutionConfig::batch_size` for immediate execution.
3. **Outcome Recording**: After each run the caller records `AttackOutcome` objects containing fitness signals (breach toggles, threat deltas, etc.).
4. **Metrics Ingestion**: Dashboard-ready CSV exports (from `scripts/prepare_telemetry_dashboard.py`) feed into `analyze_metrics_csv`, producing `HarnessAnalysis` with aggregate statistics, fitness scores, and mutation recommendations.
5. **Adversarial CLI**: `cargo run --bin adversarial_cycle -- ...` wires everything together—loading metrics, recording the outcome, emitting JSON summaries, and queueing follow-up mutations automatically.
6. **Elite Retention**: When `retain_elite` is enabled, high-performing candidates can be requeued for future mutation should no new candidate be produced.
7. **Analytics Export**: Harness consumers can call `recent_outcomes` (and, in future, richer views) to feed dashboard pipelines or regression checks.

## Near-Term Next Steps
- Implement mutation strategies that derive new `AttackCandidate`s from logged `AttackOutcome`s.
- Expose harness controls through the orchestration layer so CI smoke tests can iterate multiple generations per run (instead of a single evaluation).
- Define persistence (JSONL or DB) for harness state enabling long-running adversarial experiments.
