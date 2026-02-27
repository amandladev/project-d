#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use finance_core::entities::{Account, Category, TransactionType, User};
    use finance_core::repositories::{
        AccountRepository, CategoryRepository, TransactionRepository, UserRepository,
    };
    use finance_core::use_cases::{AccountUseCases, CategoryUseCases, StatisticsUseCases, TransactionUseCases};
    use finance_storage::database::Database;
    use finance_storage::repositories::{
        SqliteAccountRepository, SqliteCategoryRepository, SqliteTransactionRepository,
        SqliteUserRepository,
    };

    fn setup() -> Database {
        Database::open_in_memory().expect("Failed to create in-memory database")
    }

    fn create_test_user(db: &Database) -> User {
        let repo = SqliteUserRepository::new(db);
        let user = User::new("Test User".to_string(), format!("test-{}@example.com", Uuid::new_v4()));
        repo.save(&user).unwrap();
        user
    }

    fn create_test_account(db: &Database, user_id: Uuid) -> Account {
        let repo = SqliteAccountRepository::new(db);
        let use_cases = AccountUseCases::new(&repo);
        use_cases
            .create_account(user_id, "Test Account".to_string(), "USD".to_string())
            .unwrap()
    }

    fn create_test_category(db: &Database, user_id: Uuid) -> Category {
        let repo = SqliteCategoryRepository::new(db);
        let use_cases = CategoryUseCases::new(&repo);
        use_cases
            .create_category(user_id, "Food".to_string(), Some("🍕".to_string()))
            .unwrap()
    }

    // ─── User Repository Tests ───────────────────────────────────────────────

    #[test]
    fn test_user_save_and_find() {
        let db = setup();
        let repo = SqliteUserRepository::new(&db);

        let user = User::new("Alice".to_string(), "alice@example.com".to_string());
        repo.save(&user).unwrap();

        let found = repo.find_by_id(user.id()).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Alice");
    }

    #[test]
    fn test_user_find_by_email() {
        let db = setup();
        let repo = SqliteUserRepository::new(&db);

        let user = User::new("Bob".to_string(), "bob@example.com".to_string());
        repo.save(&user).unwrap();

        let found = repo.find_by_email("bob@example.com").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Bob");
    }

    #[test]
    fn test_user_soft_delete() {
        let db = setup();
        let repo = SqliteUserRepository::new(&db);

        let user = User::new("Charlie".to_string(), "charlie@example.com".to_string());
        repo.save(&user).unwrap();
        repo.delete(user.id()).unwrap();

        // Should not be found (soft-deleted)
        let found = repo.find_by_id(user.id()).unwrap();
        assert!(found.is_none());
    }

    // ─── Account Repository Tests ────────────────────────────────────────────

    #[test]
    fn test_account_create_and_find() {
        let db = setup();
        let user = create_test_user(&db);

        let repo = SqliteAccountRepository::new(&db);
        let use_cases = AccountUseCases::new(&repo);

        let account = use_cases
            .create_account(user.id(), "Checking".to_string(), "USD".to_string())
            .unwrap();

        let found = repo.find_by_id(account.id()).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Checking");
    }

    #[test]
    fn test_account_list_by_user() {
        let db = setup();
        let user = create_test_user(&db);

        let repo = SqliteAccountRepository::new(&db);
        let use_cases = AccountUseCases::new(&repo);

        use_cases
            .create_account(user.id(), "Checking".to_string(), "USD".to_string())
            .unwrap();
        use_cases
            .create_account(user.id(), "Savings".to_string(), "USD".to_string())
            .unwrap();

        let accounts = use_cases.list_accounts(user.id()).unwrap();
        assert_eq!(accounts.len(), 2);
    }

    #[test]
    fn test_account_empty_name_fails() {
        let db = setup();
        let user = create_test_user(&db);
        let repo = SqliteAccountRepository::new(&db);
        let use_cases = AccountUseCases::new(&repo);

        let result = use_cases.create_account(user.id(), "".to_string(), "USD".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_account_pending_sync() {
        let db = setup();
        let user = create_test_user(&db);
        let repo = SqliteAccountRepository::new(&db);
        let use_cases = AccountUseCases::new(&repo);

        use_cases
            .create_account(user.id(), "Account1".to_string(), "USD".to_string())
            .unwrap();

        let pending = repo.find_pending_sync().unwrap();
        assert_eq!(pending.len(), 1);
    }

    // ─── Category Repository Tests ───────────────────────────────────────────

    #[test]
    fn test_category_create_and_find() {
        let db = setup();
        let user = create_test_user(&db);

        let repo = SqliteCategoryRepository::new(&db);
        let use_cases = CategoryUseCases::new(&repo);

        let category = use_cases
            .create_category(user.id(), "Transport".to_string(), None)
            .unwrap();

        let found = repo.find_by_id(category.id()).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Transport");
    }

    // ─── Transaction Repository Tests ────────────────────────────────────────

    #[test]
    fn test_transaction_create_and_find() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let repo = SqliteTransactionRepository::new(&db);
        let use_cases = TransactionUseCases::new(&repo);

        let tx = use_cases
            .create_transaction(
                account.id(),
                category.id(),
                5000,
                TransactionType::Expense,
                "Dinner".to_string(),
                Utc::now(),
            )
            .unwrap();

        let found = repo.find_by_id(tx.id()).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().amount, 5000);
    }

    #[test]
    fn test_transaction_edit() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let repo = SqliteTransactionRepository::new(&db);
        let use_cases = TransactionUseCases::new(&repo);

        let tx = use_cases
            .create_transaction(
                account.id(),
                category.id(),
                1000,
                TransactionType::Expense,
                "Coffee".to_string(),
                Utc::now(),
            )
            .unwrap();

        let edited = use_cases
            .edit_transaction(
                tx.id(),
                2000,
                TransactionType::Income,
                "Refund".to_string(),
                category.id(),
                Utc::now(),
            )
            .unwrap();

        assert_eq!(edited.amount, 2000);
        assert_eq!(edited.description, "Refund");
        assert_eq!(edited.version, 2);
    }

    #[test]
    fn test_transaction_soft_delete() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let repo = SqliteTransactionRepository::new(&db);
        let use_cases = TransactionUseCases::new(&repo);

        let tx = use_cases
            .create_transaction(
                account.id(),
                category.id(),
                500,
                TransactionType::Expense,
                "Snack".to_string(),
                Utc::now(),
            )
            .unwrap();

        use_cases.delete_transaction(tx.id()).unwrap();

        // Should still exist in DB but with deleted_at set
        let found = repo.find_by_id(tx.id()).unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().base.is_deleted());

        // But should NOT appear in active account transactions
        let active = repo.find_by_account_id(account.id()).unwrap();
        assert_eq!(active.len(), 0);
    }

    #[test]
    fn test_balance_calculation() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let repo = SqliteTransactionRepository::new(&db);
        let use_cases = TransactionUseCases::new(&repo);

        // Income: +10000
        use_cases
            .create_transaction(
                account.id(),
                category.id(),
                10000,
                TransactionType::Income,
                "Salary".to_string(),
                Utc::now(),
            )
            .unwrap();

        // Expense: -3000
        use_cases
            .create_transaction(
                account.id(),
                category.id(),
                3000,
                TransactionType::Expense,
                "Rent".to_string(),
                Utc::now(),
            )
            .unwrap();

        // Transfer: -1500
        use_cases
            .create_transaction(
                account.id(),
                category.id(),
                1500,
                TransactionType::Transfer,
                "To savings".to_string(),
                Utc::now(),
            )
            .unwrap();

        let balance = use_cases.get_balance(account.id()).unwrap();
        assert_eq!(balance, 5500); // 10000 - 3000 - 1500
    }

    #[test]
    fn test_balance_excludes_deleted_transactions() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let repo = SqliteTransactionRepository::new(&db);
        let use_cases = TransactionUseCases::new(&repo);

        use_cases
            .create_transaction(
                account.id(),
                category.id(),
                10000,
                TransactionType::Income,
                "Salary".to_string(),
                Utc::now(),
            )
            .unwrap();

        let tx = use_cases
            .create_transaction(
                account.id(),
                category.id(),
                5000,
                TransactionType::Expense,
                "Mistake".to_string(),
                Utc::now(),
            )
            .unwrap();

        // Delete the expense
        use_cases.delete_transaction(tx.id()).unwrap();

        let balance = use_cases.get_balance(account.id()).unwrap();
        assert_eq!(balance, 10000); // Only the income remains
    }

    #[test]
    fn test_transaction_date_range_query() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let repo = SqliteTransactionRepository::new(&db);
        let use_cases = TransactionUseCases::new(&repo);

        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);
        let two_days_ago = now - chrono::Duration::days(2);

        use_cases
            .create_transaction(
                account.id(),
                category.id(),
                1000,
                TransactionType::Expense,
                "Yesterday".to_string(),
                yesterday,
            )
            .unwrap();

        use_cases
            .create_transaction(
                account.id(),
                category.id(),
                2000,
                TransactionType::Expense,
                "Today".to_string(),
                now,
            )
            .unwrap();

        // Query only yesterday to now
        let txs = use_cases
            .list_transactions_by_date_range(account.id(), two_days_ago, now)
            .unwrap();
        assert_eq!(txs.len(), 2);
    }

    #[test]
    fn test_pending_sync_transactions() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let repo = SqliteTransactionRepository::new(&db);
        let use_cases = TransactionUseCases::new(&repo);

        use_cases
            .create_transaction(
                account.id(),
                category.id(),
                1000,
                TransactionType::Expense,
                "Pending".to_string(),
                Utc::now(),
            )
            .unwrap();

        let pending = use_cases.get_pending_sync().unwrap();
        assert_eq!(pending.len(), 1);
    }

    // ─── Statistics Tests ────────────────────────────────────────────────────

    #[test]
    fn test_income_vs_expenses_date_range() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let repo = SqliteTransactionRepository::new(&db);
        let tx_use_cases = TransactionUseCases::new(&repo);

        let now = Utc::now();
        let start = now - Duration::days(30);
        let end = now + Duration::days(1);

        // Create income
        tx_use_cases
            .create_transaction(
                account.id(),
                category.id(),
                500_00,
                TransactionType::Income,
                "Salary".to_string(),
                now,
            )
            .unwrap();

        // Create expense
        tx_use_cases
            .create_transaction(
                account.id(),
                category.id(),
                200_00,
                TransactionType::Expense,
                "Groceries".to_string(),
                now,
            )
            .unwrap();

        // Create a transfer (should not count as income or expense)
        tx_use_cases
            .create_transaction(
                account.id(),
                category.id(),
                100_00,
                TransactionType::Transfer,
                "Move to savings".to_string(),
                now,
            )
            .unwrap();

        let stats_use_cases = StatisticsUseCases::new(&repo);
        let summary = stats_use_cases
            .get_income_vs_expenses(account.id(), start, end)
            .unwrap();

        assert_eq!(summary.income, 500_00);
        assert_eq!(summary.expenses, 200_00);
        assert_eq!(summary.transfers, 100_00);
        assert_eq!(summary.net, 200_00); // income - expenses - transfers
    }

    #[test]
    fn test_spending_by_category() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());

        let cat_repo = SqliteCategoryRepository::new(&db);
        let cat_use_cases = CategoryUseCases::new(&cat_repo);
        let food = cat_use_cases
            .create_category(user.id(), "Food".to_string(), Some("🍕".to_string()))
            .unwrap();
        let transport = cat_use_cases
            .create_category(user.id(), "Transport".to_string(), Some("🚗".to_string()))
            .unwrap();

        let repo = SqliteTransactionRepository::new(&db);
        let tx_use_cases = TransactionUseCases::new(&repo);

        let now = Utc::now();
        let start = now - Duration::days(30);
        let end = now + Duration::days(1);

        // Two food expenses
        tx_use_cases
            .create_transaction(
                account.id(), food.id(), 150_00,
                TransactionType::Expense, "Lunch".to_string(), now,
            )
            .unwrap();
        tx_use_cases
            .create_transaction(
                account.id(), food.id(), 80_00,
                TransactionType::Expense, "Dinner".to_string(), now,
            )
            .unwrap();

        // One transport expense
        tx_use_cases
            .create_transaction(
                account.id(), transport.id(), 50_00,
                TransactionType::Expense, "Uber".to_string(), now,
            )
            .unwrap();

        let stats = StatisticsUseCases::new(&repo);
        let spending = stats
            .get_spending_by_category(account.id(), start, end)
            .unwrap();

        assert_eq!(spending.len(), 2);

        let food_spend = spending.iter().find(|s| s.category_id == food.id()).unwrap();
        assert_eq!(food_spend.total_amount, 230_00);
        assert_eq!(food_spend.transaction_count, 2);

        let transport_spend = spending.iter().find(|s| s.category_id == transport.id()).unwrap();
        assert_eq!(transport_spend.total_amount, 50_00);
        assert_eq!(transport_spend.transaction_count, 1);
    }

    #[test]
    fn test_income_vs_expenses_excludes_out_of_range() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let repo = SqliteTransactionRepository::new(&db);
        let tx_use_cases = TransactionUseCases::new(&repo);

        let now = Utc::now();

        // Transaction inside range
        tx_use_cases
            .create_transaction(
                account.id(), category.id(), 100_00,
                TransactionType::Income, "In range".to_string(), now,
            )
            .unwrap();

        // Transaction outside range (60 days ago)
        tx_use_cases
            .create_transaction(
                account.id(), category.id(), 999_00,
                TransactionType::Income, "Out of range".to_string(),
                now - Duration::days(60),
            )
            .unwrap();

        // Query only last 30 days
        let stats = StatisticsUseCases::new(&repo);
        let summary = stats
            .get_income_vs_expenses(account.id(), now - Duration::days(30), now + Duration::days(1))
            .unwrap();

        assert_eq!(summary.income, 100_00);
        assert_eq!(summary.expenses, 0);
    }
}
