use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

use finance_core::entities::common::{BaseEntity, TransactionType};
use finance_core::entities::{RecurrenceFrequency, RecurringTransaction};
use finance_core::errors::DomainError;
use finance_core::repositories::RecurringTransactionRepository;

use crate::database::Database;
use crate::date_utils::{format_dt, format_dt_opt};
use crate::error::StorageError;

/// SQLite implementation of RecurringTransactionRepository.
pub struct SqliteRecurringTransactionRepository<'a> {
    db: &'a Database,
}

impl<'a> SqliteRecurringTransactionRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }
}

impl<'a> RecurringTransactionRepository for SqliteRecurringTransactionRepository<'a> {
    fn save(&self, recurring: &RecurringTransaction) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO recurring_transactions (id, account_id, category_id, amount, transaction_type, description, frequency, start_date, end_date, next_occurrence, is_active, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                recurring.base.id.to_string(),
                recurring.account_id.to_string(),
                recurring.category_id.to_string(),
                recurring.amount,
                recurring.transaction_type.to_string(),
                recurring.description,
                recurring.frequency.to_string(),
                format_dt(&recurring.start_date),
                format_dt_opt(&recurring.end_date),
                format_dt(&recurring.next_occurrence),
                recurring.is_active,
                format_dt(&recurring.base.created_at),
                format_dt(&recurring.base.updated_at),
                format_dt_opt(&recurring.base.deleted_at),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn find_by_id(&self, id: Uuid) -> Result<Option<RecurringTransaction>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, category_id, amount, transaction_type, description, frequency, start_date, end_date, next_occurrence, is_active, created_at, updated_at, deleted_at
                 FROM recurring_transactions WHERE id = ?1",
            )
            .map_err(StorageError::from)?;

        let result = stmt.query_row(params![id.to_string()], |row| row_to_recurring(row));

        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::from(e).into()),
        }
    }

    fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<RecurringTransaction>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, category_id, amount, transaction_type, description, frequency, start_date, end_date, next_occurrence, is_active, created_at, updated_at, deleted_at
                 FROM recurring_transactions WHERE account_id = ?1 AND deleted_at IS NULL ORDER BY created_at DESC",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(params![account_id.to_string()], |row| row_to_recurring(row))
            .map_err(StorageError::from)?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(StorageError::from)?);
        }
        Ok(result)
    }

    fn find_active(&self) -> Result<Vec<RecurringTransaction>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, category_id, amount, transaction_type, description, frequency, start_date, end_date, next_occurrence, is_active, created_at, updated_at, deleted_at
                 FROM recurring_transactions WHERE is_active = 1 AND deleted_at IS NULL",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map([], |row| row_to_recurring(row))
            .map_err(StorageError::from)?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(StorageError::from)?);
        }
        Ok(result)
    }

    fn find_due(&self, before: DateTime<Utc>) -> Result<Vec<RecurringTransaction>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, category_id, amount, transaction_type, description, frequency, start_date, end_date, next_occurrence, is_active, created_at, updated_at, deleted_at
                 FROM recurring_transactions
                 WHERE is_active = 1 AND deleted_at IS NULL AND next_occurrence <= ?1",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(params![format_dt(&before)], |row| row_to_recurring(row))
            .map_err(StorageError::from)?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(StorageError::from)?);
        }
        Ok(result)
    }

    fn update(&self, recurring: &RecurringTransaction) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "UPDATE recurring_transactions SET account_id = ?1, category_id = ?2, amount = ?3, transaction_type = ?4, description = ?5, frequency = ?6, start_date = ?7, end_date = ?8, next_occurrence = ?9, is_active = ?10, updated_at = ?11, deleted_at = ?12 WHERE id = ?13",
            params![
                recurring.account_id.to_string(),
                recurring.category_id.to_string(),
                recurring.amount,
                recurring.transaction_type.to_string(),
                recurring.description,
                recurring.frequency.to_string(),
                format_dt(&recurring.start_date),
                format_dt_opt(&recurring.end_date),
                format_dt(&recurring.next_occurrence),
                recurring.is_active,
                format_dt(&recurring.base.updated_at),
                format_dt_opt(&recurring.base.deleted_at),
                recurring.base.id.to_string(),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let now = format_dt(&Utc::now());
        conn.execute(
            "UPDATE recurring_transactions SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(StorageError::from)?;
        Ok(())
    }
}

fn row_to_recurring(row: &rusqlite::Row) -> rusqlite::Result<RecurringTransaction> {
    let id_str: String = row.get(0)?;
    let account_id_str: String = row.get(1)?;
    let category_id_str: String = row.get(2)?;
    let tx_type_str: String = row.get(4)?;
    let freq_str: String = row.get(6)?;
    let start_str: String = row.get(7)?;
    let end_str: Option<String> = row.get(8)?;
    let next_str: String = row.get(9)?;
    let created_str: String = row.get(11)?;
    let updated_str: String = row.get(12)?;
    let deleted_str: Option<String> = row.get(13)?;

    Ok(RecurringTransaction {
        base: BaseEntity {
            id: Uuid::parse_str(&id_str).unwrap(),
            created_at: DateTime::parse_from_rfc3339(&created_str)
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_str)
                .unwrap()
                .with_timezone(&Utc),
            deleted_at: deleted_str.map(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .unwrap()
                    .with_timezone(&Utc)
            }),
        },
        account_id: Uuid::parse_str(&account_id_str).unwrap(),
        category_id: Uuid::parse_str(&category_id_str).unwrap(),
        amount: row.get(3)?,
        transaction_type: TransactionType::from_str(&tx_type_str)
            .unwrap_or(TransactionType::Expense),
        description: row.get(5)?,
        frequency: RecurrenceFrequency::from_str(&freq_str).unwrap_or(RecurrenceFrequency::Monthly),
        start_date: DateTime::parse_from_rfc3339(&start_str)
            .unwrap()
            .with_timezone(&Utc),
        end_date: end_str.map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .unwrap()
                .with_timezone(&Utc)
        }),
        next_occurrence: DateTime::parse_from_rfc3339(&next_str)
            .unwrap()
            .with_timezone(&Utc),
        is_active: row.get(10)?,
    })
}
