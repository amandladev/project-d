use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::DomainError;
use crate::repositories::TransactionRepository;

/// Spending by category aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySpending {
    pub category_id: Uuid,
    pub category_name: String,
    pub total_amount: i64,
    pub transaction_count: usize,
}

/// Monthly trend data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyTrend {
    pub year: i32,
    pub month: u32,
    pub income: i64,
    pub expenses: i64,
    pub net: i64,
    pub transaction_count: usize,
}

/// Daily spending data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySpending {
    pub date: String, // "YYYY-MM-DD"
    pub amount: i64,
    pub transaction_count: usize,
}

/// Statistics use cases.
pub struct StatisticsUseCases<'a> {
    transaction_repo: &'a dyn TransactionRepository,
}

impl<'a> StatisticsUseCases<'a> {
    pub fn new(transaction_repo: &'a dyn TransactionRepository) -> Self {
        Self { transaction_repo }
    }

    /// Get spending aggregated by category for a user within a date range.
    pub fn get_spending_by_category(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CategorySpending>, DomainError> {
        if from > to {
            return Err(DomainError::Validation(
                "Start date must be before end date".to_string(),
            ));
        }

        self.transaction_repo
            .get_spending_by_category(account_id, from, to)
    }

    /// Get income vs expenses summary for a date range.
    pub fn get_income_vs_expenses(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<IncomeSummary, DomainError> {
        if from > to {
            return Err(DomainError::Validation(
                "Start date must be before end date".to_string(),
            ));
        }

        self.transaction_repo
            .get_income_vs_expenses(account_id, from, to)
    }

    /// Get month-over-month income/expense trends.
    /// Returns one MonthlyTrend per month within the date range.
    pub fn get_monthly_trends(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<MonthlyTrend>, DomainError> {
        if from > to {
            return Err(DomainError::Validation(
                "Start date must be before end date".to_string(),
            ));
        }

        self.transaction_repo
            .get_monthly_trends(account_id, from, to)
    }

    /// Get daily expense totals within a date range.
    /// Useful for bar/line charts of daily spending.
    pub fn get_daily_spending(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<DailySpending>, DomainError> {
        if from > to {
            return Err(DomainError::Validation(
                "Start date must be before end date".to_string(),
            ));
        }

        self.transaction_repo
            .get_daily_spending(account_id, from, to)
    }
}

/// Income vs expenses summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeSummary {
    pub income: i64,
    pub expenses: i64,
    pub transfers: i64,
    pub net: i64,
}
