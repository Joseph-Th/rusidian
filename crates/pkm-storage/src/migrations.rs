//! Schema migrations.
//!
//! Every schema change is an ordered, append-only migration. Never edit a past
//! migration once it has shipped — add a new one. A `schema_version` table
//! tracks what has run. See STATUS.md task B1.

use rusqlite::Connection;
use time::OffsetDateTime;

use crate::Result;

/// Ordered list of (version, sql). APPEND-ONLY: migrations are immutable once
/// shipped. Add new ones to the end. See STATUS.md task B1 (and task B2 for C1 ext).
pub const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("../migrations/0001_init.sql")),
    (
        "0002_extend_source",
        include_str!("../migrations/0002_extend_source.sql"),
    ),
    (
        "0003_fts5_indexing",
        include_str!("../migrations/0003_fts5_indexing.sql"),
    ),
    (
        "0004_entity_merge",
        include_str!("../migrations/0004_entity_merge.sql"),
    ),
    (
        "0005_link_review_state",
        include_str!("../migrations/0005_link_review_state.sql"),
    ),
    (
        "0006_add_project_field",
        include_str!("../migrations/0006_add_project_field.sql"),
    ),
    (
        "0007_add_versioning",
        include_str!("../migrations/0007_add_versioning.sql"),
    ),
    (
        "0008_add_note_metadata",
        include_str!("../migrations/0008_add_note_metadata.sql"),
    ),
    (
        "0009_fix_fts5_indexes",
        include_str!("../migrations/0009_fix_fts5_indexes.sql"),
    ),
    (
        "0010_markdown_first",
        include_str!("../migrations/0010_markdown_first.sql"),
    ),
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
