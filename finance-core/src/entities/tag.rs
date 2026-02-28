use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::common::BaseEntity;

/// A tag/label that can be attached to transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
}

impl Tag {
    pub fn new(user_id: Uuid, name: String, color: Option<String>) -> Self {
        Self {
            base: BaseEntity::new(),
            user_id,
            name,
            color,
        }
    }

    pub fn id(&self) -> Uuid {
        self.base.id
    }
}

/// Association between a transaction and a tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionTag {
    pub transaction_id: Uuid,
    pub tag_id: Uuid,
}
