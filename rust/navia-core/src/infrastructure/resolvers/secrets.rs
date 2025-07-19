//! Secrets resolver implementation using Askar
//! 
//! Resolves secrets stored in the Askar database.

use crate::core::storage::traits::SecretStorage;
use async_trait::async_trait;
use didcomm::error::{Error, ErrorKind, Result};
use didcomm::secrets::{Secret, SecretsResolver};
use std::sync::Arc;

pub const CATEGORY_SECRET: &str = "secret";

pub struct AskarSecretsResolver<S>
where
    S: SecretStorage,
{
    storage: Arc<S>,
}

impl<S> AskarSecretsResolver<S>
where
    S: SecretStorage,
{
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl<S> SecretsResolver for AskarSecretsResolver<S>
where
    S: SecretStorage + Send + Sync,
{
    async fn get_secret(&self, secret_id: &str) -> Result<Option<Secret>> {
        self.storage
            .get_secret(secret_id)
            .await
            .map_err(|err| Error::new(ErrorKind::InvalidState, err))?
            .map(|secret_bytes| {
                serde_json::from_slice(&secret_bytes)
                    .map_err(|e| Error::new(ErrorKind::InvalidState, e))
            })
            .transpose()
    }

    async fn find_secrets<'a>(&self, secret_ids: &'a [&'a str]) -> Result<Vec<&'a str>> {
        let mut found_secrets = Vec::new();

        for &secret_id in secret_ids {
            if let Some(_) = self.get_secret(secret_id).await? {
                found_secrets.push(secret_id);
            }
        }

        Ok(found_secrets)
    }
}