use crate::askardb::AskarDB;
use crate::error::{Error, ErrorKind, Result};
use crate::resolvers::did::AskarDIDResolver;
use crate::resolvers::secrets::AskarSecretsResolver;
use affinidi_did_resolver_cache_sdk::config::ClientConfigBuilder;
use affinidi_did_resolver_cache_sdk::DIDCacheClient;
use askar_storage::{generate_raw_store_key, PassKey};
use did_peer::{
    DIDPeer, DIDPeerCreateKeys, DIDPeerKeyType, DIDPeerKeys, DIDPeerService, DIDService,
};
use didcomm::secrets::{Secret, SecretMaterial, SecretType};
use didcomm::{
    Message, PackEncryptedMetadata, PackEncryptedOptions, UnpackMetadata, UnpackOptions,
};
use serde_json::json;
use std::sync::{Arc, RwLock, OnceLock};
use tokio::runtime::{Runtime, Handle};

pub const CATEGORY_SECRET: &str = "secret";

// Global Tokio runtime for Askar database operations only
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("askar-runtime")
            .build()
            .expect("Failed to create Tokio runtime for Askar")
    })
}

// Helper function to run async operations in the runtime context
async fn run_in_runtime<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    future.await
}

// DIDComm message structure for UniFFI
#[derive(uniffi::Record)]
pub struct DIDCommMessage {
    pub id: String,
    pub msg_type: String,
    pub body: String,
    pub from: Option<String>,
    pub to: Vec<String>,
}

// Simple key-value structure for batch operations
#[derive(uniffi::Record)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub metadata: Option<String>,  // For future extensibility
}

// Simplified error type for UniFFI
#[derive(Debug, thiserror::Error)]
#[derive(uniffi::Error)]
#[uniffi(flat_error)]
pub enum DidCommError {
    #[error("Database error: {message}")]
    DatabaseError { message: String },
    #[error("Parsing error: {message}")]
    ParsingError { message: String },
    #[error("DID generation error: {message}")]
    DidGenerationError { message: String },
    #[error("Packing error: {message}")]
    PackingError { message: String },
    #[error("Unpacking error: {message}")]
    UnpackingError { message: String },
    #[error("General error: {message}")]
    GeneralError { message: String },
}

impl From<Error> for DidCommError {
    fn from(err: Error) -> Self {
        match err.kind() {
            ErrorKind::DIDNotResolved | ErrorKind::DIDUrlNotFound => {
                DidCommError::DidGenerationError { message: err.to_string() }
            }
            ErrorKind::Malformed => {
                DidCommError::ParsingError { message: err.to_string() }
            }
            ErrorKind::InvalidState | ErrorKind::IoError => {
                DidCommError::DatabaseError { message: err.to_string() }
            }
            _ => {
                DidCommError::GeneralError { message: err.to_string() }
            }
        }
    }
}

impl From<serde_json::Error> for DidCommError {
    fn from(err: serde_json::Error) -> Self {
        DidCommError::ParsingError { message: err.to_string() }
    }
}

pub struct DidcommMessaging {
    db: Arc<AskarDB>,
    did_resolver: AskarDIDResolver,
    secrets_resolver: AskarSecretsResolver,
}

impl DidcommMessaging {
    fn new(db: Arc<AskarDB>, didcache_client: DIDCacheClient) -> Self {
        Self {
            db: db.clone(),
            did_resolver: AskarDIDResolver::new(didcache_client),
            secrets_resolver: AskarSecretsResolver::new(db.clone()),
        }
    }

    async fn new_local_resolver() -> Result<DIDCacheClient> {
        DIDCacheClient::new(ClientConfigBuilder::default().build())
            .await
            .map_err(|err| err.into())
    }

    pub async fn provision(path: &str, key: PassKey<'_>) -> Result<DidcommMessaging> {
        Ok(DidcommMessaging::new(
            Arc::new(AskarDB::provision(path, key).await?),
            Self::new_local_resolver().await?,
        ))
    }

    pub async fn open(path: &str, key: PassKey<'_>) -> Result<DidcommMessaging> {
        Ok(DidcommMessaging::new(
            Arc::new(AskarDB::open(path, key).await?),
            Self::new_local_resolver().await?,
        ))
    }

    pub async fn add_secret(&self, secret: &Secret) -> Result<()> {
        self.db.insert(CATEGORY_SECRET, &secret.id, &secret).await
    }

    pub async fn add_secrets(&self, secrets: &[Secret]) -> Result<()> {
        for secret in secrets {
            self.add_secret(secret).await?
        }
        Ok(())
    }

    pub async fn pack_encrypted(
        &self,
        msg: &Message,
        to: &str,
        from: Option<&str>,
        sign_by: Option<&str>,
        opts: &PackEncryptedOptions,
    ) -> Result<(String, PackEncryptedMetadata)> {
        msg.pack_encrypted(
            to,
            from,
            sign_by,
            &self.did_resolver,
            &self.secrets_resolver,
            opts,
        )
        .await
        .map_err(|err| Error::new(ErrorKind::InvalidState, err))
    }

    pub async fn pack_message(
        &self,
        msg: &Message,
        to: &str,
        from: &str,
    ) -> Result<(String, PackEncryptedMetadata)> {
        self.pack_encrypted(
            msg,
            to,
            Some(from),
            Some(from),
            &PackEncryptedOptions::default(),
        )
        .await
    }

    pub async fn unpack(
        &self,
        msg: &str,
        opts: &UnpackOptions,
    ) -> Result<(Message, UnpackMetadata)> {
        Message::unpack(msg, &self.did_resolver, &self.secrets_resolver, opts)
            .await
            .map_err(|err| Error::new(ErrorKind::InvalidState, err))
    }

