use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::entities::Transaction;
use crate::entities::pagination::{PageRequest, PaginatedResult};
use crate::entities::search::TransactionSearchFilter;
use crate::errors::DomainError;
use crate::use_cases::statistics_use_cases::{CategorySpending, DailySpending, IncomeSummary, MonthlyTrend};

/// Repository trait for Transaction persistence.
pub trait TransactionRepository: Send + Sync {
    fn save(&self, transaction: &Transaction) -> Result<(), DomainError>;
    fn find_by_id(&self, id: Uuid) -> Result<Option<Transaction>, DomainError>;
    fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<Transaction>, DomainError>;
    fn find_by_account_id_paginated(
        &self,
        account_id: Uuid,
        page: &PageRequest,
    ) -> Result<PaginatedResult<Transaction>, DomainError>;
    fn find_by_date_range(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Transaction>, DomainError>;
    fn find_by_date_range_paginated(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        page: &PageRequest,
    ) -> Result<PaginatedResult<Transaction>, DomainError>;
    fn update(&self, transaction: &Transaction) -> Result<(), DomainError>;
    fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    fn find_pending_sync(&self) -> Result<Vec<Transaction>, DomainError>;
    fn calculate_balance(&self, account_id: Uuid) -> Result<i64, DomainError>;
    fn get_spending_by_category(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CategorySpending>, DomainError>;
    /// Search transactions with flexible filtering.
    fn search(
        &self,
        filter: &TransactionSearchFilter,
    ) -> Result<PaginatedResult<Transaction>, DomainError>;

    /// Get income vs expenses aggregated directly in SQL.
    fn get_income_vs_expenses(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<IncomeSummary, DomainError>;

    /// Get monthly trends aggregated directly in SQL.
    fn get_monthly_trends(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<MonthlyTrend>, DomainError>;

    /// Get daily spending aggregated directly in SQL.
    fn get_daily_spending(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<DailySpending>, DomainError>;

    /// Get total expense amount for budget progress calculation (SQL SUM).
    fn get_budget_spent(
        &self,
        account_id: Uuid,
        category_id: Option<Uuid>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<i64, DomainError>;
}
