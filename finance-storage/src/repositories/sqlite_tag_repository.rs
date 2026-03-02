use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use finance_core::entities::common::BaseEntity;
use finance_core::entities::pagination::{PageRequest, PaginatedResult};
use finance_core::entities::Tag;
use finance_core::errors::DomainError;
use finance_core::repositories::TagRepository;

use crate::database::Database;
use crate::date_utils::{format_dt, format_dt_opt, parse_dt, parse_dt_opt, parse_uuid};
use crate::error::StorageError;

/// SQLite implementation of TagRepository.
pub struct SqliteTagRepository<'a> {
    db: &'a Database,
}

impl<'a> SqliteTagRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }
}

impl<'a> TagRepository for SqliteTagRepository<'a> {
    fn save(&self, tag: &Tag) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tags (id, user_id, name, color, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                tag.base.id.to_string(),
                tag.user_id.to_string(),
                tag.name,
                tag.color,
                format_dt(&tag.base.created_at),
                format_dt(&tag.base.updated_at),
                format_dt_opt(&tag.base.deleted_at),
            ],
        )
        .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;
        Ok(())
    }

    fn find_by_id(&self, id: Uuid) -> Result<Option<Tag>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, name, color, created_at, updated_at, deleted_at
                 FROM tags WHERE id = ?1 AND deleted_at IS NULL",
            )
            .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;

        let result = stmt
            .query_row(params![id.to_string()], |row| {
                row_to_tag(row)
            })
            .optional()
            .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;

        match result {
            Some(tag) => Ok(Some(tag)),
            None => Ok(None),
        }
    }

    fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Tag>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, name, color, created_at, updated_at, deleted_at
                 FROM tags WHERE user_id = ?1 AND deleted_at IS NULL ORDER BY name ASC",
            )
            .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;

        let rows = stmt
            .query_map(params![user_id.to_string()], |row| {
                row_to_tag(row)
            })
            .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;

        let mut tags = Vec::new();
        for row in rows {
            tags.push(
                row.map_err(|e| {
                    DomainError::Storage(StorageError::Query(e.to_string()).to_string())
                })?,
            );
        }
        Ok(tags)
    }

    fn update(&self, tag: &Tag) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "UPDATE tags SET name = ?1, color = ?2, updated_at = ?3, deleted_at = ?4
             WHERE id = ?5",
            params![
                tag.name,
                tag.color,
                format_dt(&tag.base.updated_at),
                format_dt_opt(&tag.base.deleted_at),
                tag.base.id.to_string(),
            ],
        )
        .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;
        Ok(())
    }

    fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        // Remove associations first, then soft-delete the tag
        conn.execute(
            "DELETE FROM transaction_tags WHERE tag_id = ?1",
            params![id.to_string()],
        )
        .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;

        let now = format_dt(&Utc::now());
        conn.execute(
            "UPDATE tags SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;
        Ok(())
    }

    fn add_tag_to_transaction(
        &self,
        transaction_id: Uuid,
        tag_id: Uuid,
    ) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO transaction_tags (transaction_id, tag_id) VALUES (?1, ?2)",
            params![transaction_id.to_string(), tag_id.to_string()],
        )
        .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;
        Ok(())
    }

    fn remove_tag_from_transaction(
        &self,
        transaction_id: Uuid,
        tag_id: Uuid,
    ) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM transaction_tags WHERE transaction_id = ?1 AND tag_id = ?2",
            params![transaction_id.to_string(), tag_id.to_string()],
        )
        .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;
        Ok(())
    }

    fn find_tags_for_transaction(&self, transaction_id: Uuid) -> Result<Vec<Tag>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.user_id, t.name, t.color, t.created_at, t.updated_at, t.deleted_at
                 FROM tags t
                 INNER JOIN transaction_tags tt ON tt.tag_id = t.id
                 WHERE tt.transaction_id = ?1 AND t.deleted_at IS NULL
                 ORDER BY t.name ASC",
            )
            .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;

        let rows = stmt
            .query_map(params![transaction_id.to_string()], |row| {
                row_to_tag(row)
            })
            .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;

        let mut tags = Vec::new();
        for row in rows {
            tags.push(
                row.map_err(|e| {
                    DomainError::Storage(StorageError::Query(e.to_string()).to_string())
                })?,
            );
        }
        Ok(tags)
    }

    fn find_transaction_ids_by_tag(&self, tag_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT transaction_id FROM transaction_tags WHERE tag_id = ?1")
            .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;

        let rows = stmt
            .query_map(params![tag_id.to_string()], |row| {
                let id_str: String = row.get(0)?;
                Ok(id_str)
            })
            .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;

        let mut ids = Vec::new();
        for row in rows {
            let id_str = row
                .map_err(|e| {
                    DomainError::Storage(StorageError::Query(e.to_string()).to_string())
                })?;
            let uuid = Uuid::parse_str(&id_str)
                .map_err(|e| DomainError::Storage(format!("Invalid UUID: {e}")))?;
            ids.push(uuid);
        }
        Ok(ids)
    }

    fn find_transaction_ids_by_tag_paginated(
        &self,
        tag_id: Uuid,
        page: &PageRequest,
    ) -> Result<PaginatedResult<Uuid>, DomainError> {
        let conn = self.db.conn.lock().unwrap();

        let total_count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM transaction_tags WHERE tag_id = ?1",
                params![tag_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;

        let mut stmt = conn
            .prepare("SELECT transaction_id FROM transaction_tags WHERE tag_id = ?1 LIMIT ?2 OFFSET ?3")
            .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;

        let rows = stmt
            .query_map(
                params![tag_id.to_string(), page.limit as i64, page.offset as i64],
                |row| {
                    let id_str: String = row.get(0)?;
                    Ok(id_str)
                },
            )
            .map_err(|e| DomainError::Storage(StorageError::Query(e.to_string()).to_string()))?;

        let mut ids = Vec::new();
        for row in rows {
            let id_str = row
                .map_err(|e| {
                    DomainError::Storage(StorageError::Query(e.to_string()).to_string())
                })?;
            let uuid = Uuid::parse_str(&id_str)
                .map_err(|e| DomainError::Storage(format!("Invalid UUID: {e}")))?;
            ids.push(uuid);
        }
        Ok(PaginatedResult::from_vec(ids, total_count, page))
    }
}

/// Convert a row to a Tag.
fn row_to_tag(row: &rusqlite::Row) -> rusqlite::Result<Tag> {
    let id_str: String = row.get(0)?;
    let user_id_str: String = row.get(1)?;
    let name: String = row.get(2)?;
    let color: Option<String> = row.get(3)?;
    let created_str: String = row.get(4)?;
    let updated_str: String = row.get(5)?;
    let deleted_str: Option<String> = row.get(6)?;

    Ok(Tag {
        base: BaseEntity {
            id: parse_uuid(&id_str)?,
            created_at: parse_dt(&created_str)?,
            updated_at: parse_dt(&updated_str)?,
            deleted_at: parse_dt_opt(deleted_str)?,
        },
        user_id: parse_uuid(&user_id_str)?,
        name,
        color,
    })
}

use rusqlite::OptionalExtension;
