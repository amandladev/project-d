use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

use finance_core::entities::common::BaseEntity;
use finance_core::entities::{Budget, BudgetPeriod};
use finance_core::errors::DomainError;
use finance_core::repositories::BudgetRepository;

use crate::database::Database;
use crate::date_utils::{format_dt, format_dt_opt};
use crate::error::StorageError;

/// SQLite implementation of BudgetRepository.
pub struct SqliteBudgetRepository<'a> {
    db: &'a Database,
}

impl<'a> SqliteBudgetRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }
}

impl<'a> BudgetRepository for SqliteBudgetRepository<'a> {
    fn save(&self, budget: &Budget) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO budgets (id, account_id, category_id, name, amount, period, start_date, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                budget.base.id.to_string(),
                budget.account_id.to_string(),
                budget.category_id.map(|id| id.to_string()),
                budget.name,
                budget.amount,
                budget.period.to_string(),
                format_dt(&budget.start_date),
                format_dt(&budget.base.created_at),
                format_dt(&budget.base.updated_at),
                format_dt_opt(&budget.base.deleted_at),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn find_by_id(&self, id: Uuid) -> Result<Option<Budget>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, category_id, name, amount, period, start_date, created_at, updated_at, deleted_at
                 FROM budgets WHERE id = ?1",
            )
            .map_err(StorageError::from)?;

        let result = stmt.query_row(params![id.to_string()], |row| row_to_budget(row));

        match result {
            Ok(b) => Ok(Some(b)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::from(e).into()),
        }
    }

    fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<Budget>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, category_id, name, amount, period, start_date, created_at, updated_at, deleted_at
                 FROM budgets WHERE account_id = ?1 AND deleted_at IS NULL ORDER BY created_at DESC",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(params![account_id.to_string()], |row| row_to_budget(row))
            .map_err(StorageError::from)?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(StorageError::from)?);
        }
        Ok(result)
    }

    fn update(&self, budget: &Budget) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "UPDATE budgets SET account_id = ?1, category_id = ?2, name = ?3, amount = ?4, period = ?5, start_date = ?6, updated_at = ?7, deleted_at = ?8 WHERE id = ?9",
            params![
                budget.account_id.to_string(),
                budget.category_id.map(|id| id.to_string()),
                budget.name,
                budget.amount,
                budget.period.to_string(),
                format_dt(&budget.start_date),
                format_dt(&budget.base.updated_at),
                format_dt_opt(&budget.base.deleted_at),
                budget.base.id.to_string(),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let now = format_dt(&Utc::now());
        conn.execute(
            "UPDATE budgets SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(StorageError::from)?;
        Ok(())
    }
}

fn row_to_budget(row: &rusqlite::Row) -> rusqlite::Result<Budget> {
    let id_str: String = row.get(0)?;
    let account_id_str: String = row.get(1)?;
    let category_id_str: Option<String> = row.get(2)?;
    let period_str: String = row.get(5)?;
    let start_str: String = row.get(6)?;
    let created_str: String = row.get(7)?;
    let updated_str: String = row.get(8)?;
    let deleted_str: Option<String> = row.get(9)?;

    Ok(Budget {
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
        category_id: category_id_str.map(|s| Uuid::parse_str(&s).unwrap()),
        name: row.get(3)?,
        amount: row.get(4)?,
        period: BudgetPeriod::from_str(&period_str).unwrap_or(BudgetPeriod::Monthly),
        start_date: DateTime::parse_from_rfc3339(&start_str)
            .unwrap()
            .with_timezone(&Utc),
    })
}
