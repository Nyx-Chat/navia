//! DID generation and management
//! 
//! This module contains the DID-specific functionality extracted
//! from the main messaging module.

use crate::error::core::{CoreError, CoreResult};
use did_peer::{
    DIDPeer, DIDPeerCreateKeys, DIDPeerKeyType, DIDPeerKeys, DIDPeerService, DIDService,
};
use didcomm::secrets::{Secret, SecretMaterial, SecretType};
use serde_json::json;

/// Generate a new peer DID with the specified keys and services
pub fn generate_peer_did(
    uri: String, 
    routing_keys: Vec<String>
) -> CoreResult<(String, Vec<Secret>)> {
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
    
    Ok((did, secrets))
}