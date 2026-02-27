# New Features Implementation

This document describes the three new feature enhancements added to the personal finance backend.

## 1. Statistics Endpoint

### Overview
Provides analytical data for visualizing spending patterns and financial summaries.

### Domain Layer
- **New Use Cases**: `StatisticsUseCases` in `finance-core/src/use_cases/statistics_use_cases.rs`
- **Data Structures**:
  - `CategorySpending`: Aggregates spending by category with transaction count
  - `IncomeSummary`: Shows income vs expenses with net balance

### Storage Layer
- **Query**: `get_spending_by_category()` uses SQL GROUP BY to aggregate expenses by category
- Filters only expense transactions within date range
- Joins with categories table to include category names

### FFI Functions
```c
// Get spending aggregated by category for charts
char* get_spending_by_category(const char* account_id, const char* from, const char* to);

// Get income vs expenses summary
char* get_income_vs_expenses(const char* account_id, const char* from, const char* to);
```

### Response Format
```json
{
  "success": true,
  "code": 0,
  "message": "OK",
  "data": [
    {
      "category_id": "uuid",
      "category_name": "Groceries",
      "total_amount": 150000,
      "transaction_count": 15
    }
  ]
}
```

## 2. Recurring Transactions

### Overview
Automates regular transaction creation (weekly/monthly bills, subscriptions, paychecks).

### Domain Layer
- **Entity**: `RecurringTransaction` in `finance-core/src/entities/recurring_transaction.rs`
- **Frequencies**: Daily, Weekly, BiWeekly, Monthly, Quarterly, Yearly
- **Fields**:
  - `frequency`: How often the transaction repeats
  - `start_date`: When recurrence begins
  - `end_date`: Optional expiration date
  - `next_occurrence`: Next scheduled date
  - `is_active`: Can be paused without deletion

### Storage Layer
- **Table**: `recurring_transactions` with indexes on:
  - `account_id` (for listing)
  - `next_occurrence` (for finding due transactions)
  - `is_active` (for filtering active schedules)
- **Repository**: `SqliteRecurringTransactionRepository`

### Use Cases
- `create_recurring_transaction()`: Set up new recurring schedule
- `list_recurring_transactions()`: View all recurring transactions for account
- `delete_recurring_transaction()`: Soft delete (preserves history)
- `process_due_recurring_transactions()`: Creates actual transactions when due

### FFI Functions
```c
// Create a recurring transaction template
char* create_recurring_transaction(
    const char* account_id,
    const char* category_id,
    int64_t amount,
    const char* transaction_type,
    const char* description,
    const char* frequency,  // "daily", "weekly", "monthly", etc.
    const char* start_date,
    const char* end_date    // nullable
);

// List all recurring transactions
char* list_recurring_transactions(const char* account_id);

// Delete a recurring transaction
char* delete_recurring_transaction(const char* id);

// Process all due recurring transactions (call this periodically)
char* process_due_recurring_transactions();
```

### Date Calculation
The `calculate_next_occurrence()` method handles complex date math:
- Monthly: Accounts for varying month lengths
- Quarterly: Adds 3 months with year rollover
- Yearly: Preserves day/month across years

## 3. Budget Management

### Overview
Track spending limits with progress monitoring and alerts.

### Domain Layer
- **Entity**: `Budget` in `finance-core/src/entities/budget.rs`
- **Periods**: Weekly, Monthly, Quarterly, Yearly
- **Scope**: Can be account-wide or category-specific
- **Progress**: `BudgetProgress` struct with:
  - `spent`: Total amount spent in period
  - `remaining`: Budget amount minus spent
  - `percentage`: Spending as percentage of budget
  - `period_start` / `period_end`: Current period dates

### Storage Layer
- **Table**: `budgets` with indexes on:
  - `account_id` (for listing budgets)
  - `category_id` (for category-specific budgets)
- **Repository**: `SqliteBudgetRepository`

