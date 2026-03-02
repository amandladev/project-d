use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

use finance_core::entities::{TransactionType, User};
use finance_core::entities::pagination::PageRequest;
use finance_core::entities::search::TransactionSearchFilter;
use finance_core::repositories::{
    AccountRepository, BudgetRepository, CategoryRepository,
    RecurringTransactionRepository, TransactionRepository,
    UserRepository,
};
use finance_core::use_cases::{
    AccountUseCases, BudgetUseCases, CategoryUseCases, CurrencyUseCases,
    RecurringTransactionUseCases, SearchUseCases, StatisticsUseCases, TagUseCases,
    TransactionUseCases,
};
use finance_storage::database::Database;
use finance_storage::repositories::{
    SqliteAccountRepository, SqliteBudgetRepository, SqliteCategoryRepository,
    SqliteExchangeRateRepository, SqliteRecurringTransactionRepository,
    SqliteTagRepository, SqliteTransactionRepository, SqliteUserRepository,
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
        FfiResult::error(ERR_DB_NOT_INIT, "Database not initialized. Call init_database first.".to_string())
    })
}

unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Result<&'a str, FfiResult> {
    if ptr.is_null() {
        return Err(FfiResult::error(ERR_NULL_PTR, "Null pointer received".to_string()));
    }
    CStr::from_ptr(ptr)
        .to_str()
        .map_err(|_| FfiResult::error(ERR_NULL_PTR, "Invalid UTF-8 string".to_string()))
}

// ─── Error Code Constants ────────────────────────────────────────────────────

/// Infrastructure / input errors
const ERR_DB_NOT_INIT: i32 = 1;
const ERR_NULL_PTR: i32 = 2;
const ERR_INVALID_INPUT: i32 = 3;
const ERR_SERIALIZATION: i32 = 4;

/// Database init errors
const ERR_DB_OPEN: i32 = 10;
const ERR_DB_ALREADY_INIT: i32 = 11;

/// Domain errors — accounts
const ERR_ACCOUNT: i32 = 20;

/// Domain errors — categories
const ERR_CATEGORY: i32 = 21;

/// Domain errors — transactions
const ERR_TRANSACTION: i32 = 22;
const ERR_BALANCE: i32 = 23;

/// Domain errors — users
const ERR_USER: i32 = 24;

/// Domain errors — sync / statistics
const ERR_SYNC: i32 = 30;
const ERR_INCOME_EXPENSES: i32 = 31;

/// Domain errors — recurring transactions
const ERR_RECURRING_CREATE: i32 = 40;
const ERR_RECURRING_LIST: i32 = 41;
const ERR_RECURRING_DELETE: i32 = 42;
const ERR_RECURRING_PROCESS: i32 = 43;
const ERR_RECURRING_UPDATE: i32 = 44;
const ERR_RECURRING_GET: i32 = 45;

/// Domain errors — budgets
const ERR_BUDGET_CREATE: i32 = 50;
const ERR_BUDGET_LIST: i32 = 51;
const ERR_BUDGET_DELETE: i32 = 52;
const ERR_BUDGET_PROGRESS: i32 = 53;
const ERR_BUDGET_UPDATE: i32 = 54;
const ERR_BUDGET_GET: i32 = 55;

/// Domain errors — currency / exchange rates
const ERR_RATES_SEED: i32 = 60;
const ERR_RATES_UPDATE: i32 = 61;
const ERR_RATES_MANUAL: i32 = 62;
const ERR_CURRENCY_CONVERT: i32 = 63;
const ERR_RATE_FRESHNESS: i32 = 64;
const ERR_RATES_LIST: i32 = 65;

/// Domain errors — search / transfers
const ERR_SEARCH: i32 = 70;
const ERR_LINKED_TX: i32 = 71;

/// Domain errors — tags
const ERR_TAG_CREATE: i32 = 80;
const ERR_TAG_LIST: i32 = 81;
const ERR_TAG_UPDATE: i32 = 82;
const ERR_TAG_DELETE: i32 = 83;
const ERR_TAG_ADD: i32 = 84;
const ERR_TAG_REMOVE: i32 = 85;
const ERR_TAG_GET_TX: i32 = 86;
const ERR_TAG_BY_TAG: i32 = 87;

/// Domain errors — statistics (spending trends)
const ERR_MONTHLY_TRENDS: i32 = 60;
const ERR_DAILY_SPENDING: i32 = 61;

// ─── FFI Macros ──────────────────────────────────────────────────────────────

/// Wraps an FFI function body with the standard Result closure + dispatch pattern.
/// Eliminates the 4-line match boilerplate from every function.
macro_rules! ffi_body {
    ($body:block) => {{
        let result = (|| -> Result<FfiResult, FfiResult> { $body })();
        match result {
            Ok(r) | Err(r) => r.to_json_cstring(),
        }
    }};
}

/// Parse a UUID from a C string pointer, returning ERR_INVALID_INPUT on failure.
macro_rules! parse_uuid_ffi {
    ($ptr:expr, $label:expr) => {{
        let s = cstr_to_str($ptr)?;
        Uuid::parse_str(s)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, format!("Invalid {} UUID", $label)))?
    }};
}

/// Serialize a value to JSON and wrap in FfiResult::ok, or return ERR_SERIALIZATION on failure.
macro_rules! ok_json {
    ($val:expr) => {{
        let data = serde_json::to_value(&$val)
            .map_err(|e| FfiResult::error(ERR_SERIALIZATION, format!("Serialization error: {e}")))?;
        Ok(FfiResult::ok(data))
    }};
}

/// Build a PageRequest from limit/offset i64 parameters.
macro_rules! page_request {
    ($limit:expr, $offset:expr) => {
        PageRequest {
            limit: if $limit > 0 { $limit as usize } else { 50 },
            offset: if $offset >= 0 { $offset as usize } else { 0 },
        }
    };
}

// ─── FFI Functions ───────────────────────────────────────────────────────────

