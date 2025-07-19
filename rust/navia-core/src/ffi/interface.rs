//! Main FFI interface for DIDComm functionality
//! 
//! This module provides the UniFFI-exposed interface that Kotlin/Swift
//! applications use to interact with the navia library.

use crate::ffi::types::{DIDCommMessage, KeyValue, DidCommError};
use crate::core::didcomm::handler::DidcommMessaging;
use crate::core::didcomm::message::Message;
use crate::core::storage::traits::{MessageStorage, SecretStorage};
use crate::infrastructure::storage::askar_storage::AskarStorage;
use crate::askardb::AskarDB;
use affinidi_did_resolver_cache_sdk::config::ClientConfigBuilder;
use affinidi_did_resolver_cache_sdk::DIDCacheClient;
use askar_storage::generate_raw_store_key;
use std::sync::{Arc, RwLock};
use tokio::runtime::{Runtime, Handle};

#[derive(uniffi::Object)]
pub struct DidComInterface {
    didcomm_messaging: RwLock<Option<Arc<DidcommMessaging<AskarStorage>>>>,
    pub(crate) runtime: Arc<Runtime>,
}

#[uniffi::export]
impl DidComInterface {
    #[uniffi::constructor]
    pub fn new(_path: String) -> Self {
        // Create a dedicated runtime for this instance
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("uniffi-async-runtime")
                .build()
                .expect("Failed to create Tokio runtime for UniFFI async")
        );
        
        Self { 
            didcomm_messaging: RwLock::new(None),
            runtime,
        }
    }

    // Async method for database initialization  
    pub async fn open(&self, path: String, seed: Vec<u8>) -> std::result::Result<(), DidCommError> {
        if self.didcomm_messaging.read().unwrap().is_none() {
            // Check if we're already in a Tokio context
            let (db, client) = if Handle::try_current().is_ok() {
                // We're in a context, proceed normally
                let db = AskarDB::provision(&path, generate_raw_store_key(Some(&seed)).unwrap())
                    .await
                    .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })?;
                    
                let client = DIDCacheClient::new(ClientConfigBuilder::default().build())
                    .await
                    .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?;
                    
                (db, client)
            } else {
                // We need to provide context - use our runtime
                let runtime = self.runtime.clone();
                let path_clone = path.clone();
                let seed_clone = seed.clone();
                
                let handle = runtime.spawn(async move {
                    let db = AskarDB::provision(&path_clone, generate_raw_store_key(Some(&seed_clone)).unwrap()).await?;
                    let client = DIDCacheClient::new(ClientConfigBuilder::default().build()).await
                        .map_err(|e| crate::error::Error::new(crate::error::ErrorKind::InvalidState, e))?;
                    Ok::<_, crate::error::Error>((db, client))
                });
                
                handle.await
                    .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
                    .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })?
            };
            
            let storage = Arc::new(AskarStorage::new(Arc::new(db)));
            let messaging = Arc::new(DidcommMessaging::new(storage, client));
            
            self.didcomm_messaging.write().unwrap().replace(messaging);
        }
        Ok(())
    }

    // Optimized method that returns structured data - no JSON serialization needed!
    pub async fn unpack(&self, msg: String) -> std::result::Result<DIDCommMessage, DidCommError> {
        // Clone the Arc to avoid holding the lock across await
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
        
        // Check if we're already in a Tokio context
        let (message, _metadata) = if Handle::try_current().is_ok() {
            // We're in a context, proceed normally
            messaging
                .unpack_message(&msg)
                .await
                .map_err(|e| DidCommError::UnpackingError { message: e.to_string() })?
        } else {
            // We need to provide context - use our runtime
            let runtime = self.runtime.clone();
            let msg_clone = msg.clone();
            let handle = runtime.spawn(async move {
                messaging.unpack_message(&msg_clone).await
            });
            
            handle.await
                .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
                .map_err(|e| DidCommError::UnpackingError { message: e.to_string() })?
        };
            
        // Convert core Message to FFI DIDCommMessage
        Ok(message.into())
    }

    // Optimized method that accepts structured data - no JSON serialization needed!
    pub async fn pack(&self, msg: DIDCommMessage, from: String, to: String) -> std::result::Result<String, DidCommError> {
        // Convert FFI DIDCommMessage to core Message
        let message: Message = msg.into();
        
        // Clone the Arc to avoid holding the lock across await
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
        
        // Check if we're already in a Tokio context
        if Handle::try_current().is_ok() {
            // We're in a context, proceed normally
            let (packed_message, _metadata) = messaging
                .pack_message(&message, &to, &from)
                .await
                .map_err(|e| DidCommError::PackingError { message: e.to_string() })?;
                
            Ok(packed_message)
        } else {
            // We need to provide context - use our runtime
            let runtime = self.runtime.clone();
            let handle = runtime.spawn(async move {
                messaging.pack_message(&message, &to, &from).await
            });
            
            let result = handle.await
                .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
                .map_err(|e| DidCommError::PackingError { message: e.to_string() })?;
            
            Ok(result.0)
        }
    }

    // Async method for DID generation (involves database operations)
    pub async fn generate_did(&self, uri: String, routing_keys: Vec<String>) -> std::result::Result<String, DidCommError> {
        // Clone the Arc to avoid holding the lock across await
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
        
        // Ensure we have Tokio context for database operations
        if Handle::try_current().is_ok() {
            messaging
                .generate_did(uri, routing_keys)
                .await
                .map_err(|e| DidCommError::DidGenerationError { message: e.to_string() })
        } else {
            let runtime = self.runtime.clone();
            let handle = runtime.spawn(async move {
                messaging.generate_did(uri, routing_keys).await
            });
            
            handle.await
                .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
                .map_err(|e| DidCommError::DidGenerationError { message: e.to_string() })
        }
    }

    // Database operations - these remain largely the same but use the new structure
    pub async fn insert(&self, category: String, name: String, value: String) -> std::result::Result<(), DidCommError> {
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
            
        messaging.storage
            .insert(&category, &name, &value)
            .await
            .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })
    }

    pub async fn get(&self, category: String, name: String) -> std::result::Result<String, DidCommError> {
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
            
        let result = messaging.storage
            .get(&category, &name)
            .await
            .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })?;
            
        Ok(result.unwrap_or_default())
    }

    pub async fn remove(&self, category: String, name: String) -> std::result::Result<(), DidCommError> {
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
            
        messaging.storage
            .remove(&category, &name)
            .await
            .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })
    }

    pub async fn update(&self, category: String, name: String, value: String) -> std::result::Result<(), DidCommError> {
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
            
        messaging.storage
            .update(&category, &name, &value)
            .await
            .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })
    }

    // Batch operations for better performance
    pub async fn insert_batch(&self, category: String, items: Vec<KeyValue>) -> std::result::Result<(), DidCommError> {
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
            
        for item in items {
            messaging.storage
                .insert(&category, &item.key, &item.value)
                .await
                .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })?;
        }
        Ok(())
    }
    
    pub async fn get_batch(&self, category: String, keys: Vec<String>) -> std::result::Result<Vec<KeyValue>, DidCommError> {
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
            
        let mut results = Vec::new();
        for key in keys {
            let value = messaging.storage
                .get(&category, &key)
                .await
                .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })?;
            
            if let Some(val) = value {
                results.push(KeyValue {
                    key: key.clone(),
                    value: val,
                    metadata: None,
                });
            }
        }
        Ok(results)
    }
}