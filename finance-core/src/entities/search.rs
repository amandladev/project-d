use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::TransactionType;

/// Filter criteria for searching transactions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransactionSearchFilter {
    /// Account to search within (required).
    pub account_id: Uuid,
    /// Free-text search on description.
    #[serde(default)]
    pub query: Option<String>,
    /// Filter by category.
    #[serde(default)]
    pub category_id: Option<Uuid>,
    /// Filter by transaction type.
    #[serde(default)]
    pub transaction_type: Option<TransactionType>,
    /// Minimum amount (in cents, inclusive).
    #[serde(default)]
    pub min_amount: Option<i64>,
    /// Maximum amount (in cents, inclusive).
    #[serde(default)]
    pub max_amount: Option<i64>,
    /// Start of date range (inclusive).
    #[serde(default)]
    pub date_from: Option<DateTime<Utc>>,
    /// End of date range (inclusive).
    #[serde(default)]
    pub date_to: Option<DateTime<Utc>>,
    /// Maximum number of results.
    #[serde(default)]
    pub limit: Option<usize>,
    /// Offset for pagination.
    #[serde(default)]
    pub offset: Option<usize>,
}
