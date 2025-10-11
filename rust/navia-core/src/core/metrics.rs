//! Performance metrics collection with file-based persistence

use crate::error::{NaviaError, NaviaResult};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

const MAX_ENTRIES: usize = 1000;

/// A single metric entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricEntry {
    pub timestamp: SystemTime,
    pub name: String,
    pub value: MetricValue,
}

/// Metric value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(u64),
    Duration(Duration),
    Error { message: String, operation: String },
}

/// Metrics collector with file-based persistence
pub struct MetricsCollector {
    entries: Arc<RwLock<VecDeque<MetricEntry>>>,
    file_path: Arc<RwLock<Option<PathBuf>>>,
    enabled: Arc<RwLock<bool>>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_ENTRIES))),
            file_path: Arc::new(RwLock::new(None)),
            enabled: Arc::new(RwLock::new(true)),
        }
    }
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize with a log file path
    pub fn init(&self, path: PathBuf) -> NaviaResult<()> {
        // Load existing metrics from a file if it exists
        if path.exists() {
            self.load_from_file(&path)?;
        }

        *self.file_path.write() = Some(path);
        Ok(())
    }

    /// Enable or disable metrics collection
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.write() = enabled;
    }

    /// Check if a metrics collection is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    /// Record a duration
    pub fn record_duration(&self, name: &str, duration: Duration) {
        if !self.is_enabled() {
            return;
        }

        let entry = MetricEntry {
            timestamp: SystemTime::now(),
            name: name.to_string(),
            value: MetricValue::Duration(duration),
        };

        self.append_entry(entry);
    }

    /// Time an operation
    pub fn time_operation<F, R>(&self, name: &str, operation: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = std::time::Instant::now();
        let result = operation();
        self.record_duration(name, start.elapsed());
        result
    }

    /// Log an error with context
    pub fn log_error(&self, operation: &str, error: &str) {
        if !self.is_enabled() {
            return;
        }

        let entry = MetricEntry {
            timestamp: SystemTime::now(),
            name: format!("error.{operation}"),
            value: MetricValue::Error {
                message: error.to_string(),
                operation: operation.to_string(),
            },
        };

        self.append_entry(entry);
    }

    /// Export metrics as JSON
    pub fn export_json(&self) -> NaviaResult<String> {
        let entries = self.entries.read();
        let vec: Vec<MetricEntry> = entries.iter().cloned().collect();
        serde_json::to_string_pretty(&vec)
            .map_err(|e| NaviaError::External(format!("Failed to export metrics: {e}")))
    }

    /// Clear all metrics
    pub fn clear(&self) {
        self.entries.write().clear();

        // Also clear the file
        if let Some(path) = &*self.file_path.read() {
            let _ = std::fs::write(path, "");
        }
    }

    // Private helper methods

    fn append_entry(&self, entry: MetricEntry) {
        let mut entries = self.entries.write();

        // Add a new entry
        entries.push_back(entry.clone());

        // Remove old entries if over limit
        while entries.len() > MAX_ENTRIES {
            entries.pop_front();
        }

        // Write to file
        if let Some(path) = &*self.file_path.read() {
            // Silently ignore file write errors - mobile app should handle its own logging
            let _ = self.append_to_file(path, &entry);
        }
    }

    fn append_to_file(&self, path: &Path, entry: &MetricEntry) -> NaviaResult<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| NaviaError::External(format!("Failed to open metrics file: {e}")))?;

        let line = serde_json::to_string(entry)
            .map_err(|e| NaviaError::External(format!("Failed to serialize metric: {e}")))?;

        writeln!(file, "{line}")
            .map_err(|e| NaviaError::External(format!("Failed to write metric: {e}")))?;

        Ok(())
    }

    fn load_from_file(&self, path: &Path) -> NaviaResult<()> {
        let file = File::open(path)
            .map_err(|e| NaviaError::External(format!("Failed to open metrics file: {e}")))?;
        let reader = BufReader::new(file);

        let mut entries = self.entries.write();
        entries.clear();

        // Read all lines and keep only the last MAX_ENTRIES
        let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

        let start_idx = if lines.len() > MAX_ENTRIES {
            lines.len() - MAX_ENTRIES
        } else {
            0
        };

        for line in &lines[start_idx..] {
            if let Ok(entry) = serde_json::from_str::<MetricEntry>(line) {
                entries.push_back(entry);
            }
        }

        Ok(())
    }
}

// Global metrics instance
lazy_static::lazy_static! {
    pub static ref METRICS: MetricsCollector = MetricsCollector::new();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_metrics_file_persistence() {
        let temp_file = NamedTempFile::new().unwrap();
        let collector = MetricsCollector::new();

        collector.init(temp_file.path().to_path_buf()).unwrap();

        // Add some metrics
        collector.log_error("test.operation", "Test error message");
        collector.record_duration("test.duration", Duration::from_millis(100));

        // Create a new collector and load from a file
        let collector2 = MetricsCollector::new();
        collector2.init(temp_file.path().to_path_buf()).unwrap();

        // Check metrics were loaded
        let entries = collector2.entries.read();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_max_entries_limit() {
        let collector = MetricsCollector::new();

        // Add more than MAX_ENTRIES
        for i in 0..1500 {
            collector.log_error(&format!("test.{i}"), "Error message");
        }

        // Should only keep the last 1000
        let entries = collector.entries.read();
        assert_eq!(entries.len(), MAX_ENTRIES);
    }
}
