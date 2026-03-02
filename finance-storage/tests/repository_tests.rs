#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use finance_core::entities::{Account, Category, TransactionType, User};
    use finance_core::entities::exchange_rate::RateSource;
    use finance_core::entities::search::TransactionSearchFilter;
    use finance_core::repositories::{
        AccountRepository, CategoryRepository, ExchangeRateRepository,
        TagRepository, TransactionRepository, UserRepository,
    };
    use finance_core::use_cases::{
        AccountUseCases, CategoryUseCases, CurrencyUseCases, SearchUseCases,
        StatisticsUseCases, TagUseCases, TransactionUseCases,
    };
    use finance_storage::database::Database;
    use finance_storage::repositories::{
        SqliteAccountRepository, SqliteCategoryRepository, SqliteExchangeRateRepository,
        SqliteTagRepository, SqliteTransactionRepository, SqliteUserRepository,
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

    #[test]
    fn test_seed_default_categories() {
        let db = setup();
        let user = create_test_user(&db);

        let repo = SqliteCategoryRepository::new(&db);
        let use_cases = CategoryUseCases::new(&repo);

        // First call should create 18 default categories
        let categories = use_cases.seed_default_categories(user.id()).unwrap();
        assert_eq!(categories.len(), 18);
        assert_eq!(categories[0].name, "Salary");
        assert_eq!(categories[0].icon, Some("💼".to_string()));
        assert_eq!(categories[17].name, "Other");
        assert_eq!(categories[17].icon, Some("💰".to_string()));
    }

    #[test]
    fn test_seed_default_categories_idempotent() {
        let db = setup();
        let user = create_test_user(&db);

        let repo = SqliteCategoryRepository::new(&db);
        let use_cases = CategoryUseCases::new(&repo);

        // Seed once
        let first = use_cases.seed_default_categories(user.id()).unwrap();
        assert_eq!(first.len(), 18);

        // Seed again — should return existing, not duplicate
        let second = use_cases.seed_default_categories(user.id()).unwrap();
        assert_eq!(second.len(), 18);

        // Verify same IDs
        assert_eq!(first[0].id(), second[0].id());
    }

    #[test]
    fn test_seed_skipped_when_categories_exist() {
        let db = setup();
        let user = create_test_user(&db);

        let repo = SqliteCategoryRepository::new(&db);
        let use_cases = CategoryUseCases::new(&repo);

        // Create one custom category first
        use_cases
            .create_category(user.id(), "My Custom".to_string(), Some("🔥".to_string()))
            .unwrap();

        // Seed should not add defaults since categories already exist
        let categories = use_cases.seed_default_categories(user.id()).unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].name, "My Custom");
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

    // ─── Currency Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_seed_bundled_rates() {
        let db = setup();
        let repo = SqliteExchangeRateRepository::new(&db);
        let use_cases = CurrencyUseCases::new(&repo);

        let count = use_cases.seed_bundled_rates().unwrap();
        assert!(count > 0);

        // Should find USD→EUR
        let rate = repo.find_best_rate("USD", "EUR").unwrap();
        assert!(rate.is_some());
        let rate = rate.unwrap();
        assert_eq!(rate.source, RateSource::Bundled);
    }

    #[test]
    fn test_convert_currency() {
        let db = setup();
        let repo = SqliteExchangeRateRepository::new(&db);
        let use_cases = CurrencyUseCases::new(&repo);

        use_cases.seed_bundled_rates().unwrap();

        // Convert $100.00 to EUR
        let result = use_cases.convert(10_000, "USD", "EUR").unwrap();
        assert_eq!(result.from_currency, "USD");
        assert_eq!(result.to_currency, "EUR");
        assert!(result.converted_amount > 0);
        assert!(result.converted_amount < 10_000); // EUR is worth more
    }

    #[test]
    fn test_same_currency_conversion() {
        let db = setup();
        let repo = SqliteExchangeRateRepository::new(&db);
        let use_cases = CurrencyUseCases::new(&repo);

        // No need to seed — same currency always works
        let result = use_cases.convert(5_000, "USD", "USD").unwrap();
        assert_eq!(result.converted_amount, 5_000);
    }

    #[test]
    fn test_user_override_takes_priority() {
        let db = setup();
        let repo = SqliteExchangeRateRepository::new(&db);
        let use_cases = CurrencyUseCases::new(&repo);

        use_cases.seed_bundled_rates().unwrap();

        // Set a manual rate (user override)
        use_cases.set_manual_rate("USD", "EUR", 0.95).unwrap();

        // Should use the override rate, not the bundled one
        let rate = repo.find_best_rate("USD", "EUR").unwrap().unwrap();
        assert_eq!(rate.source, RateSource::UserOverride);

        let result = use_cases.convert(10_000, "USD", "EUR").unwrap();
        assert_eq!(result.converted_amount, 9_500); // 0.95 * 10000
    }

    #[test]
    fn test_update_cached_rates() {
        let db = setup();
        let repo = SqliteExchangeRateRepository::new(&db);
        let use_cases = CurrencyUseCases::new(&repo);

        use_cases.seed_bundled_rates().unwrap();

        // Simulate API response
        let json = r#"[{"from":"USD","to":"EUR","rate":0.91},{"from":"USD","to":"GBP","rate":0.78}]"#;
        let count = use_cases.update_cached_rates(json).unwrap();
        assert_eq!(count, 2);

        // Cached rate should now have priority over bundled
        let rate = repo.find_best_rate("USD", "EUR").unwrap().unwrap();
        assert_eq!(rate.source, RateSource::Cached);
    }

    #[test]
    fn test_rate_freshness() {
        let db = setup();
        let repo = SqliteExchangeRateRepository::new(&db);
        let use_cases = CurrencyUseCases::new(&repo);

        use_cases.seed_bundled_rates().unwrap();

        let freshness = use_cases.get_rate_freshness("USD", "EUR").unwrap();
        assert!(freshness.is_some());
        let freshness = freshness.unwrap();
        assert!(freshness.age_seconds >= 0);
    }

    // ─── Search & Filtering Tests ────────────────────────────────────────────

    #[test]
    fn test_search_by_description() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let repo = SqliteTransactionRepository::new(&db);
        let tx_use_cases = TransactionUseCases::new(&repo);

        let now = Utc::now();
        tx_use_cases.create_transaction(
            account.id(), category.id(), 100_00,
            TransactionType::Expense, "Coffee at Starbucks".to_string(), now,
        ).unwrap();
        tx_use_cases.create_transaction(
            account.id(), category.id(), 50_00,
            TransactionType::Expense, "Netflix subscription".to_string(), now,
        ).unwrap();

        let search = SearchUseCases::new(&repo);
        let filter = TransactionSearchFilter {
            account_id: account.id(),
            query: Some("coffee".to_string()),
            ..Default::default()
        };

        let results = search.search_transactions(&filter).unwrap();
        assert_eq!(results.items.len(), 1);
        assert!(results.items[0].description.contains("Coffee"));
    }

    #[test]
    fn test_search_by_amount_range() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let repo = SqliteTransactionRepository::new(&db);
        let tx_use_cases = TransactionUseCases::new(&repo);

        let now = Utc::now();
        tx_use_cases.create_transaction(
            account.id(), category.id(), 10_00,
            TransactionType::Expense, "Small".to_string(), now,
        ).unwrap();
        tx_use_cases.create_transaction(
            account.id(), category.id(), 500_00,
            TransactionType::Expense, "Medium".to_string(), now,
        ).unwrap();
        tx_use_cases.create_transaction(
            account.id(), category.id(), 5000_00,
            TransactionType::Expense, "Large".to_string(), now,
        ).unwrap();

        let search = SearchUseCases::new(&repo);
        let filter = TransactionSearchFilter {
            account_id: account.id(),
            min_amount: Some(100_00),
            max_amount: Some(1000_00),
            ..Default::default()
        };

        let results = search.search_transactions(&filter).unwrap();
        assert_eq!(results.items.len(), 1);
        assert_eq!(results.items[0].description, "Medium");
    }

    #[test]
    fn test_search_by_transaction_type() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let repo = SqliteTransactionRepository::new(&db);
        let tx_use_cases = TransactionUseCases::new(&repo);

        let now = Utc::now();
        tx_use_cases.create_transaction(
            account.id(), category.id(), 100_00,
            TransactionType::Income, "Salary".to_string(), now,
        ).unwrap();
        tx_use_cases.create_transaction(
            account.id(), category.id(), 50_00,
            TransactionType::Expense, "Lunch".to_string(), now,
        ).unwrap();

        let search = SearchUseCases::new(&repo);
        let filter = TransactionSearchFilter {
            account_id: account.id(),
            transaction_type: Some(TransactionType::Income),
            ..Default::default()
        };

        let results = search.search_transactions(&filter).unwrap();
        assert_eq!(results.items.len(), 1);
        assert_eq!(results.items[0].description, "Salary");
    }

    #[test]
    fn test_search_with_pagination() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let repo = SqliteTransactionRepository::new(&db);
        let tx_use_cases = TransactionUseCases::new(&repo);

        let now = Utc::now();
        for i in 0..5 {
            tx_use_cases.create_transaction(
                account.id(), category.id(), (i + 1) * 100,
                TransactionType::Expense, format!("Item {i}"), now,
            ).unwrap();
        }

        let search = SearchUseCases::new(&repo);

        // First page
        let filter = TransactionSearchFilter {
            account_id: account.id(),
            limit: Some(2),
            offset: Some(0),
            ..Default::default()
        };
        let page1 = search.search_transactions(&filter).unwrap();
        assert_eq!(page1.items.len(), 2);
        assert!(page1.has_more);
        assert_eq!(page1.total_count, 5);

        // Second page
        let filter = TransactionSearchFilter {
            account_id: account.id(),
            limit: Some(2),
            offset: Some(2),
            ..Default::default()
        };
        let page2 = search.search_transactions(&filter).unwrap();
        assert_eq!(page2.items.len(), 2);
        assert!(page2.has_more);

        // Third page (partial)
        let filter = TransactionSearchFilter {
            account_id: account.id(),
            limit: Some(2),
            offset: Some(4),
            ..Default::default()
        };
        let page3 = search.search_transactions(&filter).unwrap();
        assert_eq!(page3.items.len(), 1);
        assert!(!page3.has_more);
    }

    #[test]
    fn test_search_combined_filters() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let cat_repo = SqliteCategoryRepository::new(&db);
        let cat_uc = CategoryUseCases::new(&cat_repo);
        let transport = cat_uc.create_category(
            user.id(), "Transport".to_string(), Some("🚗".to_string()),
        ).unwrap();

        let repo = SqliteTransactionRepository::new(&db);
        let tx_use_cases = TransactionUseCases::new(&repo);

        let now = Utc::now();
        tx_use_cases.create_transaction(
            account.id(), category.id(), 200_00,
            TransactionType::Expense, "Pizza delivery".to_string(), now,
        ).unwrap();
        tx_use_cases.create_transaction(
            account.id(), transport.id(), 300_00,
            TransactionType::Expense, "Uber ride".to_string(), now,
        ).unwrap();
        tx_use_cases.create_transaction(
            account.id(), category.id(), 50_00,
            TransactionType::Income, "Refund pizza".to_string(), now,
        ).unwrap();

        let search = SearchUseCases::new(&repo);

        // Search for expenses in Food category containing "pizza"
        let filter = TransactionSearchFilter {
            account_id: account.id(),
            query: Some("pizza".to_string()),
            category_id: Some(category.id()),
            transaction_type: Some(TransactionType::Expense),
            ..Default::default()
        };

        let results = search.search_transactions(&filter).unwrap();
        assert_eq!(results.items.len(), 1);
        assert_eq!(results.items[0].description, "Pizza delivery");
    }

    // ─── Tag Repository Tests ────────────────────────────────────────────────

    #[test]
    fn test_tag_create_and_find() {
        let db = setup();
        let user = create_test_user(&db);

        let repo = SqliteTagRepository::new(&db);
        let use_cases = TagUseCases::new(&repo);

        let tag = use_cases
            .create_tag(user.id(), "Vacation".to_string(), Some("#FF5733".to_string()))
            .unwrap();

        let found = repo.find_by_id(tag.id()).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.name, "Vacation");
        assert_eq!(found.color, Some("#FF5733".to_string()));
    }

    #[test]
    fn test_tag_list_by_user() {
        let db = setup();
        let user = create_test_user(&db);

        let repo = SqliteTagRepository::new(&db);
        let use_cases = TagUseCases::new(&repo);

        use_cases.create_tag(user.id(), "Work".to_string(), None).unwrap();
        use_cases.create_tag(user.id(), "Personal".to_string(), Some("#00FF00".to_string())).unwrap();

        let tags = use_cases.list_tags(user.id()).unwrap();
        assert_eq!(tags.len(), 2);
        // Sorted alphabetically
        assert_eq!(tags[0].name, "Personal");
        assert_eq!(tags[1].name, "Work");
    }

    #[test]
    fn test_tag_update() {
        let db = setup();
        let user = create_test_user(&db);

        let repo = SqliteTagRepository::new(&db);
        let use_cases = TagUseCases::new(&repo);

        let tag = use_cases
            .create_tag(user.id(), "Old Name".to_string(), Some("#000000".to_string()))
            .unwrap();

        let updated = use_cases
            .update_tag(tag.id(), Some("New Name".to_string()), Some(Some("#FFFFFF".to_string())))
            .unwrap();

        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.color, Some("#FFFFFF".to_string()));
    }

    #[test]
    fn test_tag_delete_removes_associations() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let tag_repo = SqliteTagRepository::new(&db);
        let tag_uc = TagUseCases::new(&tag_repo);

        let tx_repo = SqliteTransactionRepository::new(&db);
        let tx_uc = TransactionUseCases::new(&tx_repo);

        let tag = tag_uc.create_tag(user.id(), "ToDelete".to_string(), None).unwrap();
        let tx = tx_uc.create_transaction(
            account.id(), category.id(), 1000,
            TransactionType::Expense, "Test".to_string(), Utc::now(),
        ).unwrap();

        tag_uc.add_tag_to_transaction(tx.id(), tag.id()).unwrap();

        // Verify tag is there
        let tags = tag_uc.get_transaction_tags(tx.id()).unwrap();
        assert_eq!(tags.len(), 1);

        // Delete tag — should also remove association
        tag_uc.delete_tag(tag.id()).unwrap();

        let tags_after = tag_uc.get_transaction_tags(tx.id()).unwrap();
        assert_eq!(tags_after.len(), 0);
    }

    #[test]
    fn test_tag_transaction_association() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let tag_repo = SqliteTagRepository::new(&db);
        let tag_uc = TagUseCases::new(&tag_repo);

        let tx_repo = SqliteTransactionRepository::new(&db);
        let tx_uc = TransactionUseCases::new(&tx_repo);

        let tag1 = tag_uc.create_tag(user.id(), "Food".to_string(), None).unwrap();
        let tag2 = tag_uc.create_tag(user.id(), "Restaurant".to_string(), None).unwrap();

        let tx1 = tx_uc.create_transaction(
            account.id(), category.id(), 2000,
            TransactionType::Expense, "Lunch".to_string(), Utc::now(),
        ).unwrap();
        let tx2 = tx_uc.create_transaction(
            account.id(), category.id(), 3000,
            TransactionType::Expense, "Dinner".to_string(), Utc::now(),
        ).unwrap();

        // Tag both transactions with "Food"
        tag_uc.add_tag_to_transaction(tx1.id(), tag1.id()).unwrap();
        tag_uc.add_tag_to_transaction(tx2.id(), tag1.id()).unwrap();
        // Tag tx1 with "Restaurant" too
        tag_uc.add_tag_to_transaction(tx1.id(), tag2.id()).unwrap();

        // tx1 should have 2 tags
        let tags_tx1 = tag_uc.get_transaction_tags(tx1.id()).unwrap();
        assert_eq!(tags_tx1.len(), 2);

        // tx2 should have 1 tag
        let tags_tx2 = tag_uc.get_transaction_tags(tx2.id()).unwrap();
        assert_eq!(tags_tx2.len(), 1);

        // "Food" tag should be on 2 transactions
        let tx_ids = tag_uc.get_transactions_by_tag(tag1.id()).unwrap();
        assert_eq!(tx_ids.len(), 2);

        // Remove "Food" from tx2
        tag_uc.remove_tag_from_transaction(tx2.id(), tag1.id()).unwrap();
        let tx_ids_after = tag_uc.get_transactions_by_tag(tag1.id()).unwrap();
        assert_eq!(tx_ids_after.len(), 1);
    }

    #[test]
    fn test_tag_add_duplicate_ignored() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let tag_repo = SqliteTagRepository::new(&db);
        let tag_uc = TagUseCases::new(&tag_repo);

        let tx_repo = SqliteTransactionRepository::new(&db);
        let tx_uc = TransactionUseCases::new(&tx_repo);

        let tag = tag_uc.create_tag(user.id(), "Dup".to_string(), None).unwrap();
        let tx = tx_uc.create_transaction(
            account.id(), category.id(), 500,
            TransactionType::Expense, "Test".to_string(), Utc::now(),
        ).unwrap();

        // Add same tag twice — should not error (INSERT OR IGNORE)
        tag_uc.add_tag_to_transaction(tx.id(), tag.id()).unwrap();
        tag_uc.add_tag_to_transaction(tx.id(), tag.id()).unwrap();

        let tags = tag_uc.get_transaction_tags(tx.id()).unwrap();
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn test_tag_empty_name_rejected() {
        let db = setup();
        let user = create_test_user(&db);

        let repo = SqliteTagRepository::new(&db);
        let use_cases = TagUseCases::new(&repo);

        let result = use_cases.create_tag(user.id(), "".to_string(), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_tag_invalid_color_rejected() {
        let db = setup();
        let user = create_test_user(&db);

        let repo = SqliteTagRepository::new(&db);
        let use_cases = TagUseCases::new(&repo);

        let result = use_cases.create_tag(user.id(), "Bad".to_string(), Some("red".to_string()));
        assert!(result.is_err());
    }

    // ─── Budget Progress Tests ───────────────────────────────────────────────

    #[test]
    fn test_budget_progress_calculation() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let budget_repo = SqliteBudgetRepository::new(&db);
        let tx_repo = SqliteTransactionRepository::new(&db);
        let budget_uc = BudgetUseCases::new(&budget_repo, &tx_repo);
        let tx_uc = TransactionUseCases::new(&tx_repo);

        use finance_core::use_cases::BudgetUseCases;
        use finance_storage::repositories::SqliteBudgetRepository;

        let now = Utc::now();
        let budget = budget_uc
            .create_budget(
                account.id(),
                Some(category.id()),
                "Food Budget".to_string(),
                100_00, // $100
                "monthly",
                now - Duration::days(1),
            )
            .unwrap();

        // Create some expenses
        tx_uc.create_transaction(
            account.id(), category.id(), 30_00,
            TransactionType::Expense, "Groceries".to_string(), now,
        ).unwrap();
        tx_uc.create_transaction(
            account.id(), category.id(), 20_00,
            TransactionType::Expense, "Restaurant".to_string(), now,
        ).unwrap();

        let progress = budget_uc.get_budget_progress(budget.base.id).unwrap();
        assert_eq!(progress.spent, 50_00);
        assert_eq!(progress.remaining, 50_00);
        assert!((progress.percentage - 50.0).abs() < 0.01);
    }

    // ─── SQLCipher Encryption Tests ──────────────────────────────────────────

    #[test]
    fn test_encrypted_database_open_and_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("encrypted.db");
        let key = "super-secret-key-123";

        // Create encrypted database and write data
        {
            let db = Database::open_encrypted(&db_path, key).unwrap();
            let repo = SqliteUserRepository::new(&db);
            let user = User::new("Encrypted User".to_string(), "enc@example.com".to_string());
            repo.save(&user).unwrap();
        }

        // Reopen with correct key — should find the user
        {
            let db = Database::open_encrypted(&db_path, key).unwrap();
            let repo = SqliteUserRepository::new(&db);
            let found = repo.find_by_email("enc@example.com").unwrap();
            assert!(found.is_some());
            assert_eq!(found.unwrap().name, "Encrypted User");
        }
    }

    #[test]
    fn test_encrypted_database_wrong_key_fails() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("encrypted2.db");

        // Create with one key
        {
            let _db = Database::open_encrypted(&db_path, "correct-key").unwrap();
        }

        // Reopen with wrong key — should fail
        let result = Database::open_encrypted(&db_path, "wrong-key");
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypted_database_empty_key_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("encrypted3.db");

        let result = Database::open_encrypted(&db_path, "");
        assert!(result.is_err());
    }

    // ─── Monthly Trends Tests ────────────────────────────────────────────────

    #[test]
    fn test_monthly_trends() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let tx_repo = SqliteTransactionRepository::new(&db);
        let tx_uc = TransactionUseCases::new(&tx_repo);
        let stats = StatisticsUseCases::new(&tx_repo);

        let now = Utc::now();

        // Create some transactions in the current month
        tx_uc.create_transaction(
            account.id(), category.id(), 100_00,
            TransactionType::Expense, "Groceries".to_string(), now,
        ).unwrap();
        tx_uc.create_transaction(
            account.id(), category.id(), 50_00,
            TransactionType::Income, "Refund".to_string(), now,
        ).unwrap();
        tx_uc.create_transaction(
            account.id(), category.id(), 25_00,
            TransactionType::Expense, "Coffee".to_string(), now,
        ).unwrap();

        let from = now - Duration::days(30);
        let to = now + Duration::days(1);
        let trends = stats.get_monthly_trends(account.id(), from, to).unwrap();

        assert!(!trends.is_empty());
        let trend = &trends[0];
        assert_eq!(trend.income, 50_00);
        assert_eq!(trend.expenses, 125_00);
        assert_eq!(trend.net, 50_00 - 125_00);
        assert_eq!(trend.transaction_count, 3);
    }

    // ─── Daily Spending Tests ────────────────────────────────────────────────

    #[test]
    fn test_daily_spending() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let tx_repo = SqliteTransactionRepository::new(&db);
        let tx_uc = TransactionUseCases::new(&tx_repo);
        let stats = StatisticsUseCases::new(&tx_repo);

        let now = Utc::now();

        // Create expenses today
        tx_uc.create_transaction(
            account.id(), category.id(), 30_00,
            TransactionType::Expense, "Lunch".to_string(), now,
        ).unwrap();
        tx_uc.create_transaction(
            account.id(), category.id(), 10_00,
            TransactionType::Expense, "Snack".to_string(), now,
        ).unwrap();
        // Income should not appear in daily spending
        tx_uc.create_transaction(
            account.id(), category.id(), 200_00,
            TransactionType::Income, "Salary".to_string(), now,
        ).unwrap();

        let from = now - Duration::days(1);
        let to = now + Duration::days(1);
        let spending = stats.get_daily_spending(account.id(), from, to).unwrap();

        assert_eq!(spending.len(), 1);
        assert_eq!(spending[0].amount, 40_00);
        assert_eq!(spending[0].transaction_count, 2);
    }

    // ─── Transfer Linking Tests ──────────────────────────────────────────────

    #[test]
    fn test_create_transfer_links_two_transactions() {
        let db = setup();
        let user = create_test_user(&db);
        let account_a = create_test_account(&db, user.id());

        // Create a second account
        let account_repo = SqliteAccountRepository::new(&db);
        let account_uc = AccountUseCases::new(&account_repo);
        let account_b = account_uc.create_account(user.id(), "Savings".to_string(), "USD".to_string()).unwrap();

        let category = create_test_category(&db, user.id());
        let tx_repo = SqliteTransactionRepository::new(&db);
        let tx_uc = TransactionUseCases::new(&tx_repo);

        let now = Utc::now();
        let (outgoing, incoming) = tx_uc
            .create_transfer(account_a.id(), account_b.id(), category.id(), 50_00, "To savings".to_string(), now)
            .unwrap();

        // Outgoing should be Transfer type from account A
        assert_eq!(outgoing.account_id, account_a.id());
        assert_eq!(outgoing.amount, 50_00);
        assert_eq!(outgoing.transaction_type, TransactionType::Transfer);
        assert_eq!(outgoing.linked_transaction_id, Some(incoming.base.id));

        // Incoming should be Income type in account B
        assert_eq!(incoming.account_id, account_b.id());
        assert_eq!(incoming.amount, 50_00);
        assert_eq!(incoming.transaction_type, TransactionType::Income);
        assert_eq!(incoming.linked_transaction_id, Some(outgoing.base.id));
    }

    #[test]
    fn test_get_linked_transaction() {
        let db = setup();
        let user = create_test_user(&db);
        let account_a = create_test_account(&db, user.id());

        let account_repo = SqliteAccountRepository::new(&db);
        let account_uc = AccountUseCases::new(&account_repo);
        let account_b = account_uc.create_account(user.id(), "Checking".to_string(), "USD".to_string()).unwrap();

        let category = create_test_category(&db, user.id());
        let tx_repo = SqliteTransactionRepository::new(&db);
        let tx_uc = TransactionUseCases::new(&tx_repo);

        let now = Utc::now();
        let (outgoing, incoming) = tx_uc
            .create_transfer(account_a.id(), account_b.id(), category.id(), 75_00, "Rent transfer".to_string(), now)
            .unwrap();

        // Get linked from outgoing side
        let linked = tx_uc.get_linked_transaction(outgoing.base.id).unwrap();
        assert!(linked.is_some());
        assert_eq!(linked.unwrap().base.id, incoming.base.id);

        // Get linked from incoming side
        let linked2 = tx_uc.get_linked_transaction(incoming.base.id).unwrap();
        assert!(linked2.is_some());
        assert_eq!(linked2.unwrap().base.id, outgoing.base.id);
    }

    #[test]
    fn test_transfer_same_account_fails() {
        let db = setup();
        let user = create_test_user(&db);
        let account = create_test_account(&db, user.id());
        let category = create_test_category(&db, user.id());

        let tx_repo = SqliteTransactionRepository::new(&db);
        let tx_uc = TransactionUseCases::new(&tx_repo);

        let result = tx_uc.create_transfer(
            account.id(), account.id(), category.id(),
            50_00, "Self transfer".to_string(), Utc::now(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_transfer_balances() {
        let db = setup();
        let user = create_test_user(&db);

        let account_repo = SqliteAccountRepository::new(&db);
        let account_uc = AccountUseCases::new(&account_repo);
        let account_a = account_uc.create_account(user.id(), "Main".to_string(), "USD".to_string()).unwrap();
        let account_b = account_uc.create_account(user.id(), "Savings".to_string(), "USD".to_string()).unwrap();

        let category = create_test_category(&db, user.id());
        let tx_repo = SqliteTransactionRepository::new(&db);
        let tx_uc = TransactionUseCases::new(&tx_repo);

        // Fund account A first
        tx_uc.create_transaction(
            account_a.id(), category.id(), 200_00,
            TransactionType::Income, "Salary".to_string(), Utc::now(),
        ).unwrap();

        // Transfer $100 from A to B
        tx_uc.create_transfer(
            account_a.id(), account_b.id(), category.id(),
            100_00, "To savings".to_string(), Utc::now(),
        ).unwrap();

        // Account A: +200 income, -100 transfer = 100
        let balance_a = tx_repo.calculate_balance(account_a.id()).unwrap();
        assert_eq!(balance_a, 100_00);

        // Account B: +100 income from transfer = 100
        let balance_b = tx_repo.calculate_balance(account_b.id()).unwrap();
        assert_eq!(balance_b, 100_00);
    }
}