/// Initialize the database at the given path. Must be called before any other function.
///
/// # Safety
/// `path` must be a valid C string pointer.
#[no_mangle]
pub unsafe extern "C" fn init_database(path: *const c_char) -> *mut c_char {
    ffi_body!({
        let path_str = cstr_to_str(path)?;
        let db = Database::open(Path::new(path_str))
            .map_err(|e| FfiResult::error(ERR_DB_OPEN, format!("Failed to open database: {e}")))?;
        DB.set(db)
            .map_err(|_| FfiResult::error(ERR_DB_ALREADY_INIT, "Database already initialized".to_string()))?;
        Ok(FfiResult::ok_empty())
    })
}

/// Initialize an encrypted database (SQLCipher) at the given path.
/// Must be called before any other function. The key is the encryption passphrase.
/// On first use, creates an encrypted DB. On subsequent uses, decrypts it.
///
/// # Safety
/// `path` and `key` must be valid C string pointers.
#[no_mangle]
pub unsafe extern "C" fn init_database_encrypted(
    path: *const c_char,
    key: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let path_str = cstr_to_str(path)?;
        let key_str = cstr_to_str(key)?;
        let db = Database::open_encrypted(Path::new(path_str), key_str)
            .map_err(|e| FfiResult::error(ERR_DB_OPEN, format!("Failed to open encrypted database: {e}")))?;
        DB.set(db)
            .map_err(|_| FfiResult::error(ERR_DB_ALREADY_INIT, "Database already initialized".to_string()))?;
        Ok(FfiResult::ok_empty())
    })
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
    ffi_body!({
        let db = get_db()?;
        let name_str = cstr_to_str(name)?;
        let currency_str = cstr_to_str(currency)?;

        let user_uuid = parse_uuid_ffi!(user_id, "user_id");

        let repo = SqliteAccountRepository::new(db);
        let use_cases = AccountUseCases::new(&repo);

        let account = use_cases
            .create_account(user_uuid, name_str.to_string(), currency_str.to_string())
            .map_err(|e| FfiResult::error(ERR_ACCOUNT, format!("{e}")))?;

        ok_json!(account)
    })
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
    ffi_body!({
        let db = get_db()?;
        let name_str = cstr_to_str(name)?;
        let icon_str = if icon.is_null() {
            None
        } else {
            Some(cstr_to_str(icon)?.to_string())
        };

        let user_uuid = parse_uuid_ffi!(user_id, "user_id");

        let repo = SqliteCategoryRepository::new(db);
        let use_cases = CategoryUseCases::new(&repo);

        let category = use_cases
            .create_category(user_uuid, name_str.to_string(), icon_str)
            .map_err(|e| FfiResult::error(ERR_CATEGORY, format!("{e}")))?;

        ok_json!(category)
    })
}

/// Seed default categories for a user if none exist yet.
/// Idempotent — returns existing categories if already seeded.
///
/// # Safety
/// `user_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn seed_default_categories(user_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let user_uuid = parse_uuid_ffi!(user_id, "user_id");

        let repo = SqliteCategoryRepository::new(db);
        let use_cases = CategoryUseCases::new(&repo);
        let categories = use_cases
            .seed_default_categories(user_uuid)
            .map_err(|e| FfiResult::error(ERR_CATEGORY, format!("{e}")))?;

        ok_json!(categories)
    })
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
    ffi_body!({
        let db = get_db()?;
        let type_str = cstr_to_str(transaction_type)?;
        let desc_str = cstr_to_str(description)?;
        let date_str = cstr_to_str(date)?;

        let account_uuid = parse_uuid_ffi!(account_id, "account_id");
        let category_uuid = parse_uuid_ffi!(category_id, "category_id");
        let tx_type = TransactionType::from_str(type_str)
            .ok_or_else(|| FfiResult::error(ERR_INVALID_INPUT, "Invalid transaction_type".to_string()))?;
        let parsed_date = chrono::DateTime::parse_from_rfc3339(date_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid date format (expected RFC3339)".to_string()))?
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
            .map_err(|e| FfiResult::error(ERR_TRANSACTION, format!("{e}")))?;

        ok_json!(transaction)
    })
}

/// Get the balance for an account (in cents).
///
/// # Safety
/// `account_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn get_balance(account_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let account_uuid = parse_uuid_ffi!(account_id, "account_id");

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = TransactionUseCases::new(&repo);

        let balance = use_cases
            .get_balance(account_uuid)
            .map_err(|e| FfiResult::error(ERR_BALANCE, format!("{e}")))?;

        Ok(FfiResult::ok(serde_json::json!({ "balance": balance })))
    })
}

