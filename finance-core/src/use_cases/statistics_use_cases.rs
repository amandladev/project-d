use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::TransactionType;
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

        let transactions = self
            .transaction_repo
            .find_by_date_range(account_id, from, to)?;

        let mut income = 0i64;
        let mut expenses = 0i64;
        let mut transfers = 0i64;

        for tx in transactions {
            if tx.base.is_deleted() {
                continue;
            }
            match tx.transaction_type {
                TransactionType::Income => income += tx.amount,
                TransactionType::Expense => expenses += tx.amount,
                TransactionType::Transfer => transfers += tx.amount,
            }
        }

        Ok(IncomeSummary {
            income,
            expenses,
            transfers,
            net: income - expenses - transfers,
        })
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
