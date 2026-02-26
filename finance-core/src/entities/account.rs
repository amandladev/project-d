use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::common::{BaseEntity, SyncStatus};

/// Represents a financial account (e.g., bank account, wallet, credit card).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub user_id: Uuid,
    pub name: String,
    pub currency: String,
    pub sync_status: SyncStatus,
    pub version: i64,
}

impl Account {
    pub fn new(user_id: Uuid, name: String, currency: String) -> Self {
        Self {
            base: BaseEntity::new(),
            user_id,
            name,
            currency,
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
