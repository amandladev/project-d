use uuid::Uuid;

use crate::entities::User;
use crate::errors::DomainError;

/// Repository trait for User persistence.
pub trait UserRepository: Send + Sync {
    fn save(&self, user: &User) -> Result<(), DomainError>;
    fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError>;
    fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError>;
    fn update(&self, user: &User) -> Result<(), DomainError>;
    fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}
