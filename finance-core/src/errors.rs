use thiserror::Error;

/// Domain-level errors.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Sync error: {0}")]
    Sync(String),
}
