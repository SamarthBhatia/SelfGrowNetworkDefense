//! Telemetry plumbing for observing morphogenetic dynamics.

use crate::cellular::PopulationStats;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TelemetryEvent {
    Scenario {
        name: String,
    },
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
    CellDied {
        cell_id: String,
    },
    LinkAdded {
        source: String,
        target: String,
    },
    LinkRemoved {
        source: String,
        target: String,
    },
    PeerQuarantined {
        cell_id: String,
        target_id: String,
    },
    AnomalyDetected {
        cell_id: String,
        topic: String,
        confidence: f32,
    },
    VoteCast {
        cell_id: String,
        target_topic: String,
    },
    StepSummary {
        step: u32,
        threat_score: f32,
        cell_count: usize,
        population_stats: Option<PopulationStats>,
        #[serde(default)]
        topology_stats: Option<TopologyStats>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyStats {
    pub avg_degree: f32,
    pub isolation_count: u32,
}

#[allow(dead_code)]
pub trait TelemetrySink {
    fn record(&mut self, timestamp: SystemTime, event: TelemetryEvent);
}

#[derive(Debug, Clone)]
pub struct TelemetrySnapshot {
    pub timestamp: SystemTime,
    pub event: TelemetryEvent,
}

#[allow(dead_code)]
#[derive(Default)]
pub struct InMemorySink {
    events: Vec<TelemetrySnapshot>,
}

impl TelemetrySink for InMemorySink {
    fn record(&mut self, timestamp: SystemTime, event: TelemetryEvent) {
        self.events.push(TelemetrySnapshot { timestamp, event });
    }
}

impl InMemorySink {
    #[allow(dead_code)]
    pub fn events(&self) -> &[TelemetrySnapshot] {
        &self.events
    }

    #[allow(dead_code)]
    pub fn since(&self, duration: Duration) -> Vec<TelemetrySnapshot> {
        let cutoff = SystemTime::now()
            .checked_sub(duration)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        self.events
            .iter()
            .cloned()
            .filter(|snapshot| snapshot.timestamp >= cutoff)
            .collect()
    }
}

#[derive(Serialize)]
struct PersistedRecord {
    timestamp_ms: u128,
    event: TelemetryEvent,
}

fn system_time_to_millis(timestamp: SystemTime) -> u128 {
    timestamp
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[allow(dead_code)]
pub struct JsonlSink {
    writer: BufWriter<File>,
}

impl JsonlSink {
    #[allow(dead_code)]
    pub fn create<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    fn write_record(&mut self, timestamp: SystemTime, event: TelemetryEvent) -> io::Result<()> {
        let record = PersistedRecord {
            timestamp_ms: system_time_to_millis(timestamp),
            event,
        };
        serde_json::to_writer(&mut self.writer, &record)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()
    }
}

impl TelemetrySink for JsonlSink {
    fn record(&mut self, timestamp: SystemTime, event: TelemetryEvent) {
        if let Err(err) = self.write_record(timestamp, event) {
            eprintln!("Failed to write telemetry record: {err}");
        }
    }
}

#[allow(dead_code)]
pub struct TelemetryPipeline {
    memory: InMemorySink,
    file: Option<JsonlSink>,
}

impl TelemetryPipeline {
    #[allow(dead_code)]
    pub fn new(memory: InMemorySink, file: Option<JsonlSink>) -> Self {
        Self { memory, file }
    }

    #[allow(dead_code)]
    pub fn with_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = JsonlSink::create(path)?;
        Ok(Self {
            memory: InMemorySink::default(),
            file: Some(file),
        })
    }

    #[allow(dead_code)]
    pub fn events(&self) -> &[TelemetrySnapshot] {
        self.memory.events()
    }

    #[allow(dead_code)]
    pub fn memory_sink(&self) -> &InMemorySink {
        &self.memory
    }
}

impl TelemetrySink for TelemetryPipeline {
    fn record(&mut self, timestamp: SystemTime, event: TelemetryEvent) {
        let event_for_memory = event.clone();
        self.memory.record(timestamp, event_for_memory);

        if let Some(file) = &mut self.file {
            file.record(timestamp, event);
        }
    }
}
