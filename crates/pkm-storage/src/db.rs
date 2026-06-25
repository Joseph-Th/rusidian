//! Connection open + pragmas.

use std::path::Path;

use rusqlite::Connection;

use crate::{migrations, Result};

/// Open (and migrate) the database at `path`. Sets pragmas for WAL mode,
/// foreign key constraints, and busy timeout, then applies any pending
/// migrations. Safe to call multiple times (migrations are idempotent).
pub fn open(path: &Path) -> Result<Connection> {
    let mut conn = Connection::open(path)?;

    // Enable WAL mode for better concurrency.
    conn.execute_batch("PRAGMA journal_mode = WAL")?;

    // Enforce foreign key constraints.
    conn.execute_batch("PRAGMA foreign_keys = ON")?;

    // Set a reasonable busy timeout (5 seconds) to avoid immediate failures
    // under contention.
    conn.busy_timeout(std::time::Duration::from_secs(5))?;

    // Apply any pending migrations.
    migrations::run(&mut conn)?;

    Ok(conn)
}