/// Get pending sync changes as JSON.
#[no_mangle]
pub extern "C" fn get_pending_sync() -> *mut c_char {
    ffi_body!({
        let db = get_db()?;

        let account_repo = SqliteAccountRepository::new(db);
        let category_repo = SqliteCategoryRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);

        let accounts = account_repo
            .find_pending_sync()
            .map_err(|e| FfiResult::error(ERR_SYNC, format!("{e}")))?;
        let categories = category_repo
            .find_pending_sync()
            .map_err(|e| FfiResult::error(ERR_SYNC, format!("{e}")))?;
        let transactions = transaction_repo
            .find_pending_sync()
            .map_err(|e| FfiResult::error(ERR_SYNC, format!("{e}")))?;

        let data = serde_json::json!({
            "accounts": accounts,
            "categories": categories,
            "transactions": transactions,
        });

        Ok(FfiResult::ok(data))
    })
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
    ffi_body!({
        let db = get_db()?;
        let from_str = cstr_to_str(from)?;
        let to_str = cstr_to_str(to)?;

        let account_uuid = parse_uuid_ffi!(account_id, "account_id");
        let from_date = chrono::DateTime::parse_from_rfc3339(from_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid from date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);
        let to_date = chrono::DateTime::parse_from_rfc3339(to_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid to date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = StatisticsUseCases::new(&repo);
        let spending = use_cases
            .get_spending_by_category(account_uuid, from_date, to_date)
            .map_err(|e| FfiResult::error(ERR_SYNC, format!("{e}")))?;

        ok_json!(spending)
    })
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
    ffi_body!({
        let db = get_db()?;
        let from_str = cstr_to_str(from)?;
        let to_str = cstr_to_str(to)?;

        let account_uuid = parse_uuid_ffi!(account_id, "account_id");
        let from_date = chrono::DateTime::parse_from_rfc3339(from_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid from date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);
        let to_date = chrono::DateTime::parse_from_rfc3339(to_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid to date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = StatisticsUseCases::new(&repo);
        let summary = use_cases
            .get_income_vs_expenses(account_uuid, from_date, to_date)
            .map_err(|e| FfiResult::error(ERR_INCOME_EXPENSES, format!("{e}")))?;

        ok_json!(summary)
    })
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
    ffi_body!({
        let db = get_db()?;
        let tx_type_str = cstr_to_str(transaction_type)?;
        let desc_str = cstr_to_str(description)?;
        let freq_str = cstr_to_str(frequency)?;
        let start_str = cstr_to_str(start_date)?;

        let account_uuid = parse_uuid_ffi!(account_id, "account_id");
        let category_uuid = parse_uuid_ffi!(category_id, "category_id");
        let tx_type = TransactionType::from_str(tx_type_str)
            .ok_or_else(|| FfiResult::error(ERR_INVALID_INPUT, "Invalid transaction_type".to_string()))?;
        let start_dt = chrono::DateTime::parse_from_rfc3339(start_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid start_date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let end_dt = if !end_date.is_null() {
            let end_str = cstr_to_str(end_date)?;
            Some(
                chrono::DateTime::parse_from_rfc3339(end_str)
                    .map_err(|_| {
                        FfiResult::error(ERR_INVALID_INPUT, "Invalid end_date (expected RFC3339)".to_string())
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
            .map_err(|e| FfiResult::error(ERR_RECURRING_CREATE, format!("{e}")))?;

        ok_json!(recurring)
    })
}

/// List recurring transactions for an account (paginated).
///
/// # Safety
/// `account_id` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn list_recurring_transactions(
    account_id: *const c_char,
    limit: i64,
    offset: i64,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let account_uuid = parse_uuid_ffi!(account_id, "account_id");

        let page = page_request!(limit, offset);

        let recurring_repo = SqliteRecurringTransactionRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = RecurringTransactionUseCases::new(&recurring_repo, &transaction_repo);

        let paginated = use_cases
            .list_recurring_transactions_paginated(account_uuid, &page)
            .map_err(|e| FfiResult::error(ERR_RECURRING_LIST, format!("{e}")))?;

        ok_json!(paginated)
    })
}

/// Delete a recurring transaction.
///
/// # Safety
/// `id` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn delete_recurring_transaction(id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let uuid = parse_uuid_ffi!(id, "id");

        let recurring_repo = SqliteRecurringTransactionRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = RecurringTransactionUseCases::new(&recurring_repo, &transaction_repo);

        use_cases
            .delete_recurring_transaction(uuid)
            .map_err(|e| FfiResult::error(ERR_RECURRING_DELETE, format!("{e}")))?;

        Ok(FfiResult::ok_empty())
    })
}

/// Get a single recurring transaction by ID.
///
/// # Safety
/// `id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn get_recurring_transaction(id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let uuid = parse_uuid_ffi!(id, "id");

        let recurring_repo = SqliteRecurringTransactionRepository::new(db);
        let recurring = recurring_repo
            .find_by_id(uuid)
            .map_err(|e| FfiResult::error(ERR_RECURRING_GET, format!("{e}")))?
            .ok_or_else(|| FfiResult::error(ERR_RECURRING_GET, "Recurring transaction not found".to_string()))?;

        ok_json!(recurring)
    })
}

/// Update a recurring transaction.
///
/// Pass a JSON object with optional fields: `{"amount": 5000, "description": "...", "is_active": false}`.
///
/// # Safety
/// All pointer parameters must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn update_recurring_transaction(
    id: *const c_char,
    update_json: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let uuid = parse_uuid_ffi!(id, "id");
        let json_str = cstr_to_str(update_json)?;

        let update: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| FfiResult::error(ERR_INVALID_INPUT, format!("Invalid JSON: {e}")))?;

        let recurring_repo = SqliteRecurringTransactionRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = RecurringTransactionUseCases::new(&recurring_repo, &transaction_repo);

        let mut recurring = recurring_repo
            .find_by_id(uuid)
            .map_err(|e| FfiResult::error(ERR_RECURRING_UPDATE, format!("{e}")))?
            .ok_or_else(|| FfiResult::error(ERR_RECURRING_UPDATE, "Recurring transaction not found".to_string()))?;

        if let Some(amount) = update.get("amount").and_then(|v| v.as_i64()) {
            recurring.amount = amount;
        }
        if let Some(desc) = update.get("description").and_then(|v| v.as_str()) {
            recurring.description = desc.to_string();
        }
        if let Some(active) = update.get("is_active").and_then(|v| v.as_bool()) {
            recurring.is_active = active;
        }
        if let Some(category_id_str) = update.get("category_id").and_then(|v| v.as_str()) {
            recurring.category_id = Uuid::parse_str(category_id_str)
                .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid category_id UUID".to_string()))?;
        }

        recurring.base.touch();
        use_cases
            .update_recurring_transaction(&recurring)
            .map_err(|e| FfiResult::error(ERR_RECURRING_UPDATE, format!("{e}")))?;

        ok_json!(recurring)
    })
}

