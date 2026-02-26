use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::entities::Transaction;
use crate::errors::DomainError;

/// Repository trait for Transaction persistence.
pub trait TransactionRepository: Send + Sync {
    fn save(&self, transaction: &Transaction) -> Result<(), DomainError>;
    fn find_by_id(&self, id: Uuid) -> Result<Option<Transaction>, DomainError>;
    fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<Transaction>, DomainError>;
    fn find_by_date_range(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Transaction>, DomainError>;
    fn update(&self, transaction: &Transaction) -> Result<(), DomainError>;
    fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    fn find_pending_sync(&self) -> Result<Vec<Transaction>, DomainError>;
    fn calculate_balance(&self, account_id: Uuid) -> Result<i64, DomainError>;
}
