use chrono::Utc;
use uuid::Uuid;

use finance_core::entities::common::{SyncStatus, TransactionType};
use finance_core::entities::{Account, Category, Transaction};
use finance_core::errors::DomainError;
use finance_core::repositories::{AccountRepository, CategoryRepository, TransactionRepository};

use crate::{SyncEngine, SyncError, SyncPayload, SyncResponse, SyncTransport, ServerChange};

// ─── Mock Transport ──────────────────────────────────────────────────────────

struct MockTransport {
    push_response: std::sync::Mutex<Option<SyncResponse>>,
    pull_response: std::sync::Mutex<Option<Vec<ServerChange>>>,
}

impl MockTransport {
    fn new() -> Self {
        Self {
            push_response: std::sync::Mutex::new(None),
            pull_response: std::sync::Mutex::new(None),
        }
    }

    fn set_push_response(&self, response: SyncResponse) {
        *self.push_response.lock().unwrap() = Some(response);
    }

    fn set_pull_response(&self, changes: Vec<ServerChange>) {
        *self.pull_response.lock().unwrap() = Some(changes);
    }
}

impl SyncTransport for MockTransport {
    fn push(&self, _payload: &SyncPayload) -> Result<SyncResponse, SyncError> {
        self.push_response
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| SyncError::Network("No mock response configured".to_string()))
    }

    fn pull(&self) -> Result<Vec<ServerChange>, SyncError> {
        self.pull_response
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| SyncError::Network("No mock pull response configured".to_string()))
    }
}

// ─── In-Memory Repositories ─────────────────────────────────────────────────

struct InMemoryAccountRepository {
    accounts: std::sync::Mutex<Vec<Account>>,
}

impl InMemoryAccountRepository {
    fn new() -> Self {
        Self {
            accounts: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl AccountRepository for InMemoryAccountRepository {
    fn save(&self, account: &Account) -> Result<(), DomainError> {
        self.accounts.lock().unwrap().push(account.clone());
        Ok(())
    }

    fn find_by_id(&self, id: Uuid) -> Result<Option<Account>, DomainError> {
        Ok(self
            .accounts
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.base.id == id)
            .cloned())
    }

    fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Account>, DomainError> {
        Ok(self
            .accounts
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.user_id == user_id && !a.base.is_deleted())
            .cloned()
            .collect())
    }

    fn update(&self, account: &Account) -> Result<(), DomainError> {
        let mut accounts = self.accounts.lock().unwrap();
        if let Some(existing) = accounts.iter_mut().find(|a| a.base.id == account.base.id) {
            *existing = account.clone();
        }
        Ok(())
    }

    fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut accounts = self.accounts.lock().unwrap();
        if let Some(account) = accounts.iter_mut().find(|a| a.base.id == id) {
            account.base.soft_delete();
        }
        Ok(())
    }

    fn find_pending_sync(&self) -> Result<Vec<Account>, DomainError> {
        Ok(self
            .accounts
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.sync_status == SyncStatus::Pending)
            .cloned()
            .collect())
    }
}

struct InMemoryCategoryRepository {
    categories: std::sync::Mutex<Vec<Category>>,
}

impl InMemoryCategoryRepository {
    fn new() -> Self {
        Self {
            categories: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl CategoryRepository for InMemoryCategoryRepository {
    fn save(&self, category: &Category) -> Result<(), DomainError> {
        self.categories.lock().unwrap().push(category.clone());
        Ok(())
    }

    fn find_by_id(&self, id: Uuid) -> Result<Option<Category>, DomainError> {
        Ok(self
            .categories
            .lock()
            .unwrap()
            .iter()
            .find(|c| c.base.id == id)
            .cloned())
    }

    fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Category>, DomainError> {
        Ok(self
            .categories
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.user_id == user_id && !c.base.is_deleted())
            .cloned()
            .collect())
    }

    fn update(&self, category: &Category) -> Result<(), DomainError> {
        let mut categories = self.categories.lock().unwrap();
        if let Some(existing) = categories
            .iter_mut()
            .find(|c| c.base.id == category.base.id)
        {
            *existing = category.clone();
        }
        Ok(())
    }

    fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut categories = self.categories.lock().unwrap();
        if let Some(category) = categories.iter_mut().find(|c| c.base.id == id) {
            category.base.soft_delete();
        }
        Ok(())
    }

    fn find_pending_sync(&self) -> Result<Vec<Category>, DomainError> {
        Ok(self
            .categories
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.sync_status == SyncStatus::Pending)
            .cloned()
            .collect())
    }
}

