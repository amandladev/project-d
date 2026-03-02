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
    /// No encryption — for backward compatibility.
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

    /// Open (or create) an encrypted SQLite database (SQLCipher).
    /// The `key` is the passphrase used to encrypt/decrypt the database.
    /// On first call this creates an encrypted DB; subsequent calls decrypt it.
    pub fn open_encrypted(path: &Path, key: &str) -> Result<Self, StorageError> {
        if key.is_empty() {
            return Err(StorageError::Connection(
                "Encryption key cannot be empty".to_string(),
            ));
        }

        let conn =
            Connection::open(path).map_err(|e| StorageError::Connection(e.to_string()))?;

        // Set the encryption key — must be the first operation on the connection
        conn.pragma_update(None, "key", key)
            .map_err(|e| StorageError::Connection(format!("Failed to set encryption key: {e}")))?;

        // Verify the key works by reading from the DB
        conn.execute_batch("SELECT count(*) FROM sqlite_master;")
            .map_err(|_| StorageError::Connection(
                "Invalid encryption key or corrupted database".to_string(),
            ))?;

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
