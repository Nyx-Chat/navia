//! Storage trait definitions
//! 
//! These traits define the interface that storage implementations
//! must provide, allowing for different backends to be used.

use async_trait::async_trait;
use crate::error::NaviaResult;

/// Trait for general key-value storage operations.
/// 
/// This trait provides a flexible storage interface for DIDComm-related data
/// such as messages, contacts, settings, and other application data.
/// 
/// # Categories and Keys
/// 
/// Data is organized by category (namespace) and key within that category.
/// This allows for logical grouping and efficient retrieval.
/// 
/// Common categories might include:
/// - "messages" - Stored DIDComm messages
/// - "contacts" - Contact information and DIDs
/// - "settings" - Application preferences
/// - "metadata" - Message metadata and receipts
/// 
/// # Implementation Notes
/// 
/// - Implementations must be thread-safe (Send + Sync)
/// - All operations should be atomic when possible
/// - Keys should be unique within a category
/// - Values are stored as strings (can be JSON for complex data)
/// 
/// # Example
/// 
/// ```ignore
/// // Store a contact
/// storage.insert("contacts", "alice", r#"{"did": "did:peer:alice"}"#).await?;
/// 
/// // Retrieve the contact
/// let contact = storage.get("contacts", "alice").await?;
/// ```
#[async_trait]
pub trait MessageStorage: Send + Sync {
    /// Inserts a new value into storage.
    /// 
    /// # Arguments
    /// 
    /// * `category` - The category/namespace for the data
    /// * `key` - Unique identifier within the category
    /// * `value` - The data to store (as a string)
    /// 
    /// # Errors
    /// 
    /// * `StorageError` - If the storage operation fails
    /// * May fail if the key already exists (implementation-dependent)
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// storage.insert("settings", "theme", "dark").await?;
    /// ```
    async fn insert(&self, category: &str, key: &str, value: &str) -> NaviaResult<()>;
    
    /// Retrieves a value from storage.
    /// 
    /// # Arguments
    /// 
    /// * `category` - The category to search in
    /// * `key` - The key to look up
    /// 
    /// # Returns
    /// 
    /// * `Some(String)` - The stored value if found
    /// * `None` - If the key doesn't exist
    /// 
    /// # Errors
    /// 
    /// * `StorageError` - If the storage operation fails
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// if let Some(theme) = storage.get("settings", "theme").await? {
    ///     println!("Current theme: {}", theme);
    /// }
    /// ```
    async fn get(&self, category: &str, key: &str) -> NaviaResult<Option<String>>;
    
    /// Updates an existing value in storage.
    /// 
    /// # Arguments
    /// 
    /// * `category` - The category containing the data
    /// * `key` - The key to update
    /// * `value` - The new value to store
    /// 
    /// # Errors
    /// 
    /// * `StorageError` - If the storage operation fails
    /// * May fail if the key doesn't exist (implementation-dependent)
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// storage.update("settings", "theme", "light").await?;
    /// ```
    async fn update(&self, category: &str, key: &str, value: &str) -> NaviaResult<()>;
    
    /// Removes a value from storage.
    /// 
    /// # Arguments
    /// 
    /// * `category` - The category containing the data
    /// * `key` - The key to remove
    /// 
    /// # Errors
    /// 
    /// * `StorageError` - If the storage operation fails
    /// 
    /// # Note
    /// 
    /// Most implementations will succeed even if the key doesn't exist.
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// storage.remove("contacts", "alice").await?;
    /// ```
    async fn remove(&self, category: &str, key: &str) -> NaviaResult<()>;
    
    /// Performs multiple insertions in a single operation.
    /// 
    /// This method can be more efficient than multiple individual inserts,
    /// especially for storage backends that support transactions.
    /// 
    /// # Arguments
    /// 
    /// * `category` - The category for all items
    /// * `items` - Slice of (key, value) tuples to insert
    /// 
    /// # Errors
    /// 
    /// * `CoreError::Storage` - If any storage operation fails
    /// 
    /// # Default Implementation
    /// 
    /// The default implementation calls `insert` for each item sequentially.
    /// Storage backends should override this for better performance.
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// let contacts = vec![
    ///     ("alice".to_string(), r#"{"did": "did:peer:alice"}"#.to_string()),
    ///     ("bob".to_string(), r#"{"did": "did:peer:bob"}"#.to_string()),
    /// ];
    /// storage.insert_batch("contacts", &contacts).await?;
    /// ```
    async fn insert_batch(&self, category: &str, items: &[(String, String)]) -> NaviaResult<()> {
        // Default implementation calls insert for each item
        for (key, value) in items {
            self.insert(category, key, value).await?;
        }
        Ok(())
    }
    