struct InMemoryTransactionRepository {
    transactions: std::sync::Mutex<Vec<Transaction>>,
}

impl InMemoryTransactionRepository {
    fn new() -> Self {
        Self {
            transactions: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl TransactionRepository for InMemoryTransactionRepository {
    fn save(&self, transaction: &Transaction) -> Result<(), DomainError> {
        self.transactions.lock().unwrap().push(transaction.clone());
        Ok(())
    }

    fn find_by_id(&self, id: Uuid) -> Result<Option<Transaction>, DomainError> {
        Ok(self
            .transactions
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.base.id == id)
            .cloned())
    }

    fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<Transaction>, DomainError> {
        Ok(self
            .transactions
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.account_id == account_id && !t.base.is_deleted())
            .cloned()
            .collect())
    }

    fn find_by_date_range(
        &self,
        account_id: Uuid,
        from: chrono::DateTime<Utc>,
        to: chrono::DateTime<Utc>,
    ) -> Result<Vec<Transaction>, DomainError> {
        Ok(self
            .transactions
            .lock()
            .unwrap()
            .iter()
            .filter(|t| {
                t.account_id == account_id
                    && t.date >= from
                    && t.date <= to
                    && !t.base.is_deleted()
            })
            .cloned()
            .collect())
    }

    fn update(&self, transaction: &Transaction) -> Result<(), DomainError> {
        let mut transactions = self.transactions.lock().unwrap();
        if let Some(existing) = transactions
            .iter_mut()
            .find(|t| t.base.id == transaction.base.id)
        {
            *existing = transaction.clone();
        }
        Ok(())
    }

    fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut transactions = self.transactions.lock().unwrap();
        if let Some(transaction) = transactions.iter_mut().find(|t| t.base.id == id) {
            transaction.soft_delete();
        }
        Ok(())
    }

    fn find_pending_sync(&self) -> Result<Vec<Transaction>, DomainError> {
        Ok(self
            .transactions
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.sync_status == SyncStatus::Pending)
            .cloned()
            .collect())
    }

    fn calculate_balance(&self, account_id: Uuid) -> Result<i64, DomainError> {
        Ok(self
            .transactions
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.account_id == account_id && !t.base.is_deleted())
            .map(|t| t.balance_effect())
            .sum())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn test_collect_pending_changes_empty() {
    let account_repo = InMemoryAccountRepository::new();
    let category_repo = InMemoryCategoryRepository::new();
    let transaction_repo = InMemoryTransactionRepository::new();
    let transport = MockTransport::new();

    let engine = SyncEngine::new(&account_repo, &category_repo, &transaction_repo, &transport);

    let payload = engine.collect_pending_changes().unwrap();
    assert!(payload.accounts.is_empty());
    assert!(payload.categories.is_empty());
    assert!(payload.transactions.is_empty());
}

#[test]
fn test_collect_pending_changes_with_data() {
    let account_repo = InMemoryAccountRepository::new();
    let category_repo = InMemoryCategoryRepository::new();
    let transaction_repo = InMemoryTransactionRepository::new();
    let transport = MockTransport::new();

    let user_id = Uuid::new_v4();
    let account = Account::new(user_id, "Checking".to_string(), "USD".to_string());
    account_repo.save(&account).unwrap();

    let tx = Transaction::new(
        account.id(),
        Uuid::new_v4(),
        5000,
        TransactionType::Expense,
        "Dinner".to_string(),
        Utc::now(),
    )
    .unwrap();
    transaction_repo.save(&tx).unwrap();

    let engine = SyncEngine::new(&account_repo, &category_repo, &transaction_repo, &transport);

    let payload = engine.collect_pending_changes().unwrap();
    assert_eq!(payload.accounts.len(), 1);
    assert_eq!(payload.transactions.len(), 1);
}

