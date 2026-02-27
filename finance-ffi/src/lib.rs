use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

use finance_core::entities::{TransactionType, User};
use finance_core::repositories::{
    AccountRepository, CategoryRepository, TransactionRepository, UserRepository,
};
use finance_core::use_cases::{
    AccountUseCases, BudgetUseCases, CategoryUseCases, RecurringTransactionUseCases,
    StatisticsUseCases, TransactionUseCases,
};
use finance_storage::database::Database;
use finance_storage::repositories::{
    SqliteAccountRepository, SqliteBudgetRepository, SqliteCategoryRepository,
    SqliteRecurringTransactionRepository, SqliteTransactionRepository, SqliteUserRepository,
};

use serde::Serialize;
use uuid::Uuid;

/// Global database instance for FFI.
static DB: once_cell::sync::OnceCell<Database> = once_cell::sync::OnceCell::new();

/// FFI result structure.
#[derive(Debug, Serialize)]
struct FfiResult {
    success: bool,
    code: i32,
    message: String,
    data: Option<serde_json::Value>,
}

impl FfiResult {
    fn ok(data: serde_json::Value) -> Self {
        Self {
            success: true,
            code: 0,
            message: "OK".to_string(),
            data: Some(data),
        }
    }

    fn ok_empty() -> Self {
        Self {
            success: true,
            code: 0,
            message: "OK".to_string(),
            data: None,
        }
    }

    fn error(code: i32, message: String) -> Self {
        Self {
            success: false,
            code,
            message,
            data: None,
        }
    }

    fn to_json_cstring(&self) -> *mut c_char {
        let json = serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"success":false,"code":-1,"message":"Serialization error","data":null}"#.to_string()
        });
        CString::new(json).unwrap().into_raw()
    }
}

// ─── Helper ──────────────────────────────────────────────────────────────────

fn get_db() -> Result<&'static Database, FfiResult> {
    DB.get().ok_or_else(|| {
        FfiResult::error(1, "Database not initialized. Call init_database first.".to_string())
    })
}

unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Result<&'a str, FfiResult> {
    if ptr.is_null() {
        return Err(FfiResult::error(2, "Null pointer received".to_string()));
    }
    CStr::from_ptr(ptr)
        .to_str()
        .map_err(|_| FfiResult::error(2, "Invalid UTF-8 string".to_string()))
}

// ─── FFI Functions ───────────────────────────────────────────────────────────

