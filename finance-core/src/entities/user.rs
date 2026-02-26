use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::common::BaseEntity;

/// Represents a user of the personal finance application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub name: String,
    pub email: String,
}

impl User {
    pub fn new(name: String, email: String) -> Self {
        Self {
            base: BaseEntity::new(),
            name,
            email,
        }
    }

    pub fn id(&self) -> Uuid {
        self.base.id
    }
}