/// Process due recurring transactions and create actual transactions.
///
/// # Safety
/// This function is safe to call.
#[no_mangle]
pub unsafe extern "C" fn process_due_recurring_transactions() -> *mut c_char {
    ffi_body!({
        let db = get_db()?;

        let recurring_repo = SqliteRecurringTransactionRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = RecurringTransactionUseCases::new(&recurring_repo, &transaction_repo);

        let created_ids = use_cases
            .process_due_recurring_transactions()
            .map_err(|e| FfiResult::error(ERR_RECURRING_PROCESS, format!("{e}")))?;

        ok_json!(created_ids)
    })
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
    ffi_body!({
        let db = get_db()?;
        let name_str = cstr_to_str(name)?;
        let period_str = cstr_to_str(period)?;
        let start_str = cstr_to_str(start_date)?;

        let account_uuid = parse_uuid_ffi!(account_id, "account_id");

        let category_uuid = if !category_id.is_null() {
            Some(parse_uuid_ffi!(category_id, "category_id"))
        } else {
            None
        };

        let start_dt = chrono::DateTime::parse_from_rfc3339(start_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid start_date (expected RFC3339)".to_string()))?
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
            .map_err(|e| FfiResult::error(ERR_BUDGET_CREATE, format!("{e}")))?;

        ok_json!(budget)
    })
}

/// List budgets for an account.
///
/// # Safety
/// `account_id` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn list_budgets(account_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let account_uuid = parse_uuid_ffi!(account_id, "account_id");

        let budget_repo = SqliteBudgetRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = BudgetUseCases::new(&budget_repo, &transaction_repo);

        let budgets = use_cases
            .list_budgets(account_uuid)
            .map_err(|e| FfiResult::error(ERR_BUDGET_LIST, format!("{e}")))?;

        ok_json!(budgets)
    })
}

/// Delete a budget.
///
/// # Safety
/// `id` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn delete_budget(id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let uuid = parse_uuid_ffi!(id, "id");

        let budget_repo = SqliteBudgetRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = BudgetUseCases::new(&budget_repo, &transaction_repo);

        use_cases
            .delete_budget(uuid)
            .map_err(|e| FfiResult::error(ERR_BUDGET_DELETE, format!("{e}")))?;

        Ok(FfiResult::ok_empty())
    })
}

/// Get budget progress for a specific budget.
///
/// # Safety
/// `budget_id` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn get_budget_progress(budget_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let budget_uuid = parse_uuid_ffi!(budget_id, "budget_id");

        let budget_repo = SqliteBudgetRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = BudgetUseCases::new(&budget_repo, &transaction_repo);

        let progress = use_cases
            .get_budget_progress(budget_uuid)
            .map_err(|e| FfiResult::error(ERR_BUDGET_PROGRESS, format!("{e}")))?;

        ok_json!(progress)
    })
}

/// Get a single budget by ID.
///
/// # Safety
/// `budget_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn get_budget(budget_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let budget_uuid = parse_uuid_ffi!(budget_id, "budget_id");

        let budget_repo = SqliteBudgetRepository::new(db);
        let budget = budget_repo
            .find_by_id(budget_uuid)
            .map_err(|e| FfiResult::error(ERR_BUDGET_GET, format!("{e}")))?
            .ok_or_else(|| FfiResult::error(ERR_BUDGET_GET, "Budget not found".to_string()))?;

        ok_json!(budget)
    })
}

/// Update a budget.
///
/// Pass a JSON object with optional fields: `{"name": "...", "amount": 50000, "period": "monthly"}`.
///
/// # Safety
/// All pointer parameters must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn update_budget(
    budget_id: *const c_char,
    update_json: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let budget_uuid = parse_uuid_ffi!(budget_id, "budget_id");
        let json_str = cstr_to_str(update_json)?;

        let update: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| FfiResult::error(ERR_INVALID_INPUT, format!("Invalid JSON: {e}")))?;

        let budget_repo = SqliteBudgetRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = BudgetUseCases::new(&budget_repo, &transaction_repo);

        let mut budget = budget_repo
            .find_by_id(budget_uuid)
            .map_err(|e| FfiResult::error(ERR_BUDGET_UPDATE, format!("{e}")))?
            .ok_or_else(|| FfiResult::error(ERR_BUDGET_UPDATE, "Budget not found".to_string()))?;

        if let Some(name) = update.get("name").and_then(|v| v.as_str()) {
            budget.name = name.to_string();
        }
        if let Some(amount) = update.get("amount").and_then(|v| v.as_i64()) {
            budget.amount = amount;
        }
        if let Some(period_str) = update.get("period").and_then(|v| v.as_str()) {
            budget.period = finance_core::entities::BudgetPeriod::from_str(period_str)
                .ok_or_else(|| FfiResult::error(ERR_INVALID_INPUT, "Invalid budget period".to_string()))?;
        }

        budget.base.touch();
        use_cases
            .update_budget(&budget)
            .map_err(|e| FfiResult::error(ERR_BUDGET_UPDATE, format!("{e}")))?;

        ok_json!(budget)
    })
}

// ─── Currency Conversion Functions ───────────────────────────────────────────

/// Seed bundled default exchange rates into the database. Call once after init_database.
///
/// # Safety
/// Database must be initialized.
#[no_mangle]
pub unsafe extern "C" fn seed_exchange_rates() -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let repo = SqliteExchangeRateRepository::new(db);
        let use_cases = CurrencyUseCases::new(&repo);

        let count = use_cases
            .seed_bundled_rates()
            .map_err(|e| FfiResult::error(ERR_RATES_SEED, format!("{e}")))?;

        Ok(FfiResult::ok(serde_json::json!({ "seeded": count })))
    })
}

/// Update cached exchange rates from external API data.
/// `rates_json` must be a JSON array: `[{"from":"USD","to":"EUR","rate":0.92}, ...]`
///
/// # Safety
/// `rates_json` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn update_exchange_rates(rates_json: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let json_str = cstr_to_str(rates_json)?;

        let repo = SqliteExchangeRateRepository::new(db);
        let use_cases = CurrencyUseCases::new(&repo);

        let count = use_cases
            .update_cached_rates(json_str)
            .map_err(|e| FfiResult::error(ERR_RATES_UPDATE, format!("{e}")))?;

        Ok(FfiResult::ok(serde_json::json!({ "updated": count })))
    })
}

