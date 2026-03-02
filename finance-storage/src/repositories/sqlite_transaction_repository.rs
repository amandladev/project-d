use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

use finance_core::entities::common::{BaseEntity, SyncStatus, TransactionType};
use finance_core::entities::pagination::{PageRequest, PaginatedResult};
use finance_core::entities::Transaction;
use finance_core::entities::search::TransactionSearchFilter;
use finance_core::errors::DomainError;
use finance_core::repositories::TransactionRepository;
use finance_core::use_cases::statistics_use_cases::{CategorySpending, DailySpending, IncomeSummary, MonthlyTrend};

use crate::database::Database;
use crate::date_utils::{format_dt, format_dt_opt, parse_dt, parse_dt_opt, parse_uuid, parse_uuid_opt};
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
            "INSERT INTO transactions (id, account_id, category_id, amount, transaction_type, description, date, linked_transaction_id, sync_status, version, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                transaction.base.id.to_string(),
                transaction.account_id.to_string(),
                transaction.category_id.to_string(),
                transaction.amount,
                transaction.transaction_type.to_string(),
                transaction.description,
                format_dt(&transaction.date),
                transaction.linked_transaction_id.map(|id| id.to_string()),
                transaction.sync_status.to_string(),
                transaction.version,
                format_dt(&transaction.base.created_at),
                format_dt(&transaction.base.updated_at),
                format_dt_opt(&transaction.base.deleted_at),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn find_by_id(&self, id: Uuid) -> Result<Option<Transaction>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, category_id, amount, transaction_type, description, date, linked_transaction_id, sync_status, version, created_at, updated_at, deleted_at
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
                "SELECT id, account_id, category_id, amount, transaction_type, description, date, linked_transaction_id, sync_status, version, created_at, updated_at, deleted_at
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

    fn find_by_account_id_paginated(
        &self,
        account_id: Uuid,
        page: &PageRequest,
    ) -> Result<PaginatedResult<Transaction>, DomainError> {
        let conn = self.db.conn.lock().unwrap();

        let total_count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM transactions WHERE account_id = ?1 AND deleted_at IS NULL",
                params![account_id.to_string()],
                |row| row.get(0),
            )
            .map_err(StorageError::from)?;

        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, category_id, amount, transaction_type, description, date, linked_transaction_id, sync_status, version, created_at, updated_at, deleted_at
                 FROM transactions WHERE account_id = ?1 AND deleted_at IS NULL ORDER BY date DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(
                params![account_id.to_string(), page.limit as i64, page.offset as i64],
                |row| row_to_transaction(row),
            )
            .map_err(StorageError::from)?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(StorageError::from)?);
        }
        Ok(PaginatedResult::from_vec(items, total_count, page))
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
                "SELECT id, account_id, category_id, amount, transaction_type, description, date, linked_transaction_id, sync_status, version, created_at, updated_at, deleted_at
                 FROM transactions
                 WHERE account_id = ?1 AND date >= ?2 AND date <= ?3 AND deleted_at IS NULL
                 ORDER BY date DESC",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(
                params![
                    account_id.to_string(),
                    format_dt(&from),
                    format_dt(&to)
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

    fn find_by_date_range_paginated(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        page: &PageRequest,
    ) -> Result<PaginatedResult<Transaction>, DomainError> {
        let conn = self.db.conn.lock().unwrap();

        let total_count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM transactions
                 WHERE account_id = ?1 AND date >= ?2 AND date <= ?3 AND deleted_at IS NULL",
                params![account_id.to_string(), format_dt(&from), format_dt(&to)],
                |row| row.get(0),
            )
            .map_err(StorageError::from)?;

        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, category_id, amount, transaction_type, description, date, linked_transaction_id, sync_status, version, created_at, updated_at, deleted_at
                 FROM transactions
                 WHERE account_id = ?1 AND date >= ?2 AND date <= ?3 AND deleted_at IS NULL
                 ORDER BY date DESC
                 LIMIT ?4 OFFSET ?5",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(
                params![
                    account_id.to_string(),
                    format_dt(&from),
                    format_dt(&to),
                    page.limit as i64,
                    page.offset as i64,
                ],
                |row| row_to_transaction(row),
            )
            .map_err(StorageError::from)?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(StorageError::from)?);
        }
        Ok(PaginatedResult::from_vec(items, total_count, page))
    }

    fn update(&self, transaction: &Transaction) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "UPDATE transactions SET account_id = ?1, category_id = ?2, amount = ?3, transaction_type = ?4, description = ?5, date = ?6, linked_transaction_id = ?7, sync_status = ?8, version = ?9, updated_at = ?10, deleted_at = ?11 WHERE id = ?12",
            params![
                transaction.account_id.to_string(),
                transaction.category_id.to_string(),
                transaction.amount,
                transaction.transaction_type.to_string(),
                transaction.description,
                format_dt(&transaction.date),
                transaction.linked_transaction_id.map(|id| id.to_string()),
                transaction.sync_status.to_string(),
                transaction.version,
                format_dt(&transaction.base.updated_at),
                format_dt_opt(&transaction.base.deleted_at),
                transaction.base.id.to_string(),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let now = format_dt(&Utc::now());
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
                "SELECT id, account_id, category_id, amount, transaction_type, description, date, linked_transaction_id, sync_status, version, created_at, updated_at, deleted_at
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

    fn get_spending_by_category(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CategorySpending>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT c.id, c.name, SUM(t.amount) as total, COUNT(t.id) as count
                 FROM transactions t
                 INNER JOIN categories c ON t.category_id = c.id
                 WHERE t.account_id = ?1
                   AND t.date >= ?2
                   AND t.date <= ?3
                   AND t.deleted_at IS NULL
                   AND c.deleted_at IS NULL
                   AND t.transaction_type = 'expense'
                 GROUP BY c.id, c.name
                 ORDER BY total DESC",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(
                params![
                    account_id.to_string(),
                    format_dt(&from),
                    format_dt(&to)
                ],
                |row| {
                    let id_str: String = row.get(0)?;
                    let name: String = row.get(1)?;
                    let total: i64 = row.get(2)?;
                    let count: i64 = row.get(3)?;

                    Ok(CategorySpending {
                        category_id: Uuid::parse_str(&id_str).unwrap(),
                        category_name: name,
                        total_amount: total,
                        transaction_count: count as usize,
                    })
                },
            )
            .map_err(StorageError::from)?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(StorageError::from)?);
        }
        Ok(result)
    }

    fn search(
        &self,
        filter: &TransactionSearchFilter,
    ) -> Result<PaginatedResult<Transaction>, DomainError> {
        let conn = self.db.conn.lock().unwrap();

        // Build WHERE clause (shared between COUNT and SELECT queries)
        let mut where_clause = String::from("WHERE account_id = ?1 AND deleted_at IS NULL");
        let mut param_idx = 2u32;
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(filter.account_id.to_string())];

        if let Some(ref query) = filter.query {
            where_clause.push_str(&format!(" AND description LIKE ?{param_idx}"));
            params_vec.push(Box::new(format!("%{query}%")));
            param_idx += 1;
        }

        if let Some(category_id) = filter.category_id {
            where_clause.push_str(&format!(" AND category_id = ?{param_idx}"));
            params_vec.push(Box::new(category_id.to_string()));
            param_idx += 1;
        }

        if let Some(tx_type) = filter.transaction_type {
            where_clause.push_str(&format!(" AND transaction_type = ?{param_idx}"));
            params_vec.push(Box::new(tx_type.to_string()));
            param_idx += 1;
        }

        if let Some(min_amount) = filter.min_amount {
            where_clause.push_str(&format!(" AND amount >= ?{param_idx}"));
            params_vec.push(Box::new(min_amount));
            param_idx += 1;
        }

        if let Some(max_amount) = filter.max_amount {
            where_clause.push_str(&format!(" AND amount <= ?{param_idx}"));
            params_vec.push(Box::new(max_amount));
            param_idx += 1;
        }

        if let Some(date_from) = filter.date_from {
            where_clause.push_str(&format!(" AND date >= ?{param_idx}"));
            params_vec.push(Box::new(format_dt(&date_from)));
            param_idx += 1;
        }

        if let Some(date_to) = filter.date_to {
            where_clause.push_str(&format!(" AND date <= ?{param_idx}"));
            params_vec.push(Box::new(format_dt(&date_to)));
            param_idx += 1;
        }

        // Get total count
        let count_sql = format!("SELECT COUNT(*) FROM transactions {where_clause}");
        let count_params: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let total_count: usize = conn
            .query_row(&count_sql, count_params.as_slice(), |row| row.get(0))
            .map_err(StorageError::from)?;

        // Build SELECT query with pagination
        let mut sql = format!(
            "SELECT id, account_id, category_id, amount, transaction_type, description, date, linked_transaction_id, sync_status, version, created_at, updated_at, deleted_at
             FROM transactions {where_clause} ORDER BY date DESC"
        );

        let limit = filter.limit.unwrap_or(50);
        let offset = filter.offset.unwrap_or(0);
        sql.push_str(&format!(" LIMIT ?{param_idx}"));
        params_vec.push(Box::new(limit as i64));
        param_idx += 1;
        sql.push_str(&format!(" OFFSET ?{param_idx}"));
        params_vec.push(Box::new(offset as i64));
        #[allow(unused_assignments)]
        { param_idx += 1; }

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql).map_err(StorageError::from)?;
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| row_to_transaction(row))
            .map_err(StorageError::from)?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(StorageError::from)?);
        }

        let page = PageRequest { limit, offset };
        Ok(PaginatedResult::from_vec(items, total_count, &page))
    }

    fn get_income_vs_expenses(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<IncomeSummary, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let row = conn
            .query_row(
                "SELECT
                    COALESCE(SUM(CASE
                        WHEN transaction_type = 'income' AND linked_transaction_id IS NULL
                        THEN amount ELSE 0 END), 0) AS income,
                    COALESCE(SUM(CASE
                        WHEN transaction_type = 'expense' AND linked_transaction_id IS NULL
                        THEN amount ELSE 0 END), 0) AS expenses,
                    COALESCE(SUM(CASE
                        WHEN transaction_type = 'transfer' AND linked_transaction_id IS NULL
                        THEN amount ELSE 0 END), 0) AS transfers
                 FROM transactions
                 WHERE account_id = ?1
                   AND date >= ?2 AND date <= ?3
                   AND deleted_at IS NULL",
                params![
                    account_id.to_string(),
                    format_dt(&from),
                    format_dt(&to)
                ],
                |row| {
                    let income: i64 = row.get(0)?;
                    let expenses: i64 = row.get(1)?;
                    let transfers: i64 = row.get(2)?;
                    Ok(IncomeSummary {
                        income,
                        expenses,
                        transfers,
                        net: income - expenses - transfers,
                    })
                },
            )
            .map_err(StorageError::from)?;
        Ok(row)
    }

    fn get_monthly_trends(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<MonthlyTrend>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT
                    CAST(strftime('%Y', date) AS INTEGER) AS year,
                    CAST(strftime('%m', date) AS INTEGER) AS month,
                    COALESCE(SUM(CASE WHEN transaction_type = 'income' THEN amount ELSE 0 END), 0) AS income,
                    COALESCE(SUM(CASE WHEN transaction_type = 'expense' THEN amount ELSE 0 END), 0) AS expenses,
                    COUNT(*) AS transaction_count
                 FROM transactions
                 WHERE account_id = ?1
                   AND date >= ?2 AND date <= ?3
                   AND deleted_at IS NULL
                   AND linked_transaction_id IS NULL
                 GROUP BY strftime('%Y', date), strftime('%m', date)
                 ORDER BY year ASC, month ASC",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(
                params![
                    account_id.to_string(),
                    format_dt(&from),
                    format_dt(&to)
                ],
                |row| {
                    let year: i32 = row.get(0)?;
                    let month: u32 = row.get(1)?;
                    let income: i64 = row.get(2)?;
                    let expenses: i64 = row.get(3)?;
                    let count: i64 = row.get(4)?;
                    Ok(MonthlyTrend {
                        year,
                        month,
                        income,
                        expenses,
                        net: income - expenses,
                        transaction_count: count as usize,
                    })
                },
            )
            .map_err(StorageError::from)?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(StorageError::from)?);
        }
        Ok(result)
    }

    fn get_daily_spending(
        &self,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<DailySpending>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT
                    strftime('%Y-%m-%d', date) AS day,
                    SUM(amount) AS amount,
                    COUNT(*) AS transaction_count
                 FROM transactions
                 WHERE account_id = ?1
                   AND date >= ?2 AND date <= ?3
                   AND deleted_at IS NULL
                   AND transaction_type = 'expense'
                   AND linked_transaction_id IS NULL
                 GROUP BY strftime('%Y-%m-%d', date)
                 ORDER BY day ASC",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(
                params![
                    account_id.to_string(),
                    format_dt(&from),
                    format_dt(&to)
                ],
                |row| {
                    Ok(DailySpending {
                        date: row.get(0)?,
                        amount: row.get(1)?,
                        transaction_count: {
                            let c: i64 = row.get(2)?;
                            c as usize
                        },
                    })
                },
            )
            .map_err(StorageError::from)?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(StorageError::from)?);
        }
        Ok(result)
    }

    fn get_budget_spent(
        &self,
        account_id: Uuid,
        category_id: Option<Uuid>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<i64, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let spent: i64 = if let Some(cat_id) = category_id {
            conn.query_row(
                "SELECT COALESCE(SUM(amount), 0)
                 FROM transactions
                 WHERE account_id = ?1
                   AND category_id = ?2
                   AND date >= ?3 AND date <= ?4
                   AND deleted_at IS NULL
                   AND transaction_type = 'expense'",
                params![
                    account_id.to_string(),
                    cat_id.to_string(),
                    format_dt(&from),
                    format_dt(&to)
                ],
                |row| row.get(0),
            )
            .map_err(StorageError::from)?
        } else {
            conn.query_row(
                "SELECT COALESCE(SUM(amount), 0)
                 FROM transactions
                 WHERE account_id = ?1
                   AND date >= ?2 AND date <= ?3
                   AND deleted_at IS NULL
                   AND transaction_type = 'expense'",
                params![
                    account_id.to_string(),
                    format_dt(&from),
                    format_dt(&to)
                ],
                |row| row.get(0),
            )
            .map_err(StorageError::from)?
        };
        Ok(spent)
    }
}

