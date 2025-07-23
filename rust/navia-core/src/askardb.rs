use crate::error::{NaviaError, NaviaResult, SerializationError, StorageError};
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

    /// Creates a new backend session with error mapping
    fn create_session(&self) -> NaviaResult<impl BackendSession> {
        self.backend.session(None, false).map_err(|err| {
            NaviaError::Storage(StorageError::OperationFailed {
                operation: "create_session".to_string(),
                details: err.to_string(),
            })
        })
    }

    /// Serializes a value to JSON with error mapping
    fn serialize_value<T: Serialize>(
        value: &T,
        category: &str,
        name: &str,
    ) -> NaviaResult<Vec<u8>> {
        serde_json::to_vec(value).map_err(|err| {
            NaviaError::Serialization(SerializationError::JsonError {
                context: format!("serializing {category}/{name}"),
                details: err.to_string(),
            })
        })
    }

    pub async fn provision(path: &str, key: PassKey<'_>) -> NaviaResult<Self> {
        SqliteStoreOptions::from_path(path)
            .provision(StoreKeyMethod::RawKey, key, None, false)
            .await
            .map(Self::new)
            .map_err(|err| {
                NaviaError::Storage(StorageError::ConnectionFailed {
                    details: format!("askar: {err}"),
                })
            })
    }

    pub async fn open(path: &str, key: PassKey<'_>) -> NaviaResult<Self> {
        SqliteStoreOptions::from_path(path)
            .open(Some(StoreKeyMethod::RawKey), key, None)
            .await
            .map(Self::new)
            .map_err(|err| {
                NaviaError::Storage(StorageError::ConnectionFailed {
                    details: format!("askar: {err}"),
                })
            })
    }

    pub async fn get_entry(&self, category: &str, name: &str) -> NaviaResult<Option<Entry>> {
        self.create_session()?
            .fetch(EntryKind::Item, category, name, false)
            .await
            .map_err(|err| {
                NaviaError::Storage(StorageError::OperationFailed {
                    operation: "fetch_entry".to_string(),
                    details: err.to_string(),
                })
            })
    }

    pub async fn get<T>(&self, category: &str, name: &str) -> NaviaResult<Option<T>>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.get_entry(category, name).await.map(|entry_option| {
            entry_option
                .map(|entry| {
                    serde_json::from_slice(entry.value.as_ref()).map_err(|err| {
                        NaviaError::Serialization(SerializationError::JsonError {
                            context: format!("deserializing {category}/{name}"),
                            details: err.to_string(),
                        })
                    })
                })
                .transpose() // Convert Option<Result<T>> to Result<Option<T>>
        })?
    }

    pub async fn insert_entry(&self, category: &str, name: &str, value: &[u8]) -> NaviaResult<()> {
        self.create_session()?
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
            .map_err(|err| {
                NaviaError::Storage(StorageError::OperationFailed {
                    operation: "insert_entry".to_string(),
                    details: err.to_string(),
                })
            })
    }

    pub async fn insert<T: Serialize>(
        &self,
        category: &str,
        name: &str,
        value: &T,
    ) -> NaviaResult<()> {
        let data = Self::serialize_value(value, category, name)?;
        self.insert_entry(category, name, &data).await
    }

    pub async fn update_entry(&self, category: &str, name: &str, value: &[u8]) -> NaviaResult<()> {
        self.create_session()?
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
            .map_err(|err| {
                NaviaError::Storage(StorageError::OperationFailed {
                    operation: "update_entry".to_string(),
                    details: err.to_string(),
                })
            })
    }

    pub async fn update<T: Serialize>(
        &self,
        category: &str,
        name: &str,
        value: &T,
    ) -> NaviaResult<()> {
        let data = Self::serialize_value(value, category, name)?;
        self.update_entry(category, name, &data).await
    }

    pub async fn remove(&self, category: &str, name: &str) -> NaviaResult<()> {
        self.create_session()?
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
            .map_err(|err| {
                NaviaError::Storage(StorageError::OperationFailed {
                    operation: "remove_entry".to_string(),
                    details: err.to_string(),
                })
            })
    }
}
