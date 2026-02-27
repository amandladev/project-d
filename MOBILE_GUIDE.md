# Mobile Integration Quick Start

## Usage Examples for New Features

### Statistics - Show Spending by Category Chart

```swift
// Get spending data for the last 30 days
let accountId = "user-account-uuid"
let from = ISO8601DateFormatter().string(from: Date().addingTimeInterval(-30 * 24 * 3600))
let to = ISO8601DateFormatter().string(from: Date())

let resultJson = get_spending_by_category(accountId, from, to)
let result = try JSONDecoder().decode(FfiResult<[CategorySpending]>.self, from: Data(resultJson.utf8))

if result.success {
    // result.data contains array of spending by category
    for category in result.data {
        print("\(category.category_name): $\(category.total_amount / 100) (\(category.transaction_count) transactions)")
    }
}
```

### Statistics - Income vs Expenses Summary

```swift
let summaryJson = get_income_vs_expenses(accountId, from, to)
let summary = try JSONDecoder().decode(FfiResult<IncomeSummary>.self, from: Data(summaryJson.utf8))

if summary.success {
    print("Income: $\(summary.data.income / 100)")
    print("Expenses: $\(summary.data.expenses / 100)")
    print("Net: $\(summary.data.net / 100)")
}
```

### Recurring Transactions - Setup Monthly Rent

```swift
// Create a monthly rent payment
let rentJson = create_recurring_transaction(
    accountId,
    rentCategoryId,
    150000,  // $1,500.00 in cents
    "expense",
    "Monthly Rent",
    "monthly",
    "2025-02-01T00:00:00Z",
    nil  // No end date
)

let result = try JSONDecoder().decode(FfiResult<RecurringTransaction>.self, from: Data(rentJson.utf8))
if result.success {
    print("Recurring transaction created: \(result.data.id)")
}
```

### Recurring Transactions - List Active Schedules

```swift
let recurringJson = list_recurring_transactions(accountId)
let result = try JSONDecoder().decode(FfiResult<[RecurringTransaction]>.self, from: Data(recurringJson.utf8))

for recurring in result.data {
    print("\(recurring.description) - \(recurring.frequency)")
    print("Next on: \(recurring.next_occurrence)")
}
```

### Recurring Transactions - Process Due Transactions

```swift
// Call this daily (or when app opens) to create due transactions
let processedJson = process_due_recurring_transactions()
let result = try JSONDecoder().decode(FfiResult<[String]>.self, from: Data(processedJson.utf8))

if result.success {
    print("Created \(result.data.count) transactions")
    // Transaction IDs are in result.data array
}
```

### Budgets - Create Monthly Grocery Budget

```swift
let budgetJson = create_budget(
    accountId,
    groceryCategoryId,
    "Grocery Budget",
    50000,  // $500.00 in cents
    "monthly",
    "2025-02-01T00:00:00Z"
)

let result = try JSONDecoder().decode(FfiResult<Budget>.self, from: Data(budgetJson.utf8))
if result.success {
    print("Budget created: \(result.data.name)")
}
```

### Budgets - Check Budget Progress

```swift
let progressJson = get_budget_progress(budgetId)
let result = try JSONDecoder().decode(FfiResult<BudgetProgress>.self, from: Data(progressJson.utf8))

if result.success {
    let progress = result.data
    print("Budget: \(progress.budget.name)")
    print("Spent: $\(progress.spent / 100) of $\(progress.budget.amount / 100)")
    print("Remaining: $\(progress.remaining / 100)")
    print("Progress: \(progress.percentage)%")
    
    // Show alert if over budget
    if progress.percentage > 100 {
        showAlert("You've exceeded your \(progress.budget.name)!")
    }
}
```

### Budgets - List All Account Budgets

```swift
let budgetsJson = list_budgets(accountId)
let result = try JSONDecoder().decode(FfiResult<[Budget]>.self, from: Data(budgetsJson.utf8))

for budget in result.data {
    // Get progress for each budget
    let progressJson = get_budget_progress(budget.id)
    // Display in UI...
}
```

## Data Types Reference

### CategorySpending
```swift
struct CategorySpending: Codable {
    let category_id: String
    let category_name: String
    let total_amount: Int64  // Amount in cents
    let transaction_count: Int
}
```

### IncomeSummary
```swift
struct IncomeSummary: Codable {
    let income: Int64     // Total income in cents
    let expenses: Int64   // Total expenses in cents
    let transfers: Int64  // Total transfers in cents
    let net: Int64        // Net (income - expenses - transfers)
}
```

### RecurringTransaction
```swift
struct RecurringTransaction: Codable {
    let id: String
    let account_id: String
    let category_id: String
    let amount: Int64
    let transaction_type: String  // "income", "expense", "transfer"
    let description: String
    let frequency: String  // "daily", "weekly", "biweekly", "monthly", "quarterly", "yearly"
    let start_date: String  // RFC3339 timestamp
    let end_date: String?   // Optional RFC3339 timestamp
    let next_occurrence: String  // RFC3339 timestamp
    let is_active: Bool
    let created_at: String
    let updated_at: String
    let deleted_at: String?
}
```

### Budget
```swift
struct Budget: Codable {
    let id: String
    let account_id: String
    let category_id: String?  // nil for account-wide budgets
    let name: String
    let amount: Int64  // Budget limit in cents
    let period: String  // "weekly", "monthly", "quarterly", "yearly"
    let start_date: String  // RFC3339 timestamp
    let created_at: String
    let updated_at: String
    let deleted_at: String?
}
```

### BudgetProgress
```swift
struct BudgetProgress: Codable {
    let budget: Budget
    let spent: Int64       // Amount spent in current period (cents)
    let remaining: Int64   // Budget amount - spent (cents)
    let percentage: Double // Spent as percentage of budget
    let period_start: String  // RFC3339 timestamp
    let period_end: String    // RFC3339 timestamp
}
```

## Best Practices

### Amounts
All monetary amounts are stored as **Int64 in cents** to avoid floating-point precision issues:
- $10.50 → 1050
- $1,000.00 → 100000
- Display: `amount / 100` for dollars

### Dates
All dates use **RFC3339 format** (ISO 8601):
- Format: `2025-02-01T15:30:00Z`
- Swift: Use `ISO8601DateFormatter()`

### Processing Recurring Transactions
Call `process_due_recurring_transactions()` when:
- App launches (after successful authentication)
- User navigates to transactions screen
- Once per day (use background task if available)

### Budget Alerts
Check budget progress and alert users:
- At 80% (warning)
- At 100% (exceeded)
- At 120% (significantly over)

### Memory Management
Always call `free_string()` on returned C strings to prevent memory leaks:
```swift
let cString = some_ffi_function(...)
let jsonString = String(cString: cString)
free_string(cString)
```

## Error Handling

All FFI functions return JSON with error details:
```json
{
  "success": false,
  "code": 40,
  "message": "Invalid frequency",
  "data": null
}
```

Error codes:
- 1: Database not initialized
- 2: Null pointer received
- 3: Invalid UUID or parameter format
- 4: JSON serialization error
- 10-19: Database errors
- 20-29: Transaction errors
- 30-39: Statistics errors
- 40-49: Recurring transaction errors
- 50-59: Budget errors

Always check `success` field before accessing `data`.
