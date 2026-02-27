# Personal Finance Backend (Rust)

A production-ready offline-first personal finance backend built with Rust, designed for mobile integration via FFI.

## Features

### Core Functionality
- **User Management**: Create and manage user profiles
- **Accounts**: Multiple accounts per user with balance tracking
- **Categories**: Organize transactions with custom categories
- **Transactions**: Record income, expenses, and transfers
- **Balance Calculation**: Real-time balance computed from transaction history

### Analytics & Insights 🆕
- **Statistics**: Spending aggregated by category for charts and reports
- **Income vs Expenses**: Period summaries with net calculations
- **Date Range Queries**: Flexible transaction filtering

### Automation 🆕
- **Recurring Transactions**: Automate regular bills, subscriptions, and paychecks
  - Frequencies: daily, weekly, bi-weekly, monthly, quarterly, yearly
  - Optional end dates for temporary schedules
  - Automatic transaction creation when due

### Budget Management 🆕
- **Spending Limits**: Set budgets by account or category
- **Progress Tracking**: Real-time spending vs budget with percentages
- **Period Support**: Weekly, monthly, quarterly, and yearly budgets

### Offline-First Architecture
- **Local-First Storage**: SQLite persistence with WAL mode
- **Sync Engine**: Last-Write-Wins conflict resolution
- **Pending Changes Tracking**: Automatic sync status management
- **Version Control**: Optimistic locking for conflict detection

### Mobile Integration
- **FFI Layer**: 26 C-compatible functions for iOS/Android
- **JSON Responses**: Structured error handling and data serialization
- **Auto-Generated Headers**: cbindgen for Swift/Kotlin/C++ integration
- **Memory Safe**: Proper string ownership and cleanup

## Project Structure

| Crate | Purpose |
|-------|---------|
| `finance-core` | Domain entities, business rules, repository traits |
| `finance-storage` | SQLite implementation of repositories |
| `finance-sync` | Synchronization engine with conflict resolution |
| `finance-ffi` | C-compatible FFI layer for mobile integration |

## Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)**: System design, patterns, and technical decisions
- **[FEATURES.md](FEATURES.md)**: New features documentation (statistics, recurring, budgets)
- **[MOBILE_GUIDE.md](MOBILE_GUIDE.md)**: Swift code examples and integration best practices
- **[finance-ffi/include/finance_ffi.h](finance-ffi/include/finance_ffi.h)**: Auto-generated C header

## Quick Start

```bash
# Build all crates
cargo build

# Run all tests (38 passing)
cargo test

# Build release version for production
cargo build --release

# Generate C header for mobile
cd finance-ffi && cargo build --release
# Header: finance-ffi/include/finance_ffi.h
# Library: target/release/libfinance_ffi.a (or .dylib)
```

## FFI Functions (26 total)

### Core (16 functions)
- `init_database` - Initialize SQLite database
- `create_user`, `get_user` - User management
- `create_account`, `list_accounts`, `delete_account` - Account management
- `create_category`, `list_categories`, `delete_category` - Category management
- `create_transaction`, `edit_transaction`, `delete_transaction` - Transaction CRUD
- `list_transactions`, `list_transactions_by_date_range` - Transaction queries
- `get_balance` - Account balance calculation
- `get_pending_sync` - Changes awaiting synchronization
- `free_string` - Memory cleanup

### Statistics (2 functions) 🆕
- `get_spending_by_category` - Aggregated spending for charts
- `get_income_vs_expenses` - Period income/expense summary

### Recurring Transactions (4 functions) 🆕
- `create_recurring_transaction` - Set up automated transactions
- `list_recurring_transactions` - View scheduled transactions
- `delete_recurring_transaction` - Remove schedule
- `process_due_recurring_transactions` - Execute due transactions

### Budgets (4 functions) 🆕
- `create_budget` - Set spending limits
- `list_budgets` - View all budgets
- `delete_budget` - Remove budget
- `get_budget_progress` - Calculate spending vs budget

All functions return JSON:
```json
{
  "success": bool,
  "code": int,
  "message": string,
  "data": any | null
}
```

## iOS Integration Example

```swift
// Initialize database
let dbPath = NSSearchPathForDirectoriesInDomains(.documentDirectory, .userDomainMask, true)[0] + "/finance.db"
let result = init_database(dbPath)

// Get spending by category for chart
let json = get_spending_by_category(accountId, fromDate, toDate)
let spending = try JSONDecoder().decode(FfiResult<[CategorySpending]>.self, from: Data(json.utf8))

// Create monthly rent recurring transaction
let rent = create_recurring_transaction(
    accountId, categoryId, 150000, "expense", 
    "Rent", "monthly", startDate, nil
)

// Check budget progress
let progress = get_budget_progress(budgetId)
```

See [MOBILE_GUIDE.md](MOBILE_GUIDE.md) for complete integration examples.

## Testing

```bash
cargo test

# Results:
# ✓ 17 domain entity tests
# ✓ 15 storage repository tests
# ✓ 6 sync engine tests
# Total: 38 tests passing
```

## Tech Stack

- **Rust 1.93.1** (edition 2021)
- **SQLite** via rusqlite 0.32
- **UUID v4** for entity identifiers
- **chrono** for date/time handling
- **serde** for JSON serialization
- **cbindgen** for C header generation
- **thiserror** for error handling

## License

MIT