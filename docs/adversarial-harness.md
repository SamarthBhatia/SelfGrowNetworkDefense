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
4. **Elite Retention**: When `retain_elite` is enabled, high-performing candidates can be mutated and pushed back onto the backlog via `maybe_requeue`.
5. **Analytics Export**: Harness consumers can call `recent_outcomes` (and, in future, richer views) to feed dashboard pipelines or regression checks.

## Near-Term Next Steps
- Implement mutation strategies that derive new `AttackCandidate`s from logged `AttackOutcome`s.
- Connect outcome scoring to telemetry aggregates produced by `scripts/prepare_telemetry_dashboard.py`.
- Expose harness controls through the orchestration layer so CI smoke tests can iterate multiple generations per run.
- Define persistence (JSONL or DB) for harness state enabling long-running adversarial experiments.
