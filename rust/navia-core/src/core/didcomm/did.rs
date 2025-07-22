//! DID generation and management
//!
//! This module provides functionality for creating and managing Decentralized
//! Identifiers (DIDs) using the did:peer method. DIDs are the foundation of
//! DIDComm messaging, providing cryptographically verifiable identities.
//!
//! # DID Structure
//!
//! A generated peer DID includes:
//! - An Ed25519 key for signing (authentication)
//! - A P-256 key for encryption (key agreement)
//! - Service endpoints for message routing
//!
//! # Example
//!
//! ```ignore
//! use navia_core::core::didcomm::did::generate_peer_did;
//!
//! // Generate a DID with a service endpoint
//! let (did, secrets) = generate_peer_did(
//!     "https://example.com/didcomm".to_string(),
//!     vec![] // No routing keys
//! )?;
//!
//! println!("Generated DID: {}", did);
//! println!("Number of secrets: {}", secrets.len()); // Will be 2 (signing + encryption)
//! ```

use crate::error::{DidError, NaviaError, NaviaResult};
use did_peer::{
    DIDPeer, DIDPeerCreateKeys, DIDPeerKeyType, DIDPeerKeys, DIDPeerService, DIDService,
};
use didcomm::secrets::{Secret, SecretMaterial, SecretType};
use serde_json::json;

/// Generates a new peer DID with associated cryptographic keys.
///
/// Creates a DID using the did:peer method (specifically numalgo 2), which embeds
/// the public keys and service endpoints directly in the DID itself. This makes
/// peer DIDs self-contained and doesn't require a blockchain or registry.
///
/// # Arguments
///
/// * `uri` - The service endpoint URI where this DID can receive DIDComm messages.
///           Should be a valid HTTPS URL or WebSocket endpoint.
/// * `routing_keys` - Optional list of mediator DIDs for message forwarding.
///                    Use empty vec if direct messaging without mediators.
///
/// # Returns
///
/// A tuple containing:
/// - The generated DID string (e.g., "did:peer:2.Ez...")
/// - A vector of `Secret` objects containing the private keys
///
/// # Generated Keys
///
/// Two keys are generated for each DID:
/// 1. **Verification Key** (Ed25519): Used for signing messages and authentication
/// 2. **Encryption Key** (P-256): Used for key agreement and message encryption
///
/// # Errors
///
/// * `NaviaError::Did(DidError::GenerationFailed)` - If DID generation fails due to
///   invalid parameters or cryptographic errors
///
/// # Security Considerations
///
/// The generated private keys should be:
/// - Stored securely in encrypted storage
/// - Never logged or transmitted
/// - Zeroized from memory after use
///
/// # Example
///
/// ```ignore
/// // Direct messaging (no mediators)
/// let (did, secrets) = generate_peer_did(
///     "https://example.com/didcomm".to_string(),
///     vec![]
/// )?;
///
/// // With mediator routing
/// let (did, secrets) = generate_peer_did(
///     "https://mediator.com/forward".to_string(),
///     vec!["did:peer:mediator123".to_string()]
/// )?;
///
/// // Store the secrets securely
/// for secret in secrets {
///     secure_storage.store_secret(&secret.id, &secret)?;
/// }
/// ```
pub fn generate_peer_did(
    uri: String,
    routing_keys: Vec<String>,
) -> NaviaResult<(String, Vec<Secret>)> {
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

    let (did, keys) = DIDPeer::create_peer_did(&keys, Some(&services)).map_err(|err| {
        NaviaError::Did(DidError::GenerationFailed {
            details: err.to_string(),
        })
    })?;

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

    Ok((did, secrets))
}
