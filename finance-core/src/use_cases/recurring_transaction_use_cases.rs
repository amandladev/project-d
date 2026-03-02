use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::entities::pagination::{PageRequest, PaginatedResult};
use crate::entities::{RecurringTransaction, Transaction, TransactionType};
use crate::errors::DomainError;
use crate::repositories::{RecurringTransactionRepository, TransactionRepository};

/// Use cases for managing recurring transactions.
pub struct RecurringTransactionUseCases<'a> {
    recurring_repo: &'a dyn RecurringTransactionRepository,
    transaction_repo: &'a dyn TransactionRepository,
}

impl<'a> RecurringTransactionUseCases<'a> {
    pub fn new(
        recurring_repo: &'a dyn RecurringTransactionRepository,
        transaction_repo: &'a dyn TransactionRepository,
    ) -> Self {
        Self {
            recurring_repo,
            transaction_repo,
        }
    }

    /// Create a new recurring transaction template.
    pub fn create_recurring_transaction(
        &self,
        account_id: Uuid,
        category_id: Uuid,
        amount: i64,
        transaction_type: TransactionType,
        description: String,
        frequency_str: &str,
        start_date: DateTime<Utc>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<RecurringTransaction, DomainError> {
        use crate::entities::RecurrenceFrequency;

        let frequency = RecurrenceFrequency::from_str(frequency_str)
            .ok_or_else(|| DomainError::Validation("Invalid frequency".to_string()))?;

        let recurring = RecurringTransaction::new(
            account_id,
            category_id,
            amount,
            transaction_type,
            description,
            frequency,
            start_date,
            end_date,
        )
        .map_err(DomainError::Validation)?;

        self.recurring_repo.save(&recurring)?;
        Ok(recurring)
    }

    /// List recurring transactions for an account.
    pub fn list_recurring_transactions(
        &self,
        account_id: Uuid,
    ) -> Result<Vec<RecurringTransaction>, DomainError> {
        self.recurring_repo.find_by_account_id(account_id)
    }

    /// List recurring transactions for an account (paginated).
    pub fn list_recurring_transactions_paginated(
        &self,
        account_id: Uuid,
        page: &PageRequest,
    ) -> Result<PaginatedResult<RecurringTransaction>, DomainError> {
        self.recurring_repo.find_by_account_id_paginated(account_id, page)
    }

    /// Update a recurring transaction.
    pub fn update_recurring_transaction(
        &self,
        recurring: &RecurringTransaction,
    ) -> Result<(), DomainError> {
        self.recurring_repo.update(recurring)
    }

    /// Soft delete a recurring transaction.
    pub fn delete_recurring_transaction(&self, id: Uuid) -> Result<(), DomainError> {
        self.recurring_repo.delete(id)
    }

    /// Process all due recurring transactions and create actual transactions.
    /// Returns the IDs of transactions that were created.
    pub fn process_due_recurring_transactions(&self) -> Result<Vec<Uuid>, DomainError> {
        let now = Utc::now();
        let due_recurrings = self.recurring_repo.find_due(now)?;

        let mut created_ids = Vec::new();

        for mut recurring in due_recurrings {
            if recurring.is_expired() {
                continue;
            }

            // Create the actual transaction
            let transaction = Transaction::new(
                recurring.account_id,
                recurring.category_id,
                recurring.amount,
                recurring.transaction_type,
                recurring.description.clone(),
                recurring.next_occurrence,
            )?;

            self.transaction_repo.save(&transaction)?;
            created_ids.push(transaction.base.id);

            // Update next occurrence
            recurring.next_occurrence = recurring.calculate_next_occurrence();
            self.recurring_repo.update(&recurring)?;
        }

        Ok(created_ids)
    }
}