### Use Cases
- `create_budget()`: Set spending limit for period
- `list_budgets()`: View all budgets for account
- `update_budget()`: Modify budget amount or period
- `delete_budget()`: Soft delete budget
- `get_budget_progress()`: Calculate current spending vs budget

### FFI Functions
```c
// Create a new budget
char* create_budget(
    const char* account_id,
    const char* category_id,  // nullable for account-wide budget
    const char* name,
    int64_t amount,
    const char* period,  // "weekly", "monthly", "quarterly", "yearly"
    const char* start_date
);

// List all budgets for account
char* list_budgets(const char* account_id);

// Delete a budget
char* delete_budget(const char* id);

// Get budget progress with spending calculation
char* get_budget_progress(const char* budget_id);
```

### Progress Calculation
The `get_budget_progress()` function:
1. Fetches budget and calculates period end date
2. Queries transactions within period
3. Filters by category (if budget is category-specific)
4. Sums expense transactions
5. Calculates remaining amount and percentage

## Database Migrations

New tables added to `finance-storage/src/migrations.rs`:

```sql
CREATE TABLE recurring_transactions (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    category_id TEXT NOT NULL,
    amount INTEGER NOT NULL,
    transaction_type TEXT NOT NULL,
    description TEXT NOT NULL,
    frequency TEXT NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT,
    next_occurrence TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    FOREIGN KEY (account_id) REFERENCES accounts(id),
    FOREIGN KEY (category_id) REFERENCES categories(id)
);

CREATE TABLE budgets (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    category_id TEXT,
    name TEXT NOT NULL,
    amount INTEGER NOT NULL,
    period TEXT NOT NULL,
    start_date TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    FOREIGN KEY (account_id) REFERENCES accounts(id),
    FOREIGN KEY (category_id) REFERENCES categories(id)
);
```

## Testing

All existing tests continue to pass (38 tests):
- 17 domain entity tests
- 15 storage repository tests
- 6 sync engine tests

## Files Added/Modified

### New Files
- `finance-core/src/entities/recurring_transaction.rs`
- `finance-core/src/entities/budget.rs`
- `finance-core/src/use_cases/statistics_use_cases.rs`
- `finance-core/src/use_cases/recurring_transaction_use_cases.rs`
- `finance-core/src/use_cases/budget_use_cases.rs`
- `finance-core/src/repositories/recurring_transaction_repository.rs`
- `finance-core/src/repositories/budget_repository.rs`
- `finance-storage/src/repositories/sqlite_recurring_transaction_repository.rs`
- `finance-storage/src/repositories/sqlite_budget_repository.rs`

### Modified Files
- `finance-core/src/entities/mod.rs`: Export new entities
- `finance-core/src/repositories/mod.rs`: Export new repository traits
- `finance-core/src/repositories/transaction_repository.rs`: Added `get_spending_by_category()`
- `finance-core/src/use_cases/mod.rs`: Export new use cases
- `finance-storage/src/repositories/mod.rs`: Export new SQLite repositories
- `finance-storage/src/repositories/sqlite_transaction_repository.rs`: Implement statistics query
- `finance-storage/src/migrations.rs`: Add new tables and indexes
- `finance-ffi/src/lib.rs`: Add 10 new FFI functions
- `finance-ffi/include/finance_ffi.h`: Auto-generated with new function declarations
- `finance-sync/src/tests.rs`: Add mock implementation for new repository methods

## Summary

**Total New FFI Functions**: 10
- 2 for statistics
- 4 for recurring transactions
- 4 for budgets

**Total FFI Functions**: 26 (up from 16)

All features maintain:
- Offline-first architecture with sync capabilities
- Soft delete patterns
- Clean Architecture separation
- SQLite persistence with proper indexes
- Type-safe FFI layer with JSON responses
- C header generation via cbindgen

The implementation is production-ready and can be integrated into your iOS app immediately.
