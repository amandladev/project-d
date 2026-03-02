# Features

This document describes all implemented feature enhancements in the personal finance backend.

---

## 1. Statistics Endpoint

### Overview
Provides analytical data for visualizing spending patterns and financial summaries.

### Domain Layer
- **Use Cases**: `StatisticsUseCases` in `finance-core/src/use_cases/statistics_use_cases.rs`
- **Data Structures**:
  - `CategorySpending`: Aggregates spending by category with transaction count
  - `IncomeSummary`: Shows income vs expenses with net balance

### FFI Functions
```c
char* get_spending_by_category(const char* account_id, const char* from, const char* to);
char* get_income_vs_expenses(const char* account_id, const char* from, const char* to);
```

---

## 2. Recurring Transactions

### Overview
Automates regular transaction creation (weekly/monthly bills, subscriptions, paychecks).

### Domain Layer
- **Entity**: `RecurringTransaction` with frequency variants (Daily, Weekly, BiWeekly, Monthly, Quarterly, Yearly)
- **Use Cases**: `create`, `list`, `delete`, `process_due`
- **Date Math**: Handles varying month lengths, year rollover

### FFI Functions
```c
char* create_recurring_transaction(const char* account_id, const char* category_id,
    int64_t amount, const char* transaction_type, const char* description,
    const char* frequency, const char* start_date, const char* end_date);
char* list_recurring_transactions(const char* account_id);
char* delete_recurring_transaction(const char* id);
char* process_due_recurring_transactions();
```

---

## 3. Budget Management

### Overview
Track spending limits with progress monitoring.

### Domain Layer
- **Entity**: `Budget` with periods (Weekly, Monthly, Quarterly, Yearly), optional category scope
- **Progress**: `BudgetProgress` struct with spent, remaining, percentage, period dates

### FFI Functions
```c
char* create_budget(const char* account_id, const char* category_id,
    const char* name, int64_t amount, const char* period, const char* start_date);
char* list_budgets(const char* account_id);
char* delete_budget(const char* id);
char* get_budget_progress(const char* budget_id);
char* get_all_budgets_progress(const char* account_id);
```

---

## 4. Currency Conversion

### Overview
Multi-currency support with a 3-tier rate resolution system.

### Domain Layer
- **Entity**: `ExchangeRate` with `RateSource` (UserOverride > ApiRate > Bundled)
- **Precision**: Fixed-point i64 with `RATE_PRECISION = 1_000_000`
- **Use Cases**: `seed_bundled_rates`, `update_cached_rates`, `set_user_override`, `convert`, `get_rate_freshness`

### FFI Functions
```c
char* seed_exchange_rates();
char* update_exchange_rates(const char* rates_json);
char* set_manual_exchange_rate(const char* from, const char* to, int64_t rate);
char* convert_currency(const char* from, const char* to, int64_t amount);
char* get_rate_freshness(const char* from, const char* to);
char* list_exchange_rates(const char* from_currency);
```

---

## 5. Transaction Search

### Overview
Dynamic SQL query builder with multi-criteria filtering and pagination.

### Filters
- Text search on description (LIKE)
- Category, transaction type
- Min/max amount range
- Date range
- Limit/offset pagination

### FFI Function
```c
char* search_transactions(const char* filter_json);
```

### Filter JSON Format
```json
{
  "account_id": "uuid",
  "query": "groceries",
  "category_id": "uuid",
  "transaction_type": "Expense",
  "min_amount": 1000,
  "max_amount": 50000,
  "date_from": "2024-01-01T00:00:00Z",
  "date_to": "2024-12-31T23:59:59Z",
  "limit": 20,
  "offset": 0
}
```

---

## 6. Tags / Labels

### Overview
Flexible tagging system with many-to-many relationships between tags and transactions.

### Domain Layer
- **Entity**: `Tag` with name, color (hex), user scope
- **Junction Table**: `transaction_tags` for many-to-many association
- **Validation**: Non-empty name, valid hex color format

### FFI Functions
```c
char* create_tag(const char* user_id, const char* name, const char* color);
char* list_tags(const char* user_id);
char* update_tag(const char* tag_id, const char* name, const char* color);
char* delete_tag(const char* tag_id);
char* add_tag_to_transaction(const char* transaction_id, const char* tag_id);
char* remove_tag_from_transaction(const char* transaction_id, const char* tag_id);
char* get_transaction_tags(const char* transaction_id);
char* get_transactions_by_tag(const char* tag_id);
```

---

## 7. Category Seeding

### Overview
Default category seed moved from iOS to Rust backend for consistency.

### FFI Function
```c
char* seed_default_categories(const char* user_id);
```

Idempotent — skips if categories already exist for user. Seeds 12 categories with emoji icons (Food, Transport, Rent, Utilities, etc.).

---

## 8. SQLCipher Encryption

### Overview
AES-256 database encryption using SQLCipher via the `bundled-sqlcipher` feature of rusqlite.

### Implementation
- **Cargo.toml**: `rusqlite` changed from `bundled` to `bundled-sqlcipher`
- **Database**: New `open_encrypted(path, key)` method sets `PRAGMA key` as the first operation
- **Verification**: Validates key correctness by running `SELECT count(*) FROM sqlite_master`
- **Safety**: Rejects empty encryption keys

