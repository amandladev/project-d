use thiserror::Error;

/// Storage-layer errors.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

impl From<StorageError> for finance_core::errors::DomainError {
    fn from(e: StorageError) -> Self {
        finance_core::errors::DomainError::Storage(e.to_string())
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(e: rusqlite::Error) -> Self {
        StorageError::Query(e.to_string())
    }
}
