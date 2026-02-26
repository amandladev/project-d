use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

use finance_core::entities::User;
use finance_core::entities::common::BaseEntity;
use finance_core::errors::DomainError;
use finance_core::repositories::UserRepository;

use crate::database::Database;
use crate::error::StorageError;

/// SQLite implementation of UserRepository.
pub struct SqliteUserRepository<'a> {
    db: &'a Database,
}

impl<'a> SqliteUserRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }
}

impl<'a> UserRepository for SqliteUserRepository<'a> {
    fn save(&self, user: &User) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO users (id, name, email, created_at, updated_at, deleted_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                user.base.id.to_string(),
                user.name,
                user.email,
                user.base.created_at.to_rfc3339(),
                user.base.updated_at.to_rfc3339(),
                user.base.deleted_at.map(|d| d.to_rfc3339()),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, email, created_at, updated_at, deleted_at FROM users WHERE id = ?1 AND deleted_at IS NULL")
            .map_err(StorageError::from)?;

        let result = stmt
            .query_row(params![id.to_string()], |row| {
                Ok(User {
                    base: row_to_base_entity(row)?,
                    name: row.get(1)?,
                    email: row.get(2)?,
                })
            });

        match result {
            Ok(user) => Ok(Some(user)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::from(e).into()),
        }
    }

    fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, email, created_at, updated_at, deleted_at FROM users WHERE email = ?1 AND deleted_at IS NULL")
            .map_err(StorageError::from)?;

        let result = stmt.query_row(params![email], |row| {
            Ok(User {
                base: row_to_base_entity(row)?,
                name: row.get(1)?,
                email: row.get(2)?,
            })
        });

        match result {
            Ok(user) => Ok(Some(user)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::from(e).into()),
        }
    }

    fn update(&self, user: &User) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "UPDATE users SET name = ?1, email = ?2, updated_at = ?3, deleted_at = ?4 WHERE id = ?5",
            params![
                user.name,
                user.email,
                user.base.updated_at.to_rfc3339(),
                user.base.deleted_at.map(|d| d.to_rfc3339()),
                user.base.id.to_string(),
            ],
        ).map_err(StorageError::from)?;
        Ok(())
    }

    fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE users SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(StorageError::from)?;
        Ok(())
    }
}

fn row_to_base_entity(row: &rusqlite::Row) -> rusqlite::Result<BaseEntity> {
    let id_str: String = row.get(0)?;
    let created_str: String = row.get(3)?;
    let updated_str: String = row.get(4)?;
    let deleted_str: Option<String> = row.get(5)?;

    Ok(BaseEntity {
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
    })
}
