use uuid::Uuid;

use crate::entities::Account;
use crate::errors::DomainError;

/// Repository trait for Account persistence.
pub trait AccountRepository: Send + Sync {
    fn save(&self, account: &Account) -> Result<(), DomainError>;
    fn find_by_id(&self, id: Uuid) -> Result<Option<Account>, DomainError>;
    fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Account>, DomainError>;
    fn update(&self, account: &Account) -> Result<(), DomainError>;
    fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    fn find_pending_sync(&self) -> Result<Vec<Account>, DomainError>;
}
