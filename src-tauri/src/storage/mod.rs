pub mod history;
pub mod settings;

use rusqlite::Connection;
use std::path::Path;
use crate::error::AppError;

/// Initialize the SQLite database with required tables.
pub fn init_database(db_path: &Path) -> Result<Connection, AppError> {
    let conn = Connection::open(db_path)?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS history (
            id TEXT PRIMARY KEY,
            url TEXT NOT NULL,
            platform TEXT NOT NULL,
            author TEXT,
            title TEXT,
            status TEXT NOT NULL,
            save_path TEXT,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_history_status ON history(status);
        CREATE INDEX IF NOT EXISTS idx_history_created_at ON history(created_at);
        ",
    )?;

    Ok(conn)
}
