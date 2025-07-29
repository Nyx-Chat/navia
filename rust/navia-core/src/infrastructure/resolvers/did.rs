//! DID resolver implementation using Askar
//!
//! Resolves DIDs using the Affinidi DID resolver cache.

use affinidi_did_resolver_cache_sdk::DIDCacheClient;
use async_trait::async_trait;
use navia_didcomm::did::{
    DIDCommMessagingService, DIDDoc, DIDResolver, Service, ServiceKind, VerificationMaterial,
    VerificationMethod, VerificationMethodType,
};
use navia_didcomm::error::{Error, ErrorKind, Result};
use serde_json::Value;
use ssi_core::OneOrMany;
use ssi_dids_core::document::service::Endpoint;
use ssi_dids_core::document::service::Service as ResolvedService;
use ssi_dids_core::document::verification_method::DIDVerificationMethod;
use ssi_dids_core::DID;
use std::vec;

// Type alias for boxed error results in internal functions
type BoxedResult<T> = std::result::Result<T, Box<Error>>;

pub struct AskarDIDResolver {
    resolver: DIDCacheClient,
}

impl AskarDIDResolver {
    pub fn new(resolver: DIDCacheClient) -> Self {
        Self { resolver }
    }
}

#[async_trait]
impl DIDResolver for AskarDIDResolver {
    async fn resolve(&self, did: &str) -> Result<Option<DIDDoc>> {
        let response = self
            .resolver
            .resolve(did)
            .await
            .map_err(|err| Error::new(ErrorKind::DIDNotResolved, err))?;

        let resolved_doc = response.doc.clone();
        let base = unsafe { DID::new_unchecked(did.as_bytes()) };

        let doc = DIDDoc {
            id: resolved_doc.id.to_string(),
            key_agreement: resolved_doc
                .verification_relationships
                .key_agreement
                .iter()
                .map(|x| x.id().resolve(base).to_string())
                .collect(),
            authentication: resolved_doc
                .verification_relationships
                .authentication
                .iter()
                .map(|x| x.id().resolve(base).to_string())
                .collect(),
            service: resolved_doc
                .service
                .iter()
                .map(|s| map_service(s).map_err(|e| *e))
                .collect::<Result<Vec<_>>>()?,
            verification_method: resolved_doc
                .verification_method
                .iter()
                .map(|vm| map_verification_method(vm).map_err(|e| *e))
                .collect::<Result<Vec<_>>>()?,
        };
        Ok(Some(doc))
    }
}

fn map_service(sc: &ResolvedService) -> BoxedResult<Service> {
    let didcomm_type = "DIDCommMessaging".to_string();

    let is_didcomm = match &sc.type_ {
        OneOrMany::One(t) => t.eq(&didcomm_type),
        OneOrMany::Many(ts) => ts.contains(&didcomm_type),
    };

    let id = sc.id.to_string();

    match &sc.service_endpoint {
        Some(endpoints) => match endpoints {
            OneOrMany::One(ep) if is_didcomm => Ok(Service {
                id,
                service_endpoint: endpoint_to_didcomm_messaging_service(ep)?,
            }),
            OneOrMany::Many(eps) if is_didcomm && !eps.is_empty() => Ok(Service {
                id,
                service_endpoint: endpoint_to_didcomm_messaging_service(eps.first().unwrap())?,
            }),
            _ => Ok(Service {
                id,
                service_endpoint: ServiceKind::Other { value: Value::Null },
            }),
        },
        None => Ok(Service {
            id,
            service_endpoint: ServiceKind::Other { value: Value::Null },
        }),
    }
}

fn map_verification_method(vm: &DIDVerificationMethod) -> BoxedResult<VerificationMethod> {
    let (vmtype, material) = match vm.type_.as_ref() {
        "JsonWebKey2020" => (
            VerificationMethodType::JsonWebKey2020,
            VerificationMaterial::JWK {
                public_key_jwk: vm.properties["publicKeyJwk"].clone(),
            },
        ),
        "X25519KeyAgreementKey2019" => (
            VerificationMethodType::X25519KeyAgreementKey2019,
            VerificationMaterial::Base58 {
                public_key_base58: vm.properties["publicKeyBase58"].to_string(),
            },
        ),
        "Ed25519VerificationKey2018" => (
            VerificationMethodType::Ed25519VerificationKey2018,
            VerificationMaterial::Base58 {
                public_key_base58: vm.properties["publicKeyBase58"].to_string(),
            },
        ),
        "EcdsaSecp256k1VerificationKey2019" => (
            VerificationMethodType::EcdsaSecp256k1VerificationKey2019,
            VerificationMaterial::JWK {
                public_key_jwk: vm.properties["publicKeyJwk"].clone(),
            },
        ),
        "X25519KeyAgreementKey2020" => (
            VerificationMethodType::X25519KeyAgreementKey2020,
            VerificationMaterial::Multibase {
                public_key_multibase: vm.properties["publicKeyMultibase"].to_string(),
            },
        ),
        "Ed25519VerificationKey2020" => (
            VerificationMethodType::Ed25519VerificationKey2020,
            VerificationMaterial::Multibase {
                public_key_multibase: vm.properties["publicKeyMultibase"].to_string(),
            },
        ),
        _ => {
            return Err(Box::new(Error::msg(
                ErrorKind::Unsupported,
                "Unsupported method",
            )))
        }
    };
    Ok(VerificationMethod {
        id: vm.id.to_string(),
        type_: vmtype,
        controller: vm.controller.to_string(),
        verification_material: material,
    })
}

fn endpoint_to_didcomm_messaging_service(endpoint: &Endpoint) -> BoxedResult<ServiceKind> {
    match endpoint {
        Endpoint::Uri(uri) => Ok(ServiceKind::DIDCommMessaging {
            value: DIDCommMessagingService {
                uri: uri.to_string(),
                accept: None,
                routing_keys: vec![],
            },
        }),
        Endpoint::Map(value) => {
            let value = fix_routing_keys(value.clone());
            Ok(ServiceKind::DIDCommMessaging {
                value: serde_json::from_value(value).map_err(|e| {
                    Box::new(Error::msg(
                        ErrorKind::Malformed,
                        format!("Failed to parse DIDCommMessaging service: {e}"),
                    ))
                })?,
            })
        }
    }
}

// fixes the bug in resolver code, routing_keys instead of routingKeys
fn fix_routing_keys(mut value: Value) -> Value {
    if let Value::Object(ref mut map) = value {
        if let Some(v) = map.remove("routing_keys") {
            map.insert("routingKeys".to_string(), v);
        }
    }
    value
}
