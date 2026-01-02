//! Inter-cell signaling and coordination abstractions.

use crate::immune::Attestation;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub topic: String,
    pub value: f32,
    pub source: Option<String>,
    pub target: Option<String>,
    pub attestation: Option<Attestation>,
}

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct SignalBus {
    queue: VecDeque<Signal>,
}

impl SignalBus {
    #[allow(dead_code)]
    pub fn publish(&mut self, signal: Signal) {
        self.queue.push_back(signal);
    }

    #[allow(dead_code)]
    pub fn drain(&mut self) -> Vec<Signal> {
        self.queue.drain(..).collect()
    }

    #[allow(dead_code)]
    pub fn purge_from(&mut self, source_id: &str, _target_id: &str) {
        self.queue.retain(|s| {
            // Drop ALL signals from source_id.
            // In a swarm-wide mute or local disconnection, we want to immediately
            // clear the shared bus of any pending messages (broadcast or targeted)
            // from the quarantined node.
            if let Some(src) = &s.source
                && src == source_id
            {
                return false; // Drop
            }
            true
        });
    }
}
