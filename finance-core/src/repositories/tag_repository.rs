use uuid::Uuid;

use crate::entities::Tag;
use crate::errors::DomainError;

/// Repository trait for Tag persistence.
pub trait TagRepository: Send + Sync {
    /// Save a new tag.
    fn save(&self, tag: &Tag) -> Result<(), DomainError>;

    /// Find a tag by ID.
    fn find_by_id(&self, id: Uuid) -> Result<Option<Tag>, DomainError>;

    /// Find all tags for a user.
    fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Tag>, DomainError>;

    /// Update a tag.
    fn update(&self, tag: &Tag) -> Result<(), DomainError>;

    /// Delete a tag.
    fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Add a tag to a transaction.
    fn add_tag_to_transaction(
        &self,
        transaction_id: Uuid,
        tag_id: Uuid,
    ) -> Result<(), DomainError>;

    /// Remove a tag from a transaction.
    fn remove_tag_from_transaction(
        &self,
        transaction_id: Uuid,
        tag_id: Uuid,
    ) -> Result<(), DomainError>;

    /// Get all tags for a transaction.
    fn find_tags_for_transaction(&self, transaction_id: Uuid) -> Result<Vec<Tag>, DomainError>;

    /// Get all transaction IDs that have a given tag.
    fn find_transaction_ids_by_tag(&self, tag_id: Uuid) -> Result<Vec<Uuid>, DomainError>;
}
