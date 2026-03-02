use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::entities::RecurringTransaction;
use crate::entities::pagination::{PageRequest, PaginatedResult};
use crate::errors::DomainError;

/// Repository for recurring transactions.
pub trait RecurringTransactionRepository: Send + Sync {
    fn save(&self, recurring: &RecurringTransaction) -> Result<(), DomainError>;
    fn find_by_id(&self, id: Uuid) -> Result<Option<RecurringTransaction>, DomainError>;
    fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<RecurringTransaction>, DomainError>;
    fn find_by_account_id_paginated(
        &self,
        account_id: Uuid,
        page: &PageRequest,
    ) -> Result<PaginatedResult<RecurringTransaction>, DomainError>;
    fn find_active(&self) -> Result<Vec<RecurringTransaction>, DomainError>;
    fn find_due(&self, before: DateTime<Utc>) -> Result<Vec<RecurringTransaction>, DomainError>;
    fn update(&self, recurring: &RecurringTransaction) -> Result<(), DomainError>;
    fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}
