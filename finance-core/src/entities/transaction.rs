use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::common::{BaseEntity, SyncStatus, TransactionType};
use crate::errors::DomainError;

/// Represents a financial transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub account_id: Uuid,
    pub category_id: Uuid,
    pub amount: i64, // stored in cents to avoid floating point issues
    pub transaction_type: TransactionType,
    pub description: String,
    pub date: DateTime<Utc>,
    /// For transfers: the ID of the paired transaction on the other account.
    pub linked_transaction_id: Option<Uuid>,
    pub sync_status: SyncStatus,
    pub version: i64,
}

impl Transaction {
    /// Creates a new transaction with validation.
    /// `amount` is in cents (e.g., 1050 = $10.50).
    pub fn new(
        account_id: Uuid,
        category_id: Uuid,
        amount: i64,
        transaction_type: TransactionType,
        description: String,
        date: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::validate_amount(amount, transaction_type)?;

        Ok(Self {
            base: BaseEntity::new(),
            account_id,
            category_id,
            amount,
            transaction_type,
            description,
            date,
            linked_transaction_id: None,
            sync_status: SyncStatus::Pending,
            version: 1,
        })
    }

    pub fn id(&self) -> Uuid {
        self.base.id
    }

    /// Update the transaction fields with validation.
    pub fn update(
        &mut self,
        amount: i64,
        transaction_type: TransactionType,
        description: String,
        category_id: Uuid,
        date: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        Self::validate_amount(amount, transaction_type)?;

        self.amount = amount;
        self.transaction_type = transaction_type;
        self.description = description;
        self.category_id = category_id;
        self.date = date;
        self.sync_status = SyncStatus::Pending;
        self.version += 1;
        self.base.touch();

        Ok(())
    }

    /// Soft-delete this transaction.
    pub fn soft_delete(&mut self) {
        self.base.soft_delete();
        self.sync_status = SyncStatus::Pending;
        self.version += 1;
    }

    pub fn mark_synced(&mut self) {
        self.sync_status = SyncStatus::Synced;
        self.base.touch();
    }

    pub fn mark_pending(&mut self) {
        self.sync_status = SyncStatus::Pending;
        self.base.touch();
    }

    /// Validate amount based on transaction type.
    fn validate_amount(amount: i64, _tx_type: TransactionType) -> Result<(), DomainError> {
        if amount <= 0 {
            return Err(DomainError::InvalidAmount(
                "Amount must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }

    /// Returns the signed effect of this transaction on the account balance.
    /// Income adds, Expense subtracts, Transfer subtracts (from source account).
    pub fn balance_effect(&self) -> i64 {
        match self.transaction_type {
            TransactionType::Income => self.amount,
            TransactionType::Expense => -self.amount,
            TransactionType::Transfer => -self.amount,
        }
    }
}
