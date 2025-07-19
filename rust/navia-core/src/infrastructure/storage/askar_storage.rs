//! Askar-based storage implementation
//! 
//! Provides storage capabilities using the Askar database backend.

use crate::core::storage::traits::{MessageStorage, SecretStorage};
use crate::error::core::{CoreError, CoreResult};
use crate::askardb::AskarDB;
use async_trait::async_trait;
use std::sync::Arc;

/// Askar-based storage implementation
pub struct AskarStorage {
    db: Arc<AskarDB>,
}

impl AskarStorage {
    /// Create a new Askar storage instance
    pub fn new(db: Arc<AskarDB>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl MessageStorage for AskarStorage {
    async fn insert(&self, category: &str, key: &str, value: &str) -> CoreResult<()> {
        self.db
            .insert(category, key, &value.to_string())
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))
    }
    
    async fn get(&self, category: &str, key: &str) -> CoreResult<Option<String>> {
        self.db
            .get(category, key)
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))
    }
    
    async fn update(&self, category: &str, key: &str, value: &str) -> CoreResult<()> {
        self.db
            .update(category, key, &value.to_string())
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))
    }
    
    async fn remove(&self, category: &str, key: &str) -> CoreResult<()> {
        self.db
            .remove(category, key)
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))
    }
}

#[async_trait]
impl SecretStorage for AskarStorage {
    async fn store_secret(&self, id: &str, secret: &[u8]) -> CoreResult<()> {
        self.db
            .insert_entry("secret", id, secret)
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))
    }
    
    async fn get_secret(&self, id: &str) -> CoreResult<Option<Vec<u8>>> {
        self.db
            .get_entry("secret", id)
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .map(|entry| Ok(entry.value.to_vec()))
            .transpose()
    }
    
    async fn delete_secret(&self, id: &str) -> CoreResult<()> {
        self.db
            .remove("secret", id)
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))
    }
}