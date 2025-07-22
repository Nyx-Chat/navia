//! Performance metrics collection with file-based persistence

use crate::error::{NaviaResult, NaviaError};
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

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_ENTRIES))),
            file_path: Arc::new(RwLock::new(None)),
            enabled: Arc::new(RwLock::new(true)),
        }
    }
    
    /// Initialize with a log file path
    pub fn init(&self, path: PathBuf) -> NaviaResult<()> {
        // Load existing metrics from file if it exists
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
    
    /// Check if metrics collection is enabled
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
    
    /// Time an async operation
    pub async fn time_async_operation<F, R>(&self, name: &str, operation: F) -> R
    where
        F: std::future::Future<Output = R>,
    {
        let start = std::time::Instant::now();
        let result = operation.await;
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
            name: format!("error.{}", operation),
            value: MetricValue::Error {
                message: error.to_string(),
                operation: operation.to_string(),
            },
        };
        
        self.append_entry(entry);
    }
    
    /// Get metrics summary
    pub fn get_summary(&self, since: SystemTime) -> MetricsSummary {
        let entries = self.entries.read();
        let mut summary = MetricsSummary {
            start_time: since,
            end_time: SystemTime::now(),
            messages_packed: 0,
            messages_unpacked: 0,
            pack_errors: 0,
            unpack_errors: 0,
            avg_pack_time_ms: 0.0,
            avg_unpack_time_ms: 0.0,
            dids_generated: 0,
            did_resolution_attempts: 0,
            did_resolution_failures: 0,
            storage_reads: 0,
            storage_writes: 0,
            storage_errors: 0,
            avg_storage_read_ms: 0.0,
            avg_storage_write_ms: 0.0,
            total_operations: 0,
            total_errors: 0,
            error_rate: 0.0,
            pack_durations: Vec::new(),
            unpack_durations: Vec::new(),
            storage_read_durations: Vec::new(),
            storage_write_durations: Vec::new(),
        };
        
        // Count metrics since timestamp
        for entry in entries.iter() {
            if entry.timestamp < since {
                continue;
            }
            
            match &entry.value {
                MetricValue::Counter(_) => {
                    match entry.name.as_str() {
                        "message.pack.total" => summary.messages_packed += 1,
                        "message.unpack.total" => summary.messages_unpacked += 1,
                        "message.pack.errors" => summary.pack_errors += 1,
                        "message.unpack.errors" => summary.unpack_errors += 1,
                        "did.generate.total" => summary.dids_generated += 1,
                        "did.resolve.attempts" => summary.did_resolution_attempts += 1,
                        "did.resolve.failures" => summary.did_resolution_failures += 1,
                        "storage.read.total" => summary.storage_reads += 1,
                        "storage.write.total" => summary.storage_writes += 1,
                        "storage.errors" => summary.storage_errors += 1,
                        "operations.total" => summary.total_operations += 1,
                        "errors.total" => summary.total_errors += 1,
                        _ => {}
                    }
                }
                MetricValue::Duration(d) => {
                    let ms = d.as_millis() as f64;
                    match entry.name.as_str() {
                        "message.pack.duration" => {
                            summary.pack_durations.push(ms);
                        }
                        "message.unpack.duration" => {
                            summary.unpack_durations.push(ms);
                        }
                        "storage.read.duration" => {
                            summary.storage_read_durations.push(ms);
                        }
                        "storage.write.duration" => {
                            summary.storage_write_durations.push(ms);
                        }
                        _ => {}
                    }
                }
                MetricValue::Error { .. } => {
                    // Errors are logged for debugging, not counted in summary
                    summary.total_errors += 1;
                }
            }
        }
        
        // Calculate averages
        summary.avg_pack_time_ms = avg(&summary.pack_durations);
        summary.avg_unpack_time_ms = avg(&summary.unpack_durations);
        summary.avg_storage_read_ms = avg(&summary.storage_read_durations);
        summary.avg_storage_write_ms = avg(&summary.storage_write_durations);
        
        // Calculate error rate
        if summary.total_operations > 0 {
            summary.error_rate = (summary.total_errors as f64 / summary.total_operations as f64) * 100.0;
        }
        
        // Clear the duration vectors as they're not needed in output
        summary.pack_durations.clear();
        summary.unpack_durations.clear();
        summary.storage_read_durations.clear();
        summary.storage_write_durations.clear();
        
        summary
    }
    
    /// Export metrics as JSON
    pub fn export_json(&self) -> NaviaResult<String> {
        let entries = self.entries.read();
        let vec: Vec<MetricEntry> = entries.iter().cloned().collect();
        serde_json::to_string_pretty(&vec)
            .map_err(|e| NaviaError::External(format!("Failed to export metrics: {}", e)))
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
        
        // Add new entry
        entries.push_back(entry.clone());
        
        // Remove old entries if over limit
        while entries.len() > MAX_ENTRIES {
            entries.pop_front();
        }
        
        // Write to file
        if let Some(path) = &*self.file_path.read() {
            if let Err(e) = self.append_to_file(path, &entry) {
                log::error!("Failed to write metric to file: {}", e);
            }
        }
    }
    
    fn append_to_file(&self, path: &Path, entry: &MetricEntry) -> NaviaResult<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| NaviaError::External(format!("Failed to open metrics file: {}", e)))?;
            
        let line = serde_json::to_string(entry)
            .map_err(|e| NaviaError::External(format!("Failed to serialize metric: {}", e)))?;
            
        writeln!(file, "{}", line)
            .map_err(|e| NaviaError::External(format!("Failed to write metric: {}", e)))?;
            
        Ok(())
    }
    
    fn load_from_file(&self, path: &Path) -> NaviaResult<()> {
        let file = File::open(path)
            .map_err(|e| NaviaError::External(format!("Failed to open metrics file: {}", e)))?;
        let reader = BufReader::new(file);
        
        let mut entries = self.entries.write();
        entries.clear();
        
        // Read all lines and keep only the last MAX_ENTRIES
        let lines: Vec<String> = reader.lines()
            .filter_map(|line| line.ok())
            .collect();
            
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

/// Metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    
    // Message operations
    pub messages_packed: u64,
    pub messages_unpacked: u64,
    pub pack_errors: u64,
    pub unpack_errors: u64,
    pub avg_pack_time_ms: f64,
    pub avg_unpack_time_ms: f64,
    
    // DID operations
    pub dids_generated: u64,
    pub did_resolution_attempts: u64,
    pub did_resolution_failures: u64,
    
    // Storage operations
    pub storage_reads: u64,
    pub storage_writes: u64,
    pub storage_errors: u64,
    pub avg_storage_read_ms: f64,
    pub avg_storage_write_ms: f64,
    
    // System health
    pub total_operations: u64,
    pub total_errors: u64,
    pub error_rate: f64,
    
    // Internal vectors for calculating averages
    #[serde(skip)]
    pack_durations: Vec<f64>,
    #[serde(skip)]
    unpack_durations: Vec<f64>,
    #[serde(skip)]
    storage_read_durations: Vec<f64>,
    #[serde(skip)]
    storage_write_durations: Vec<f64>,
}

fn avg(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
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
        
        // Create new collector and load from file
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
            collector.log_error(&format!("test.{}", i), "Error message");
        }
        
        // Should only keep last 1000
        let entries = collector.entries.read();
        assert_eq!(entries.len(), MAX_ENTRIES);
    }
}