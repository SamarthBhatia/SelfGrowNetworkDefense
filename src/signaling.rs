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
            // Remove if source matches AND target matches (or is broadcast and we want to block?
            // Blocking broadcast for one recipient in a shared bus is hard without cloning.
            // But if target is explicit, we can remove it.
            // If target is None (broadcast), we can't remove it just for one recipient here.
            // However, the caller (MorphogeneticApp) filters broadcasts based on blacklist in `step()`.
            // The purpose of this `purge_from` is likely to remove pending targeted signals in the queue.
            // If we are just queueing up signals, they aren't delivered yet.
            // When we drain, we deliver. If we drain now, we lose them.
            // But `drain` happens in `step`. `handle_action` happens inside `step` loop?
            // No, `handle_action` is called after collecting all actions.
            // So `step` -> collect actions -> `handle_action` -> `purge_from`.
            // The signals for the *next* step might already be in queue if other cells emitted them?
            // Yes, if other cells processed before this one in the same step.
            // Actually, `step` drains the bus at the START.
            // So the bus is empty when `handle_action` runs, except for signals emitted by other cells in the CURRENT step
            // that are added to the `SignalBus` for the NEXT step.
            // So we are purging signals destined for the NEXT step.

            if let Some(src) = &s.source
                && src == source_id
                && let Some(tgt) = &s.target
                && tgt == target_id
            {
                return false; // Drop
            }
            true
        });
    }
}
