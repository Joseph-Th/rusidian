//! Schema migrations.
//!
//! Every schema change is an ordered, append-only migration. Never edit a past
//! migration once it has shipped — add a new one. A `schema_version` table
//! tracks what has run. See STATUS.md task B1.

use rusqlite::Connection;
use time::OffsetDateTime;

use crate::Result;

/// Ordered list of (version, sql). STUB — task B1 defines migration 1 (the
/// initial schema for sources, notes, blocks, entities, links, views,
/// agent_actions). Keep DDL here, not scattered across the codebase.
pub const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("../migrations/0001_init.sql")),
];

/// Apply all pending migrations inside a transaction. Idempotent: safe to call
/// multiple times. Creates schema_version table if absent, then applies any
/// migrations not yet recorded.
pub fn run(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;

    // Create schema_version table if it doesn't exist (needed for first run).
    tx.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
        )",
        [],
    )?;

    // Apply each migration that hasn't been recorded yet.
    for (version, sql) in MIGRATIONS {
        let exists: bool = tx
            .query_row(
                "SELECT COUNT(*) > 0 FROM schema_version WHERE version = ?",
                [version],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !exists {
            // Execute the migration SQL.
            tx.execute_batch(sql)?;

            // Record that this migration has been applied.
            let now = OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "unknown".to_string());
            tx.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (?, ?)",
                [*version, &now],
            )?;
        }
    }

    tx.commit()?;
    Ok(())
}
