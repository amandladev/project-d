use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

use finance_core::entities::TransactionType;
use finance_core::repositories::{AccountRepository, TransactionRepository, CategoryRepository};
use finance_core::use_cases::{AccountUseCases, CategoryUseCases, TransactionUseCases};
use finance_storage::database::Database;
use finance_storage::repositories::{
    SqliteAccountRepository, SqliteCategoryRepository, SqliteTransactionRepository,
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
