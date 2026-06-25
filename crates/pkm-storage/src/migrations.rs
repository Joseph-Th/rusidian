//! Schema migrations.
//!
//! Every schema change is an ordered, append-only migration. Never edit a past
//! migration once it has shipped — add a new one. A `schema_version` table
//! tracks what has run. See STATUS.md task B1.

use rusqlite::Connection;

use crate::Result;

/// Ordered list of (version, sql). STUB — task B1 defines migration 1 (the
/// initial schema for sources, notes, blocks, entities, links, views,
/// agent_actions). Keep DDL here, not scattered across the codebase.
pub const MIGRATIONS: &[(&str, &str)] = &[
    // ("0001_init", include_str!("../migrations/0001_init.sql")),
];

/// Apply all pending migrations inside a transaction. STUB — task B1.
pub fn run(_conn: &Connection) -> Result<()> {
    // TODO(B1): create schema_version table if absent; for each entry in
    //           MIGRATIONS not yet applied, run it transactionally and record
    //           it. Add a test that migrating a fresh db succeeds and is
    //           idempotent (AGENTS.md Testing Expectations).
    unimplemented!("migration runner — STATUS.md task B1")
}
