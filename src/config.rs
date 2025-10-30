//! Scenario configuration and loading utilities.

use serde::Deserialize;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioConfig {
    #[serde(default = "default_scenario_name")]
    pub scenario_name: String,
    #[serde(default = "default_initial_cells")]
    pub initial_cell_count: usize,
    #[serde(default = "default_simulation_steps")]
    pub simulation_steps: u32,
    #[serde(default)]
    pub threat_profile: ThreatProfile,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThreatProfile {
    #[serde(default = "default_background_threat")]
    pub background_threat: f32,
    #[serde(default = "default_spike_threshold")]
    pub spike_threshold: f32,
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
}
