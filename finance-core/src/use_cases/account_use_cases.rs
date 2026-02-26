use uuid::Uuid;

use crate::entities::Account;
use crate::errors::DomainError;
use crate::repositories::AccountRepository;

/// Use cases for Account management.
pub struct AccountUseCases<'a> {
    repo: &'a dyn AccountRepository,
}

impl<'a> AccountUseCases<'a> {
    pub fn new(repo: &'a dyn AccountRepository) -> Self {
        Self { repo }
    }

    /// Create a new account.
    pub fn create_account(
        &self,
        user_id: Uuid,
        name: String,
        currency: String,
    ) -> Result<Account, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "Account name cannot be empty".to_string(),
            ));
        }
        if currency.trim().is_empty() {
            return Err(DomainError::Validation(
                "Currency cannot be empty".to_string(),
            ));
        }

        let account = Account::new(user_id, name, currency);
        self.repo.save(&account)?;
        Ok(account)
    }

    /// Get an account by ID.
    pub fn get_account(&self, id: Uuid) -> Result<Account, DomainError> {
        self.repo
            .find_by_id(id)?
            .ok_or_else(|| DomainError::NotFound(format!("Account {id} not found")))
    }

    /// List all accounts for a user.
    pub fn list_accounts(&self, user_id: Uuid) -> Result<Vec<Account>, DomainError> {
        self.repo.find_by_user_id(user_id)
    }

    /// Soft-delete an account.
    pub fn delete_account(&self, id: Uuid) -> Result<(), DomainError> {
        let mut account = self.get_account(id)?;
        account.base.soft_delete();
        account.sync_status = crate::entities::SyncStatus::Pending;
        account.version += 1;
        self.repo.update(&account)
    }
}
