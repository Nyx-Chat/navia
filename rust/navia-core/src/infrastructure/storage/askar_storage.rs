//! Askar-based storage implementation
//!
//! Provides storage capabilities using the Askar database backend.

use crate::askardb::AskarDB;
use crate::core::storage::traits::{MessageStorage, SecretStorage};
use crate::error::{NaviaError, NaviaResult, StorageError};
use async_trait::async_trait;
use std::sync::Arc;

/// Askar-based storage implementation providing encrypted persistence.
///
/// This struct implements both `MessageStorage` and `SecretStorage` traits
/// using Aries Askar as the underlying encrypted database. Askar provides:
///
/// # Features
///
/// - Hardware-backed encryption when available (Android Keystore, iOS Keychain)
/// - Encrypted storage for all data at rest
/// - Key derivation from a primary seed
/// - Support for multiple profiles/wallets
///
/// # Security
///
/// All data stored through this implementation is automatically encrypted
/// using keys derived from the database's pass key. The pass key itself
/// should be protected using platform-specific secure storage.
///
/// # Thread Safety
///
/// This implementation is thread-safe through the use of Arc-wrapped AskarDB.
/// Multiple threads can safely read/write concurrently.
///
/// # Example
///
/// ```ignore
/// let db = AskarDB::provision("./wallet.db", store_key).await?;
/// let storage = AskarStorage::new(Arc::new(db));
///
/// // Use for general storage
/// storage.insert("contacts", "alice", contact_json).await?;
///
/// // Use for secrets
/// storage.store_secret("did:peer:alice#key-1", &private_key).await?;
/// ```
pub struct AskarStorage {
    db: Arc<AskarDB>,
}

impl AskarStorage {
    /// Creates a new Askar storage instance.
    ///
    /// # Arguments
    ///
    /// * `db` - Arc-wrapped AskarDB instance that has been provisioned/opened
    ///
    /// # Returns
    ///
    /// A new `AskarStorage` instance ready for use
    pub fn new(db: Arc<AskarDB>) -> Self {
        Self { db }
    }
}

/// Implementation of general key-value storage using Askar.
///
/// All values are stored as strings and are automatically encrypted
/// by the underlying Askar database.
#[async_trait]
impl MessageStorage for AskarStorage {
    /// Inserts a new value into the encrypted storage.
    ///
    /// The value is automatically encrypted before being persisted.
    async fn insert(&self, category: &str, key: &str, value: &str) -> NaviaResult<()> {
        self.db
            .insert(category, key, &value.to_string())
            .await
            .map_err(|e| {
                NaviaError::from(StorageError::OperationFailed {
                    operation: "insert".to_string(),
                    details: e.to_string(),
                })
            })
    }

    /// Retrieves and decrypts a value from storage.
    ///
    /// Returns `None` if the key doesn't exist in the given category.
    async fn get(&self, category: &str, key: &str) -> NaviaResult<Option<String>> {
        self.db.get(category, key).await.map_err(|e| {
            NaviaError::from(StorageError::OperationFailed {
                operation: "get".to_string(),
                details: e.to_string(),
            })
        })
    }

    /// Updates an existing encrypted value.
    ///
    /// Note: In the current implementation, this behaves the same as insert.
    async fn update(&self, category: &str, key: &str, value: &str) -> NaviaResult<()> {
        self.db
            .update(category, key, &value.to_string())
            .await
            .map_err(|e| {
                NaviaError::Storage(StorageError::OperationFailed {
                    operation: "update".to_string(),
                    details: e.to_string(),
                })
            })
    }

    /// Removes a value from the encrypted storage.
    ///
    /// This operation succeeds even if the key doesn't exist.
    async fn remove(&self, category: &str, key: &str) -> NaviaResult<()> {
        self.db.remove(category, key).await.map_err(|e| {
            NaviaError::Storage(StorageError::OperationFailed {
                operation: "remove".to_string(),
                details: e.to_string(),
            })
        })
    }
}

/// Implementation of secure secret storage using Askar.
///
/// Secrets are stored in a dedicated "secret" category with additional
/// security considerations. The underlying Askar database ensures proper
/// encryption and key management.
#[async_trait]
impl SecretStorage for AskarStorage {
    /// Stores a cryptographic secret in the encrypted database.
    ///
    /// Secrets are stored as raw bytes in the "secret" category.
    /// The Askar database handles encryption transparently.
    ///
    /// # Security Note
    ///
    /// Callers should zero out the secret bytes after calling this method.
    async fn store_secret(&self, id: &str, secret: &[u8]) -> NaviaResult<()> {
        self.db
            .insert_entry("secret", id, secret)
            .await
            .map_err(|e| {
                NaviaError::Storage(StorageError::OperationFailed {
                    operation: "store_secret".to_string(),
                    details: e.to_string(),
                })
            })
    }

    /// Retrieves a cryptographic secret from the encrypted database.
    ///
    /// # Returns
    ///
    /// The secret as raw bytes if found, or `None` if not found.
    ///
    /// # Security Note
    ///
    /// The returned bytes contain sensitive key material and should be
    /// handled appropriately (zeroed after use, not logged, etc.).
    async fn get_secret(&self, id: &str) -> NaviaResult<Option<Vec<u8>>> {
        self.db
            .get_entry("secret", id)
            .await
            .map_err(|e| {
                NaviaError::Storage(StorageError::OperationFailed {
                    operation: "get_secret".to_string(),
                    details: e.to_string(),
                })
            })?
            .map(|entry| Ok(entry.value.to_vec()))
            .transpose()
    }

    /// Permanently deletes a cryptographic secret.
    ///
    /// # Warning
    ///
    /// This operation is irreversible. Any data encrypted with this
    /// secret will become unrecoverable.
    async fn delete_secret(&self, id: &str) -> NaviaResult<()> {
        self.db.remove("secret", id).await.map_err(|e| {
            NaviaError::Storage(StorageError::OperationFailed {
                operation: "delete_secret".to_string(),
                details: e.to_string(),
            })
        })
    }
}
