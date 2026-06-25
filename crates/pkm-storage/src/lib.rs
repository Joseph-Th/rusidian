//! pkm-storage: embedded local persistence (SQLite).
//!
//! INVARIANTS (AGENTS.md "Data Integrity Rules"):
//! - Structured data is the source of truth; markdown is an export format.
//! - Every schema change ships with a migration (see [`migrations`]).
//! - All durable objects keep stable ids.
//! - Destructive operations need a recovery path.
//!
//! This crate OWNS SQL and is the only place `rusqlite` appears. It implements
//! the repository + retriever ports from `pkm_core::ports`. Keep IO at the
//! edges: pure row<->`pkm_core` mapping lives in small, testable functions.
//!
//! Layout (designer-mandated, see STATUS):
//!   migrations/*.sql       append-only schema migrations (never edit shipped)
//!   src/db.rs              connection open + pragmas
//!   src/migrations.rs      migration runner
//!   src/repositories/*.rs  one port implementation per aggregate
//!   tests/                 migration + round-trip tests
//! See SCHEMA.md for the current schema overview.

pub mod blob_store;
pub mod db;
pub mod migrations;
pub mod repositories;

pub use blob_store::BlobStore;
pub use repositories::SqliteSourceRepo;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("migration: {0}")]
    Migration(String),

    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),

    #[error(transparent)]
    Core(#[from] pkm_core::CoreError),
}

impl From<StorageError> for pkm_core::CoreError {
    /// Port methods return `pkm_core::Result`, so storage errors must collapse
    /// into a `CoreError` at the boundary. This keeps `rusqlite` from leaking
    /// out of this crate. Refine the mapping (e.g. NotFound) in task B2.
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::Core(c) => c,
            other => pkm_core::CoreError::Invariant(other.to_string()),
        }
    }
}

pub use db::open;
