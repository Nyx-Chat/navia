//! Security audit logging
//!
//! Provides structured logging for security-sensitive operations
//! to enable monitoring, compliance, and forensic analysis.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Security event types that should be audited
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEvent {
    /// Database opened with encryption
    DatabaseOpened { path: String, timestamp: SystemTime },
    /// DID generate
    DidGenerated { did: String, timestamp: SystemTime },
    /// Message encrypted
    MessagePacked {
        from: Option<String>,
        to: String,
        message_id: String,
        timestamp: SystemTime,
    },
    /// Message decrypted
    MessageUnpacked {
        from: Option<String>,
        to: Option<String>,
        message_id: String,
        timestamp: SystemTime,
    },
    /// Rate limit exceeded
    RateLimitExceeded {
        operation: String,
        timestamp: SystemTime,
    },
    /// Invalid input rejected
    ValidationFailed {
        input_type: String,
        reason: String,
        timestamp: SystemTime,
    },
    /// Storage access
    StorageAccess {
        operation: StorageOperation,
        category: String,
        key: String,
        timestamp: SystemTime,
    },
    /// Authentication failure
    AuthenticationFailed {
        reason: String,
        timestamp: SystemTime,
    },
}

/// Storage operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageOperation {
    Insert,
    Get,
    Update,
    Remove,
}

/// Audit log entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique ID for this log entry
    pub id: String,
    /// The security event
    pub event: SecurityEvent,
    /// Additional context (e.g., IP address, user agent)
    pub context: Option<String>,
    /// Thread ID that generated this event
    pub thread_id: String,
}

/// Trait for audit log backends
pub trait AuditLogger: Send + Sync {
    /// Log a security event
    fn log(&self, entry: AuditEntry);

    /// Flush any buffered logs
    fn flush(&self);
}

/// Simple in-memory audit logger for testing
pub struct InMemoryAuditLogger {
    entries: Mutex<Vec<AuditEntry>>,
}

impl Default for InMemoryAuditLogger {
    fn default() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }
}

impl InMemoryAuditLogger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_entries(&self) -> Vec<AuditEntry> {
        self.entries.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }
}

impl AuditLogger for InMemoryAuditLogger {
    fn log(&self, entry: AuditEntry) {
        self.entries.lock().unwrap().push(entry);
    }

    fn flush(&self) {
        // No-op for in-memory logger
    }
}

// Implement AuditLogger for Arc<InMemoryAuditLogger> to allow sharing
impl AuditLogger for Arc<InMemoryAuditLogger> {
    fn log(&self, entry: AuditEntry) {
        (**self).log(entry);
    }

    fn flush(&self) {
        (**self).flush();
    }
}

/// Console audit logger that prints to stderr
pub struct ConsoleAuditLogger {
    prefix: String,
}

impl ConsoleAuditLogger {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }
}

impl AuditLogger for ConsoleAuditLogger {
    fn log(&self, entry: AuditEntry) {
        eprintln!("{} [AUDIT] {}: {:?}", self.prefix, entry.id, entry.event);
    }

    fn flush(&self) {
        // Console output is unbuffered
    }
}

/// Global audit logger instance
static AUDIT_LOGGER: once_cell::sync::OnceCell<Arc<dyn AuditLogger>> =
    once_cell::sync::OnceCell::new();

/// Initialize the global audit logger
///
/// This should be called once at application startup
pub fn init_audit_logger(logger: Arc<dyn AuditLogger>) {
    let _ = AUDIT_LOGGER.set(logger);
}

/// Log a security event
///
/// If no logger is initialized, events are silently dropped
pub fn audit_log(event: SecurityEvent, context: Option<String>) {
    if let Some(logger) = AUDIT_LOGGER.get() {
        let entry = AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            event,
            context,
            thread_id: format!("{:?}", std::thread::current().id()),
        };
        logger.log(entry);
    }
}

/// Create an audit context from common request information
pub fn create_context(user_agent: Option<&str>, ip: Option<&str>) -> Option<String> {
    match (user_agent, ip) {
        (Some(ua), Some(ip)) => Some(format!("UA: {ua}, IP: {ip}")),
        (Some(ua), None) => Some(format!("UA: {ua}")),
        (None, Some(ip)) => Some(format!("IP: {ip}")),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_logger() {
        let logger = InMemoryAuditLogger::new();

        let event = SecurityEvent::DidGenerated {
            did: "did:peer:123".to_string(),
            timestamp: SystemTime::now(),
        };

        logger.log(AuditEntry {
            id: "test-1".to_string(),
            event,
            context: None,
            thread_id: format!("{:?}", std::thread::current().id()),
        });

        let entries = logger.get_entries();
        assert_eq!(entries.len(), 1);

        match &entries[0].event {
            SecurityEvent::DidGenerated { did, .. } => {
                assert_eq!(did, "did:peer:123");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_create_context() {
        assert_eq!(
            create_context(Some("Mozilla/5.0"), Some("192.168.1.1")),
            Some("UA: Mozilla/5.0, IP: 192.168.1.1".to_string())
        );

        assert_eq!(
            create_context(Some("Mozilla/5.0"), None),
            Some("UA: Mozilla/5.0".to_string())
        );

        assert_eq!(
            create_context(None, Some("192.168.1.1")),
            Some("IP: 192.168.1.1".to_string())
        );

        assert_eq!(create_context(None, None), None);
    }
}
