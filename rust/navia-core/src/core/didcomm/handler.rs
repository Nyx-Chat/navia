//! Core DIDComm message handling logic
//! 
//! This module contains the actual implementation of DIDComm messaging,
//! including packing, unpacking, and DID generation functionality.

use crate::core::didcomm::message::{Message, MessageBody};
use crate::core::storage::traits::{MessageStorage, SecretStorage};
use crate::error::core::{CoreError, CoreResult};
use affinidi_did_resolver_cache_sdk::config::ClientConfigBuilder;
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

pub const CATEGORY_SECRET: &str = "secret";

/// Core DIDComm messaging handler
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
    /// Create a new messaging handler with the given storage backend
    pub fn new(storage: Arc<S>, didcache_client: DIDCacheClient) -> Self {
        Self {
            storage: storage.clone(),
            did_resolver: AskarDIDResolver::new(didcache_client),
            secrets_resolver: AskarSecretsResolver::new(storage),
        }
    }

    /// Create a new local DID resolver
    async fn new_local_resolver() -> CoreResult<DIDCacheClient> {
        DIDCacheClient::new(ClientConfigBuilder::default().build())
            .await
            .map_err(|err| CoreError::Resolution(err.to_string()))
    }

    /// Add a secret to storage
    pub async fn add_secret(&self, secret: &Secret) -> CoreResult<()> {
        let secret_bytes = serde_json::to_vec(secret)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;
        self.storage.store_secret(&secret.id, &secret_bytes).await
    }

    /// Add multiple secrets to storage
    pub async fn add_secrets(&self, secrets: &[Secret]) -> CoreResult<()> {
        for secret in secrets {
            self.add_secret(secret).await?;
        }
        Ok(())
    }

    /// Pack an encrypted message
    pub async fn pack_encrypted(
        &self,
        msg: &Message,
        to: &str,
        from: Option<&str>,
        sign_by: Option<&str>,
        opts: &PackEncryptedOptions,
    ) -> CoreResult<(String, PackEncryptedMetadata)> {
        // Convert our core Message to DIDComm Message
        let didcomm_msg = self.message_to_didcomm(msg);
        
        didcomm_msg.pack_encrypted(
            to,
            from,
            sign_by,
            &self.did_resolver,
            &self.secrets_resolver,
            opts,
        )
        .await
        .map_err(|err| CoreError::Packing(err.to_string()))
    }

    /// Pack a message with default options
    pub async fn pack_message(
        &self,
        msg: &Message,
        to: &str,
        from: &str,
    ) -> CoreResult<(String, PackEncryptedMetadata)> {
        self.pack_encrypted(
            msg,
            to,
            Some(from),
            Some(from),
            &PackEncryptedOptions::default(),
        )
        .await
    }

    /// Unpack an encrypted message
    pub async fn unpack(
        &self,
        msg: &str,
        opts: &UnpackOptions,
    ) -> CoreResult<(Message, UnpackMetadata)> {
        let (didcomm_msg, metadata) = DIDCommMessage::unpack(
            msg, 
            &self.did_resolver, 
            &self.secrets_resolver, 
            opts
        )
        .await
        .map_err(|err| CoreError::Unpacking(err.to_string()))?;
        
        // Convert DIDComm message to our core Message
        let core_msg = self.didcomm_to_message(didcomm_msg)?;
        
        Ok((core_msg, metadata))
    }

    /// Unpack a message with default options
    pub async fn unpack_message(&self, msg: &str) -> CoreResult<(Message, UnpackMetadata)> {
        self.unpack(msg, &UnpackOptions::default()).await
    }

    /// Generate a new DID with the given service endpoint and routing keys
    pub async fn generate_did(&self, uri: String, routing_keys: Vec<String>) -> CoreResult<String> {
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
        
        let (did, keys) = DIDPeer::create_peer_did(&keys, Some(&services))
            .map_err(|err| CoreError::DidGeneration(err.to_string()))?;
            
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
    
    /// Convert core Message to DIDComm Message
    fn message_to_didcomm(&self, msg: &Message) -> DIDCommMessage {
        let body = match &msg.body {
            MessageBody::String(s) => json!({"content": s}),
            MessageBody::Object(obj) => obj.clone(),
        };
        
        let mut builder = DIDCommMessage::build(
            msg.id.clone(),
            msg.msg_type.clone(),
            body
        );
        
        // Add all recipients
        for recipient in &msg.to {
            builder = builder.to(recipient.clone());
        }
        
        // Add sender if present
        if let Some(from) = &msg.from {
            builder = builder.from(from.clone());
        }
        
        builder.finalize()
    }
    
    /// Convert DIDComm Message to core Message
    fn didcomm_to_message(&self, msg: DIDCommMessage) -> CoreResult<Message> {
        let body = if let Some(content) = msg.body.get("content").and_then(|v| v.as_str()) {
            MessageBody::String(content.to_string())
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