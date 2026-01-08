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
    TrustScoreUpdated {
        cell_id: String,
        target_id: String,
        new_score: f32,
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
            .filter(|snapshot| snapshot.timestamp >= cutoff)
            .cloned()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read_to_string;
    use tempfile::NamedTempFile;

    #[test]
    fn test_jsonl_persistence() {
        let tmp_file = NamedTempFile::new().expect("temp file");
        let path = tmp_file.path().to_owned();
        
        // Re-open in sink (append mode)
        let mut sink = JsonlSink::create(&path).expect("create sink");

        let event1 = TelemetryEvent::Scenario { name: "test".into() };
        let event2 = TelemetryEvent::StepSummary { 
            step: 1, 
            threat_score: 0.5, 
            cell_count: 10, 
            population_stats: None,
            topology_stats: Some(TopologyStats { avg_degree: 2.5, isolation_count: 1 }),
        };

        sink.record(SystemTime::now(), event1);
        sink.record(SystemTime::now(), event2);

        // Read back
        let contents = read_to_string(&path).expect("read file");
        let lines: Vec<&str> = contents.trim().split('\n').collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"Scenario\""));
        assert!(lines[1].contains("\"StepSummary\""));
        assert!(lines[1].contains("\"avg_degree\":2.5"));
    }

    #[test]
    fn test_topology_stats_aggregation() {
        let mut sink = InMemorySink::default();
        let stats = TopologyStats { avg_degree: 3.0, isolation_count: 5 };
        let event = TelemetryEvent::StepSummary {
            step: 10,
            threat_score: 0.1,
            cell_count: 100,
            population_stats: None,
            topology_stats: Some(stats),
        };
        
        sink.record(SystemTime::now(), event);
        
        let snapshots = sink.events();
        assert_eq!(snapshots.len(), 1);
        if let TelemetryEvent::StepSummary { topology_stats, .. } = &snapshots[0].event {
            let ts = topology_stats.as_ref().unwrap();
            assert_eq!(ts.avg_degree, 3.0);
            assert_eq!(ts.isolation_count, 5);
        } else {
            panic!("Wrong event type");
        }
    }
    
    #[test]
    fn test_persistence_error_handling() {
        // Try to create a sink on a directory path, which should fail or fail to write
        let tmp_dir = tempfile::tempdir().expect("temp dir");
        let path = tmp_dir.path().to_owned(); // This is a directory
        
        // On some OS, opening a dir with OpenOptions might fail or succeed. 
        // If create succeeds (unlikely for directory), write should fail.
        
        if let Ok(mut sink) = JsonlSink::create(&path) {
            // If we somehow opened it, writing should definitely fail or be handled
            // TelemetrySink::record swallows errors to stderr. We can't easily assert stderr.
            // But we can check if it panics (it shouldn't).
            sink.record(SystemTime::now(), TelemetryEvent::CellDied { cell_id: "A".into() });
        } else {
            // Expected failure to create sink on directory
        }
    }
}