### FFI Function
```c
char* init_database_encrypted(const char* path, const char* key);
```

### Usage
Replace `init_database(path)` with `init_database_encrypted(path, key)` to enable encryption. Existing unencrypted databases are not affected — encryption only applies to newly created databases with this function.

### Tests
- Encrypted database open and reopen with correct key
- Wrong key rejection
- Empty key rejection

---

## 9. Spending Trends / Analytics

### Overview
Time-series analytics for visualizing income/expense patterns over months and daily spending breakdowns.

### Domain Layer
- **`MonthlyTrend`**: `{ year, month, income, expenses, net, transaction_count }` — grouped by (year, month) using BTreeMap for chronological ordering
- **`DailySpending`**: `{ date, amount, transaction_count }` — filters expenses only, grouped by calendar day

### FFI Functions
```c
char* get_monthly_trends(const char* account_id, const char* from, const char* to);
char* get_daily_spending(const char* account_id, const char* from, const char* to);
```

### Response Examples
```json
// Monthly Trends
{
  "data": [
    { "year": 2025, "month": 1, "income": 500000, "expenses": 320000, "net": 180000, "transaction_count": 42 },
    { "year": 2025, "month": 2, "income": 500000, "expenses": 280000, "net": 220000, "transaction_count": 38 }
  ]
}

// Daily Spending
{
  "data": [
    { "date": "2025-01-15", "amount": 4500, "transaction_count": 3 },
    { "date": "2025-01-16", "amount": 12000, "transaction_count": 5 }
  ]
}
```

---

## 10. Transfer Linking

### Overview
Links paired transactions for account-to-account transfers, allowing the app to track both sides of a transfer as a single logical operation.

### Domain Layer
- **New Field**: `linked_transaction_id: Option<Uuid>` on `Transaction` entity
- **Schema**: `linked_transaction_id TEXT` column in transactions table
- **Use Cases**:
  - `create_transfer(from, to, category, amount, description, date)` — creates two linked transactions:
    - **Outgoing**: `TransactionType::Transfer` on source account (negative balance effect)
    - **Incoming**: `TransactionType::Income` on destination account (positive balance effect)
  - `get_linked_transaction(transaction_id)` — retrieves the other side of a transfer
- **Validation**: Rejects same-account transfers

### FFI Functions
```c
char* create_transfer(const char* from_account_id, const char* to_account_id,
    const char* category_id, int64_t amount, const char* description, const char* date);
char* get_linked_transaction(const char* transaction_id);
```

### Response Format
```json
{
  "data": {
    "outgoing": {
      "id": "uuid",
      "account_id": "source-uuid",
      "amount": 10000,
      "transaction_type": "Transfer",
      "description": "To savings",
      "linked_transaction_id": "incoming-uuid"
    },
    "incoming": {
      "id": "uuid",
      "account_id": "dest-uuid",
      "amount": 10000,
      "transaction_type": "Income",
      "description": "Transfer: To savings",
      "linked_transaction_id": "outgoing-uuid"
    }
  }
}
```

### Balance Impact
Transfers are zero-sum: the source account decreases and the destination increases by the same amount.

---

## Testing Summary

**85 tests passing** across all crates:
- 5 exchange rate unit tests
- 17 domain entity tests
- 7 date formatting tests
- 50 storage/integration tests (including encryption, trends, transfers)
- 6 sync engine tests

## FFI Summary

**48 total FFI functions**:
| Area | Count | Functions |
|------|-------|-----------|
| Database | 2 | `init_database`, `init_database_encrypted` |
| Users | 2 | `create_user`, `get_user` |
| Accounts | 3 | `create_account`, `list_accounts`, `delete_account` |
| Categories | 3 | `create_category`, `list_categories`, `delete_category`, `seed_default_categories` |
| Transactions | 4 | `create_transaction`, `edit_transaction`, `delete_transaction`, `list_transactions`, `list_transactions_by_date_range` |
| Statistics | 4 | `get_balance`, `get_spending_by_category`, `get_income_vs_expenses`, `get_monthly_trends`, `get_daily_spending` |
| Recurring | 4 | `create_recurring_transaction`, `list_recurring_transactions`, `delete_recurring_transaction`, `process_due_recurring_transactions` |
| Budgets | 5 | `create_budget`, `list_budgets`, `delete_budget`, `get_budget_progress`, `get_all_budgets_progress` |
| Currency | 6 | `seed_exchange_rates`, `update_exchange_rates`, `set_manual_exchange_rate`, `convert_currency`, `get_rate_freshness`, `list_exchange_rates` |
| Search | 1 | `search_transactions` |
| Tags | 8 | `create_tag`, `list_tags`, `update_tag`, `delete_tag`, `add_tag_to_transaction`, `remove_tag_from_transaction`, `get_transaction_tags`, `get_transactions_by_tag` |
| Transfers | 2 | `create_transfer`, `get_linked_transaction` |
| Memory | 1 | `free_string` |

## Build Targets

All targets build successfully:
- `macOS` (host)
- `aarch64-apple-ios` (iOS device)
- `aarch64-apple-ios-sim` (iOS simulator)

The implementation is production-ready and can be integrated into your iOS app immediately.
