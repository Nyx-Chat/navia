//! Storage trait definitions
//! 
//! These traits define the interface that storage implementations
//! must provide, allowing for different backends to be used.

use async_trait::async_trait;
use crate::error::core::CoreResult;

/// Trait for key-value storage operations
#[async_trait]
pub trait MessageStorage: Send + Sync {
    /// Insert a value with the given category and key
    async fn insert(&self, category: &str, key: &str, value: &str) -> CoreResult<()>;
    
    /// Get a value by category and key
    async fn get(&self, category: &str, key: &str) -> CoreResult<Option<String>>;
    
    /// Update an existing value
    async fn update(&self, category: &str, key: &str, value: &str) -> CoreResult<()>;
    
    /// Remove a value
    async fn remove(&self, category: &str, key: &str) -> CoreResult<()>;
    
    /// Batch insert operation
    async fn insert_batch(&self, category: &str, items: &[(String, String)]) -> CoreResult<()> {
        // Default implementation calls insert for each item
        for (key, value) in items {
            self.insert(category, key, value).await?;
        }
        Ok(())
    }
    
    /// Batch get operation
    async fn get_batch(&self, category: &str, keys: &[String]) -> CoreResult<Vec<(String, Option<String>)>> {
        // Default implementation calls get for each key
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            let value = self.get(category, key).await?;
            results.push((key.clone(), value));
        }
        Ok(results)
    }
}

/// Trait for cryptographic operations and secret storage
#[async_trait]
pub trait SecretStorage: Send + Sync {
    /// Store a secret
    async fn store_secret(&self, id: &str, secret: &[u8]) -> CoreResult<()>;
    
    /// Retrieve a secret
    async fn get_secret(&self, id: &str) -> CoreResult<Option<Vec<u8>>>;
    
    /// Delete a secret
    async fn delete_secret(&self, id: &str) -> CoreResult<()>;
}