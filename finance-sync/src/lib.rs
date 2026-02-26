use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use finance_core::entities::{Account, Category, SyncStatus, Transaction};
use finance_core::errors::DomainError;
use finance_core::repositories::{AccountRepository, CategoryRepository, TransactionRepository};

/// Sync engine errors.
#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
}

/// Represents a batch of changes to be sent to the remote server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPayload {
    pub accounts: Vec<Account>,
    pub categories: Vec<Category>,
    pub transactions: Vec<Transaction>,
}

/// Represents a single entity change from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerChange {
    pub entity_type: String,
    pub entity_id: String,
    pub data: serde_json::Value,
    pub version: i64,
    pub server_updated_at: String,
}

/// Response from the remote server after a sync push.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub accepted: Vec<String>,   // IDs that were accepted
    pub conflicts: Vec<ServerChange>, // conflicting changes from server
}

/// Trait for the remote sync transport (HTTP, WebSocket, etc.).
/// This can be stubbed for testing.
pub trait SyncTransport: Send + Sync {
    fn push(&self, payload: &SyncPayload) -> Result<SyncResponse, SyncError>;
    fn pull(&self) -> Result<Vec<ServerChange>, SyncError>;
}

/// The sync engine coordinates offline-first synchronization.
pub struct SyncEngine<'a> {
    account_repo: &'a dyn AccountRepository,
    category_repo: &'a dyn CategoryRepository,
    transaction_repo: &'a dyn TransactionRepository,
    transport: &'a dyn SyncTransport,
}

impl<'a> SyncEngine<'a> {
    pub fn new(
        account_repo: &'a dyn AccountRepository,
        category_repo: &'a dyn CategoryRepository,
        transaction_repo: &'a dyn TransactionRepository,
        transport: &'a dyn SyncTransport,
    ) -> Self {
        Self {
            account_repo,
            category_repo,
            transaction_repo,
            transport,
        }
    }

    /// Collect all pending changes into a JSON-serializable payload.
    pub fn collect_pending_changes(&self) -> Result<SyncPayload, SyncError> {
        let accounts = self.account_repo.find_pending_sync()?;
        let categories = self.category_repo.find_pending_sync()?;
        let transactions = self.transaction_repo.find_pending_sync()?;

        Ok(SyncPayload {
            accounts,
            categories,
            transactions,
        })
    }

    /// Serialize the pending changes to JSON.
    pub fn serialize_payload(&self, payload: &SyncPayload) -> Result<String, SyncError> {
        serde_json::to_string(payload)
            .map_err(|e| SyncError::Serialization(e.to_string()))
    }

    /// Push pending changes to the server and process the response.
    pub fn push_changes(&self) -> Result<SyncResult, SyncError> {
        let payload = self.collect_pending_changes()?;

        if payload.accounts.is_empty()
            && payload.categories.is_empty()
            && payload.transactions.is_empty()
        {
            return Ok(SyncResult {
                pushed: 0,
                conflicts: 0,
                pulled: 0,
            });
        }

        let total_pushed = payload.accounts.len()
            + payload.categories.len()
            + payload.transactions.len();

        let response = self.transport.push(&payload)?;

        // Mark accepted entities as synced
        for id_str in &response.accepted {
            if let Ok(id) = Uuid::parse_str(id_str) {
                self.mark_entity_synced(id)?;
            }
        }

        // Handle conflicts using Last Write Wins strategy
        let conflict_count = response.conflicts.len();
        for conflict in &response.conflicts {
            self.resolve_conflict_lww(conflict)?;
        }

        Ok(SyncResult {
            pushed: total_pushed,
            conflicts: conflict_count,
            pulled: 0,
        })
    }

    /// Pull changes from the server and apply them locally.
    pub fn pull_changes(&self) -> Result<usize, SyncError> {
        let changes = self.transport.pull()?;
        let count = changes.len();

        for change in &changes {
            self.apply_server_change(change)?;
        }

        Ok(count)
    }

    /// Execute a full sync cycle: push then pull.
    pub fn sync(&self) -> Result<SyncResult, SyncError> {
        let mut result = self.push_changes()?;
        let pulled = self.pull_changes()?;
        result.pulled = pulled;
        Ok(result)
    }

    /// Mark an entity as synced across all repositories.
    fn mark_entity_synced(&self, id: Uuid) -> Result<(), SyncError> {
        // Try each repository — only one will match
        if let Ok(Some(mut account)) = self.account_repo.find_by_id(id) {
            account.mark_synced();
            self.account_repo.update(&account)?;
            return Ok(());
        }
        if let Ok(Some(mut category)) = self.category_repo.find_by_id(id) {
            category.mark_synced();
            self.category_repo.update(&category)?;
            return Ok(());
        }
        if let Ok(Some(mut transaction)) = self.transaction_repo.find_by_id(id) {
            transaction.mark_synced();
            self.transaction_repo.update(&transaction)?;
            return Ok(());
        }
        Ok(())
    }

    /// Resolve a conflict using Last Write Wins strategy.
    /// The server version wins if its updated_at is more recent.
    fn resolve_conflict_lww(&self, change: &ServerChange) -> Result<(), SyncError> {
        if let Ok(id) = Uuid::parse_str(&change.entity_id) {
            match change.entity_type.as_str() {
                "transaction" => {
                    if let Ok(Some(mut local)) = self.transaction_repo.find_by_id(id) {
                        // Last Write Wins: server version takes precedence
                        if change.version > local.version {
                            if let Ok(server_tx) =
                                serde_json::from_value::<Transaction>(change.data.clone())
                            {
                                local.amount = server_tx.amount;
                                local.transaction_type = server_tx.transaction_type;
                                local.description = server_tx.description;
                                local.category_id = server_tx.category_id;
                                local.date = server_tx.date;
                                local.version = change.version;
                                local.sync_status = SyncStatus::Synced;
                                self.transaction_repo.update(&local)?;
                            }
                        } else {
                            // Local version wins, mark as pending to re-push
                            local.sync_status = SyncStatus::Pending;
                            self.transaction_repo.update(&local)?;
                        }
                    }
                }
                "account" => {
                    if let Ok(Some(mut local)) = self.account_repo.find_by_id(id) {
                        if change.version > local.version {
                            if let Ok(server_acc) =
                                serde_json::from_value::<Account>(change.data.clone())
                            {
                                local.name = server_acc.name;
                                local.currency = server_acc.currency;
                                local.version = change.version;
                                local.sync_status = SyncStatus::Synced;
                                self.account_repo.update(&local)?;
                            }
                        } else {
                            local.sync_status = SyncStatus::Pending;
                            self.account_repo.update(&local)?;
                        }
                    }
                }
                "category" => {
                    if let Ok(Some(mut local)) = self.category_repo.find_by_id(id) {
                        if change.version > local.version {
                            if let Ok(server_cat) =
                                serde_json::from_value::<Category>(change.data.clone())
                            {
                                local.name = server_cat.name;
                                local.icon = server_cat.icon;
                                local.version = change.version;
                                local.sync_status = SyncStatus::Synced;
                                self.category_repo.update(&local)?;
                            }
                        } else {
                            local.sync_status = SyncStatus::Pending;
                            self.category_repo.update(&local)?;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Apply a server change locally (from pull).
    fn apply_server_change(&self, change: &ServerChange) -> Result<(), SyncError> {
        // For pull, we trust the server data (it's already resolved server-side)
        self.resolve_conflict_lww(change)
    }
}

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub pushed: usize,
    pub conflicts: usize,
    pub pulled: usize,
}

#[cfg(test)]
mod tests;