/// Set a manual exchange rate (user override — highest priority).
///
/// # Safety
/// `from_currency` and `to_currency` must be valid C strings. `rate` is a float.
#[no_mangle]
pub unsafe extern "C" fn set_manual_exchange_rate(
    from_currency: *const c_char,
    to_currency: *const c_char,
    rate: f64,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let from_str = cstr_to_str(from_currency)?;
        let to_str = cstr_to_str(to_currency)?;

        let repo = SqliteExchangeRateRepository::new(db);
        let use_cases = CurrencyUseCases::new(&repo);

        let exchange_rate = use_cases
            .set_manual_rate(from_str, to_str, rate)
            .map_err(|e| FfiResult::error(ERR_RATES_MANUAL, format!("{e}")))?;

        ok_json!(exchange_rate)
    })
}

/// Convert an amount between currencies using the 3-tier rate resolution.
///
/// # Safety
/// `from_currency` and `to_currency` must be valid C strings. `amount_cents` is in cents.
#[no_mangle]
pub unsafe extern "C" fn convert_currency(
    amount_cents: i64,
    from_currency: *const c_char,
    to_currency: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let from_str = cstr_to_str(from_currency)?;
        let to_str = cstr_to_str(to_currency)?;

        let repo = SqliteExchangeRateRepository::new(db);
        let use_cases = CurrencyUseCases::new(&repo);

        let conversion = use_cases
            .convert(amount_cents, from_str, to_str)
            .map_err(|e| FfiResult::error(ERR_CURRENCY_CONVERT, format!("{e}")))?;

        ok_json!(conversion)
    })
}

/// Get rate freshness info for a currency pair.
///
/// # Safety
/// `from_currency` and `to_currency` must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn get_rate_freshness(
    from_currency: *const c_char,
    to_currency: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let from_str = cstr_to_str(from_currency)?;
        let to_str = cstr_to_str(to_currency)?;

        let repo = SqliteExchangeRateRepository::new(db);
        let use_cases = CurrencyUseCases::new(&repo);

        let freshness = use_cases
            .get_rate_freshness(from_str, to_str)
            .map_err(|e| FfiResult::error(ERR_RATE_FRESHNESS, format!("{e}")))?;

        ok_json!(freshness)
    })
}

/// List all exchange rates from a base currency.
///
/// # Safety
/// `from_currency` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn list_exchange_rates(from_currency: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let from_str = cstr_to_str(from_currency)?;

        let repo = SqliteExchangeRateRepository::new(db);
        let use_cases = CurrencyUseCases::new(&repo);

        let rates = use_cases
            .list_rates(from_str)
            .map_err(|e| FfiResult::error(ERR_RATES_LIST, format!("{e}")))?;

        ok_json!(rates)
    })
}

// ─── Search & Filtering ──────────────────────────────────────────────────────

/// Search transactions with flexible filtering.
/// `filter_json` is a JSON object with optional fields:
/// - account_id (required, UUID string)
/// - query (optional, text search on description)
/// - category_id (optional, UUID string)
/// - transaction_type (optional, "income"/"expense"/"transfer")
/// - min_amount, max_amount (optional, cents)
/// - date_from, date_to (optional, RFC3339)
/// - limit, offset (optional, pagination)
///
/// # Safety
/// `filter_json` must be a valid C string containing JSON.
#[no_mangle]
pub unsafe extern "C" fn search_transactions(filter_json: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let json_str = cstr_to_str(filter_json)?;

        let filter: TransactionSearchFilter = serde_json::from_str(json_str)
            .map_err(|e| FfiResult::error(ERR_INVALID_INPUT, format!("Invalid filter JSON: {e}")))?;

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = SearchUseCases::new(&repo);

        let paginated = use_cases
            .search_transactions(&filter)
            .map_err(|e| FfiResult::error(ERR_SEARCH, format!("{e}")))?;

        ok_json!(paginated)
    })
}

// ─── Tag Functions ───────────────────────────────────────────────────────────

/// Create a new tag for a user.
///
/// # Safety
/// All pointer parameters must be valid C strings. `color` can be null.
#[no_mangle]
pub unsafe extern "C" fn create_tag(
    user_id: *const c_char,
    name: *const c_char,
    color: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let name_str = cstr_to_str(name)?;
        let color_str = if color.is_null() {
            None
        } else {
            Some(cstr_to_str(color)?.to_string())
        };

        let user_uuid = parse_uuid_ffi!(user_id, "user_id");

        let repo = SqliteTagRepository::new(db);
        let use_cases = TagUseCases::new(&repo);

        let tag = use_cases
            .create_tag(user_uuid, name_str.to_string(), color_str)
            .map_err(|e| FfiResult::error(ERR_TAG_CREATE, format!("{e}")))?;

        ok_json!(tag)
    })
}

/// List all tags for a user.
///
/// # Safety
/// `user_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn list_tags(user_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let user_uuid = parse_uuid_ffi!(user_id, "user_id");

        let repo = SqliteTagRepository::new(db);
        let use_cases = TagUseCases::new(&repo);
        let tags = use_cases
            .list_tags(user_uuid)
            .map_err(|e| FfiResult::error(ERR_TAG_LIST, format!("{e}")))?;

        ok_json!(tags)
    })
}

/// Update a tag's name and/or color.
/// Pass JSON: {"name": "New Name", "color": "#FF5733"} — both fields are optional.
///
/// # Safety
/// All pointer parameters must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn update_tag(
    tag_id: *const c_char,
    update_json: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let json_str = cstr_to_str(update_json)?;

        let tag_uuid = parse_uuid_ffi!(tag_id, "tag_id");

        let update: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| FfiResult::error(ERR_SERIALIZATION, format!("Invalid JSON: {e}")))?;

        let name = update.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
        let color = update.get("color").map(|v| v.as_str().map(|s| s.to_string()));

        let repo = SqliteTagRepository::new(db);
        let use_cases = TagUseCases::new(&repo);
        let tag = use_cases
            .update_tag(tag_uuid, name, color)
            .map_err(|e| FfiResult::error(ERR_TAG_UPDATE, format!("{e}")))?;

        ok_json!(tag)
    })
}

