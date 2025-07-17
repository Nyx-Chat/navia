use crate::askardb::AskarDB;
use crate::messaging::CATEGORY_SECRET;
use async_trait::async_trait;
use didcomm::error::{Error, ErrorKind, Result};
use didcomm::secrets::{Secret, SecretsResolver};
use std::sync::Arc;

pub struct AskarSecretsResolver {
    db: Arc<AskarDB>,
}

impl AskarSecretsResolver {
    pub fn new(db: Arc<AskarDB>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SecretsResolver for AskarSecretsResolver {
    async fn get_secret(&self, secret_id: &str) -> Result<Option<Secret>> {
        self.db
            .get(CATEGORY_SECRET, secret_id)
            .await
            .map_err(|err| Error::new(ErrorKind::InvalidState, err))
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
