//! Main FFI interface for DIDComm functionality
//! 
//! This module provides the UniFFI-exposed interface that Kotlin/Swift
//! applications use to interact with the navia library.

use crate::ffi::types::{DIDCommMessage, KeyValue, DidCommError};
use crate::core::audit::{audit_log, SecurityEvent, StorageOperation};
use crate::core::didcomm::handler::DidcommMessaging;
use crate::core::didcomm::message::Message;
use crate::core::storage::traits::{MessageStorage};
use crate::core::validation::{validate_seed, validate_storage_category, validate_storage_key, validate_message_body};
use crate::infrastructure::storage::askar_storage::AskarStorage;
use crate::askardb::AskarDB;
use affinidi_did_resolver_cache_sdk::config::ClientConfigBuilder;
use affinidi_did_resolver_cache_sdk::DIDCacheClient;
use askar_storage::generate_raw_store_key;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use tokio::runtime::{Runtime, Handle};
use zeroize::Zeroize;

/// DIDComm messaging interface for secure peer-to-peer communication.
/// 
/// `DidComInterface` provides a high-level API for DIDComm v2 messaging operations,
/// including message encryption/decryption, DID generation, and secure storage.
/// This is the main entry point for FFI consumers (Kotlin/Swift).
/// 
/// # Architecture
/// 
/// The interface maintains an internal `DidcommMessaging` instance that handles
/// the actual DIDComm protocol operations. All operations are thread-safe through
/// the use of `RwLock`.
/// 
/// # Examples
/// 
/// ```ignore
/// # use crate::ffi::DidComInterface;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create and initialize the interface
/// let interface = DidComInterface::new("/path/to/db".to_string());
/// let seed = vec![0u8; 32]; // Use a secure seed in production
/// interface.open("/path/to/db".to_string(), seed).await?;
/// 
/// // Generate a DID
/// let my_did = interface.generate_did(
///     "https://example.com/didcomm".to_string(),
///     vec![] // No routing keys
/// ).await?;
/// 
/// // Pack a message
/// let message = DidCommMessage {
///     id: "unique-id".to_string(),
///     msg_type: "https://example.org/protocols/basicmessage/2.0/message".to_string(),
///     body: r#"{"content": "Hello, World!"}"#.to_string(),
///     from: Some(my_did.clone()),
///     to: vec!["did:peer:recipient".to_string()],
/// };
/// 
/// let packed = interface.pack(
///     message,
///     my_did,
///     "did:peer:recipient".to_string()
/// ).await?;
/// # Ok(())
/// # }
/// ```
#[derive(uniffi::Object)]
pub struct DidComInterface {
    /// Thread-safe storage for the DIDComm messaging handler
    didcomm_messaging: RwLock<Option<Arc<DidcommMessaging<AskarStorage>>>>,
    /// Dedicated Tokio runtime for async operations
    pub(crate) runtime: Arc<Runtime>,
}

#[uniffi::export]
impl DidComInterface {
    /// Creates a new `DidComInterface` instance.
    /// 
    /// # Arguments
    /// 
    /// * `_path` - Database path (currently unused, kept for API compatibility)
    /// 
    /// # Returns
    /// 
    /// A new instance with an uninitialized messaging handler. You must call
    /// [`open`](#method.open) before using any other methods.
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// # use crate::ffi::DidComInterface;
    /// let interface = DidComInterface::new("/path/to/db".to_string());
    /// ```
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

    /// Initializes the DIDComm interface with a secure database.
    /// 
    /// This method must be called before any other operations. It sets up:
    /// - An encrypted Askar database for storing DIDs and keys
    /// - A DID resolver client for looking up external DIDs
    /// - The internal messaging handler
    /// 
    /// # Arguments
    /// 
    /// * `path` - File path where the encrypted database will be stored
    /// * `seed` - A 32-byte seed for database encryption. This should be:
    ///   - Cryptographically secure (e.g., from Android Keystore)
    ///   - Consistent across app launches
    ///   - Unique per installation
    /// 
    /// # Errors
    /// 
    /// Returns `DidCommError::DatabaseError` if:
    /// - Database creation fails
    /// - Invalid seed is provided
    /// - File system permissions are insufficient
    /// 
    /// Returns `DidCommError::GeneralError` if:
    /// - DID resolver client initialization fails
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// # use crate::ffi::DidComInterface;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let interface = DidComInterface::new("/data/navia.db".to_string());
    /// let seed = generate_secure_seed(); // Your secure seed generation
    /// interface.open("/data/navia.db".to_string(), seed).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open(&self, path: String, mut seed: Vec<u8>) -> std::result::Result<(), DidCommError> {
        // Validate seed before use
        validate_seed(&seed)
            .map_err(|e| DidCommError::ValidationError { message: e.to_string() })?;
            
