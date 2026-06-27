//! Connection pool for SQLite database.
//!
//! Uses r2d2 to manage a pool of connections, allowing concurrent reads
//! while serializing writes. This fully utilizes SQLite's WAL mode.

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::Path;

pub type DbPool = Pool<SqliteConnectionManager>;

/// Custom connection manager that initializes SQLite pragmas.
pub struct InitializedSqliteManager {
    manager: SqliteConnectionManager,
    initialized: std::sync::atomic::AtomicBool,
}

/// Create a new database connection pool.
pub fn create_pool(db_path: &Path) -> Result<DbPool, String> {
    let manager = SqliteConnectionManager::file(db_path);

    let pool = Pool::new(manager)
        .map_err(|e| format!("Failed to create connection pool: {}", e))?;

    // Initialize pragmas on one connection to set up the database
    {
        let conn = pool
            .get()
            .map_err(|e| format!("Failed to get connection for initialization: {}", e))?;
        init_pragmas(&conn)?;
    }

    Ok(pool)
}

fn init_pragmas(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("PRAGMA journal_mode = WAL")
        .map_err(|e| format!("Failed to enable WAL: {}", e))?;
    conn.execute_batch("PRAGMA foreign_keys = ON")
        .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| format!("Failed to set busy timeout: {}", e))?;
    Ok(())
}
