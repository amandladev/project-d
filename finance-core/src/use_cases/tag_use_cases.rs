use uuid::Uuid;

use crate::entities::pagination::{PageRequest, PaginatedResult};
use crate::entities::Tag;
use crate::errors::DomainError;
use crate::repositories::TagRepository;

/// Use cases for Tag management.
pub struct TagUseCases<'a> {
    repo: &'a dyn TagRepository,
}

impl<'a> TagUseCases<'a> {
    pub fn new(repo: &'a dyn TagRepository) -> Self {
        Self { repo }
    }

    /// Create a new tag.
    pub fn create_tag(
        &self,
        user_id: Uuid,
        name: String,
        color: Option<String>,
    ) -> Result<Tag, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "Tag name cannot be empty".to_string(),
            ));
        }

        // Validate color format if provided (e.g., "#FF5733")
        if let Some(ref c) = color {
            if !c.starts_with('#') || (c.len() != 7 && c.len() != 4) {
                return Err(DomainError::Validation(
                    "Color must be a hex string like #FF5733 or #F53".to_string(),
                ));
            }
        }

        let tag = Tag::new(user_id, name, color);
        self.repo.save(&tag)?;
        Ok(tag)
    }

    /// Get a tag by ID.
    pub fn get_tag(&self, id: Uuid) -> Result<Tag, DomainError> {
        self.repo
            .find_by_id(id)?
            .ok_or_else(|| DomainError::NotFound(format!("Tag {id} not found")))
    }

    /// List all tags for a user.
    pub fn list_tags(&self, user_id: Uuid) -> Result<Vec<Tag>, DomainError> {
        self.repo.find_by_user_id(user_id)
    }

    /// Update a tag's name and/or color.
    pub fn update_tag(
        &self,
        id: Uuid,
        name: Option<String>,
        color: Option<Option<String>>,
    ) -> Result<Tag, DomainError> {
        let mut tag = self.get_tag(id)?;

        if let Some(new_name) = name {
            if new_name.trim().is_empty() {
                return Err(DomainError::Validation(
                    "Tag name cannot be empty".to_string(),
                ));
            }
            tag.name = new_name;
        }

        if let Some(new_color) = color {
            if let Some(ref c) = new_color {
                if !c.starts_with('#') || (c.len() != 7 && c.len() != 4) {
                    return Err(DomainError::Validation(
                        "Color must be a hex string like #FF5733 or #F53".to_string(),
                    ));
                }
            }
            tag.color = new_color;
        }

        tag.base.touch();
        self.repo.update(&tag)?;
        Ok(tag)
    }

    /// Delete a tag (also removes all transaction associations).
    pub fn delete_tag(&self, id: Uuid) -> Result<(), DomainError> {
        self.repo.delete(id)
    }

    /// Add a tag to a transaction.
    pub fn add_tag_to_transaction(
        &self,
        transaction_id: Uuid,
        tag_id: Uuid,
    ) -> Result<(), DomainError> {
        // Verify the tag exists
        self.get_tag(tag_id)?;
        self.repo.add_tag_to_transaction(transaction_id, tag_id)
    }

    /// Remove a tag from a transaction.
    pub fn remove_tag_from_transaction(
        &self,
        transaction_id: Uuid,
        tag_id: Uuid,
    ) -> Result<(), DomainError> {
        self.repo.remove_tag_from_transaction(transaction_id, tag_id)
    }

    /// Get all tags for a transaction.
    pub fn get_transaction_tags(&self, transaction_id: Uuid) -> Result<Vec<Tag>, DomainError> {
        self.repo.find_tags_for_transaction(transaction_id)
    }

    /// Get all transaction IDs that have a given tag.
    pub fn get_transactions_by_tag(&self, tag_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        self.repo.find_transaction_ids_by_tag(tag_id)
    }

    /// Get paginated transaction IDs that have a given tag.
    pub fn get_transactions_by_tag_paginated(
        &self,
        tag_id: Uuid,
        page: &PageRequest,
    ) -> Result<PaginatedResult<Uuid>, DomainError> {
        self.repo.find_transaction_ids_by_tag_paginated(tag_id, page)
    }
}
