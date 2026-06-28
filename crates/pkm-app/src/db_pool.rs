//! Connection pool for SQLite database.
//!
//! Uses r2d2 to manage a pool of connections, allowing concurrent reads
//! while serializing writes. This fully utilizes SQLite's WAL mode.

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::Path;

pub type DbPool = Pool<SqliteConnectionManager>;

/// Enforces SQLite pragmas on every connection acquired from the pool.
/// This ensures foreign key constraints and busy timeout are enabled on all connections.
#[derive(Debug)]
struct PragmaCustomizer;

impl r2d2::CustomizeConnection<Connection, rusqlite::Error> for PragmaCustomizer {
    fn on_acquire(&self, conn: &mut Connection) -> Result<(), rusqlite::Error> {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(())
    }
}

/// Create a new database connection pool.
pub fn create_pool(db_path: &Path) -> Result<DbPool, String> {
    let manager = SqliteConnectionManager::file(db_path);

    let pool = Pool::builder()
        .connection_customizer(Box::new(PragmaCustomizer))
        .build(manager)
        .map_err(|e| format!("Failed to create connection pool: {}", e))?;

    // Initialize WAL mode on one connection (subsequent connections inherit via CustomizeConnection)
    {
        let mut conn = pool
            .get()
            .map_err(|e| format!("Failed to get connection for initialization: {}", e))?;
        init_pragmas(&conn)?;
        pkm_storage::migrations::run(&mut conn)
            .map_err(|e| format!("Failed to run database migrations: {}", e))?;
    }

    Ok(pool)
}

fn init_pragmas(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("PRAGMA journal_mode = WAL")
        .map_err(|e| format!("Failed to enable WAL: {}", e))?;
    Ok(())
}
