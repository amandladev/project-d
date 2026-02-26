use uuid::Uuid;

use crate::entities::Category;
use crate::errors::DomainError;
use crate::repositories::CategoryRepository;

/// Use cases for Category management.
pub struct CategoryUseCases<'a> {
    repo: &'a dyn CategoryRepository,
}

impl<'a> CategoryUseCases<'a> {
    pub fn new(repo: &'a dyn CategoryRepository) -> Self {
        Self { repo }
    }

    /// Create a new category.
    pub fn create_category(
        &self,
        user_id: Uuid,
        name: String,
        icon: Option<String>,
    ) -> Result<Category, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "Category name cannot be empty".to_string(),
            ));
        }

        let category = Category::new(user_id, name, icon);
        self.repo.save(&category)?;
        Ok(category)
    }

    /// Get a category by ID.
    pub fn get_category(&self, id: Uuid) -> Result<Category, DomainError> {
        self.repo
            .find_by_id(id)?
            .ok_or_else(|| DomainError::NotFound(format!("Category {id} not found")))
    }

    /// List all categories for a user.
    pub fn list_categories(&self, user_id: Uuid) -> Result<Vec<Category>, DomainError> {
        self.repo.find_by_user_id(user_id)
    }

    /// Soft-delete a category.
    pub fn delete_category(&self, id: Uuid) -> Result<(), DomainError> {
        let mut category = self.get_category(id)?;
        category.base.soft_delete();
        category.sync_status = crate::entities::SyncStatus::Pending;
        category.version += 1;
        self.repo.update(&category)
    }
}
