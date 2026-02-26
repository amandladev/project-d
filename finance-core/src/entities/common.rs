use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Synchronization status for offline-first tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Pending,
    Synced,
    Conflicted,
}

impl fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncStatus::Pending => write!(f, "pending"),
            SyncStatus::Synced => write!(f, "synced"),
            SyncStatus::Conflicted => write!(f, "conflicted"),
        }
    }
}

impl SyncStatus {
    pub fn from_str(s: &str) -> Option<SyncStatus> {
        match s {
            "pending" => Some(SyncStatus::Pending),
            "synced" => Some(SyncStatus::Synced),
            "conflicted" => Some(SyncStatus::Conflicted),
            _ => None,
        }
    }
}

/// Transaction type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Expense,
    Income,
    Transfer,
}

impl fmt::Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionType::Expense => write!(f, "expense"),
            TransactionType::Income => write!(f, "income"),
            TransactionType::Transfer => write!(f, "transfer"),
        }
    }
}

impl TransactionType {
    pub fn from_str(s: &str) -> Option<TransactionType> {
        match s {
            "expense" => Some(TransactionType::Expense),
            "income" => Some(TransactionType::Income),
            "transfer" => Some(TransactionType::Transfer),
            _ => None,
        }
    }
}

/// Base fields shared by all entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEntity {
    pub id: uuid::Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl BaseEntity {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    pub fn with_id(id: uuid::Uuid) -> Self {
        let now = Utc::now();
        Self {
            id,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    pub fn soft_delete(&mut self) {
        let now = Utc::now();
        self.deleted_at = Some(now);
        self.updated_at = now;
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

impl Default for BaseEntity {
    fn default() -> Self {
        Self::new()
    }
}
