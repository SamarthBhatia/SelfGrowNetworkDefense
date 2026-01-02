//! Scenario configuration and loading utilities.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScenarioConfig {
    #[serde(default = "default_scenario_name")]
    pub scenario_name: String,
    #[serde(default = "default_initial_cells")]
    pub initial_cell_count: usize,
    #[serde(default = "default_simulation_steps")]
    pub simulation_steps: u32,
    #[serde(default)]
    pub threat_profile: ThreatProfile,
    #[serde(default)]
    pub spikes: Vec<ThreatSpike>,
    #[serde(default = "default_cell_reproduction_rate")]
    pub cell_reproduction_rate: f32,
    #[serde(default)]
    pub topology: TopologyConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThreatProfile {
    #[serde(default = "default_background_threat")]
    pub background_threat: f32,
    #[serde(default = "default_spike_threshold")]
    pub spike_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyConfig {
    pub strategy: TopologyStrategy,
    #[serde(default)]
    pub explicit_links: Option<Vec<Vec<String>>>, // List of [source, target] pairs
}

impl Default for TopologyConfig {
    fn default() -> Self {
        Self {
            strategy: TopologyStrategy::Global,
            explicit_links: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum TopologyStrategy {
    Global, // Broadcast to all
    Graph,  // Explicit neighbor list
}

#[allow(dead_code)]
fn default_topology_strategy() -> TopologyStrategy {
    TopologyStrategy::Global
}

impl Default for ThreatProfile {
    fn default() -> Self {
        Self {
            background_threat: default_background_threat(),
            spike_threshold: default_spike_threshold(),
        }
    }
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        Self {
            scenario_name: default_scenario_name(),
            initial_cell_count: default_initial_cells(),
            simulation_steps: default_simulation_steps(),
            threat_profile: ThreatProfile::default(),
            spikes: Vec::new(),
            cell_reproduction_rate: default_cell_reproduction_rate(),
            topology: TopologyConfig::default(),
        }
    }
}

fn default_scenario_name() -> String {
    "baseline".to_string()
}

fn default_initial_cells() -> usize {
    1
}

fn default_simulation_steps() -> u32 {
    1
}

fn default_background_threat() -> f32 {
    0.1
}

fn default_spike_threshold() -> f32 {
    0.8
}

fn default_cell_reproduction_rate() -> f32 {
    1.0
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ThreatSpike {
    pub step: u32,
    pub intensity: f32,
    #[serde(default = "default_spike_duration")]
    pub duration: u32,
}

fn default_spike_duration() -> u32 {
    1
}

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Parse(serde_yaml::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(err) => write!(f, "I/O error while reading config: {err}"),
            ConfigError::Parse(err) => write!(f, "Failed to parse config: {err}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Io(err) => Some(err),
            ConfigError::Parse(err) => Some(err),
        }
    }
}

impl From<io::Error> for ConfigError {
    fn from(value: io::Error) -> Self {
        ConfigError::Io(value)
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(value: serde_yaml::Error) -> Self {
        ConfigError::Parse(value)
    }
}

pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<ScenarioConfig, ConfigError> {
    let file = File::open(path)?;
    load_from_reader(file)
}

pub fn load_from_reader<R: Read>(mut reader: R) -> Result<ScenarioConfig, ConfigError> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    let config = serde_yaml::from_str(&buf)?;
    Ok(config)
}

impl ScenarioConfig {
    #[allow(dead_code)]
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let file = File::create(path)?;
        serde_yaml::to_writer(file, self)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn threat_level_for_step(&self, step: u32) -> f32 {
        let mut threat = self.threat_profile.background_threat;
        for spike in &self.spikes {
            if step >= spike.step && step < spike.step + spike.duration {
                threat += spike.intensity;
            }
        }
        threat.max(0.0)
    }

    #[allow(dead_code)]
    pub fn apply_mutation(&mut self, mutation: &crate::adversarial::Mutation) {
        use crate::adversarial::Mutation;
        match mutation {
            Mutation::AddSpike { step, intensity } => {
                self.spikes.push(ThreatSpike {
                    step: *step,
                    intensity: *intensity,
                    duration: default_spike_duration(), // Add default duration
                });
                self.spikes.sort_by_key(|s| s.step);
            }
            Mutation::ChangeThreatSpike {
                event_index,
                new_step,
                new_intensity,
            } => {
                if let Some(spike) = self.spikes.get_mut(*event_index) {
                    spike.step = *new_step;
                    spike.intensity = *new_intensity;
                }
            }
            Mutation::ChangeReproductionRate { factor } => {
                self.cell_reproduction_rate *= factor;
            }
            Mutation::ChangeInitialCellCount { count } => {
                self.initial_cell_count = *count;
            }
            Mutation::ChangeThreatProfile { profile } => {
                self.threat_profile = profile.clone();
            }
            Mutation::ChangeThreatSpikeTime {
                spike_index,
                new_step,
            } => {
                if let Some(spike) = self.spikes.get_mut(*spike_index) {
                    spike.step = *new_step;
                }
            }
            Mutation::ChangeThreatSpikeDuration {
                spike_index,
                new_duration,
            } => {
                if let Some(spike) = self.spikes.get_mut(*spike_index) {
                    spike.duration = *new_duration;
                }
            }
            _ => {
                // Other mutations are handled by stimulus or other config fields
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_applied_when_fields_missing() {
        let yaml = "scenario_name: test-lab\n";
        let config = load_from_reader(yaml.as_bytes()).expect("config should parse");
        assert_eq!(config.scenario_name, "test-lab");
        assert_eq!(config.initial_cell_count, 1);
        assert_eq!(config.simulation_steps, 1);
        assert!((config.threat_profile.background_threat - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn explicit_values_override_defaults() {
        let yaml = r#"
scenario_name: stress-test
initial_cell_count: 3
simulation_steps: 5
threat_profile:
  background_threat: 0.3
  spike_threshold: 0.6
"#;
        let config = load_from_reader(yaml.as_bytes()).expect("config should parse");
        assert_eq!(config.scenario_name, "stress-test");
        assert_eq!(config.initial_cell_count, 3);
        assert_eq!(config.simulation_steps, 5);
        assert!((config.threat_profile.background_threat - 0.3).abs() < f32::EPSILON);
        assert!((config.threat_profile.spike_threshold - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn threat_schedule_adds_spikes_on_matching_steps() {
        let yaml = r#"
threat_profile:
  background_threat: 0.2
spikes:
  - step: 1
    intensity: 0.3
    duration: 1
  - step: 3
    intensity: 0.5
    duration: 1
"#;
        let config = load_from_reader(yaml.as_bytes()).expect("config should parse");
        assert!((config.threat_level_for_step(0) - 0.2).abs() < f32::EPSILON);
        assert!((config.threat_level_for_step(1) - 0.5).abs() < f32::EPSILON);
        assert!((config.threat_level_for_step(2) - 0.2).abs() < f32::EPSILON);
        assert!((config.threat_level_for_step(3) - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_change_initial_cell_count_mutation() {
        let mut scenario_config = ScenarioConfig::default();
        assert_eq!(scenario_config.initial_cell_count, 1);

        let mutation = crate::adversarial::Mutation::ChangeInitialCellCount { count: 5 };
        scenario_config.apply_mutation(&mutation);
        assert_eq!(scenario_config.initial_cell_count, 5);

        let mutation = crate::adversarial::Mutation::ChangeInitialCellCount { count: 10 };
        scenario_config.apply_mutation(&mutation);
        assert_eq!(scenario_config.initial_cell_count, 10);
    }

    #[test]
    fn test_change_threat_profile_mutation() {
        let mut scenario_config = ScenarioConfig::default();
        assert!((scenario_config.threat_profile.background_threat - 0.1).abs() < f32::EPSILON);
        assert!((scenario_config.threat_profile.spike_threshold - 0.8).abs() < f32::EPSILON);

        let new_profile = ThreatProfile {
            background_threat: 0.5,
            spike_threshold: 0.9,
        };
        let mutation = crate::adversarial::Mutation::ChangeThreatProfile {
            profile: new_profile.clone(),
        };
        scenario_config.apply_mutation(&mutation);
        assert!((scenario_config.threat_profile.background_threat - 0.5).abs() < f32::EPSILON);
        assert!((scenario_config.threat_profile.spike_threshold - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn test_change_threat_spike_duration_mutation() {
        let mut scenario_config = ScenarioConfig::default();
        scenario_config.spikes.push(ThreatSpike {
            step: 10,
            intensity: 0.5,
            duration: 5,
        });

        assert_eq!(scenario_config.spikes[0].duration, 5);

        let mutation = crate::adversarial::Mutation::ChangeThreatSpikeDuration {
            spike_index: 0,
            new_duration: 10,
        };
        scenario_config.apply_mutation(&mutation);
        assert_eq!(scenario_config.spikes[0].duration, 10);

        // Test out of bounds index (should not panic or change anything)
        let mutation_oob = crate::adversarial::Mutation::ChangeThreatSpikeDuration {
            spike_index: 99,
            new_duration: 20,
        };
        scenario_config.apply_mutation(&mutation_oob);
        assert_eq!(scenario_config.spikes[0].duration, 10);
    }
}
