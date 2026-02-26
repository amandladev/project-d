use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::entities::{Transaction, TransactionType};
use crate::errors::DomainError;
use crate::repositories::TransactionRepository;

/// Use cases for Transaction management.
pub struct TransactionUseCases<'a> {
    repo: &'a dyn TransactionRepository,
}

impl<'a> TransactionUseCases<'a> {
    pub fn new(repo: &'a dyn TransactionRepository) -> Self {
        Self { repo }
    }

    /// Create a new transaction.
    pub fn create_transaction(
        &self,
        account_id: Uuid,
        category_id: Uuid,
        amount: i64,
        transaction_type: TransactionType,
        description: String,
        date: DateTime<Utc>,
    ) -> Result<Transaction, DomainError> {
        let transaction =
            Transaction::new(account_id, category_id, amount, transaction_type, description, date)?;
        self.repo.save(&transaction)?;
        Ok(transaction)
    }

    /// Edit an existing transaction.
    pub fn edit_transaction(
        &self,
        id: Uuid,
        amount: i64,
        transaction_type: TransactionType,
        description: String,
        category_id: Uuid,
        date: DateTime<Utc>,
    ) -> Result<Transaction, DomainError> {
        let mut transaction = self
            .repo
            .find_by_id(id)?
            .ok_or_else(|| DomainError::NotFound(format!("Transaction {id} not found")))?;

        if transaction.base.is_deleted() {
            return Err(DomainError::Validation(
                "Cannot edit a deleted transaction".to_string(),
            ));
        }

        transaction.update(amount, transaction_type, description, category_id, date)?;
        self.repo.update(&transaction)?;
        Ok(transaction)
    }

    /// Soft-delete a transaction.
    pub fn delete_transaction(&self, id: Uuid) -> Result<(), DomainError> {
        let mut transaction = self
            .repo
            .find_by_id(id)?
            .ok_or_else(|| DomainError::NotFound(format!("Transaction {id} not found")))?;

        transaction.soft_delete();
        self.repo.update(&transaction)
    }

    /// Get the balance for a specific account (computed from transactions).
    pub fn get_balance(&self, account_id: Uuid) -> Result<i64, DomainError> {
        self.repo.calculate_balance(account_id)
    }

    /// List transactions for an account within a date range.
    pub fn list_transactions_by_date_range(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Transaction>, DomainError> {
        if from > to {
            return Err(DomainError::Validation(
                "Start date must be before end date".to_string(),
            ));
        }
        self.repo.find_by_date_range(account_id, from, to)
    }

    /// List all transactions for an account.
    pub fn list_transactions(&self, account_id: Uuid) -> Result<Vec<Transaction>, DomainError> {
        self.repo.find_by_account_id(account_id)
    }

    /// Get pending sync transactions.
    pub fn get_pending_sync(&self) -> Result<Vec<Transaction>, DomainError> {
        self.repo.find_pending_sync()
    }
}
