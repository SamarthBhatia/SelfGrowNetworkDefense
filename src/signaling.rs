//! Inter-cell signaling and coordination abstractions.

use std::collections::VecDeque;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Signal {
    pub topic: String,
    pub value: f32,
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
}
