//! Core DIDComm message handling logic
//! 
//! This module contains the actual implementation of DIDComm messaging,
//! including packing, unpacking, and DID generation functionality.

use crate::core::audit::{audit_log, SecurityEvent};
use crate::core::didcomm::message::{Message, MessageBody};
use crate::core::rate_limit::DID_GENERATION_LIMITER;
use crate::core::storage::traits::{MessageStorage, SecretStorage};
use crate::core::validation::{validate_did, validate_uri};
use crate::error::{NaviaResult, NaviaError, PackingError, UnpackingError, DidError, SerializationError};
use affinidi_did_resolver_cache_sdk::DIDCacheClient;
use did_peer::{
    DIDPeer, DIDPeerCreateKeys, DIDPeerKeyType, DIDPeerKeys, DIDPeerService, DIDService,
};
use didcomm::secrets::{Secret, SecretMaterial, SecretType};
use didcomm::{
    Message as DIDCommMessage, PackEncryptedMetadata, PackEncryptedOptions, 
    UnpackMetadata, UnpackOptions,
};
use serde_json::json;
use std::sync::Arc;
use std::time::SystemTime;
use zeroize::Zeroize;

/// Storage category constant for secret materials
/// 
/// All secret keys and private materials are stored under this category
/// in the encrypted database for easy retrieval and management.
pub const CATEGORY_SECRET: &str = "secret";

/// Core DIDComm messaging handler that orchestrates all DIDComm operations.
/// 
/// This struct is the heart of the DIDComm implementation, providing:
/// - Message packing (encryption) and unpacking (decryption)
/// - DID generation and management
/// - Secret key storage and retrieval
/// - Integration with DID resolvers for external DID lookups
/// 
/// # Type Parameters
/// 
/// * `S` - Storage backend that implements both `MessageStorage` and `SecretStorage` traits
/// 
/// # Architecture
/// 
/// The handler uses dependency injection for storage and resolvers, making it
/// flexible and testable. It coordinates between:
/// - Storage layer for persisting secrets and messages
/// - DID resolver for looking up external DIDs
/// - Secrets resolver for retrieving local private keys
/// 
/// # Thread Safety
/// 
/// The handler is thread-safe through the use of `Arc` for shared storage access.
pub struct DidcommMessaging<S>
where
    S: MessageStorage + SecretStorage,
{
    pub storage: Arc<S>,
    did_resolver: AskarDIDResolver,
    secrets_resolver: AskarSecretsResolver<S>,
}

