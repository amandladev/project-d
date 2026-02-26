use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::common::{BaseEntity, SyncStatus};

/// Represents a transaction category (e.g., Food, Transportation, Salary).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub user_id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub sync_status: SyncStatus,
    pub version: i64,
}

impl Category {
    pub fn new(user_id: Uuid, name: String, icon: Option<String>) -> Self {
        Self {
            base: BaseEntity::new(),
            user_id,
            name,
            icon,
            sync_status: SyncStatus::Pending,
            version: 1,
        }
    }

    pub fn id(&self) -> Uuid {
        self.base.id
    }

    pub fn mark_synced(&mut self) {
        self.sync_status = SyncStatus::Synced;
        self.base.touch();
    }

    pub fn mark_pending(&mut self) {
        self.sync_status = SyncStatus::Pending;
        self.base.touch();
    }
}
