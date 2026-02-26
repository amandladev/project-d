#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use finance_core::entities::common::{BaseEntity, SyncStatus, TransactionType};
    use finance_core::entities::transaction::Transaction;
    use finance_core::entities::{Account, Category, User};

    #[test]
    fn test_base_entity_creation() {
        let base = BaseEntity::new();
        assert!(!base.is_deleted());
        assert!(base.deleted_at.is_none());
    }

    #[test]
    fn test_base_entity_soft_delete() {
        let mut base = BaseEntity::new();
        base.soft_delete();
        assert!(base.is_deleted());
        assert!(base.deleted_at.is_some());
    }

    #[test]
    fn test_base_entity_touch_updates_timestamp() {
        let mut base = BaseEntity::new();
        let original = base.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        base.touch();
        assert!(base.updated_at >= original);
    }

    #[test]
    fn test_user_creation() {
        let user = User::new("Alice".to_string(), "alice@example.com".to_string());
        assert_eq!(user.name, "Alice");
        assert_eq!(user.email, "alice@example.com");
        assert!(!user.base.is_deleted());
    }

    #[test]
    fn test_account_creation() {
        let user_id = Uuid::new_v4();
        let account = Account::new(user_id, "Checking".to_string(), "USD".to_string());
        assert_eq!(account.name, "Checking");
        assert_eq!(account.currency, "USD");
        assert_eq!(account.sync_status, SyncStatus::Pending);
        assert_eq!(account.version, 1);
    }

    #[test]
    fn test_account_mark_synced() {
        let user_id = Uuid::new_v4();
        let mut account = Account::new(user_id, "Savings".to_string(), "EUR".to_string());
        account.mark_synced();
        assert_eq!(account.sync_status, SyncStatus::Synced);
    }

    #[test]
    fn test_category_creation() {
        let user_id = Uuid::new_v4();
        let cat = Category::new(user_id, "Food".to_string(), Some("🍕".to_string()));
        assert_eq!(cat.name, "Food");
        assert_eq!(cat.icon, Some("🍕".to_string()));
        assert_eq!(cat.sync_status, SyncStatus::Pending);
    }

    #[test]
    fn test_transaction_creation_valid() {
        let tx = Transaction::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            5000,
            TransactionType::Expense,
            "Dinner".to_string(),
            Utc::now(),
        );
        assert!(tx.is_ok());
        let tx = tx.unwrap();
        assert_eq!(tx.amount, 5000);
        assert_eq!(tx.transaction_type, TransactionType::Expense);
        assert_eq!(tx.sync_status, SyncStatus::Pending);
        assert_eq!(tx.version, 1);
    }

    #[test]
    fn test_transaction_creation_zero_amount_fails() {
        let tx = Transaction::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            0,
            TransactionType::Income,
            "Invalid".to_string(),
            Utc::now(),
        );
        assert!(tx.is_err());
    }

    #[test]
    fn test_transaction_creation_negative_amount_fails() {
        let tx = Transaction::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            -100,
            TransactionType::Expense,
            "Negative".to_string(),
            Utc::now(),
        );
        assert!(tx.is_err());
    }

    #[test]
    fn test_transaction_update() {
        let mut tx = Transaction::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000,
            TransactionType::Expense,
            "Lunch".to_string(),
            Utc::now(),
        )
        .unwrap();

        let new_cat = Uuid::new_v4();
        let result = tx.update(
            2000,
            TransactionType::Income,
            "Salary".to_string(),
            new_cat,
            Utc::now(),
        );

        assert!(result.is_ok());
        assert_eq!(tx.amount, 2000);
        assert_eq!(tx.transaction_type, TransactionType::Income);
        assert_eq!(tx.description, "Salary");
        assert_eq!(tx.version, 2);
        assert_eq!(tx.sync_status, SyncStatus::Pending);
    }

    #[test]
    fn test_transaction_soft_delete() {
        let mut tx = Transaction::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            500,
            TransactionType::Expense,
            "Coffee".to_string(),
            Utc::now(),
        )
        .unwrap();

        tx.soft_delete();
        assert!(tx.base.is_deleted());
        assert_eq!(tx.sync_status, SyncStatus::Pending);
        assert_eq!(tx.version, 2);
    }

    #[test]
    fn test_transaction_balance_effect() {
        let income = Transaction::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            10000,
            TransactionType::Income,
            "Salary".to_string(),
            Utc::now(),
        )
        .unwrap();
        assert_eq!(income.balance_effect(), 10000);

        let expense = Transaction::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            3000,
            TransactionType::Expense,
            "Rent".to_string(),
            Utc::now(),
        )
        .unwrap();
        assert_eq!(expense.balance_effect(), -3000);

        let transfer = Transaction::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1500,
            TransactionType::Transfer,
            "To savings".to_string(),
            Utc::now(),
        )
        .unwrap();
        assert_eq!(transfer.balance_effect(), -1500);
    }

    #[test]
    fn test_sync_status_display() {
        assert_eq!(SyncStatus::Pending.to_string(), "pending");
        assert_eq!(SyncStatus::Synced.to_string(), "synced");
        assert_eq!(SyncStatus::Conflicted.to_string(), "conflicted");
    }

    #[test]
    fn test_transaction_type_display() {
        assert_eq!(TransactionType::Expense.to_string(), "expense");
        assert_eq!(TransactionType::Income.to_string(), "income");
        assert_eq!(TransactionType::Transfer.to_string(), "transfer");
    }

    #[test]
    fn test_sync_status_from_str() {
        assert_eq!(SyncStatus::from_str("pending"), Some(SyncStatus::Pending));
        assert_eq!(SyncStatus::from_str("synced"), Some(SyncStatus::Synced));
        assert_eq!(SyncStatus::from_str("conflicted"), Some(SyncStatus::Conflicted));
        assert_eq!(SyncStatus::from_str("unknown"), None);
    }

    #[test]
    fn test_transaction_type_from_str() {
        assert_eq!(TransactionType::from_str("expense"), Some(TransactionType::Expense));
        assert_eq!(TransactionType::from_str("income"), Some(TransactionType::Income));
        assert_eq!(TransactionType::from_str("transfer"), Some(TransactionType::Transfer));
        assert_eq!(TransactionType::from_str("unknown"), None);
    }
}
