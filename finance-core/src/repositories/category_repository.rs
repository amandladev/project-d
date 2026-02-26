use uuid::Uuid;

use crate::entities::Category;
use crate::errors::DomainError;

/// Repository trait for Category persistence.
pub trait CategoryRepository: Send + Sync {
    fn save(&self, category: &Category) -> Result<(), DomainError>;
    fn find_by_id(&self, id: Uuid) -> Result<Option<Category>, DomainError>;
    fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Category>, DomainError>;
    fn update(&self, category: &Category) -> Result<(), DomainError>;
    fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    fn find_pending_sync(&self) -> Result<Vec<Category>, DomainError>;
}