/// Delete a tag and all its transaction associations.
///
/// # Safety
/// `tag_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn delete_tag(tag_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let tag_uuid = parse_uuid_ffi!(tag_id, "tag_id");

        let repo = SqliteTagRepository::new(db);
        let use_cases = TagUseCases::new(&repo);
        use_cases
            .delete_tag(tag_uuid)
            .map_err(|e| FfiResult::error(ERR_TAG_DELETE, format!("{e}")))?;

        Ok(FfiResult::ok_empty())
    })
}

/// Add a tag to a transaction.
///
/// # Safety
/// Both parameters must be valid C strings containing UUIDs.
#[no_mangle]
pub unsafe extern "C" fn add_tag_to_transaction(
    transaction_id: *const c_char,
    tag_id: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let tx_uuid = parse_uuid_ffi!(transaction_id, "transaction_id");
        let tag_uuid = parse_uuid_ffi!(tag_id, "tag_id");

        let repo = SqliteTagRepository::new(db);
        let use_cases = TagUseCases::new(&repo);
        use_cases
            .add_tag_to_transaction(tx_uuid, tag_uuid)
            .map_err(|e| FfiResult::error(ERR_TAG_ADD, format!("{e}")))?;

        Ok(FfiResult::ok_empty())
    })
}

/// Remove a tag from a transaction.
///
/// # Safety
/// Both parameters must be valid C strings containing UUIDs.
#[no_mangle]
pub unsafe extern "C" fn remove_tag_from_transaction(
    transaction_id: *const c_char,
    tag_id: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let tx_uuid = parse_uuid_ffi!(transaction_id, "transaction_id");
        let tag_uuid = parse_uuid_ffi!(tag_id, "tag_id");

        let repo = SqliteTagRepository::new(db);
        let use_cases = TagUseCases::new(&repo);
        use_cases
            .remove_tag_from_transaction(tx_uuid, tag_uuid)
            .map_err(|e| FfiResult::error(ERR_TAG_REMOVE, format!("{e}")))?;

        Ok(FfiResult::ok_empty())
    })
}

/// Get all tags attached to a transaction.
///
/// # Safety
/// `transaction_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn get_transaction_tags(transaction_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let tx_uuid = parse_uuid_ffi!(transaction_id, "transaction_id");

        let repo = SqliteTagRepository::new(db);
        let use_cases = TagUseCases::new(&repo);
        let tags = use_cases
            .get_transaction_tags(tx_uuid)
            .map_err(|e| FfiResult::error(ERR_TAG_GET_TX, format!("{e}")))?;

        ok_json!(tags)
    })
}

/// Get paginated transaction IDs that have a given tag.
///
/// # Safety
/// `tag_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn get_transactions_by_tag(
    tag_id: *const c_char,
    limit: i64,
    offset: i64,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let tag_uuid = parse_uuid_ffi!(tag_id, "tag_id");

        let page = page_request!(limit, offset);

        let repo = SqliteTagRepository::new(db);
        let use_cases = TagUseCases::new(&repo);
        let paginated = use_cases
            .get_transactions_by_tag_paginated(tag_uuid, &page)
            .map_err(|e| FfiResult::error(ERR_TAG_BY_TAG, format!("{e}")))?;

        ok_json!(paginated)
    })
}

// ─── Budget Progress (All) ──────────────────────────────────────────────────

/// Get progress for all budgets in an account.
///
/// # Safety
/// `account_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn get_all_budgets_progress(account_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let account_uuid = parse_uuid_ffi!(account_id, "account_id");

        let budget_repo = SqliteBudgetRepository::new(db);
        let transaction_repo = SqliteTransactionRepository::new(db);
        let use_cases = BudgetUseCases::new(&budget_repo, &transaction_repo);

        let budgets = use_cases
            .list_budgets(account_uuid)
            .map_err(|e| FfiResult::error(ERR_BUDGET_LIST, format!("{e}")))?;

        let mut progress_list = Vec::new();
        for budget in &budgets {
            let progress = use_cases
                .get_budget_progress(budget.base.id)
                .map_err(|e| FfiResult::error(ERR_BUDGET_PROGRESS, format!("{e}")))?;
            progress_list.push(progress);
        }

        ok_json!(progress_list)
    })
}

// ─── Spending Trends ─────────────────────────────────────────────────────────

/// Get monthly trends for an account within a date range.
///
/// Returns JSON array of { year, month, income, expenses, net, transaction_count }.
///
/// # Safety
/// All pointer parameters must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn get_monthly_trends(
    account_id: *const c_char,
    from: *const c_char,
    to: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let from_str = cstr_to_str(from)?;
        let to_str = cstr_to_str(to)?;

        let account_uuid = parse_uuid_ffi!(account_id, "account_id");
        let from_date = chrono::DateTime::parse_from_rfc3339(from_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid from date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);
        let to_date = chrono::DateTime::parse_from_rfc3339(to_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid to date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = StatisticsUseCases::new(&repo);
        let trends = use_cases
            .get_monthly_trends(account_uuid, from_date, to_date)
            .map_err(|e| FfiResult::error(ERR_MONTHLY_TRENDS, format!("{e}")))?;

        ok_json!(trends)
    })
}

