//! Inter-cell signaling and coordination abstractions.

use std::collections::VecDeque;
use crate::immune::Attestation;
use serde::{Deserialize, Serialize};

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
            // But usually targeted attacks (like consensus spam) are targeted?
            // "consensus:..." usually has source.
            // If `s.target` is `Some(target_id)`, we remove it.
            // If `s.target` is `None`, we can't remove it from the bus without affecting others.
            // But Graph mode signals are usually routed via `signals_by_source` later.
            // Wait, if I remove it from bus, it's removed for everyone.
            // Immediate mute for Graph mode implies the link is CUT.
            // If the topology is Graph, signals ONLY travel along links.
            // If I cut the link, the signal physically cannot travel.
            // The `step` logic handles this: `if let Some(neighbors) ...`.
            // If I removed the link in `handle_action`, `step` (next tick) won't see the neighbor.
            // So messages are dropped.
            // The review says: "buffered signals... delivered in the same step... because blacklist check...".
            // If the reviewer insists on purging, I will implement it for targeted signals.
            
            if let Some(src) = &s.source {
                if src == source_id {
                    if let Some(tgt) = &s.target {
                        if tgt == target_id {
                            return false; // Drop
                        }
                    }
                }
            }
            true
        });
    }
}