    /// Retrieves multiple values in a single operation.
    /// 
    /// This method can be more efficient than multiple individual gets,
    /// especially for storage backends that support batched reads.
    /// 
    /// # Arguments
    /// 
    /// * `category` - The category to search in
    /// * `keys` - Slice of keys to retrieve
    /// 
    /// # Returns
    /// 
    /// A vector of (key, value) tuples where:
    /// - `key` matches the requested key
    /// - `value` is `Some(data)` if found, `None` if not found
    /// 
    /// # Errors
    /// 
    /// * `CoreError::Storage` - If any storage operation fails
    /// 
    /// # Default Implementation
    /// 
    /// The default implementation calls `get` for each key sequentially.
    /// Storage backends should override this for better performance.
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// let keys = vec!["alice".to_string(), "bob".to_string()];
    /// let results = storage.get_batch("contacts", &keys).await?;
    /// for (key, value) in results {
    ///     if let Some(data) = value {
    ///         println!("{}: {}", key, data);
    ///     }
    /// }
    /// ```
    async fn get_batch(&self, category: &str, keys: &[String]) -> NaviaResult<Vec<(String, Option<String>)>> {
        // Default implementation calls get for each key
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            let value = self.get(category, key).await?;
            results.push((key.clone(), value));
        }
        Ok(results)
    }
}

/// Trait for secure storage of cryptographic secrets.
/// 
/// This trait provides specialized storage for sensitive cryptographic materials
/// such as private keys, seeds, and other secrets. Implementations should ensure:
/// 
/// # Security Requirements
/// 
/// - All data is encrypted at rest
/// - Keys are never exposed in logs or error messages  
/// - Memory is properly zeroed after use (where possible)
/// - Access is restricted to authorized operations only
/// 
/// # Storage Format
/// 
/// Secrets are stored as byte arrays to accommodate various key formats.
/// The storage layer doesn't interpret the contents - that's left to the caller.
/// 
/// # Implementation Notes
/// 
/// - Implementations must be thread-safe (Send + Sync)
/// - Consider using hardware security modules (HSM) or secure enclaves
/// - Implement key rotation support where possible
/// - Use constant-time operations for cryptographic materials
/// 
/// # Example
/// 
/// ```ignore
/// // Store a private key
/// let secret = Secret {
///     id: "did:peer:alice#key-1".to_string(),
///     type_: SecretType::JsonWebKey2020,
///     secret_material: SecretMaterial::JWK { /* ... */ },
/// };
/// let secret_bytes = serde_json::to_vec(&secret)?;
/// storage.store_secret(&secret.id, &secret_bytes).await?;
/// ```
#[async_trait]
pub trait SecretStorage: Send + Sync {
    /// Stores a cryptographic secret securely.
    /// 
    /// # Arguments
    /// 
    /// * `id` - Unique identifier for the secret (e.g., "did:peer:alice#key-1")
    /// * `secret` - The secret material as raw bytes
    /// 
    /// # Errors
    /// 
    /// * `StorageError` - If the storage operation fails
    /// * May fail if a secret with the same ID already exists
    /// 
    /// # Security Notes
    /// 
    /// - The secret bytes should be zeroed after calling this method
    /// - Implementations must encrypt the data before persistence
    /// - Consider using key derivation for additional security
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// let private_key = generate_private_key();
    /// storage.store_secret("my-signing-key", &private_key).await?;
    /// // Zero out the key from memory
    /// private_key.zeroize();
    /// ```
    async fn store_secret(&self, id: &str, secret: &[u8]) -> NaviaResult<()>;
    
    /// Retrieves a stored cryptographic secret.
    /// 
    /// # Arguments
    /// 
    /// * `id` - The identifier of the secret to retrieve
    /// 
    /// # Returns
    /// 
    /// * `Some(Vec<u8>)` - The secret material if found
    /// * `None` - If no secret with the given ID exists
    /// 
    /// # Errors
    /// 
    /// * `StorageError` - If the storage operation fails
    /// * `StorageError` - If the secret cannot be decrypted
    /// 
    /// # Security Notes
    /// 
    /// - Callers must properly handle the returned secret bytes
    /// - Consider zeroing the returned data after use
    /// - Avoid logging or displaying secret materials
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// if let Some(key_bytes) = storage.get_secret("my-signing-key").await? {
    ///     let key: PrivateKey = deserialize(&key_bytes)?;
    ///     // Use the key...
    ///     key_bytes.zeroize();
    /// }
    /// ```
    async fn get_secret(&self, id: &str) -> NaviaResult<Option<Vec<u8>>>;
    
    /// Permanently deletes a cryptographic secret.
    /// 
    /// # Arguments
    /// 
    /// * `id` - The identifier of the secret to delete
    /// 
    /// # Errors
    /// 
    /// * `CoreError::Storage` - If the deletion fails
    /// 
    /// # Security Notes
    /// 
    /// - Implementations should ensure secure deletion (overwrite if possible)
    /// - This operation should be irreversible
    /// - Consider keeping audit logs of deletions
    /// 
    /// # Warning
    /// 
    /// Deleting a secret will make any data encrypted with it unrecoverable.
    /// Ensure you have backups or have re-encrypted data before deletion.
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// // Rotate keys by deleting the old one after generating new
    /// storage.store_secret("my-key-v2", &new_key).await?;
    /// storage.delete_secret("my-key-v1").await?;
    /// ```
    async fn delete_secret(&self, id: &str) -> NaviaResult<()>;
}