    pub async fn unpack_message(&self, msg: &str) -> Result<(Message, UnpackMetadata)> {
        self.unpack(msg, &UnpackOptions::default()).await
    }

    pub async fn generate_did(&self, uri: String, routing_keys: Vec<String>) -> Result<String> {
        let keys = vec![
            DIDPeerCreateKeys::new(
                DIDPeerKeys::Verification,
                Some(DIDPeerKeyType::Ed25519),
                None,
            ),
            DIDPeerCreateKeys::new(DIDPeerKeys::Encryption, Some(DIDPeerKeyType::P256), None),
        ];
        let services = vec![DIDPeerService::from(DIDService::new(
            uri,
            vec!["didcomm/v2".to_owned()],
            routing_keys,
            None,
        ))];
        let (did, keys) = DIDPeer::create_peer_did(&keys, Some(&services))
            .map_err(|err| Error::new(ErrorKind::Malformed, err))?;
        let mut kid = 0;
        let secrets: Vec<Secret> = keys
            .iter()
            .map(|key| {
                kid += 1;
                Secret {
                    id: format!("{}#key-{}", did, kid),
                    type_: SecretType::JsonWebKey2020,
                    secret_material: SecretMaterial::JWK {
                        private_key_jwk: match key.curve.as_ref() {
                            "Ed25519" => json!({
                                "kty": "OKP",
                                "crv": key.curve,
                                "d": key.d,
                                "x": key.x,
                            }),
                            // P-256
                            _ => json!({
                                "kty": "EC",
                                "crv": key.curve,
                                "d": key.d,
                                "x": key.x,
                                "y": key.y,
                            }),
                        },
                    },
                }
            })
            .collect();

        self.add_secrets(&secrets).await?;

        Ok(did)
    }
}

#[derive(uniffi::Object)]
pub struct DidComInterface {
    didcomm_messaging: RwLock<Option<Arc<DidcommMessaging>>>,
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
            let x = if Handle::try_current().is_ok() {
                // We're in a context, proceed normally
                DidcommMessaging::provision(&path, generate_raw_store_key(Some(&seed)).unwrap())
                    .await
                    .map_err(|e| DidCommError::from(e))?
            } else {
                // We need to provide context - use our runtime
                let runtime = self.runtime.clone();
                let path_clone = path.clone();
                let seed_clone = seed.clone();
                
                let handle = runtime.spawn(async move {
                    DidcommMessaging::provision(&path_clone, generate_raw_store_key(Some(&seed_clone)).unwrap()).await
                });
                
                handle.await
                    .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
                    .map_err(|e| DidCommError::from(e))?
            };
            
            self.didcomm_messaging.write().unwrap().replace(Arc::new(x));
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
            
        // Extract the content from the message body
        let body = message.body.get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        Ok(DIDCommMessage {
            id: message.id,
            msg_type: message.type_,
            body,
            from: message.from.clone(),
            to: message.to.unwrap_or_default(),
        })
    }

    // Optimized method that accepts structured data - no JSON serialization needed!
    pub async fn pack(&self, msg: DIDCommMessage, from: String, to: String) -> std::result::Result<String, DidCommError> {
        // Convert our simple DIDCommMessage to the full didcomm::Message
        // For now, we'll use the 'to' parameter rather than msg.to since DIDComm expects a single recipient
        let message = Message::build(
            msg.id,
            msg.msg_type,
            serde_json::json!({"content": msg.body})
        )
        .to(to.clone())
        .from(msg.from.unwrap_or(from.clone()))
        .finalize();
        
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
                .map_err(|e| e.into())
        } else {
            let runtime = self.runtime.clone();
            let handle = runtime.spawn(async move {
                messaging.generate_did(uri, routing_keys).await
            });
            
            handle.await
                .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
                .map_err(|e| e.into())
        }
    }

    // Async method for database operations
    pub async fn insert(&self, category: String, name: String, value: String) -> std::result::Result<(), DidCommError> {
        // Clone the Arc to avoid holding the lock across await
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
            
        messaging.db
            .insert(&category, &name, &value)
            .await
            .map_err(|e| e.into())
    }

    // Async method for database operations
    pub async fn get(&self, category: String, name: String) -> std::result::Result<String, DidCommError> {
        // Clone the Arc to avoid holding the lock across await
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
            
        let result = messaging.db
            .get(&category, &name)
            .await
            .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })?;
            
        Ok(result.unwrap_or_default())
    }

    // Async method for database operations
    pub async fn remove(&self, category: String, name: String) -> std::result::Result<(), DidCommError> {
        // Clone the Arc to avoid holding the lock across await
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
            
        messaging.db
            .remove(&category, &name)
            .await
            .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })
    }

    // Async method for database operations
    pub async fn update(&self, category: String, name: String, value: String) -> std::result::Result<(), DidCommError> {
        // Clone the Arc to avoid holding the lock across await
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
            
        messaging.db
            .update(&category, &name, &value)
            .await
            .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })
    }

    // Batch operations for better performance
    // Async batch operations for better performance
    pub async fn insert_batch(&self, category: String, items: Vec<KeyValue>) -> std::result::Result<(), DidCommError> {
        // Clone the Arc to avoid holding the lock across await
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
            
        for item in items {
            messaging.db
                .insert(&category, &item.key, &item.value)
                .await
                .map_err(|e| DidCommError::from(e))?;
        }
        Ok(())
    }
    
    pub async fn get_batch(&self, category: String, keys: Vec<String>) -> std::result::Result<Vec<KeyValue>, DidCommError> {
        // Clone the Arc to avoid holding the lock across await
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
            
        let mut results = Vec::new();
        for key in keys {
            let value = messaging.db
                .get(&category, &key)
                .await
                .map_err(|e| DidCommError::from(e))?;
            
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