/// Get daily spending for an account within a date range.
///
/// Returns JSON array of { date, amount, transaction_count }.
///
/// # Safety
/// All pointer parameters must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn get_daily_spending(
    account_id: *const c_char,
    from: *const c_char,
    to: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let from_str = cstr_to_str(from)?;
        let to_str = cstr_to_str(to)?;

        let account_uuid = parse_uuid_ffi!(account_id, "account_id");
        let from_date = chrono::DateTime::parse_from_rfc3339(from_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid from date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);
        let to_date = chrono::DateTime::parse_from_rfc3339(to_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid to date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = StatisticsUseCases::new(&repo);
        let spending = use_cases
            .get_daily_spending(account_uuid, from_date, to_date)
            .map_err(|e| FfiResult::error(ERR_DAILY_SPENDING, format!("{e}")))?;

        ok_json!(spending)
    })
}

// ─── Transfer Linking ────────────────────────────────────────────────────────

/// Create a transfer between two accounts.
///
/// Creates two linked transactions: an expense from the source account and an
/// income to the destination account. Returns JSON with both transactions.
///
/// # Safety
/// All pointer parameters must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn create_transfer(
    from_account_id: *const c_char,
    to_account_id: *const c_char,
    category_id: *const c_char,
    amount: i64,
    description: *const c_char,
    date: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let desc_str = cstr_to_str(description)?;
        let date_str = cstr_to_str(date)?;

        let from_uuid = parse_uuid_ffi!(from_account_id, "from_account_id");
        let to_uuid = parse_uuid_ffi!(to_account_id, "to_account_id");
        let cat_uuid = parse_uuid_ffi!(category_id, "category_id");
        let parsed_date = chrono::DateTime::parse_from_rfc3339(date_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = TransactionUseCases::new(&repo);
        let (outgoing, incoming) = use_cases
            .create_transfer(from_uuid, to_uuid, cat_uuid, amount, desc_str.to_string(), parsed_date)
            .map_err(|e| FfiResult::error(ERR_SEARCH, format!("{e}")))?;

        let data = serde_json::json!({
            "outgoing": serde_json::to_value(&outgoing)
                .map_err(|e| FfiResult::error(ERR_SERIALIZATION, format!("Serialization error: {e}")))?,
            "incoming": serde_json::to_value(&incoming)
                .map_err(|e| FfiResult::error(ERR_SERIALIZATION, format!("Serialization error: {e}")))?,
        });

        Ok(FfiResult::ok(data))
    })
}

/// Get the linked transaction of a transfer.
///
/// Given one side of a transfer, returns the other side.
///
/// # Safety
/// `transaction_id` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn get_linked_transaction(transaction_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let tx_uuid = parse_uuid_ffi!(transaction_id, "transaction_id");

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = TransactionUseCases::new(&repo);
        let linked = use_cases
            .get_linked_transaction(tx_uuid)
            .map_err(|e| FfiResult::error(ERR_LINKED_TX, format!("{e}")))?;

        match linked {
            Some(tx) => ok_json!(tx),
            None => Ok(FfiResult::ok(serde_json::Value::Null)),
        }
    })
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
    ffi_body!({
        let db = get_db()?;
        let name_str = cstr_to_str(name)?;
        let email_str = cstr_to_str(email)?;

        let user = User::new(name_str.to_string(), email_str.to_string());
        let repo = SqliteUserRepository::new(db);
        repo.save(&user)
            .map_err(|e| FfiResult::error(ERR_USER, format!("{e}")))?;

        ok_json!(user)
    })
}

/// Get a user by ID.
///
/// # Safety
/// `user_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn get_user(user_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let user_uuid = parse_uuid_ffi!(user_id, "user_id");

        let repo = SqliteUserRepository::new(db);
        let user = repo
            .find_by_id(user_uuid)
            .map_err(|e| FfiResult::error(ERR_USER, format!("{e}")))?
            .ok_or_else(|| FfiResult::error(ERR_USER, "User not found".to_string()))?;

        ok_json!(user)
    })
}

// ─── Account List/Delete ─────────────────────────────────────────────────────

/// List all accounts for a user.
///
/// # Safety
/// `user_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn list_accounts(user_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let user_uuid = parse_uuid_ffi!(user_id, "user_id");

        let repo = SqliteAccountRepository::new(db);
        let use_cases = AccountUseCases::new(&repo);
        let accounts = use_cases
            .list_accounts(user_uuid)
            .map_err(|e| FfiResult::error(ERR_ACCOUNT, format!("{e}")))?;

        ok_json!(accounts)
    })
}

/// Soft-delete an account.
///
/// # Safety
/// `account_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn delete_account(account_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let account_uuid = parse_uuid_ffi!(account_id, "account_id");

        let repo = SqliteAccountRepository::new(db);
        let use_cases = AccountUseCases::new(&repo);
        use_cases
            .delete_account(account_uuid)
            .map_err(|e| FfiResult::error(ERR_ACCOUNT, format!("{e}")))?;

        Ok(FfiResult::ok_empty())
    })
}

/// Get a single account by ID.
///
/// # Safety
/// `account_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn get_account(account_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let account_uuid = parse_uuid_ffi!(account_id, "account_id");

        let repo = SqliteAccountRepository::new(db);
        let use_cases = AccountUseCases::new(&repo);
        let account = use_cases
            .get_account(account_uuid)
            .map_err(|e| FfiResult::error(ERR_ACCOUNT, format!("{e}")))?;

        ok_json!(account)
    })
}

/// Update an account's name and/or currency.
///
/// Pass a JSON object with optional fields: `{"name": "...", "currency": "..."}`.
///
/// # Safety
/// All pointer parameters must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn update_account(
    account_id: *const c_char,
    update_json: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let account_uuid = parse_uuid_ffi!(account_id, "account_id");
        let json_str = cstr_to_str(update_json)?;

        let update: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| FfiResult::error(ERR_INVALID_INPUT, format!("Invalid JSON: {e}")))?;

        let name = update.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
        let currency = update.get("currency").and_then(|v| v.as_str()).map(|s| s.to_string());

        let repo = SqliteAccountRepository::new(db);
        let use_cases = AccountUseCases::new(&repo);
        let account = use_cases
            .update_account(account_uuid, name, currency)
            .map_err(|e| FfiResult::error(ERR_ACCOUNT, format!("{e}")))?;

        ok_json!(account)
    })
}

