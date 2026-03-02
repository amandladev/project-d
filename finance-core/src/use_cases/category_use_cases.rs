use uuid::Uuid;

use crate::entities::Category;
use crate::errors::DomainError;
use crate::repositories::CategoryRepository;

/// Default categories seeded on first launch.
const DEFAULT_CATEGORIES: &[(&str, &str)] = &[
    ("Salary", "💼"),
    ("Food & Dining", "🍔"),
    ("Groceries", "🛒"),
    ("Transportation", "🚗"),
    ("Housing & Rent", "🏠"),
    ("Utilities", "⚡"),
    ("Entertainment", "🎬"),
    ("Shopping", "👕"),
    ("Health", "💊"),
    ("Education", "📚"),
    ("Travel", "✈️"),
    ("Subscriptions", "📱"),
    ("Fitness", "🏋️"),
    ("Coffee", "☕"),
    ("Gifts", "🎁"),
    ("Freelance", "💻"),
    ("Investments", "📈"),
    ("Other", "💰"),
];

/// Use cases for Category management.
pub struct CategoryUseCases<'a> {
    repo: &'a dyn CategoryRepository,
}

impl<'a> CategoryUseCases<'a> {
    pub fn new(repo: &'a dyn CategoryRepository) -> Self {
        Self { repo }
    }

    /// Seed default categories for a user if none exist yet.
    /// Returns the list of categories (existing or newly created).
    /// This is idempotent — calling it multiple times is safe.
    pub fn seed_default_categories(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Category>, DomainError> {
        let existing = self.repo.find_by_user_id(user_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }

        let mut categories = Vec::with_capacity(DEFAULT_CATEGORIES.len());
        for &(name, icon) in DEFAULT_CATEGORIES {
            let category = Category::new(
                user_id,
                name.to_string(),
                Some(icon.to_string()),
            );
            self.repo.save(&category)?;
            categories.push(category);
        }

        Ok(categories)
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

    /// Update a category's name and/or icon.
    pub fn update_category(
        &self,
        id: Uuid,
        name: Option<String>,
        icon: Option<Option<String>>,
    ) -> Result<Category, DomainError> {
        let mut category = self.get_category(id)?;

        if let Some(n) = name {
            if n.trim().is_empty() {
                return Err(DomainError::Validation(
                    "Category name cannot be empty".to_string(),
                ));
            }
            category.name = n;
        }

        if let Some(i) = icon {
            category.icon = i;
        }

        category.base.touch();
        category.sync_status = crate::entities::SyncStatus::Pending;
        category.version += 1;
        self.repo.update(&category)?;
        Ok(category)
    }
}
