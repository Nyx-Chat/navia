use crate::error::Result;
use askar_storage::entry::{Entry, EntryKind, EntryOperation};
use askar_storage::sqlite::{SqliteBackend, SqliteStoreOptions};
use askar_storage::{Backend, BackendSession, PassKey, StoreKeyMethod};
use serde::{Deserialize, Serialize};


pub struct AskarDB {
    backend: SqliteBackend,
}

impl AskarDB {
    pub fn new(backend: SqliteBackend) -> Self {
        Self { backend }
    }

    pub async fn provision(path: &str, key: PassKey<'_>) -> Result<Self> {
        SqliteStoreOptions::from_path(path)
            .provision(StoreKeyMethod::RawKey, key, None, false)
            .await
            .map(Self::new)
            .map_err(|err| err.into())
    }

    pub async fn open(path: &str, key: PassKey<'_>) -> Result<Self> {
        SqliteStoreOptions::from_path(path)
            .open(Some(StoreKeyMethod::RawKey), key, None)
            .await
            .map(Self::new)
            .map_err(|err| err.into())
    }

    pub async fn get_entry(&self, category: &str, name: &str) -> Result<Option<Entry>> {
        self.backend
            .session(None, false)?
            .fetch(EntryKind::Item, category, name, false)
            .await
            .map_err(|err| err.into())
    }

    pub async fn get<T>(&self, category: &str, name: &str) -> Result<Option<T>>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.get_entry(category, name).await.map(|entry_option| {
            entry_option
                .map(|entry| serde_json::from_slice(entry.value.as_ref()).map_err(|err| err.into()))
                .transpose() // Convert Option<Result<T>> to Result<Option<T>>
        })?
    }

    pub async fn insert_entry(&self, category: &str, name: &str, value: &[u8]) -> Result<()> {
        self.backend
            .session(None, false)?
            .update(
                EntryKind::Item,
                EntryOperation::Insert,
                category,
                name,
                Some(value),
                None,
                None,
            )
            .await
            .map_err(|err| err.into())
    }

    pub async fn insert<T: Serialize>(&self, category: &str, name: &str, value: &T) -> Result<()> {
        self.insert_entry(category, name, &serde_json::to_vec(value)?)
            .await
    }

    pub async fn update_entry(&self, category: &str, name: &str, value: &[u8]) -> Result<()> {
        self.backend
            .session(None, false)?
            .update(
                EntryKind::Item,
                EntryOperation::Replace,
                category,
                name,
                Some(value),
                None,
                None,
            )
            .await
            .map_err(|err| err.into())
    }

    pub async fn update<T: Serialize>(&self, category: &str, name: &str, value: &T) -> Result<()> {
        self.update_entry(category, name, &serde_json::to_vec(value)?)
            .await
    }

    pub async fn remove(&self, category: &str, name: &str) -> Result<()> {
        self.backend
            .session(None, false)?
            .update(
                EntryKind::Item,
                EntryOperation::Remove,
                category,
                name,
                None,
                None,
                None,
            )
            .await
            .map_err(|err| err.into())
    }
}

