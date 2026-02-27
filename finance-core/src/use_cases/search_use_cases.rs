use crate::entities::search::TransactionSearchFilter;
use crate::entities::Transaction;
use crate::errors::DomainError;
use crate::repositories::TransactionRepository;

/// Use cases for searching and filtering transactions.
pub struct SearchUseCases<'a> {
    transaction_repo: &'a dyn TransactionRepository,
}

impl<'a> SearchUseCases<'a> {
    pub fn new(transaction_repo: &'a dyn TransactionRepository) -> Self {
        Self { transaction_repo }
    }

    /// Search transactions with flexible filtering.
    pub fn search_transactions(
        &self,
        filter: &TransactionSearchFilter,
    ) -> Result<Vec<Transaction>, DomainError> {
        // Validate filter
        if let (Some(min), Some(max)) = (filter.min_amount, filter.max_amount) {
            if min > max {
                return Err(DomainError::Validation(
                    "min_amount must be <= max_amount".to_string(),
                ));
            }
        }

        if let (Some(from), Some(to)) = (filter.date_from, filter.date_to) {
            if from > to {
                return Err(DomainError::Validation(
                    "date_from must be <= date_to".to_string(),
                ));
            }
        }

        self.transaction_repo.search(filter)
    }
}
