use uuid::Uuid;

use crate::entities::Budget;
use crate::errors::DomainError;

/// Repository for budgets.
pub trait BudgetRepository: Send + Sync {
    fn save(&self, budget: &Budget) -> Result<(), DomainError>;
    fn find_by_id(&self, id: Uuid) -> Result<Option<Budget>, DomainError>;
    fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<Budget>, DomainError>;
    fn update(&self, budget: &Budget) -> Result<(), DomainError>;
    fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}
