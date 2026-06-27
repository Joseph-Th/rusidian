//! Connection pool for SQLite database.
//!
//! Uses r2d2 to manage a pool of connections, allowing concurrent reads
//! while serializing writes. This fully utilizes SQLite's WAL mode.

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::path::Path;

pub type DbPool = Pool<SqliteConnectionManager>;

/// Create a new database connection pool.
pub fn create_pool(db_path: &Path) -> Result<DbPool, String> {
    let manager = SqliteConnectionManager::file(db_path)
        .init(|conn| {
            conn.execute_batch("PRAGMA journal_mode = WAL")?;
            conn.execute_batch("PRAGMA foreign_keys = ON")?;
            conn.busy_timeout(std::time::Duration::from_secs(5))?;
            Ok(())
        });

    Pool::new(manager)
        .map_err(|e| format!("Failed to create connection pool: {}", e))
}
