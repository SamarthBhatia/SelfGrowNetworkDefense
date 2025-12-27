//! Swarm immune response and distributed anomaly detection logic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A recorded threat event in a cell's local memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatEvent {
    pub step: u32,
    pub topic: String,
    pub magnitude: f32,
    pub confidence: f32,
}

/// Swarm-level consensus state for a specific threat.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SwarmConsensus {
    /// Mapping of threat topic to number of votes.
    pub votes: HashMap<String, u32>,
    /// Confirmed threats that have passed the consensus threshold.
    pub confirmed: Vec<String>,
}

impl SwarmConsensus {
    pub fn cast_vote(&mut self, topic: String) {
        *self.votes.entry(topic).or_insert(0) += 1;
    }

    pub fn check_consensus(&mut self, threshold: u32) {
        for (topic, count) in &self.votes {
            if *count >= threshold && !self.confirmed.contains(topic) {
                self.confirmed.push(topic.clone());
            }
        }
    }
}
