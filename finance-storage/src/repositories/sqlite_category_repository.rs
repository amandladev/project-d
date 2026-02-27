use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

use finance_core::entities::common::{BaseEntity, SyncStatus};
use finance_core::entities::Category;
use finance_core::errors::DomainError;
use finance_core::repositories::CategoryRepository;

use crate::database::Database;
use crate::date_utils::{format_dt, format_dt_opt};
use crate::error::StorageError;

/// SQLite implementation of CategoryRepository.
pub struct SqliteCategoryRepository<'a> {
    db: &'a Database,
}

impl<'a> SqliteCategoryRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }
}

impl<'a> CategoryRepository for SqliteCategoryRepository<'a> {
    fn save(&self, category: &Category) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO categories (id, user_id, name, icon, sync_status, version, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                category.base.id.to_string(),
                category.user_id.to_string(),
                category.name,
                category.icon,
                category.sync_status.to_string(),
                category.version,
                format_dt(&category.base.created_at),
                format_dt(&category.base.updated_at),
                format_dt_opt(&category.base.deleted_at),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn find_by_id(&self, id: Uuid) -> Result<Option<Category>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, name, icon, sync_status, version, created_at, updated_at, deleted_at
                 FROM categories WHERE id = ?1 AND deleted_at IS NULL",
            )
            .map_err(StorageError::from)?;

        let result = stmt.query_row(params![id.to_string()], |row| row_to_category(row));

        match result {
            Ok(category) => Ok(Some(category)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::from(e).into()),
        }
    }

    fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Category>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, name, icon, sync_status, version, created_at, updated_at, deleted_at
                 FROM categories WHERE user_id = ?1 AND deleted_at IS NULL",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(params![user_id.to_string()], |row| row_to_category(row))
            .map_err(StorageError::from)?;

        let mut categories = Vec::new();
        for row in rows {
            categories.push(row.map_err(StorageError::from)?);
        }
        Ok(categories)
    }

    fn update(&self, category: &Category) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "UPDATE categories SET name = ?1, icon = ?2, sync_status = ?3, version = ?4, updated_at = ?5, deleted_at = ?6 WHERE id = ?7",
            params![
                category.name,
                category.icon,
                category.sync_status.to_string(),
                category.version,
                format_dt(&category.base.updated_at),
                format_dt_opt(&category.base.deleted_at),
                category.base.id.to_string(),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let now = format_dt(&Utc::now());
        conn.execute(
            "UPDATE categories SET deleted_at = ?1, updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(StorageError::from)?;
        Ok(())
    }

    fn find_pending_sync(&self) -> Result<Vec<Category>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, name, icon, sync_status, version, created_at, updated_at, deleted_at
                 FROM categories WHERE sync_status = 'pending'",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map([], |row| row_to_category(row))
            .map_err(StorageError::from)?;

        let mut categories = Vec::new();
        for row in rows {
            categories.push(row.map_err(StorageError::from)?);
        }
        Ok(categories)
    }
}

fn row_to_category(row: &rusqlite::Row) -> rusqlite::Result<Category> {
    let id_str: String = row.get(0)?;
    let user_id_str: String = row.get(1)?;
    let sync_str: String = row.get(4)?;
    let created_str: String = row.get(6)?;
    let updated_str: String = row.get(7)?;
    let deleted_str: Option<String> = row.get(8)?;

    Ok(Category {
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
        user_id: Uuid::parse_str(&user_id_str).unwrap(),
        name: row.get(2)?,
        icon: row.get(3)?,
        sync_status: SyncStatus::from_str(&sync_str).unwrap_or(SyncStatus::Pending),
        version: row.get(5)?,
    })
}