impl<S> DidcommMessaging<S>
where
    S: MessageStorage + SecretStorage,
{
    /// Creates a new DIDComm messaging handler.
    /// 
    /// # Arguments
    /// 
    /// * `storage` - Arc-wrapped storage backend for secrets and messages
    /// * `didcache_client` - DID resolver client for external DID lookups
    /// 
    /// # Returns
    /// 
    /// A new `DidcommMessaging` instance ready for DIDComm operations.
    pub fn new(storage: Arc<S>, didcache_client: DIDCacheClient) -> Self {
        Self {
            storage: storage.clone(),
            did_resolver: AskarDIDResolver::new(didcache_client),
            secrets_resolver: AskarSecretsResolver::new(storage),
        }
    }


    /// Stores a secret (private key) in the encrypted storage.
    /// 
    /// Secrets are serialized to JSON before storage for compatibility
    /// with the DIDComm library's secret format.
    /// 
    /// # Arguments
    /// 
    /// * `secret` - The secret to store, containing key material and metadata
    /// 
    /// # Errors
    /// 
    /// * `CoreError::Serialization` - If the secret cannot be serialized to JSON
    /// * `CoreError::Storage` - If the storage operation fails
    pub async fn add_secret(&self, secret: &Secret) -> NaviaResult<()> {
        let secret_bytes = serde_json::to_vec(secret)
            .map_err(|e| NaviaError::Serialization(SerializationError::JsonError {
                context: "serializing secret".to_string(),
                details: e.to_string(),
            }))?;
        self.storage.store_secret(&secret.id, &secret_bytes).await
    }

    /// Stores multiple secrets in the encrypted storage.
    /// 
    /// This is a convenience method that calls `add_secret` for each secret
    /// in the provided slice. Useful when generating DIDs with multiple keys.
    /// 
    /// # Arguments
    /// 
    /// * `secrets` - Slice of secrets to store
    /// 
    /// # Errors
    /// 
    /// Returns the first error encountered. Note that this means some secrets
    /// may be stored before an error occurs.
    pub async fn add_secrets(&self, secrets: &[Secret]) -> NaviaResult<()> {
        for secret in secrets {
            self.add_secret(secret).await?;
        }
        Ok(())
    }

    /// Packs (encrypts) a message using DIDComm encryption.
    /// 
    /// This is the low-level packing method that provides full control over
    /// the packing process, including signing and encryption options.
    /// 
    /// # Arguments
    /// 
    /// * `msg` - The message to encrypt
    /// * `to` - Recipient's DID
    /// * `from` - Optional sender's DID (required for authenticated encryption)
    /// * `sign_by` - Optional DID to sign the message (usually same as `from`)
    /// * `opts` - Packing options (algorithms, forward secrecy, etc.)
    /// 
    /// # Returns
    /// 
    /// A tuple of:
    /// - The encrypted message as a JSON string
    /// - Metadata about the packing operation
    /// 
    /// # Errors
    /// 
    /// * `CoreError::Packing` - If encryption fails due to missing keys,
    ///   invalid DIDs, or algorithm errors
    pub async fn pack_encrypted(
        &self,
        msg: &Message,
        to: &str,
        from: Option<&str>,
        sign_by: Option<&str>,
        opts: &PackEncryptedOptions,
    ) -> NaviaResult<(String, PackEncryptedMetadata)> {
        // Validate inputs
        validate_did(to)?;
        if let Some(from_did) = from {
            validate_did(from_did)?;
        }
        if let Some(sign_by_did) = sign_by {
            validate_did(sign_by_did)?;
        }
        
        // Convert our core Message to DIDComm Message
        let didcomm_msg = self.message_to_didcomm(msg);
        
        let result = didcomm_msg.pack_encrypted(
            to,
            from,
            sign_by,
            &self.did_resolver,
            &self.secrets_resolver,
            opts,
        )
        .await
        .map_err(|err| {
            let error_str = err.to_string();
            if error_str.contains("Sender key not found") {
                NaviaError::from(PackingError::SenderKeyNotFound {
                    did: from.unwrap_or("unknown").to_string(),
                })
            } else if error_str.contains("Recipient keys not found") {
                NaviaError::from(PackingError::RecipientKeyNotFound {
                    did: to.to_string(),
                })
            } else if error_str.contains("Invalid recipient DID") {
                NaviaError::from(PackingError::InvalidRecipientDid {
                    did: to.to_string(),
                })
            } else {
                NaviaError::from(PackingError::EncryptionFailed {
                    details: error_str,
                })
            }
        })?;
        
        // Audit log the encryption event
        audit_log(
            SecurityEvent::MessagePacked {
                from: from.map(|s| s.to_string()),
                to: to.to_string(),
                message_id: msg.id.clone(),
                timestamp: SystemTime::now(),
            },
            None,
        );
        
        Ok(result)
    }

    /// Packs (encrypts) a message with default encryption settings.
    /// 
    /// This is the simplified packing method that uses sensible defaults:
    /// - Authenticated encryption (sender is verified)
    /// - Message is signed by the sender
    /// - Default encryption algorithms
    /// 
    /// # Arguments
    /// 
    /// * `msg` - The message to encrypt
    /// * `to` - Recipient's DID
    /// * `from` - Sender's DID (used for both authentication and signing)
    /// 
    /// # Returns
    /// 
    /// A tuple of:
    /// - The encrypted message as a JSON string
    /// - Metadata about the packing operation
    /// 
    /// # Errors
    /// 
    /// * `CoreError::Packing` - If encryption fails
    pub async fn pack_message(
        &self,
        msg: &Message,
        to: &str,
        from: &str,
    ) -> NaviaResult<(String, PackEncryptedMetadata)> {
        self.pack_encrypted(
            msg,
            to,
            Some(from),
            Some(from),
            &PackEncryptedOptions::default(),
        )
        .await
    }

    /// Unpacks (decrypts) an encrypted DIDComm message.
    /// 
    /// This is the low-level unpacking method that provides full control
    /// over the unpacking process through options.
    /// 
    /// # Arguments
    /// 
    /// * `msg` - The encrypted message as a JSON string
    /// * `opts` - Unpacking options (verification settings, etc.)
    /// 
    /// # Returns
    /// 
    /// A tuple of:
    /// - The decrypted message
    /// - Metadata about the unpacking operation (sender verification, etc.)
    /// 
    /// # Errors
    /// 
    /// * `CoreError::Unpacking` - If decryption fails due to:
    ///   - Wrong recipient (no matching private keys)
    ///   - Corrupted message
    ///   - Invalid signature
    ///   - Malformed message structure
    pub async fn unpack(
        &self,
        msg: &str,
        opts: &UnpackOptions,
    ) -> NaviaResult<(Message, UnpackMetadata)> {
        let (didcomm_msg, metadata) = DIDCommMessage::unpack(
            msg, 
            &self.did_resolver, 
            &self.secrets_resolver, 
            opts
        )
        .await
        .map_err(|err| {
            let error_str = err.to_string();
            if error_str.contains("Wrong recipient") || error_str.contains("Recipient key not found") {
                NaviaError::from(UnpackingError::RecipientKeyNotFound {
                    details: error_str,
                })
            } else if error_str.contains("Invalid signature") {
                NaviaError::from(UnpackingError::InvalidSignature {
                    signer: "unknown".to_string(),
                })
            } else if error_str.contains("Malformed") || error_str.contains("Invalid format") {
                NaviaError::from(UnpackingError::MalformedMessage {
                    details: error_str,
                })
            } else {
                NaviaError::from(UnpackingError::DecryptionFailed {
                    details: error_str,
                })
            }
        })?;
        
        // Convert DIDComm message to our core Message
        let core_msg = self.didcomm_to_message(didcomm_msg)?;
        
        // Audit log the decryption event
        audit_log(
            SecurityEvent::MessageUnpacked {
                from: metadata.encrypted_from_kid.clone(),
                to: None, // Recipient info not available in metadata
                message_id: core_msg.id.clone(),
                timestamp: SystemTime::now(),
            },
            None,
        );
        
        Ok((core_msg, metadata))
    }

    /// Unpacks (decrypts) a message with default settings.
    /// 
    /// This is the simplified unpacking method that uses sensible defaults:
    /// - Verifies signatures when present
    /// - Resolves sender DIDs automatically
    /// 
    /// # Arguments
    /// 
    /// * `msg` - The encrypted message as a JSON string
    /// 
    /// # Returns
    /// 
    /// A tuple of:
    /// - The decrypted message
    /// - Metadata about the unpacking operation
    /// 
    /// # Errors
    /// 
    /// * `CoreError::Unpacking` - If decryption fails
    pub async fn unpack_message(&self, msg: &str) -> NaviaResult<(Message, UnpackMetadata)> {
        self.unpack(msg, &UnpackOptions::default()).await
    }

    /// Generates a new peer DID with associated encryption and signing keys.
    /// 
    /// Creates a DID using the did:peer method, which includes:
    /// - An Ed25519 key for signing (authentication)
    /// - A P-256 key for encryption (key agreement)
    /// - Service endpoint for message routing
    /// 
    /// The generated keys are automatically stored in the encrypted database.
    /// 
    /// # Arguments
    /// 
    /// * `uri` - Service endpoint URI where this DID can receive messages
    /// * `routing_keys` - Optional mediator DIDs for message forwarding
    /// 
    /// # Returns
    /// 
    /// The generated DID string (e.g., "did:peer:1zQm...")
    /// 
    /// # Errors
    /// 
    /// * `CoreError::DidGeneration` - If DID generation fails
    /// * `CoreError::Storage` - If storing the generated keys fails
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// // Direct messaging
    /// let did = messaging.generate_did(
    ///     "https://example.com/didcomm".to_string(),
    ///     vec![]
    /// ).await?;
    /// 
    /// // With mediator routing
    /// let did = messaging.generate_did(
    ///     "https://mediator.com/forward".to_string(),
    ///     vec!["did:peer:mediator123".to_string()]
    /// ).await?;
    /// ```
    pub async fn generate_did(&self, uri: String, routing_keys: Vec<String>) -> NaviaResult<String> {
        // Check rate limit
        if let Err(e) = DID_GENERATION_LIMITER.check_rate_limit("global") {
            audit_log(
                SecurityEvent::RateLimitExceeded {
                    operation: "generate_did".to_string(),
                    timestamp: SystemTime::now(),
                },
                None,
            );
            return Err(e);
        }
        
        // Validate inputs
        validate_uri(&uri)?;
        for routing_key in &routing_keys {
            validate_did(routing_key)?;
        }
        
        let keys = vec![
            DIDPeerCreateKeys::new(
                DIDPeerKeys::Verification,
                Some(DIDPeerKeyType::Ed25519),
                None,
            ),
            DIDPeerCreateKeys::new(
                DIDPeerKeys::Encryption, 
                Some(DIDPeerKeyType::P256), 
                None
            ),
        ];
        
        let services = vec![DIDPeerService::from(DIDService::new(
            uri,
            vec!["didcomm/v2".to_owned()],
            routing_keys,
            None,
        ))];
        
        let (did, mut keys) = DIDPeer::create_peer_did(&keys, Some(&services))
            .map_err(|err| NaviaError::from(DidError::GenerationFailed {
                details: err.to_string(),
            }))?;
            
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

        // Zeroize the private key material after storing
        for key in &mut keys {
            key.d.zeroize();
        }

        // Audit log the DID generation
        audit_log(
            SecurityEvent::DidGenerated {
                did: did.clone(),
                timestamp: SystemTime::now(),
            },
            None,
        );

        Ok(did)
    }
    
    /// Converts internal Message representation to DIDComm format.
    /// 
    /// Handles the transformation of message body types:
    /// - String bodies are wrapped in JSON
    /// - Object bodies are passed through as-is
    /// 
    /// # Arguments
    /// 
    /// * `msg` - Internal message representation
    /// 
    /// # Returns
    /// 
    /// A DIDComm library compatible message
    fn message_to_didcomm(&self, msg: &Message) -> DIDCommMessage {
        let body = match &msg.body {
            MessageBody::String(s) => json!(s),
            MessageBody::Object(obj) => obj.clone(),
        };
        
        let mut builder = DIDCommMessage::build(
            msg.id.clone(),
            msg.msg_type.clone(),
            body
        );
        
        // Add all recipients (already validated in pack_encrypted)
        for recipient in &msg.to {
            builder = builder.to(recipient.clone());
        }
        
        // Add sender if present (already validated in pack_encrypted)
        if let Some(from) = &msg.from {
            builder = builder.from(from.clone());
        }
        
        builder.finalize()
    }
    
    /// Converts DIDComm format message to internal representation.
    /// 
    /// Handles the transformation of message body types:
    /// - String JSON values are extracted as strings
    /// - Complex JSON objects are preserved as objects
    /// 
    /// # Arguments
    /// 
    /// * `msg` - DIDComm library message
    /// 
    /// # Returns
    /// 
    /// Internal message representation
    /// 
    /// # Errors
    /// 
    /// Currently infallible but returns Result for future extensibility
    fn didcomm_to_message(&self, msg: DIDCommMessage) -> NaviaResult<Message> {
        // If body is a string value, extract it. Otherwise keep as object
        let body = if msg.body.is_string() {
            MessageBody::String(msg.body.as_str().unwrap_or("").to_string())
        } else {
            MessageBody::Object(msg.body)
        };
        
        Ok(Message {
            id: msg.id,
            msg_type: msg.type_,
            body,
            from: msg.from,
            to: msg.to.unwrap_or_default(),
            headers: Default::default(),
        })
    }
}

// The actual resolver implementations from the infrastructure layer
use crate::infrastructure::resolvers::did::AskarDIDResolver;
use crate::infrastructure::resolvers::secrets::AskarSecretsResolver;