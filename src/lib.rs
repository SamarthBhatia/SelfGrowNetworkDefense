//! Core library scaffolding for the morphogenetic security architecture.

pub mod adversarial;
pub mod cellular;
pub mod config;
pub mod orchestration;
pub mod signaling;
pub mod stimulus;
pub mod telemetry;

pub use adversarial::{AdversarialHarness, AttackCandidate, AttackOutcome, EvolutionConfig};
pub use config::{ConfigError, ScenarioConfig, ThreatSpike};
pub use orchestration::MorphogeneticApp;
