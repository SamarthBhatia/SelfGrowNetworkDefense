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
    pub fn purge_from(&mut self, source_id: &str, target_id: &str) {
        self.queue.retain(|s| {
            // Mute signals from source_id.
            // If the signal is explicitly targeted at target_id, we MUST drop it.
            // If the signal is a broadcast (target is None), we also drop it for this
            // recipient if purge_from is called in the context of a disconnection.
            //
            // NOTE: Since the SignalBus is shared, dropping a broadcast here affects
            // ALL potential recipients. In a swarm quarantine scenario (consensus),
            // this is the desired behavior (swarm-wide mute). For local disconnects,
            // we rely on receiver-side filtering in MorphogeneticApp::step, but
            // purge_from provides an immediate optimization for the NEXT step's queue.

            if let Some(src) = &s.source
                && src == source_id
                && (s.target.is_none() || s.target.as_deref() == Some(target_id))
            {
                return false; // Drop
            }
            true
        });
    }
}