/// Initialize the database at the given path. Must be called before any other function.
///
/// # Safety
/// `path` must be a valid C string pointer.
#[no_mangle]
pub unsafe extern "C" fn init_database(path: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let path_str = cstr_to_str(path)?;
        let db = Database::open(Path::new(path_str))
            .map_err(|e| FfiResult::error(10, format!("Failed to open database: {e}")))?;
        DB.set(db)
            .map_err(|_| FfiResult::error(11, "Database already initialized".to_string()))?;
        Ok(FfiResult::ok_empty())
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Create a new account.
///
/// # Safety
/// All pointer parameters must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn create_account(
    user_id: *const c_char,
    name: *const c_char,
    currency: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let user_id_str = cstr_to_str(user_id)?;
        let name_str = cstr_to_str(name)?;
        let currency_str = cstr_to_str(currency)?;

        let user_uuid = Uuid::parse_str(user_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid user_id UUID".to_string()))?;

        let repo = SqliteAccountRepository::new(db);
        let use_cases = AccountUseCases::new(&repo);

        let account = use_cases
            .create_account(user_uuid, name_str.to_string(), currency_str.to_string())
            .map_err(|e| FfiResult::error(20, format!("{e}")))?;

        let data = serde_json::to_value(&account)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Create a new category.
///
/// # Safety
/// All pointer parameters must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn create_category(
    user_id: *const c_char,
    name: *const c_char,
    icon: *const c_char, // can be null
) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let user_id_str = cstr_to_str(user_id)?;
        let name_str = cstr_to_str(name)?;
        let icon_str = if icon.is_null() {
            None
        } else {
            Some(cstr_to_str(icon)?.to_string())
        };

        let user_uuid = Uuid::parse_str(user_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid user_id UUID".to_string()))?;

        let repo = SqliteCategoryRepository::new(db);
        let use_cases = CategoryUseCases::new(&repo);

        let category = use_cases
            .create_category(user_uuid, name_str.to_string(), icon_str)
            .map_err(|e| FfiResult::error(21, format!("{e}")))?;

        let data = serde_json::to_value(&category)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Create a new transaction.
///
/// # Safety
/// All pointer parameters must be valid C strings.
/// `transaction_type` must be one of: "expense", "income", "transfer".
/// `amount` is in cents (integer).
#[no_mangle]
pub unsafe extern "C" fn create_transaction(
    account_id: *const c_char,
    category_id: *const c_char,
    amount: i64,
    transaction_type: *const c_char,
    description: *const c_char,
    date: *const c_char, // RFC3339 formatted date string
) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let account_id_str = cstr_to_str(account_id)?;
        let category_id_str = cstr_to_str(category_id)?;
        let type_str = cstr_to_str(transaction_type)?;
        let desc_str = cstr_to_str(description)?;
        let date_str = cstr_to_str(date)?;

        let account_uuid = Uuid::parse_str(account_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid account_id UUID".to_string()))?;
        let category_uuid = Uuid::parse_str(category_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid category_id UUID".to_string()))?;
        let tx_type = TransactionType::from_str(type_str)
            .ok_or_else(|| FfiResult::error(3, "Invalid transaction_type".to_string()))?;
        let parsed_date = chrono::DateTime::parse_from_rfc3339(date_str)
            .map_err(|_| FfiResult::error(3, "Invalid date format (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = TransactionUseCases::new(&repo);

        let transaction = use_cases
            .create_transaction(
                account_uuid,
                category_uuid,
                amount,
                tx_type,
                desc_str.to_string(),
                parsed_date,
            )
            .map_err(|e| FfiResult::error(22, format!("{e}")))?;

        let data = serde_json::to_value(&transaction)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Get the balance for an account (in cents).
///
/// # Safety
/// `account_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn get_balance(account_id: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let account_id_str = cstr_to_str(account_id)?;

        let account_uuid = Uuid::parse_str(account_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid account_id UUID".to_string()))?;

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = TransactionUseCases::new(&repo);

        let balance = use_cases
            .get_balance(account_uuid)
            .map_err(|e| FfiResult::error(23, format!("{e}")))?;

        Ok(FfiResult::ok(serde_json::json!({ "balance": balance })))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Get pending sync changes as JSON.
#[no_mangle]
pub extern "C" fn get_pending_sync() -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;

        let account_repo = SqliteAccountRepository::new(db);
        let category_repo = SqliteCategoryRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);

        let accounts = account_repo
            .find_pending_sync()
            .map_err(|e| FfiResult::error(30, format!("{e}")))?;
        let categories = category_repo
            .find_pending_sync()
            .map_err(|e| FfiResult::error(30, format!("{e}")))?;
        let transactions = transaction_repo
            .find_pending_sync()
            .map_err(|e| FfiResult::error(30, format!("{e}")))?;

        let data = serde_json::json!({
            "accounts": accounts,
            "categories": categories,
            "transactions": transactions,
        });

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Get spending aggregated by category for an account within a date range.
///
/// # Safety
/// All pointer parameters must be valid C strings. Dates must be RFC3339 formatted.
#[no_mangle]
pub unsafe extern "C" fn get_spending_by_category(
    account_id: *const c_char,
    from: *const c_char,
    to: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let account_id_str = cstr_to_str(account_id)?;
        let from_str = cstr_to_str(from)?;
        let to_str = cstr_to_str(to)?;

        let account_uuid = Uuid::parse_str(account_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid account_id UUID".to_string()))?;
        let from_date = chrono::DateTime::parse_from_rfc3339(from_str)
            .map_err(|_| FfiResult::error(3, "Invalid from date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);
        let to_date = chrono::DateTime::parse_from_rfc3339(to_str)
            .map_err(|_| FfiResult::error(3, "Invalid to date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = StatisticsUseCases::new(&repo);
        let spending = use_cases
            .get_spending_by_category(account_uuid, from_date, to_date)
            .map_err(|e| FfiResult::error(30, format!("{e}")))?;

        let data = serde_json::to_value(&spending)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Get income vs expenses summary for an account within a date range.
///
/// # Safety
/// All pointer parameters must be valid C strings. Dates must be RFC3339 formatted.
#[no_mangle]
pub unsafe extern "C" fn get_income_vs_expenses(
    account_id: *const c_char,
    from: *const c_char,
    to: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let account_id_str = cstr_to_str(account_id)?;
        let from_str = cstr_to_str(from)?;
        let to_str = cstr_to_str(to)?;

        let account_uuid = Uuid::parse_str(account_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid account_id UUID".to_string()))?;
        let from_date = chrono::DateTime::parse_from_rfc3339(from_str)
            .map_err(|_| FfiResult::error(3, "Invalid from date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);
        let to_date = chrono::DateTime::parse_from_rfc3339(to_str)
            .map_err(|_| FfiResult::error(3, "Invalid to date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = StatisticsUseCases::new(&repo);
        let summary = use_cases
            .get_income_vs_expenses(account_uuid, from_date, to_date)
            .map_err(|e| FfiResult::error(31, format!("{e}")))?;

        let data = serde_json::to_value(&summary)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

// ─── Recurring Transaction Functions ─────────────────────────────────────────

/// Create a new recurring transaction.
///
/// # Safety
/// All pointer parameters must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn create_recurring_transaction(
    account_id: *const c_char,
    category_id: *const c_char,
    amount: i64,
    transaction_type: *const c_char,
    description: *const c_char,
    frequency: *const c_char,
    start_date: *const c_char,
    end_date: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let account_id_str = cstr_to_str(account_id)?;
        let category_id_str = cstr_to_str(category_id)?;
        let tx_type_str = cstr_to_str(transaction_type)?;
        let desc_str = cstr_to_str(description)?;
        let freq_str = cstr_to_str(frequency)?;
        let start_str = cstr_to_str(start_date)?;

        let account_uuid = Uuid::parse_str(account_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid account_id UUID".to_string()))?;
        let category_uuid = Uuid::parse_str(category_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid category_id UUID".to_string()))?;
        let tx_type = TransactionType::from_str(tx_type_str)
            .ok_or_else(|| FfiResult::error(3, "Invalid transaction_type".to_string()))?;
        let start_dt = chrono::DateTime::parse_from_rfc3339(start_str)
            .map_err(|_| FfiResult::error(3, "Invalid start_date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let end_dt = if !end_date.is_null() {
            let end_str = cstr_to_str(end_date)?;
            Some(
                chrono::DateTime::parse_from_rfc3339(end_str)
                    .map_err(|_| {
                        FfiResult::error(3, "Invalid end_date (expected RFC3339)".to_string())
                    })?
                    .with_timezone(&chrono::Utc),
            )
        } else {
            None
        };

        let recurring_repo = SqliteRecurringTransactionRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = RecurringTransactionUseCases::new(&recurring_repo, &transaction_repo);

        let recurring = use_cases
            .create_recurring_transaction(
                account_uuid,
                category_uuid,
                amount,
                tx_type,
                desc_str.to_string(),
                freq_str,
                start_dt,
                end_dt,
            )
            .map_err(|e| FfiResult::error(40, format!("{e}")))?;

        let data = serde_json::to_value(&recurring)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// List recurring transactions for an account.
///
/// # Safety
/// `account_id` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn list_recurring_transactions(account_id: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let account_id_str = cstr_to_str(account_id)?;

        let account_uuid = Uuid::parse_str(account_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid account_id UUID".to_string()))?;

        let recurring_repo = SqliteRecurringTransactionRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = RecurringTransactionUseCases::new(&recurring_repo, &transaction_repo);

        let recurrings = use_cases
            .list_recurring_transactions(account_uuid)
            .map_err(|e| FfiResult::error(41, format!("{e}")))?;

        let data = serde_json::to_value(&recurrings)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Delete a recurring transaction.
///
/// # Safety
/// `id` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn delete_recurring_transaction(id: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let id_str = cstr_to_str(id)?;

        let uuid = Uuid::parse_str(id_str)
            .map_err(|_| FfiResult::error(3, "Invalid id UUID".to_string()))?;

        let recurring_repo = SqliteRecurringTransactionRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = RecurringTransactionUseCases::new(&recurring_repo, &transaction_repo);

        use_cases
            .delete_recurring_transaction(uuid)
            .map_err(|e| FfiResult::error(42, format!("{e}")))?;

        Ok(FfiResult::ok_empty())
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Process due recurring transactions and create actual transactions.
///
/// # Safety
/// This function is safe to call.
#[no_mangle]
pub unsafe extern "C" fn process_due_recurring_transactions() -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;

        let recurring_repo = SqliteRecurringTransactionRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = RecurringTransactionUseCases::new(&recurring_repo, &transaction_repo);

        let created_ids = use_cases
            .process_due_recurring_transactions()
            .map_err(|e| FfiResult::error(43, format!("{e}")))?;

        let data = serde_json::to_value(&created_ids)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

// ─── Budget Functions ────────────────────────────────────────────────────────

/// Create a new budget.
///
/// # Safety
/// All pointer parameters must be valid C strings. `category_id` can be null for account-wide budgets.
#[no_mangle]
pub unsafe extern "C" fn create_budget(
    account_id: *const c_char,
    category_id: *const c_char,
    name: *const c_char,
    amount: i64,
    period: *const c_char,
    start_date: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let account_id_str = cstr_to_str(account_id)?;
        let name_str = cstr_to_str(name)?;
        let period_str = cstr_to_str(period)?;
        let start_str = cstr_to_str(start_date)?;

        let account_uuid = Uuid::parse_str(account_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid account_id UUID".to_string()))?;

        let category_uuid = if !category_id.is_null() {
            let cat_str = cstr_to_str(category_id)?;
            Some(
                Uuid::parse_str(cat_str)
                    .map_err(|_| FfiResult::error(3, "Invalid category_id UUID".to_string()))?,
            )
        } else {
            None
        };

        let start_dt = chrono::DateTime::parse_from_rfc3339(start_str)
            .map_err(|_| FfiResult::error(3, "Invalid start_date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let budget_repo = SqliteBudgetRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = BudgetUseCases::new(&budget_repo, &transaction_repo);

        let budget = use_cases
            .create_budget(
                account_uuid,
                category_uuid,
                name_str.to_string(),
                amount,
                period_str,
                start_dt,
            )
            .map_err(|e| FfiResult::error(50, format!("{e}")))?;

        let data = serde_json::to_value(&budget)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// List budgets for an account.
///
/// # Safety
/// `account_id` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn list_budgets(account_id: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let account_id_str = cstr_to_str(account_id)?;

        let account_uuid = Uuid::parse_str(account_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid account_id UUID".to_string()))?;

        let budget_repo = SqliteBudgetRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = BudgetUseCases::new(&budget_repo, &transaction_repo);

        let budgets = use_cases
            .list_budgets(account_uuid)
            .map_err(|e| FfiResult::error(51, format!("{e}")))?;

        let data = serde_json::to_value(&budgets)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Delete a budget.
///
/// # Safety
/// `id` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn delete_budget(id: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let id_str = cstr_to_str(id)?;

        let uuid = Uuid::parse_str(id_str)
            .map_err(|_| FfiResult::error(3, "Invalid id UUID".to_string()))?;

        let budget_repo = SqliteBudgetRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = BudgetUseCases::new(&budget_repo, &transaction_repo);

        use_cases
            .delete_budget(uuid)
            .map_err(|e| FfiResult::error(52, format!("{e}")))?;

        Ok(FfiResult::ok_empty())
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Get budget progress for a specific budget.
///
/// # Safety
/// `budget_id` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn get_budget_progress(budget_id: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let budget_id_str = cstr_to_str(budget_id)?;

        let budget_uuid = Uuid::parse_str(budget_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid budget_id UUID".to_string()))?;

        let budget_repo = SqliteBudgetRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = BudgetUseCases::new(&budget_repo, &transaction_repo);

        let progress = use_cases
            .get_budget_progress(budget_uuid)
            .map_err(|e| FfiResult::error(53, format!("{e}")))?;

        let data = serde_json::to_value(&progress)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Free a string that was allocated by the FFI layer.
///
/// # Safety
/// `ptr` must have been returned by one of the FFI functions above.
#[no_mangle]
pub unsafe extern "C" fn free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

// ─── User Functions ──────────────────────────────────────────────────────────

/// Create a new user.
///
/// # Safety
/// All pointer parameters must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn create_user(
    name: *const c_char,
    email: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let name_str = cstr_to_str(name)?;
        let email_str = cstr_to_str(email)?;

        let user = User::new(name_str.to_string(), email_str.to_string());
        let repo = SqliteUserRepository::new(db);
        repo.save(&user)
            .map_err(|e| FfiResult::error(24, format!("{e}")))?;

        let data = serde_json::to_value(&user)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Get a user by ID.
///
/// # Safety
/// `user_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn get_user(user_id: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let user_id_str = cstr_to_str(user_id)?;
        let user_uuid = Uuid::parse_str(user_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid user_id UUID".to_string()))?;

        let repo = SqliteUserRepository::new(db);
        let user = repo
            .find_by_id(user_uuid)
            .map_err(|e| FfiResult::error(24, format!("{e}")))?
            .ok_or_else(|| FfiResult::error(24, "User not found".to_string()))?;

        let data = serde_json::to_value(&user)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

// ─── Account List/Delete ─────────────────────────────────────────────────────

/// List all accounts for a user.
///
/// # Safety
/// `user_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn list_accounts(user_id: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let user_id_str = cstr_to_str(user_id)?;
        let user_uuid = Uuid::parse_str(user_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid user_id UUID".to_string()))?;

        let repo = SqliteAccountRepository::new(db);
        let use_cases = AccountUseCases::new(&repo);
        let accounts = use_cases
            .list_accounts(user_uuid)
            .map_err(|e| FfiResult::error(20, format!("{e}")))?;

        let data = serde_json::to_value(&accounts)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Soft-delete an account.
///
/// # Safety
/// `account_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn delete_account(account_id: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let account_id_str = cstr_to_str(account_id)?;
        let account_uuid = Uuid::parse_str(account_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid account_id UUID".to_string()))?;

        let repo = SqliteAccountRepository::new(db);
        let use_cases = AccountUseCases::new(&repo);
        use_cases
            .delete_account(account_uuid)
            .map_err(|e| FfiResult::error(20, format!("{e}")))?;

        Ok(FfiResult::ok_empty())
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

// ─── Category List/Delete ────────────────────────────────────────────────────

/// List all categories for a user.
///
/// # Safety
/// `user_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn list_categories(user_id: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let user_id_str = cstr_to_str(user_id)?;
        let user_uuid = Uuid::parse_str(user_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid user_id UUID".to_string()))?;

        let repo = SqliteCategoryRepository::new(db);
        let use_cases = CategoryUseCases::new(&repo);
        let categories = use_cases
            .list_categories(user_uuid)
            .map_err(|e| FfiResult::error(21, format!("{e}")))?;

        let data = serde_json::to_value(&categories)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Soft-delete a category.
///
/// # Safety
/// `category_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn delete_category(category_id: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let category_id_str = cstr_to_str(category_id)?;
        let category_uuid = Uuid::parse_str(category_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid category_id UUID".to_string()))?;

        let repo = SqliteCategoryRepository::new(db);
        let use_cases = CategoryUseCases::new(&repo);
        use_cases
            .delete_category(category_uuid)
            .map_err(|e| FfiResult::error(21, format!("{e}")))?;

        Ok(FfiResult::ok_empty())
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

// ─── Transaction Edit/Delete/List ────────────────────────────────────────────

/// Edit an existing transaction.
///
/// # Safety
/// All pointer parameters must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn edit_transaction(
    transaction_id: *const c_char,
    amount: i64,
    transaction_type: *const c_char,
    description: *const c_char,
    category_id: *const c_char,
    date: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let tx_id_str = cstr_to_str(transaction_id)?;
        let type_str = cstr_to_str(transaction_type)?;
        let desc_str = cstr_to_str(description)?;
        let cat_id_str = cstr_to_str(category_id)?;
        let date_str = cstr_to_str(date)?;

        let tx_uuid = Uuid::parse_str(tx_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid transaction_id UUID".to_string()))?;
        let cat_uuid = Uuid::parse_str(cat_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid category_id UUID".to_string()))?;
        let tx_type = TransactionType::from_str(type_str)
            .ok_or_else(|| FfiResult::error(3, "Invalid transaction_type".to_string()))?;
        let parsed_date = chrono::DateTime::parse_from_rfc3339(date_str)
            .map_err(|_| FfiResult::error(3, "Invalid date format (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = TransactionUseCases::new(&repo);

        let transaction = use_cases
            .edit_transaction(tx_uuid, amount, tx_type, desc_str.to_string(), cat_uuid, parsed_date)
            .map_err(|e| FfiResult::error(22, format!("{e}")))?;

        let data = serde_json::to_value(&transaction)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// Soft-delete a transaction.
///
/// # Safety
/// `transaction_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn delete_transaction(transaction_id: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let tx_id_str = cstr_to_str(transaction_id)?;
        let tx_uuid = Uuid::parse_str(tx_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid transaction_id UUID".to_string()))?;

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = TransactionUseCases::new(&repo);
        use_cases
            .delete_transaction(tx_uuid)
            .map_err(|e| FfiResult::error(22, format!("{e}")))?;

        Ok(FfiResult::ok_empty())
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// List transactions for an account.
///
/// # Safety
/// `account_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn list_transactions(account_id: *const c_char) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let account_id_str = cstr_to_str(account_id)?;
        let account_uuid = Uuid::parse_str(account_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid account_id UUID".to_string()))?;

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = TransactionUseCases::new(&repo);
        let transactions = use_cases
            .list_transactions(account_uuid)
            .map_err(|e| FfiResult::error(22, format!("{e}")))?;

        let data = serde_json::to_value(&transactions)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}

/// List transactions for an account within a date range.
///
/// # Safety
/// All pointer parameters must be valid C strings. Dates must be RFC3339 formatted.
#[no_mangle]
pub unsafe extern "C" fn list_transactions_by_date_range(
    account_id: *const c_char,
    from: *const c_char,
    to: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<FfiResult, FfiResult> {
        let db = get_db()?;
        let account_id_str = cstr_to_str(account_id)?;
        let from_str = cstr_to_str(from)?;
        let to_str = cstr_to_str(to)?;

        let account_uuid = Uuid::parse_str(account_id_str)
            .map_err(|_| FfiResult::error(3, "Invalid account_id UUID".to_string()))?;
        let from_date = chrono::DateTime::parse_from_rfc3339(from_str)
            .map_err(|_| FfiResult::error(3, "Invalid from date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);
        let to_date = chrono::DateTime::parse_from_rfc3339(to_str)
            .map_err(|_| FfiResult::error(3, "Invalid to date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = TransactionUseCases::new(&repo);
        let transactions = use_cases
            .list_transactions_by_date_range(account_uuid, from_date, to_date)
            .map_err(|e| FfiResult::error(22, format!("{e}")))?;

        let data = serde_json::to_value(&transactions)
            .map_err(|e| FfiResult::error(4, format!("Serialization error: {e}")))?;

        Ok(FfiResult::ok(data))
    })();

    match result {
        Ok(r) => r.to_json_cstring(),
        Err(r) => r.to_json_cstring(),
    }
}