fn row_to_transaction(row: &rusqlite::Row) -> rusqlite::Result<Transaction> {
    let id_str: String = row.get(0)?;
    let account_id_str: String = row.get(1)?;
    let category_id_str: String = row.get(2)?;
    let tx_type_str: String = row.get(4)?;
    let date_str: String = row.get(6)?;
    let linked_tx_str: Option<String> = row.get(7)?;
    let sync_str: String = row.get(8)?;
    let created_str: String = row.get(10)?;
    let updated_str: String = row.get(11)?;
    let deleted_str: Option<String> = row.get(12)?;

    Ok(Transaction {
        base: BaseEntity {
            id: parse_uuid(&id_str)?,
            created_at: parse_dt(&created_str)?,
            updated_at: parse_dt(&updated_str)?,
            deleted_at: parse_dt_opt(deleted_str)?,
        },
        account_id: parse_uuid(&account_id_str)?,
        category_id: parse_uuid(&category_id_str)?,
        amount: row.get(3)?,
        transaction_type: TransactionType::from_str(&tx_type_str)
            .unwrap_or(TransactionType::Expense),
        description: row.get(5)?,
        date: parse_dt(&date_str)?,
        linked_transaction_id: parse_uuid_opt(linked_tx_str)?,
        sync_status: SyncStatus::from_str(&sync_str).unwrap_or(SyncStatus::Pending),
        version: row.get(9)?,
    })
}
