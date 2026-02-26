use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

use finance_core::entities::common::{BaseEntity, SyncStatus, TransactionType};
use finance_core::entities::Transaction;
use finance_core::errors::DomainError;
use finance_core::repositories::TransactionRepository;

use crate::database::Database;
use crate::error::StorageError;

/// SQLite implementation of TransactionRepository.
pub struct SqliteTransactionRepository<'a> {
    db: &'a Database,
}

impl<'a> SqliteTransactionRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }
}

impl<'a> TransactionRepository for SqliteTransactionRepository<'a> {
    fn save(&self, transaction: &Transaction) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO transactions (id, account_id, category_id, amount, transaction_type, description, date, sync_status, version, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                transaction.base.id.to_string(),
                transaction.account_id.to_string(),
                transaction.category_id.to_string(),
                transaction.amount,
                transaction.transaction_type.to_string(),
                transaction.description,
                transaction.date.to_rfc3339(),
                transaction.sync_status.to_string(),
                transaction.version,
                transaction.base.created_at.to_rfc3339(),
                transaction.base.updated_at.to_rfc3339(),
                transaction.base.deleted_at.map(|d| d.to_rfc3339()),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn find_by_id(&self, id: Uuid) -> Result<Option<Transaction>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, category_id, amount, transaction_type, description, date, sync_status, version, created_at, updated_at, deleted_at
                 FROM transactions WHERE id = ?1",
            )
            .map_err(StorageError::from)?;

        let result = stmt.query_row(params![id.to_string()], |row| row_to_transaction(row));

        match result {
            Ok(tx) => Ok(Some(tx)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::from(e).into()),
        }
    }

    fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<Transaction>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, category_id, amount, transaction_type, description, date, sync_status, version, created_at, updated_at, deleted_at
                 FROM transactions WHERE account_id = ?1 AND deleted_at IS NULL ORDER BY date DESC",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(params![account_id.to_string()], |row| {
                row_to_transaction(row)
            })
            .map_err(StorageError::from)?;

        let mut transactions = Vec::new();
        for row in rows {
            transactions.push(row.map_err(StorageError::from)?);
        }
        Ok(transactions)
    }

    fn find_by_date_range(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Transaction>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, category_id, amount, transaction_type, description, date, sync_status, version, created_at, updated_at, deleted_at
                 FROM transactions
                 WHERE account_id = ?1 AND date >= ?2 AND date <= ?3 AND deleted_at IS NULL
                 ORDER BY date DESC",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(
                params![
                    account_id.to_string(),
                    from.to_rfc3339(),
                    to.to_rfc3339()
                ],
                |row| row_to_transaction(row),
            )
            .map_err(StorageError::from)?;

        let mut transactions = Vec::new();
        for row in rows {
            transactions.push(row.map_err(StorageError::from)?);
        }
        Ok(transactions)
    }

    fn update(&self, transaction: &Transaction) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "UPDATE transactions SET account_id = ?1, category_id = ?2, amount = ?3, transaction_type = ?4, description = ?5, date = ?6, sync_status = ?7, version = ?8, updated_at = ?9, deleted_at = ?10 WHERE id = ?11",
            params![
                transaction.account_id.to_string(),
                transaction.category_id.to_string(),
                transaction.amount,
                transaction.transaction_type.to_string(),
                transaction.description,
                transaction.date.to_rfc3339(),
                transaction.sync_status.to_string(),
                transaction.version,
                transaction.base.updated_at.to_rfc3339(),
                transaction.base.deleted_at.map(|d| d.to_rfc3339()),
                transaction.base.id.to_string(),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE transactions SET deleted_at = ?1, updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(StorageError::from)?;
        Ok(())
    }

    fn find_pending_sync(&self) -> Result<Vec<Transaction>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, category_id, amount, transaction_type, description, date, sync_status, version, created_at, updated_at, deleted_at
                 FROM transactions WHERE sync_status = 'pending'",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map([], |row| row_to_transaction(row))
            .map_err(StorageError::from)?;

        let mut transactions = Vec::new();
        for row in rows {
            transactions.push(row.map_err(StorageError::from)?);
        }
        Ok(transactions)
    }

    fn calculate_balance(&self, account_id: Uuid) -> Result<i64, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT
                    COALESCE(SUM(CASE WHEN transaction_type = 'income' THEN amount ELSE 0 END), 0) -
                    COALESCE(SUM(CASE WHEN transaction_type IN ('expense', 'transfer') THEN amount ELSE 0 END), 0)
                 FROM transactions
                 WHERE account_id = ?1 AND deleted_at IS NULL",
            )
            .map_err(StorageError::from)?;

        let balance: i64 = stmt
            .query_row(params![account_id.to_string()], |row| row.get(0))
            .map_err(StorageError::from)?;

        Ok(balance)
    }
}

fn row_to_transaction(row: &rusqlite::Row) -> rusqlite::Result<Transaction> {
    let id_str: String = row.get(0)?;
    let account_id_str: String = row.get(1)?;
    let category_id_str: String = row.get(2)?;
    let tx_type_str: String = row.get(4)?;
    let date_str: String = row.get(6)?;
    let sync_str: String = row.get(7)?;
    let created_str: String = row.get(9)?;
    let updated_str: String = row.get(10)?;
    let deleted_str: Option<String> = row.get(11)?;

    Ok(Transaction {
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
        date: DateTime::parse_from_rfc3339(&date_str)
            .unwrap()
            .with_timezone(&Utc),
        sync_status: SyncStatus::from_str(&sync_str).unwrap_or(SyncStatus::Pending),
        version: row.get(8)?,
    })
}
