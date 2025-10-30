//! Telemetry plumbing for observing morphogenetic dynamics.

use std::time::{Duration, SystemTime};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    CellReplicated {
        cell_id: String,
        child_id: String,
    },
    LineageShift {
        cell_id: String,
        lineage: String,
    },
    SignalEmitted {
        cell_id: String,
        topic: String,
        value: f32,
    },
}

#[allow(dead_code)]
pub trait TelemetrySink {
    fn record(&mut self, timestamp: SystemTime, event: TelemetryEvent);
}

#[allow(dead_code)]
#[derive(Default)]
pub struct InMemorySink {
    events: Vec<(SystemTime, TelemetryEvent)>,
}

impl TelemetrySink for InMemorySink {
    fn record(&mut self, timestamp: SystemTime, event: TelemetryEvent) {
        self.events.push((timestamp, event));
    }
}

impl InMemorySink {
    #[allow(dead_code)]
    pub fn events(&self) -> &[(SystemTime, TelemetryEvent)] {
        &self.events
    }

    #[allow(dead_code)]
    pub fn since(&self, duration: Duration) -> Vec<(SystemTime, TelemetryEvent)> {
        let cutoff = SystemTime::now()
            .checked_sub(duration)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        self.events
            .iter()
            .cloned()
            .filter(|(ts, _)| *ts >= cutoff)
            .collect()
    }
}
