//! Ports: the trait boundaries the rest of the system programs against.
//!
//! This is the hexagonal seam. `pkm-storage` and `pkm-search` provide concrete
//! IMPLEMENTATIONS of these traits; `pkm-agent`/`pkm-ingestion` and the app
//! depend only on the TRAITS. Consequences:
//! - No crate except `pkm-storage` ever touches SQLite. Persistence details do
//!   not leak (AGENTS.md "Forbidden Shortcuts": agents must not scrape / reach
//!   around the typed API).
//! - The db engine or search backend can be swapped without changing callers.
//!
//! All methods return [`crate::Result`]; implementations map their internal
//! errors (sqlite, io) into [`crate::CoreError`]. No method panics.
//!
//! STUB — method sets are the agreed minimum shape. Tasks B2 (repos) and E2
//! (retriever) flesh them out. Keep the trait split one-per-aggregate.

use crate::id::{NoteId, SourceId};
use crate::note::Note;
use crate::source::Source;
use crate::Result;

/// Persistence for [`Source`]. Raw content is write-once except an explicit,
/// audited user edit — enforce that in the implementation, not the UI.
pub trait SourceRepo {
    fn create(&self, source: &Source) -> Result<()>;
    fn get(&self, id: SourceId) -> Result<Option<Source>>;
    // TODO(B2): list/filter, user_edit_raw (audited), soft-delete w/ recovery.
}

/// Persistence for [`Note`] and its blocks.
pub trait NoteRepo {
    fn create(&self, note: &Note) -> Result<()>;
    fn get(&self, id: NoteId) -> Result<Option<Note>>;
    // TODO(B2): block CRUD, ordered block fetch, metadata, version history.
}

// TODO(B2): EntityRepo, LinkRepo, ViewRepo, AgentActionRepo (append-only).

/// Multi-mode retrieval boundary. The SQLite/FTS implementation lives in
/// `pkm-storage`; pure query-parsing/ranking helpers live in `pkm-search`.
/// Every hit must carry its [`crate::provenance::ContentStatus`] so callers can
/// distinguish raw / reviewed / generated / unreviewed content.
pub trait Retriever {
    fn search(&self, query: &SearchQuery) -> Result<Vec<SearchHit>>;
}

/// Distinct search modes. Each is a separate capability (AGENTS.md: "Search is
/// not just one box"). Invariant enum — change via ADR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    ExactText,
    FuzzyText,
    Semantic,
    Entity,
    Source,
    LinkTraversal,
}

/// STUB — task E2 adds filters (date, type, review-state, project).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchQuery {
    pub mode: SearchMode,
    pub text: String,
}

/// STUB — task E2 adds score, snippet, cited sources.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchHit {
    pub object: crate::id::ObjectRef,
    pub status: crate::provenance::ContentStatus,
}
