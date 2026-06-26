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

use crate::agent_action::{AgentAction, AgentActionStatus};
use crate::block::Block;
use crate::entity::Entity;
use crate::id::{AgentActionId, BlockId, EntityId, NoteId, ObjectRef, SourceId};
use crate::link::Link;
use crate::note::Note;
use crate::review::ReviewState;
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
    /// Update a block's content. Returns the updated block after applying the change.
    fn update_block(
        &self,
        note_id: NoteId,
        block_id: BlockId,
        new_content: crate::block::BlockContent,
    ) -> Result<Block>;
    // TODO(B2): block CRUD, ordered block fetch, metadata, version history.
}

/// Persistence for [`Entity`] (a normalized, referenceable object).
/// Entities support non-lossy merges: when merging, the loser is marked
/// merged_into the survivor, preserving history and enabling rollback.
pub trait EntityRepo {
    fn create(&self, entity: &Entity) -> Result<()>;
    fn get(&self, id: EntityId) -> Result<Option<Entity>>;
    /// Mark an entity as merged into another, preserving the loser's history.
    fn set_merged_into(&self, loser_id: EntityId, survivor_id: EntityId) -> Result<()>;
    // TODO(C4): list/filter, search by alias, count by kind.
}

/// Persistence for [`Link`] (typed relationships between objects).
pub trait LinkRepo {
    fn create(&self, link: &Link) -> Result<()>;
    fn get(&self, link_id: crate::id::LinkId) -> Result<Option<Link>>;
    /// Get all links pointing to a target object.
    fn get_by_to(&self, target: ObjectRef) -> Result<Vec<Link>>;
    /// Get all links originating from a source object.
    fn get_by_from(&self, source: ObjectRef) -> Result<Vec<Link>>;
    /// Update a link's to target (used for entity merge re-pointing).
    fn set_to(&self, link_id: crate::id::LinkId, new_to: ObjectRef) -> Result<()>;
    /// Update a link's from target (used for entity merge re-pointing).
    fn set_from(&self, link_id: crate::id::LinkId, new_from: ObjectRef) -> Result<()>;
}

/// Append-only persistence for agent action audit trail (AGENTS.md "Agent Action").
/// Actions record what agents changed, who/what triggered it, before/after state,
/// and rollback references. The log is append-only: statuses advance (Proposed →
/// Accepted → Applied), but actions are never deleted or modified in place.
pub trait AgentActionRepo {
    /// Persist a new action record. Returns the action as persisted (with any DB-
    /// generated fields like id, created_at).
    fn create(&self, action: &AgentAction) -> Result<()>;
    /// Retrieve an action by id.
    fn get(&self, id: AgentActionId) -> Result<Option<AgentAction>>;
    /// Update the status of an action. Only valid status transitions are allowed:
    /// Proposed → Accepted/Rejected/Applied → Reverted/Failed.
    fn set_status(&self, id: AgentActionId, new_status: AgentActionStatus) -> Result<()>;
    /// Update the diff of an action (to record before/after states when applied).
    fn set_diff(&self, id: AgentActionId, diff: serde_json::Value) -> Result<()>;
    // TODO(D2): list/filter (by target, by status, by actor, by date range), batch get.
}

// TODO(B2): ViewRepo.

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

/// A multi-mode search query with optional filters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchQuery {
    pub mode: SearchMode,
    pub text: String,
    /// Optional filters to narrow results.
    pub filters: SearchFilters,
}

/// Optional filters to apply to search results.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchFilters {
    /// Filter by object type (source, note, block, entity, link, view).
    pub object_type: Option<String>,
    /// Filter by content status (user_authored, raw_source, ai_summary, etc.).
    pub content_status: Option<String>,
    /// Filter by review state (proposed, accepted, rejected).
    pub review_state: Option<ReviewState>,
    /// Filter by date range (RFC3339 start, exclusive end).
    pub date_range: Option<(String, String)>,
    /// Filter by project/tag.
    pub project: Option<String>,
}

/// A search result. Preserves content status so the UI can distinguish
/// raw/reviewed/generated/unreviewed material.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SearchHit {
    /// The object that matched.
    pub object: crate::id::ObjectRef,
    /// Content status (for display and filtering).
    pub status: crate::provenance::ContentStatus,
    /// Optional relevance score (0.0-1.0), higher is better.
    pub score: Option<f64>,
    /// Optional matched text snippet (first ~150 chars of matching context).
    pub snippet: Option<String>,
}
