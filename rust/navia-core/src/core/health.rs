//! Health check functionality for monitoring library status
//!
//! Provides methods to verify that all core components are functioning correctly.

use crate::core::storage::traits::{MessageStorage, SecretStorage};
use crate::error::{NaviaError, NaviaResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Health check status for a component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    /// Component is functioning normally
    Healthy,
    /// Component is experiencing issues but still operational
    Degraded { reason: String },
    /// Component is not functioning
    Unhealthy { reason: String },
}

/// Health check result for a specific component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Name of the component
    pub name: String,
    /// Current status
    pub status: HealthStatus,
    /// Time taken to perform the check
    pub check_duration: Duration,
    /// Additional details about the check
    pub details: Option<String>,
}

/// Overall health check results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Overall system status
    pub status: HealthStatus,
    /// Individual component results
    pub components: Vec<ComponentHealth>,
    /// Total time for all checks
    pub total_duration: Duration,
    /// Timestamp of the check
    pub timestamp: std::time::SystemTime,
}

/// Health checker for the DIDComm library
pub struct HealthChecker<S>
where
    S: MessageStorage + SecretStorage,
{
    storage: Arc<S>,
}

impl<S> HealthChecker<S>
where
    S: MessageStorage + SecretStorage,
{
    /// Creates a new health checker
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Performs a comprehensive health check of all components
    pub async fn check_health(&self) -> NaviaResult<HealthCheckResult> {
        let start = Instant::now();
        let mut components = Vec::new();

        // Check storage health
        components.push(self.check_storage_health().await);

        // Check cryptographic operations
        components.push(self.check_crypto_health().await);

        // Determine overall status
        let overall_status = if components
            .iter()
            .all(|c| matches!(c.status, HealthStatus::Healthy))
        {
            HealthStatus::Healthy
        } else if components
            .iter()
            .any(|c| matches!(c.status, HealthStatus::Unhealthy { .. }))
        {
            HealthStatus::Unhealthy {
                reason: "One or more components are unhealthy".to_string(),
            }
        } else {
            HealthStatus::Degraded {
                reason: "Some components are degraded".to_string(),
            }
        };

        Ok(HealthCheckResult {
            status: overall_status,
            components,
            total_duration: start.elapsed(),
            timestamp: std::time::SystemTime::now(),
        })
    }

    /// Checks storage connectivity and operations
    async fn check_storage_health(&self) -> ComponentHealth {
        let start = Instant::now();
        let test_key = "__health_check_test__";
        let test_value = "test_value";
        let test_category = "__health__";

        // Try to write a test value
        match self
            .storage
            .insert(test_category, test_key, test_value)
            .await
        {
            Ok(_) => {
                // Try to read it back
                match self.storage.get(test_category, test_key).await {
                    Ok(Some(value)) if value == test_value => {
                        // Clean up
                        let _ = self.storage.remove(test_category, test_key).await;

                        ComponentHealth {
                            name: "Storage".to_string(),
                            status: HealthStatus::Healthy,
                            check_duration: start.elapsed(),
                            details: Some("Read/write operations successful".to_string()),
                        }
                    }
                    Ok(Some(_)) => ComponentHealth {
                        name: "Storage".to_string(),
                        status: HealthStatus::Unhealthy {
                            reason: "Data corruption detected".to_string(),
                        },
                        check_duration: start.elapsed(),
                        details: Some("Written value doesn't match read value".to_string()),
                    },
                    Ok(None) => ComponentHealth {
                        name: "Storage".to_string(),
                        status: HealthStatus::Unhealthy {
                            reason: "Write operation failed silently".to_string(),
                        },
                        check_duration: start.elapsed(),
                        details: None,
                    },
                    Err(e) => ComponentHealth {
                        name: "Storage".to_string(),
                        status: HealthStatus::Unhealthy {
                            reason: format!("Read failed: {e}"),
                        },
                        check_duration: start.elapsed(),
                        details: None,
                    },
                }
            }
            Err(e) => ComponentHealth {
                name: "Storage".to_string(),
                status: HealthStatus::Unhealthy {
                    reason: format!("Write failed: {e}"),
                },
                check_duration: start.elapsed(),
                details: None,
            },
        }
    }

    /// Checks cryptographic operations
    async fn check_crypto_health(&self) -> ComponentHealth {
        let start = Instant::now();

        // Try to generate a test key using did_peer
        use did_peer::{DIDPeer, DIDPeerCreateKeys, DIDPeerKeyType, DIDPeerKeys};

        let keys = vec![DIDPeerCreateKeys::new(
            DIDPeerKeys::Verification,
            Some(DIDPeerKeyType::Ed25519),
            None,
        )];

        match DIDPeer::create_peer_did(&keys, None) {
            Ok(_) => ComponentHealth {
                name: "Cryptography".to_string(),
                status: HealthStatus::Healthy,
                check_duration: start.elapsed(),
                details: Some("Key generation successful".to_string()),
            },
            Err(e) => ComponentHealth {
                name: "Cryptography".to_string(),
                status: HealthStatus::Unhealthy {
                    reason: format!("Key generation failed: {e}"),
                },
                check_duration: start.elapsed(),
                details: None,
            },
        }
    }

    /// Quick health check that only verifies storage connectivity
    pub async fn quick_check(&self) -> NaviaResult<bool> {
        let test_key = "__quick_health_check__";
        let test_category = "__health__";

        // Try to read from storage (non-existent key is fine)
        match self.storage.get(test_category, test_key).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// Health check functions for FFI
impl<S> HealthChecker<S>
where
    S: MessageStorage + SecretStorage,
{
    /// Returns a JSON string with health status for FFI
    pub async fn get_health_json(&self) -> NaviaResult<String> {
        let result = self.check_health().await?;
        serde_json::to_string(&result).map_err(|e| {
            NaviaError::Serialization(crate::error::SerializationError::JsonError {
                context: "health check result".to_string(),
                details: e.to_string(),
            })
        })
    }

    /// Simple boolean health check for FFI
    pub async fn is_healthy(&self) -> bool {
        self.quick_check().await.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StorageError;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock storage for testing
    struct MockStorage {
        data: Mutex<HashMap<String, HashMap<String, String>>>,
        fail_mode: Mutex<Option<String>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
                fail_mode: Mutex::new(None),
            }
        }

        fn set_fail_mode(&self, mode: Option<String>) {
            *self.fail_mode.lock().unwrap() = mode;
        }
    }

    #[async_trait]
    impl MessageStorage for MockStorage {
        async fn insert(&self, category: &str, key: &str, value: &str) -> NaviaResult<()> {
            if let Some(ref mode) = *self.fail_mode.lock().unwrap() {
                if mode == "write" {
                    return Err(NaviaError::Storage(StorageError::OperationFailed {
                        operation: "insert".to_string(),
                        details: "Mock failure".to_string(),
                    }));
                }
            }

            let mut data = self.data.lock().unwrap();
            data.entry(category.to_string())
                .or_insert_with(HashMap::new)
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        async fn get(&self, category: &str, key: &str) -> NaviaResult<Option<String>> {
            if let Some(ref mode) = *self.fail_mode.lock().unwrap() {
                if mode == "read" {
                    return Err(NaviaError::Storage(StorageError::OperationFailed {
                        operation: "get".to_string(),
                        details: "Mock failure".to_string(),
                    }));
                }
            }

            let data = self.data.lock().unwrap();
            Ok(data.get(category).and_then(|cat| cat.get(key).cloned()))
        }

        async fn update(&self, category: &str, key: &str, value: &str) -> NaviaResult<()> {
            self.insert(category, key, value).await
        }

        async fn remove(&self, category: &str, key: &str) -> NaviaResult<()> {
            let mut data = self.data.lock().unwrap();
            if let Some(cat) = data.get_mut(category) {
                cat.remove(key);
            }
            Ok(())
        }
    }

    #[async_trait]
    impl SecretStorage for MockStorage {
        async fn store_secret(&self, _id: &str, _secret: &[u8]) -> NaviaResult<()> {
            Ok(())
        }

        async fn get_secret(&self, _id: &str) -> NaviaResult<Option<Vec<u8>>> {
            Ok(None)
        }

        async fn delete_secret(&self, _id: &str) -> NaviaResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_healthy_system() {
        let storage = Arc::new(MockStorage::new());
        let checker = HealthChecker::new(storage);

        let result = checker.check_health().await.unwrap();
        assert!(matches!(result.status, HealthStatus::Healthy));

        // Storage and crypto should be healthy
        let storage_health = result
            .components
            .iter()
            .find(|c| c.name == "Storage")
            .unwrap();
        assert!(matches!(storage_health.status, HealthStatus::Healthy));
    }

    #[tokio::test]
    async fn test_storage_failure() {
        let storage = Arc::new(MockStorage::new());
        storage.set_fail_mode(Some("write".to_string()));

        let checker = HealthChecker::new(storage);
        let result = checker.check_health().await.unwrap();

        assert!(matches!(result.status, HealthStatus::Unhealthy { .. }));

        let storage_health = result
            .components
            .iter()
            .find(|c| c.name == "Storage")
            .unwrap();
        assert!(matches!(
            storage_health.status,
            HealthStatus::Unhealthy { .. }
        ));
    }

    #[tokio::test]
    async fn test_quick_check() {
        let storage = Arc::new(MockStorage::new());
        let checker = HealthChecker::new(storage.clone());

        assert!(checker.quick_check().await.unwrap());

        storage.set_fail_mode(Some("read".to_string()));
        assert!(!checker.quick_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_health_json() {
        let storage = Arc::new(MockStorage::new());
        let checker = HealthChecker::new(storage);

        let json = checker.get_health_json().await.unwrap();
        let parsed: HealthCheckResult = serde_json::from_str(&json).unwrap();

        assert!(matches!(parsed.status, HealthStatus::Healthy));
        assert!(!parsed.components.is_empty());
    }
}
