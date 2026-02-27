use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::entities::{Budget, BudgetPeriod, BudgetProgress, TransactionType};
use crate::errors::DomainError;
use crate::repositories::{BudgetRepository, TransactionRepository};

/// Use cases for managing budgets.
pub struct BudgetUseCases<'a> {
    budget_repo: &'a dyn BudgetRepository,
    transaction_repo: &'a dyn TransactionRepository,
}

impl<'a> BudgetUseCases<'a> {
    pub fn new(
        budget_repo: &'a dyn BudgetRepository,
        transaction_repo: &'a dyn TransactionRepository,
    ) -> Self {
        Self {
            budget_repo,
            transaction_repo,
        }
    }

    /// Create a new budget.
    pub fn create_budget(
        &self,
        account_id: Uuid,
        category_id: Option<Uuid>,
        name: String,
        amount: i64,
        period_str: &str,
        start_date: DateTime<Utc>,
    ) -> Result<Budget, DomainError> {
        let period = BudgetPeriod::from_str(period_str)
            .ok_or_else(|| DomainError::Validation("Invalid budget period".to_string()))?;

        let budget = Budget::new(account_id, category_id, name, amount, period, start_date)
            .map_err(DomainError::Validation)?;

        self.budget_repo.save(&budget)?;
        Ok(budget)
    }

    /// List budgets for an account.
    pub fn list_budgets(&self, account_id: Uuid) -> Result<Vec<Budget>, DomainError> {
        self.budget_repo.find_by_account_id(account_id)
    }

    /// Update a budget.
    pub fn update_budget(&self, budget: &Budget) -> Result<(), DomainError> {
        self.budget_repo.update(budget)
    }

    /// Delete a budget.
    pub fn delete_budget(&self, id: Uuid) -> Result<(), DomainError> {
        self.budget_repo.delete(id)
    }

    /// Get budget progress for a specific budget.
    pub fn get_budget_progress(&self, budget_id: Uuid) -> Result<BudgetProgress, DomainError> {
        let budget = self
            .budget_repo
            .find_by_id(budget_id)?
            .ok_or_else(|| DomainError::NotFound("Budget not found".to_string()))?;

        let period_end = budget.period_end_date();
        let period_start = budget.start_date;

        // Get transactions for the budget period
        let transactions = self
            .transaction_repo
            .find_by_date_range(budget.account_id, period_start, period_end)?;

        // Calculate total spending
        let mut spent = 0i64;
        for tx in transactions {
            if tx.base.is_deleted() {
                continue;
            }

            // Filter by category if budget is category-specific
            if let Some(cat_id) = budget.category_id {
                if tx.category_id != cat_id {
                    continue;
                }
            }

            // Only count expenses
            if tx.transaction_type == TransactionType::Expense {
                spent += tx.amount;
            }
        }

        let remaining = budget.amount - spent;
        let percentage = if budget.amount > 0 {
            (spent as f64 / budget.amount as f64) * 100.0
        } else {
            0.0
        };

        Ok(BudgetProgress {
            budget,
            spent,
            remaining,
            percentage,
            period_start,
            period_end,
        })
    }
}
