//! Core DIDComm message handling logic
//!
//! This module provides DIDComm messaging with validation, audit logging, and storage abstraction.
//! Uses navia_messaging types but adds additional safety and observability features.

use crate::core::audit::{audit_log, SecurityEvent};
use crate::core::didcomm::message::{Message, MessageBody};
use crate::core::validation::{validate_did, validate_uri};
use crate::error::{DidError, NaviaError, NaviaResult, PackingError, UnpackingError};
use affinidi_did_resolver_cache_sdk::DIDCacheClient;
use did_peer::{
    DIDPeer, DIDPeerCreateKeys, DIDPeerKeyType, DIDPeerKeys, DIDPeerService, DIDService,
};
use navia_didcomm::secrets::{Secret, SecretMaterial, SecretType};
use navia_didcomm::{
    Message as DIDCommMessage, PackEncryptedMetadata, PackEncryptedOptions, UnpackMetadata,
    UnpackOptions,
};
use navia_messaging::askardb::AskarDB;
use navia_messaging::resolvers::did::AskarDIDResolver;
use navia_messaging::resolvers::secrets::AskarSecretsResolver;
use serde_json::json;
use std::sync::Arc;
use std::time::SystemTime;
use zeroize::Zeroize;

/// Storage category constant for secret materials
pub const CATEGORY_SECRET: &str = "secret";

/// Core DIDComm messaging handler with validation, audit logging, and AskarDB storage.
pub struct DidcommMessaging {
    pub db: Arc<AskarDB>,
    did_resolver: AskarDIDResolver,
    secrets_resolver: AskarSecretsResolver,
}

impl DidcommMessaging {
    /// Creates a new DIDComm messaging handler.
    pub fn new(db: Arc<AskarDB>, didcache_client: DIDCacheClient) -> Self {
        Self {
            db: db.clone(),
            did_resolver: AskarDIDResolver::new(didcache_client),
            secrets_resolver: AskarSecretsResolver::new(db),
        }
    }

    /// Stores a secret (private key) in the encrypted storage.
    pub async fn add_secret(&self, secret: &Secret) -> NaviaResult<()> {
        self.db
            .insert(CATEGORY_SECRET, &secret.id, secret)
            .await
            .map_err(|e| NaviaError::External(e.to_string()))
    }

    /// Stores multiple secrets in the encrypted storage.
    pub async fn add_secrets(&self, secrets: &[Secret]) -> NaviaResult<()> {
        for secret in secrets {
            self.add_secret(secret).await?;
        }
        Ok(())
    }

    /// Packs (encrypts) a message using DIDComm encryption with full validation and audit logging.
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

        let result = didcomm_msg
            .pack_encrypted(
                to,
                from,
                sign_by,
                &self.did_resolver,
                &self.secrets_resolver,
                opts,
            )
            .await
            .map_err(|err| Self::map_packing_error(err, to, from))?;

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
    pub async fn unpack(
        &self,
        msg: &str,
        opts: &UnpackOptions,
    ) -> NaviaResult<(Message, UnpackMetadata)> {
        let (didcomm_msg, metadata) =
            DIDCommMessage::unpack(msg, &self.did_resolver, &self.secrets_resolver, opts)
                .await
                .map_err(Self::map_unpacking_error)?;

        // Convert DIDComm message to our core Message
        let core_msg = self.didcomm_to_message(didcomm_msg)?;

        // Audit log the decryption event
        audit_log(
            SecurityEvent::MessageUnpacked {
                from: metadata.encrypted_from_kid.clone(),
                to: None,
                message_id: core_msg.id.clone(),
                timestamp: SystemTime::now(),
            },
            None,
        );

        Ok((core_msg, metadata))
    }

    /// Unpacks (decrypts) a message with default settings.
    pub async fn unpack_message(&self, msg: &str) -> NaviaResult<(Message, UnpackMetadata)> {
        self.unpack(msg, &UnpackOptions::default()).await
    }

    /// Generates a new peer DID with associated encryption and signing keys.
    pub async fn generate_did(
        &self,
        uri: String,
        routing_keys: Vec<String>,
    ) -> NaviaResult<String> {
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
            DIDPeerCreateKeys::new(DIDPeerKeys::Encryption, Some(DIDPeerKeyType::P256), None),
        ];

        let services = vec![DIDPeerService::from(DIDService::new(
            uri,
            vec!["didcomm/v2".to_owned()],
            routing_keys,
            None,
        ))];

        let (did, mut keys) = DIDPeer::create_peer_did(&keys, Some(&services)).map_err(|err| {
            NaviaError::from(DidError::GenerationFailed {
                details: err.to_string(),
            })
        })?;

        let mut kid = 0;
        let secrets: Vec<Secret> = keys
            .iter()
            .map(|key| {
                kid += 1;
                Secret {
                    id: format!("{did}#key-{kid}"),
                    type_: SecretType::JsonWebKey2020,
                    secret_material: SecretMaterial::JWK {
                        private_key_jwk: crate::core::didcomm::did::create_jwk(
                            &key.curve, &key.d, &key.x, &key.y,
                        ),
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
    fn message_to_didcomm(&self, msg: &Message) -> DIDCommMessage {
        let body = match &msg.body {
            MessageBody::String(s) => json!(s),
            MessageBody::Object(obj) => obj.clone(),
        };

        let mut builder = DIDCommMessage::build(msg.id.clone(), msg.msg_type.clone(), body);

        for recipient in &msg.to {
            builder = builder.to(recipient.clone());
        }

        if let Some(from) = &msg.from {
            builder = builder.from(from.clone());
        }

        builder.finalize()
    }

    /// Converts DIDComm format message to internal representation.
    fn didcomm_to_message(&self, msg: DIDCommMessage) -> NaviaResult<Message> {
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

    /// Maps DIDComm library errors to specific NaviaError variants.
    fn map_packing_error(
        err: navia_didcomm::error::Error,
        to: &str,
        from: Option<&str>,
    ) -> NaviaError {
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
            NaviaError::from(PackingError::EncryptionFailed { details: error_str })
        }
    }

    fn map_unpacking_error(err: navia_didcomm::error::Error) -> NaviaError {
        let error_str = err.to_string();
        if error_str.contains("Wrong recipient") || error_str.contains("Recipient key not found") {
            NaviaError::from(UnpackingError::RecipientKeyNotFound { details: error_str })
        } else if error_str.contains("Invalid signature") {
            NaviaError::from(UnpackingError::InvalidSignature {
                signer: "unknown".to_string(),
            })
        } else if error_str.contains("Malformed") || error_str.contains("Invalid format") {
            NaviaError::from(UnpackingError::MalformedMessage { details: error_str })
        } else {
            NaviaError::from(UnpackingError::DecryptionFailed { details: error_str })
        }
    }
}
