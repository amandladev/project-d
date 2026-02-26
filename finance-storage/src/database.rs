use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

use crate::error::StorageError;
use crate::migrations;

/// Thread-safe wrapper around a SQLite connection.
pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    /// Open (or create) a SQLite database at the given path and run migrations.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let conn =
            Connection::open(path).map_err(|e| StorageError::Connection(e.to_string()))?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        migrations::run_migrations(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open an in-memory database (useful for testing).
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn =
            Connection::open_in_memory().map_err(|e| StorageError::Connection(e.to_string()))?;

        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        migrations::run_migrations(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}