#[test]
fn test_serialize_payload() {
    let account_repo = InMemoryAccountRepository::new();
    let category_repo = InMemoryCategoryRepository::new();
    let transaction_repo = InMemoryTransactionRepository::new();
    let transport = MockTransport::new();

    let user_id = Uuid::new_v4();
    let account = Account::new(user_id, "Test".to_string(), "USD".to_string());
    account_repo.save(&account).unwrap();

    let engine = SyncEngine::new(&account_repo, &category_repo, &transaction_repo, &transport);

    let payload = engine.collect_pending_changes().unwrap();
    let json = engine.serialize_payload(&payload).unwrap();

    assert!(json.contains("Test"));
    assert!(json.contains("USD"));
}

#[test]
fn test_push_changes_marks_synced() {
    let account_repo = InMemoryAccountRepository::new();
    let category_repo = InMemoryCategoryRepository::new();
    let transaction_repo = InMemoryTransactionRepository::new();
    let transport = MockTransport::new();

    let user_id = Uuid::new_v4();
    let account = Account::new(user_id, "Checking".to_string(), "USD".to_string());
    let account_id = account.id().to_string();
    account_repo.save(&account).unwrap();

    // Server accepts the account
    transport.set_push_response(SyncResponse {
        accepted: vec![account_id.clone()],
        conflicts: vec![],
    });
    transport.set_pull_response(vec![]);

    let engine = SyncEngine::new(&account_repo, &category_repo, &transaction_repo, &transport);

    let result = engine.push_changes().unwrap();
    assert_eq!(result.pushed, 1);
    assert_eq!(result.conflicts, 0);

    // Verify the account is now synced
    let updated = account_repo
        .find_by_id(Uuid::parse_str(&account_id).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(updated.sync_status, SyncStatus::Synced);
}

#[test]
fn test_sync_with_no_pending_returns_zero() {
    let account_repo = InMemoryAccountRepository::new();
    let category_repo = InMemoryCategoryRepository::new();
    let transaction_repo = InMemoryTransactionRepository::new();
    let transport = MockTransport::new();

    transport.set_pull_response(vec![]);

    let engine = SyncEngine::new(&account_repo, &category_repo, &transaction_repo, &transport);

    let result = engine.sync().unwrap();
    assert_eq!(result.pushed, 0);
    assert_eq!(result.conflicts, 0);
    assert_eq!(result.pulled, 0);
}

#[test]
fn test_full_sync_flow() {
    let account_repo = InMemoryAccountRepository::new();
    let category_repo = InMemoryCategoryRepository::new();
    let transaction_repo = InMemoryTransactionRepository::new();
    let transport = MockTransport::new();

    // Step 1: Create a transaction offline (sync_status = pending)
    let user_id = Uuid::new_v4();
    let account = Account::new(user_id, "Main".to_string(), "USD".to_string());
    let account_id = account.id();
    account_repo.save(&account).unwrap();

    let tx = Transaction::new(
        account_id,
        Uuid::new_v4(),
        7500,
        TransactionType::Expense,
        "Groceries".to_string(),
        Utc::now(),
    )
    .unwrap();
    let tx_id = tx.id();
    transaction_repo.save(&tx).unwrap();

    // Verify pending
    let pending = transaction_repo.find_pending_sync().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].sync_status, SyncStatus::Pending);

    // Step 2: Server accepts both entities
    transport.set_push_response(SyncResponse {
        accepted: vec![account_id.to_string(), tx_id.to_string()],
        conflicts: vec![],
    });
    transport.set_pull_response(vec![]);

    let engine = SyncEngine::new(&account_repo, &category_repo, &transaction_repo, &transport);

    // Step 3: Execute sync
    let result = engine.sync().unwrap();
    assert_eq!(result.pushed, 2); // 1 account + 1 transaction
    assert_eq!(result.conflicts, 0);

    // Step 4: Verify both are marked as synced
    let synced_account = account_repo.find_by_id(account_id).unwrap().unwrap();
    assert_eq!(synced_account.sync_status, SyncStatus::Synced);

    let synced_tx = transaction_repo.find_by_id(tx_id).unwrap().unwrap();
    assert_eq!(synced_tx.sync_status, SyncStatus::Synced);

    // Step 5: No more pending
    let pending = transaction_repo.find_pending_sync().unwrap();
    assert_eq!(pending.len(), 0);
}