// ─── Category List/Delete ────────────────────────────────────────────────────

/// List all categories for a user.
///
/// # Safety
/// `user_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn list_categories(user_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let user_uuid = parse_uuid_ffi!(user_id, "user_id");

        let repo = SqliteCategoryRepository::new(db);
        let use_cases = CategoryUseCases::new(&repo);
        let categories = use_cases
            .list_categories(user_uuid)
            .map_err(|e| FfiResult::error(ERR_CATEGORY, format!("{e}")))?;

        ok_json!(categories)
    })
}

/// Soft-delete a category.
///
/// # Safety
/// `category_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn delete_category(category_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let category_uuid = parse_uuid_ffi!(category_id, "category_id");

        let repo = SqliteCategoryRepository::new(db);
        let use_cases = CategoryUseCases::new(&repo);
        use_cases
            .delete_category(category_uuid)
            .map_err(|e| FfiResult::error(ERR_CATEGORY, format!("{e}")))?;

        Ok(FfiResult::ok_empty())
    })
}

/// Get a single category by ID.
///
/// # Safety
/// `category_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn get_category(category_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let category_uuid = parse_uuid_ffi!(category_id, "category_id");

        let repo = SqliteCategoryRepository::new(db);
        let use_cases = CategoryUseCases::new(&repo);
        let category = use_cases
            .get_category(category_uuid)
            .map_err(|e| FfiResult::error(ERR_CATEGORY, format!("{e}")))?;

        ok_json!(category)
    })
}

/// Update a category's name and/or icon.
///
/// Pass a JSON object with optional fields: `{"name": "...", "icon": "..."}`.
/// Set `"icon": null` to remove the icon.
///
/// # Safety
/// All pointer parameters must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn update_category(
    category_id: *const c_char,
    update_json: *const c_char,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let category_uuid = parse_uuid_ffi!(category_id, "category_id");
        let json_str = cstr_to_str(update_json)?;

        let update: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| FfiResult::error(ERR_INVALID_INPUT, format!("Invalid JSON: {e}")))?;

        let name = update.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
        // icon: if key exists, update it (null → None, string → Some)
        let icon = if update.get("icon").is_some() {
            Some(update["icon"].as_str().map(|s| s.to_string()))
        } else {
            None
        };

        let repo = SqliteCategoryRepository::new(db);
        let use_cases = CategoryUseCases::new(&repo);
        let category = use_cases
            .update_category(category_uuid, name, icon)
            .map_err(|e| FfiResult::error(ERR_CATEGORY, format!("{e}")))?;

        ok_json!(category)
    })
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
    ffi_body!({
        let db = get_db()?;
        let type_str = cstr_to_str(transaction_type)?;
        let desc_str = cstr_to_str(description)?;
        let date_str = cstr_to_str(date)?;

        let tx_uuid = parse_uuid_ffi!(transaction_id, "transaction_id");
        let cat_uuid = parse_uuid_ffi!(category_id, "category_id");
        let tx_type = TransactionType::from_str(type_str)
            .ok_or_else(|| FfiResult::error(ERR_INVALID_INPUT, "Invalid transaction_type".to_string()))?;
        let parsed_date = chrono::DateTime::parse_from_rfc3339(date_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid date format (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = TransactionUseCases::new(&repo);

        let transaction = use_cases
            .edit_transaction(tx_uuid, amount, tx_type, desc_str.to_string(), cat_uuid, parsed_date)
            .map_err(|e| FfiResult::error(ERR_TRANSACTION, format!("{e}")))?;

        ok_json!(transaction)
    })
}

/// Soft-delete a transaction.
///
/// # Safety
/// `transaction_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn delete_transaction(transaction_id: *const c_char) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let tx_uuid = parse_uuid_ffi!(transaction_id, "transaction_id");

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = TransactionUseCases::new(&repo);
        use_cases
            .delete_transaction(tx_uuid)
            .map_err(|e| FfiResult::error(ERR_TRANSACTION, format!("{e}")))?;

        Ok(FfiResult::ok_empty())
    })
}

/// List transactions for an account.
///
/// Returns a paginated result with `items`, `total_count`, and `has_more`.
///
/// # Safety
/// `account_id` must be a valid C string containing a UUID.
#[no_mangle]
pub unsafe extern "C" fn list_transactions(
    account_id: *const c_char,
    limit: i64,
    offset: i64,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let account_uuid = parse_uuid_ffi!(account_id, "account_id");

        let page = page_request!(limit, offset);

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = TransactionUseCases::new(&repo);
        let paginated = use_cases
            .list_transactions_paginated(account_uuid, &page)
            .map_err(|e| FfiResult::error(ERR_TRANSACTION, format!("{e}")))?;

        ok_json!(paginated)
    })
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
    limit: i64,
    offset: i64,
) -> *mut c_char {
    ffi_body!({
        let db = get_db()?;
        let from_str = cstr_to_str(from)?;
        let to_str = cstr_to_str(to)?;

        let account_uuid = parse_uuid_ffi!(account_id, "account_id");
        let from_date = chrono::DateTime::parse_from_rfc3339(from_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid from date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);
        let to_date = chrono::DateTime::parse_from_rfc3339(to_str)
            .map_err(|_| FfiResult::error(ERR_INVALID_INPUT, "Invalid to date (expected RFC3339)".to_string()))?
            .with_timezone(&chrono::Utc);

        let page = page_request!(limit, offset);

        let repo = SqliteTransactionRepository::new(db);
        let use_cases = TransactionUseCases::new(&repo);
        let paginated = use_cases
            .list_transactions_by_date_range_paginated(account_uuid, from_date, to_date, &page)
            .map_err(|e| FfiResult::error(ERR_TRANSACTION, format!("{e}")))?;

        ok_json!(paginated)
    })
}
