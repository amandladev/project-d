use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use finance_core::entities::common::{BaseEntity, SyncStatus};
use finance_core::entities::Account;
use finance_core::errors::DomainError;
use finance_core::repositories::AccountRepository;

use crate::database::Database;
use crate::date_utils::{format_dt, format_dt_opt, parse_dt, parse_dt_opt, parse_uuid};
use crate::error::StorageError;

/// SQLite implementation of AccountRepository.
pub struct SqliteAccountRepository<'a> {
    db: &'a Database,
}

impl<'a> SqliteAccountRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }
}

impl<'a> AccountRepository for SqliteAccountRepository<'a> {
    fn save(&self, account: &Account) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO accounts (id, user_id, name, currency, sync_status, version, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                account.base.id.to_string(),
                account.user_id.to_string(),
                account.name,
                account.currency,
                account.sync_status.to_string(),
                account.version,
                format_dt(&account.base.created_at),
                format_dt(&account.base.updated_at),
                format_dt_opt(&account.base.deleted_at),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn find_by_id(&self, id: Uuid) -> Result<Option<Account>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, name, currency, sync_status, version, created_at, updated_at, deleted_at
                 FROM accounts WHERE id = ?1 AND deleted_at IS NULL",
            )
            .map_err(StorageError::from)?;

        let result = stmt.query_row(params![id.to_string()], |row| row_to_account(row));

        match result {
            Ok(account) => Ok(Some(account)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::from(e).into()),
        }
    }

    fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Account>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, name, currency, sync_status, version, created_at, updated_at, deleted_at
                 FROM accounts WHERE user_id = ?1 AND deleted_at IS NULL",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(params![user_id.to_string()], |row| row_to_account(row))
            .map_err(StorageError::from)?;

        let mut accounts = Vec::new();
        for row in rows {
            accounts.push(row.map_err(StorageError::from)?);
        }
        Ok(accounts)
    }

    fn update(&self, account: &Account) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "UPDATE accounts SET name = ?1, currency = ?2, sync_status = ?3, version = ?4, updated_at = ?5, deleted_at = ?6 WHERE id = ?7",
            params![
                account.name,
                account.currency,
                account.sync_status.to_string(),
                account.version,
                format_dt(&account.base.updated_at),
                format_dt_opt(&account.base.deleted_at),
                account.base.id.to_string(),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let now = format_dt(&Utc::now());
        conn.execute(
            "UPDATE accounts SET deleted_at = ?1, updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(StorageError::from)?;
        Ok(())
    }

    fn find_pending_sync(&self) -> Result<Vec<Account>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, name, currency, sync_status, version, created_at, updated_at, deleted_at
                 FROM accounts WHERE sync_status = 'pending'",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map([], |row| row_to_account(row))
            .map_err(StorageError::from)?;

        let mut accounts = Vec::new();
        for row in rows {
            accounts.push(row.map_err(StorageError::from)?);
        }
        Ok(accounts)
    }
}

fn row_to_account(row: &rusqlite::Row) -> rusqlite::Result<Account> {
    let id_str: String = row.get(0)?;
    let user_id_str: String = row.get(1)?;
    let sync_str: String = row.get(4)?;
    let created_str: String = row.get(6)?;
    let updated_str: String = row.get(7)?;
    let deleted_str: Option<String> = row.get(8)?;

    Ok(Account {
        base: BaseEntity {
            id: parse_uuid(&id_str)?,
            created_at: parse_dt(&created_str)?,
            updated_at: parse_dt(&updated_str)?,
            deleted_at: parse_dt_opt(deleted_str)?,
        },
        user_id: parse_uuid(&user_id_str)?,
        name: row.get(2)?,
        currency: row.get(3)?,
        sync_status: SyncStatus::from_str(&sync_str).unwrap_or(SyncStatus::Pending),
        version: row.get(5)?,
    })
}
