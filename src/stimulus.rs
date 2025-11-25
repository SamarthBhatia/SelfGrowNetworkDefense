//! Stimulus scheduling utilities for injecting signals during simulation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StimulusCommand {
    pub step: u32,
    pub topic: String,
    pub value: f32,
}

#[allow(dead_code)]
pub struct StimulusSchedule {
    pub commands: BTreeMap<u32, Vec<StimulusCommand>>,
    pub source: Option<PathBuf>,
}

impl StimulusSchedule {
    pub fn new(commands: BTreeMap<u32, Vec<StimulusCommand>>, source: Option<PathBuf>) -> Self {
        Self { commands, source }
    }
    #[allow(dead_code)]
    pub fn load<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let reader = BufReader::new(OpenOptions::new().read(true).open(path.as_ref())?);
        let mut commands: BTreeMap<u32, Vec<StimulusCommand>> = BTreeMap::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let command: StimulusCommand = serde_json::from_str(&line)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            commands.entry(command.step).or_default().push(command);
        }

        Ok(Self {
            commands,
            source: Some(path.as_ref().to_path_buf()),
        })
    }

    #[allow(dead_code)]
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = OpenOptions::new().create(true).write(true).truncate(true).open(path)?;
        for (_, commands_at_step) in &self.commands {
            for command in commands_at_step {
                serde_json::to_writer(&mut file, command)?;
                file.write_all(b"\n")?;
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn take_for_step(&mut self, step: u32) -> Vec<StimulusCommand> {
        self.commands.remove(&step).unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn source(&self) -> Option<&Path> {
        self.source.as_deref()
    }

    #[allow(dead_code)]
    pub fn apply_mutation(&mut self, mutation: &crate::adversarial::Mutation) {
        use crate::adversarial::Mutation;
        match mutation {
            Mutation::IncreaseStimulus { topic, factor } => {
                for (_, commands_at_step) in self.commands.iter_mut() {
                    for cmd in commands_at_step.iter_mut() {
                        if cmd.topic == *topic {
                            cmd.value *= factor;
                        }
                    }
                }
            }
            Mutation::DecreaseStimulus { topic, factor } => {
                for (_, commands_at_step) in self.commands.iter_mut() {
                    for cmd in commands_at_step.iter_mut() {
                        if cmd.topic == *topic {
                            cmd.value /= factor;
                        }
                    }
                }
            }
            Mutation::ChangeEventTiming { event_index, new_step } => {
                let mut all_commands: Vec<StimulusCommand> = Vec::new();
                for (_, commands_at_step) in self.commands.clone().into_iter() { // Use into_iter to consume and move
                    all_commands.extend(commands_at_step);
                }

                if let Some(cmd) = all_commands.get_mut(*event_index) {
                    cmd.step = *new_step;
                }
                
                // Rebuild the BTreeMap
                let mut new_commands: BTreeMap<u32, Vec<StimulusCommand>> = BTreeMap::new();
                for cmd in all_commands {
                    new_commands.entry(cmd.step).or_default().push(cmd);
                }
                self.commands = new_commands;
            }
            _ => {
                // Other mutations are handled by config or other types
            }
        }
    }
}

#[allow(dead_code)]
pub fn append_command<P: AsRef<Path>>(path: P, command: &StimulusCommand) -> io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, command)?;
    file.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{read_to_string, remove_file};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn take_for_step_consumes_commands() {
        let mut schedule = StimulusSchedule {
            commands: BTreeMap::new(),
            source: None,
        };
        let cmd_a = StimulusCommand {
            step: 2,
            topic: "activator".into(),
            value: 0.8,
        };
        let cmd_b = StimulusCommand {
            step: 2,
            topic: "inhibitor".into(),
            value: 0.4,
        };
        schedule
            .commands
            .entry(2)
            .or_default()
            .extend([cmd_a.clone(), cmd_b.clone()]);

        let result = schedule.take_for_step(2);
        assert_eq!(result, vec![cmd_a, cmd_b]);
        assert!(schedule.take_for_step(2).is_empty());
    }

    #[test]
    fn append_command_writes_json_line() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let path = std::env::temp_dir().join(format!("stimulus_test_{timestamp}.jsonl"));
        let command = StimulusCommand {
            step: 5,
            topic: "activator".into(),
            value: 0.9,
        };

        append_command(&path, &command).expect("append should succeed");
        let contents = read_to_string(&path).expect("file should exist");
        assert!(contents.contains("\"step\":5"));
        assert!(contents.contains("\"topic\":\"activator\""));
        assert!(contents.contains("\"value\":0.9"));

        remove_file(&path).ok();
    }
}