        if self.didcomm_messaging.read().unwrap().is_none() {
            // Generate the store key before any async operations
            let store_key = generate_raw_store_key(Some(&seed)).unwrap();
            
            // Zeroize the seed immediately after use
            seed.zeroize();
            
            // Check if we're already in a Tokio context
            let (db, client) = if Handle::try_current().is_ok() {
                // We're in a context, proceed normally
                let db = AskarDB::provision(&path, store_key)
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
                
                let handle = runtime.spawn(async move {
                    let db = AskarDB::provision(&path_clone, store_key).await
                        .map_err(|e| crate::error::NaviaError::External(e.to_string()))?;
                    let client = DIDCacheClient::new(ClientConfigBuilder::default().build()).await
                        .map_err(|e| crate::error::NaviaError::External(e.to_string()))?;
                    Ok::<_, crate::error::NaviaError>((db, client))
                });
                
                handle.await
                    .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
                    .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })?
            };
            
            let storage = Arc::new(AskarStorage::new(Arc::new(db)));
            let messaging = Arc::new(DidcommMessaging::new(storage, client));
            
            self.didcomm_messaging.write().unwrap().replace(messaging);
            
            // Audit log database opening
            audit_log(
                SecurityEvent::DatabaseOpened {
                    path: path.clone(),
                    timestamp: SystemTime::now(),
                },
                None,
            );
        }
        Ok(())
    }

    /// Unpacks (decrypts) a DIDComm encrypted message.
    /// 
    /// This method handles the full DIDComm unpacking process:
    /// 1. Decrypts the message envelope
    /// 2. Verifies signatures (if present)
    /// 3. Resolves sender DID
    /// 4. Returns structured message data
    /// 
    /// # Arguments
    /// 
    /// * `msg` - The encrypted DIDComm message as a JSON string
    /// 
    /// # Returns
    /// 
    /// A `DIDCommMessage` containing:
    /// - `id`: Message identifier
    /// - `msg_type`: Protocol message type  
    /// - `body`: Message body (as JSON string)
    /// - `from`: Sender DID (if authenticated)
    /// - `to`: List of recipient DIDs
    /// 
    /// # Errors
    /// 
    /// Returns `DidCommError::UnpackingError` if:
    /// - Message is malformed or corrupted
    /// - Decryption fails (wrong recipient)
    /// - Signature verification fails
    /// - Required keys are not found in storage
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// # use crate::ffi::DidComInterface;
    /// # async fn example(interface: &DidComInterface) -> Result<(), Box<dyn std::error::Error>> {
    /// let encrypted_msg = receive_message(); // Your message receiving logic
    /// let message = interface.unpack(encrypted_msg).await?;
    /// 
    /// println!("Received message type: {}", message.msg_type);
    /// println!("From: {:?}", message.from);
    /// println!("Body: {}", message.body);
    /// # Ok(())
    /// # }
    /// ```
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

    /// Packs (encrypts) a message using DIDComm encryption.
    /// 
    /// Creates an encrypted DIDComm message that can only be decrypted by the
    /// intended recipient. The message is also signed by the sender for
    /// authentication.
    /// 
    /// # Arguments
    /// 
    /// * `msg` - The message to encrypt, containing:
    ///   - `id`: Unique message identifier
    ///   - `msg_type`: Protocol identifier (e.g., "https://example.org/protocols/1.0/message")
    ///   - `body`: Message content as JSON string
    ///   - `from`: Should match the `from` parameter
    ///   - `to`: Should contain the `to` parameter
    /// * `from` - Sender's DID (must have keys in storage)
    /// * `to` - Recipient's DID (will be resolved to fetch encryption keys)
    /// 
    /// # Returns
    /// 
    /// An encrypted DIDComm message as a JSON string, ready for transmission.
    /// 
    /// # Errors
    /// 
    /// Returns `DidCommError::PackingError` if:
    /// - Sender keys not found in storage
    /// - Recipient DID cannot be resolved
    /// - Encryption fails
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// # use crate::ffi::{DidComInterface, DIDCommMessage};
    /// # async fn example(interface: &DidComInterface) -> Result<(), Box<dyn std::error::Error>> {
    /// let message = DIDCommMessage {
    ///     id: "msg-123".to_string(),
    ///     msg_type: "https://didcomm.org/basicmessage/2.0/message".to_string(),
    ///     body: r#"{"content": "Hello!"}"#.to_string(),
    ///     from: Some("did:peer:my-did".to_string()),
    ///     to: vec!["did:peer:their-did".to_string()],
    /// };
    /// 
    /// let encrypted = interface.pack(
    ///     message,
    ///     "did:peer:my-did".to_string(),
    ///     "did:peer:their-did".to_string()
    /// ).await?;
    /// 
    /// // Send encrypted message over transport
    /// send_message(encrypted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pack(&self, msg: DIDCommMessage, from: String, to: String) -> std::result::Result<String, DidCommError> {
        // Validate message body size
        validate_message_body(&msg.body)
            .map_err(|e| DidCommError::ValidationError { message: e.to_string() })?;
            
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

    /// Generates a new peer DID with associated encryption and signing keys.
    /// 
    /// Creates a DID using the did:peer method, which includes:
    /// - An Ed25519 key for signing (authentication)
    /// - A P-256 key for encryption
    /// - Service endpoint for message routing
    /// 
    /// The generated keys are automatically stored in the encrypted database.
    /// 
    /// # Arguments
    /// 
    /// * `uri` - Service endpoint URI where this DID can receive messages
    ///           (e.g., "https://example.com/didcomm")
    /// * `routing_keys` - Optional mediator DIDs for message forwarding.
    ///                    Use empty vector for direct messaging.
    /// 
    /// # Returns
    /// 
    /// The generated DID string (e.g., "did:peer:1zQm...")
    /// 
    /// # Errors
    /// 
    /// Returns `DidCommError::DidGenerationError` if:
    /// - Key generation fails
    /// - Database storage fails
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// # use crate::ffi::DidComInterface;
    /// # async fn example(interface: &DidComInterface) -> Result<(), Box<dyn std::error::Error>> {
    /// // Generate DID for direct messaging
    /// let my_did = interface.generate_did(
    ///     "https://myserver.com/didcomm".to_string(),
    ///     vec![]
    /// ).await?;
    /// 
    /// // Generate DID with mediator routing
    /// let routed_did = interface.generate_did(
    ///     "https://mediator.com/forward".to_string(),
    ///     vec!["did:peer:mediator123".to_string()]
    /// ).await?;
    /// 
    /// println!("Generated DID: {}", my_did);
    /// # Ok(())
    /// # }
    /// ```
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

    /// Stores a value in the encrypted database.
    /// 
    /// Data is organized by category and name for efficient retrieval.
    /// All data is encrypted at rest using the database seed.
    /// 
    /// # Arguments
    /// 
    /// * `category` - Category for grouping related data (e.g., "contacts", "settings")
    /// * `name` - Unique identifier within the category
    /// * `value` - Data to store (any string value)
    /// 
    /// # Errors
    /// 
    /// Returns `DidCommError::DatabaseError` if storage fails
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// # use crate::ffi::DidComInterface;
    /// # async fn example(interface: &DidComInterface) -> Result<(), Box<dyn std::error::Error>> {
    /// // Store user preferences
    /// interface.insert(
    ///     "settings".to_string(),
    ///     "theme".to_string(),
    ///     "dark".to_string()
    /// ).await?;
    /// 
    /// // Store contact information
    /// interface.insert(
    ///     "contacts".to_string(),
    ///     "alice".to_string(),
    ///     r#"{"did": "did:peer:alice123", "name": "Alice"}"#.to_string()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn insert(&self, category: String, name: String, value: String) -> std::result::Result<(), DidCommError> {
        // Validate inputs
        validate_storage_category(&category)
            .map_err(|e| DidCommError::ValidationError { message: e.to_string() })?;
        validate_storage_key(&name)
            .map_err(|e| DidCommError::ValidationError { message: e.to_string() })?;
            
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
        
        // Audit log the storage operation
        audit_log(
            SecurityEvent::StorageAccess {
                operation: StorageOperation::Insert,
                category: category.clone(),
                key: name.clone(),
                timestamp: SystemTime::now(),
            },
            None,
        );
        
        // Check if we're already in a Tokio context
        if Handle::try_current().is_ok() {
            messaging.storage
                .insert(&category, &name, &value)
                .await
                .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })
        } else {
            // Use our runtime
            let runtime = self.runtime.clone();
            let handle = runtime.spawn(async move {
                messaging.storage
                    .insert(&category, &name, &value)
                    .await
            });
            
            handle.await
                .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
                .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })
        }
    }

    /// Retrieves a value from the encrypted database.
    /// 
    /// # Arguments
    /// 
    /// * `category` - Category where the data is stored
    /// * `name` - Identifier of the data to retrieve
    /// 
    /// # Returns
    /// 
    /// The stored value, or empty string if not found
    /// 
    /// # Errors
    /// 
    /// Returns `DidCommError::DatabaseError` if retrieval fails
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// # use crate::ffi::DidComInterface;
    /// # async fn example(interface: &DidComInterface) -> Result<(), Box<dyn std::error::Error>> {
    /// let theme = interface.get(
    ///     "settings".to_string(),
    ///     "theme".to_string()
    /// ).await?;
    /// 
    /// if !theme.is_empty() {
    ///     println!("Current theme: {}", theme);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, category: String, name: String) -> std::result::Result<String, DidCommError> {
        // Validate inputs
        validate_storage_category(&category)
            .map_err(|e| DidCommError::ValidationError { message: e.to_string() })?;
        validate_storage_key(&name)
            .map_err(|e| DidCommError::ValidationError { message: e.to_string() })?;
            
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
        
        // Check if we're already in a Tokio context
        if Handle::try_current().is_ok() {
            let result = messaging.storage
                .get(&category, &name)
                .await
                .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })?;
            Ok(result.unwrap_or_default())
        } else {
            // Use our runtime
            let runtime = self.runtime.clone();
            let handle = runtime.spawn(async move {
                messaging.storage.get(&category, &name).await
            });
            
            let result = handle.await
                .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
                .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })?;
            Ok(result.unwrap_or_default())
        }
    }

    /// Removes a value from the encrypted database.
    /// 
    /// # Arguments
    /// 
    /// * `category` - Category containing the data
    /// * `name` - Identifier of the data to remove
    /// 
    /// # Errors
    /// 
    /// Returns `DidCommError::DatabaseError` if removal fails
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// # use crate::ffi::DidComInterface;
    /// # async fn example(interface: &DidComInterface) -> Result<(), Box<dyn std::error::Error>> {
    /// interface.remove(
    ///     "contacts".to_string(),
    ///     "alice".to_string()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remove(&self, category: String, name: String) -> std::result::Result<(), DidCommError> {
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
        
        // Check if we're already in a Tokio context
        if Handle::try_current().is_ok() {
            messaging.storage
                .remove(&category, &name)
                .await
                .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })
        } else {
            // Use our runtime
            let runtime = self.runtime.clone();
            let handle = runtime.spawn(async move {
                messaging.storage
                    .remove(&category, &name)
                    .await
            });
            
            handle.await
                .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
                .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })
        }
    }

    /// Updates an existing value in the encrypted database.
    /// 
    /// # Arguments
    /// 
    /// * `category` - Category containing the data
    /// * `name` - Identifier of the data to update
    /// * `value` - New value to store
    /// 
    /// # Errors
    /// 
    /// Returns `DidCommError::DatabaseError` if update fails
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// # use crate::ffi::DidComInterface;
    /// # async fn example(interface: &DidComInterface) -> Result<(), Box<dyn std::error::Error>> {
    /// interface.update(
    ///     "settings".to_string(),
    ///     "theme".to_string(),
    ///     "light".to_string()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(&self, category: String, name: String, value: String) -> std::result::Result<(), DidCommError> {
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
        
        // Check if we're already in a Tokio context
        if Handle::try_current().is_ok() {
            messaging.storage
                .update(&category, &name, &value)
                .await
                .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })
        } else {
            // Use our runtime
            let runtime = self.runtime.clone();
            let handle = runtime.spawn(async move {
                messaging.storage
                    .update(&category, &name, &value)
                    .await
            });
            
            handle.await
                .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
                .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })
        }
    }

    /// Inserts multiple values in a single transaction for better performance.
    /// 
    /// Use this method when you need to store multiple related items to avoid
    /// the overhead of individual transactions.
    /// 
    /// # Arguments
    /// 
    /// * `category` - Category for all items
    /// * `items` - Vector of key-value pairs to insert
    /// 
    /// # Errors
    /// 
    /// Returns `DidCommError::DatabaseError` if any insertion fails.
    /// Note: Insertions are not atomic - some items may be inserted before failure.
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// # use crate::ffi::{DidComInterface, KeyValue};
    /// # async fn example(interface: &DidComInterface) -> Result<(), Box<dyn std::error::Error>> {
    /// let contacts = vec![
    ///     KeyValue {
    ///         key: "alice".to_string(),
    ///         value: r#"{"did": "did:peer:alice"}"#.to_string(),
    ///         metadata: None,
    ///     },
    ///     KeyValue {
    ///         key: "bob".to_string(),
    ///         value: r#"{"did": "did:peer:bob"}"#.to_string(),
    ///         metadata: None,
    ///     },
    /// ];
    /// 
    /// interface.insert_batch("contacts".to_string(), contacts).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn insert_batch(&self, category: String, items: Vec<KeyValue>) -> std::result::Result<(), DidCommError> {
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
        
        // Check if we're already in a Tokio context
        if Handle::try_current().is_ok() {
            for item in items {
                messaging.storage
                    .insert(&category, &item.key, &item.value)
                    .await
                    .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })?;
            }
            Ok(())
        } else {
            // Use our runtime
            let runtime = self.runtime.clone();
            let handle = runtime.spawn(async move {
                for item in items {
                    messaging.storage
                        .insert(&category, &item.key, &item.value)
                        .await
                        .map_err(|e| DidCommError::DatabaseError { message: e.to_string() })?;
                }
                Ok::<(), DidCommError>(())
            });
            
            handle.await
                .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
        }
    }
    
    /// Retrieves multiple values in a single operation for better performance.
    /// 
    /// # Arguments
    /// 
    /// * `category` - Category to search in
    /// * `keys` - List of identifiers to retrieve
    /// 
    /// # Returns
    /// 
    /// Vector of `KeyValue` items for keys that were found.
    /// Missing keys are silently skipped.
    /// 
    /// # Errors
    /// 
    /// Returns `DidCommError::DatabaseError` if retrieval fails
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// # use crate::ffi::DidComInterface;
    /// # async fn example(interface: &DidComInterface) -> Result<(), Box<dyn std::error::Error>> {
    /// let keys = vec!["alice".to_string(), "bob".to_string(), "charlie".to_string()];
    /// let contacts = interface.get_batch("contacts".to_string(), keys).await?;
    /// 
    /// for contact in contacts {
    ///     println!("Found contact {}: {}", contact.key, contact.value);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_batch(&self, category: String, keys: Vec<String>) -> std::result::Result<Vec<KeyValue>, DidCommError> {
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Not initialized".to_string() })?
                .clone()
        };
        
        // Check if we're already in a Tokio context
        if Handle::try_current().is_ok() {
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
        } else {
            // Use our runtime
            let runtime = self.runtime.clone();
            let handle = runtime.spawn(async move {
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
                Ok::<Vec<KeyValue>, DidCommError>(results)
            });
            
            handle.await
                .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
        }
    }

    /// Performs a comprehensive health check of all library components.
    /// 
    /// This method checks:
    /// - Storage connectivity and operations
    /// - Cryptographic functionality 
    /// - Rate limiter status
    /// 
    /// # Returns
    /// 
    /// A JSON string containing the health check results with:
    /// - Overall status (Healthy, Degraded, or Unhealthy)
    /// - Individual component statuses
    /// - Check duration and timestamp
    /// 
    /// # Errors
    /// 
    /// Returns `DidCommError::GeneralError` if health check fails
    /// 
    /// # Example Output
    /// 
    /// ```json
    /// {
    ///   "status": "Healthy",
    ///   "components": [
    ///     {
    ///       "name": "Storage",
    ///       "status": "Healthy",
    ///       "check_duration": { "secs": 0, "nanos": 1234567 },
    ///       "details": "Read/write operations successful"
    ///     },
    ///     {
    ///       "name": "Cryptography",
    ///       "status": "Healthy",
    ///       "check_duration": { "secs": 0, "nanos": 2345678 },
    ///       "details": "Key generation successful"
    ///     }
    ///   ],
    ///   "total_duration": { "secs": 0, "nanos": 3456789 },
    ///   "timestamp": { "secs_since_epoch": 1234567890, "nanos_since_epoch": 0 }
    /// }
    /// ```
    pub async fn check_health(&self) -> std::result::Result<String, DidCommError> {
        // Clone the Arc to avoid holding the lock across await
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            lock.as_ref()
                .ok_or(DidCommError::GeneralError { message: "Database not opened".to_string() })?
                .clone()
        };
        
        let health_checker = crate::core::health::HealthChecker::new(messaging.storage.clone());
        
        if Handle::try_current().is_ok() {
            health_checker.get_health_json().await
                .map_err(|e| DidCommError::GeneralError { message: e.to_string() })
        } else {
            let runtime = self.runtime.clone();
            let handle = runtime.spawn(async move {
                health_checker.get_health_json().await
            });
            
            handle.await
                .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?
                .map_err(|e| DidCommError::GeneralError { message: e.to_string() })
        }
    }

    /// Performs a quick health check to verify basic functionality.
    /// 
    /// This is a lightweight check that only verifies storage connectivity.
    /// Use this for frequent health monitoring where speed is important.
    /// 
    /// # Returns
    /// 
    /// `true` if the system is operational, `false` otherwise
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// if interface.is_healthy().await {
    ///     // System is ready for operations
    /// } else {
    ///     // System needs attention
    /// }
    /// ```
    pub async fn is_healthy(&self) -> bool {
        // Clone the Arc to avoid holding the lock across await
        let messaging = {
            let lock = self.didcomm_messaging.read().unwrap();
            match lock.as_ref() {
                Some(m) => m.clone(),
                None => return false,
            }
        };
        
        let health_checker = crate::core::health::HealthChecker::new(messaging.storage.clone());
        
        if Handle::try_current().is_ok() {
            health_checker.is_healthy().await
        } else {
            let runtime = self.runtime.clone();
            let handle = runtime.spawn(async move {
                health_checker.is_healthy().await
            });
            
            handle.await.unwrap_or(false)
        }
    }

    /// Initialize with development configuration.
    /// 
    /// Disables rate limiting and enables debug logging.
    /// Useful for testing and development.
    pub fn configure_development(&self) -> std::result::Result<(), DidCommError> {
        let config = crate::core::config::NaviaConfig::development();
        
        crate::core::config::init_config(config)
            .map_err(|e| DidCommError::GeneralError { message: e.to_string() })?;
        
        Ok(())
    }

    
    /// Export error logs for crash reporting.
    /// 
    /// Returns all error logs as JSON array. Use this when user 
    /// consents to send crash logs for debugging.
    /// 
    /// # Example
    /// 
    /// ```kotlin
    /// // After crash, ask user:
    /// if (showCrashDialog("Send error logs?")) {
    ///     val logs = didcomm.exportErrorLogs()
    ///     sendToSupport(logs)
    /// }
    /// ```
    pub fn export_error_logs(&self) -> std::result::Result<String, DidCommError> {
        use crate::core::metrics::METRICS;
        
        METRICS.export_json()
            .map_err(|e| DidCommError::GeneralError { message: e.to_string() })
    }
    
    /// Clear all error logs.
    /// 
    /// Removes all logged errors from the file.
    pub fn clear_error_logs(&self) -> std::result::Result<(), DidCommError> {
        use crate::core::metrics::METRICS;
        
        METRICS.clear();
        Ok(())
    }
    
    /// Enable or disable error logging.
    /// 
    /// # Arguments
    /// 
    /// * `enabled` - Whether to log errors to file
    pub fn set_error_logging_enabled(&self, enabled: bool) -> std::result::Result<(), DidCommError> {
        use crate::core::metrics::METRICS;
        
        METRICS.set_enabled(enabled);
        Ok(())
    }
    
    /// Initialize error logging with a file path.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path to the error log file (e.g., "/data/app/errors.log")
    /// 
    /// # Note
    /// 
    /// The file will be created if it doesn't exist. Keeps last 1000 errors.
    pub fn init_error_logging(&self, path: String) -> std::result::Result<(), DidCommError> {
        use crate::core::metrics::METRICS;
        use std::path::PathBuf;
        
        METRICS.init(PathBuf::from(path))
            .map_err(|e| DidCommError::GeneralError { message: e.to_string() })
    }

}