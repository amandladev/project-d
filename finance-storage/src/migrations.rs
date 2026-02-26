use rusqlite::Connection;

use crate::error::StorageError;

/// Run all database migrations.
pub fn run_migrations(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT
        );

        CREATE TABLE IF NOT EXISTS accounts (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            name TEXT NOT NULL,
            currency TEXT NOT NULL,
            sync_status TEXT NOT NULL DEFAULT 'pending',
            version INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS categories (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            name TEXT NOT NULL,
            icon TEXT,
            sync_status TEXT NOT NULL DEFAULT 'pending',
            version INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS transactions (
            id TEXT PRIMARY KEY NOT NULL,
            account_id TEXT NOT NULL,
            category_id TEXT NOT NULL,
            amount INTEGER NOT NULL,
            transaction_type TEXT NOT NULL,
            description TEXT NOT NULL,
            date TEXT NOT NULL,
            sync_status TEXT NOT NULL DEFAULT 'pending',
            version INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (account_id) REFERENCES accounts(id),
            FOREIGN KEY (category_id) REFERENCES categories(id)
        );

        -- Indexes for frequent queries
        CREATE INDEX IF NOT EXISTS idx_accounts_user_id ON accounts(user_id);
        CREATE INDEX IF NOT EXISTS idx_categories_user_id ON categories(user_id);
        CREATE INDEX IF NOT EXISTS idx_transactions_account_id ON transactions(account_id);
        CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions(date);
        CREATE INDEX IF NOT EXISTS idx_transactions_sync_status ON transactions(sync_status);
        CREATE INDEX IF NOT EXISTS idx_accounts_sync_status ON accounts(sync_status);
        CREATE INDEX IF NOT EXISTS idx_categories_sync_status ON categories(sync_status);
        CREATE INDEX IF NOT EXISTS idx_transactions_account_date ON transactions(account_id, date);
        ",
    )
    .map_err(|e| StorageError::Migration(e.to_string()))?;

    Ok(())
}
