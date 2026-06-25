//! Provenance + content status — cross-cutting, so they live in core.
//!
//! AGENTS.md treats source provenance as foundational, not a feature: it
//! touches notes, blocks, summaries, links, retrieval, views, and agent
//! actions. Defining `ContentStatus` and `Provenance` here (not in `search`)
//! means every layer shares ONE definition; search/UI *filter* by these,
//! they do not redefine them.

use serde::{Deserialize, Serialize};

use crate::id::SourceId;
use crate::{Actor, Timestamp};

/// What kind of content this is and how trustworthy it is. Retrieval and the UI
/// must surface this so generated/unreviewed content is never shown as settled
/// knowledge (AGENTS.md "Search and Retrieval Rules").
///
/// Invariant enum — change via ADR (+ migration, since it is persisted).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentStatus {
    UserAuthored,
    RawSource,
    AiSummary,
    ExtractedMetadata,
    InferredLink,
    Reviewed,
    UnreviewedSuggestion,
}

/// Where a piece of derived knowledge came from. Attach to any derived object
/// (summary block, inferred link, extracted entity) so it can always be traced
/// back to its source(s) and originating actor.
///
/// STUB — task C5 finalizes the field set (e.g. originating agent action,
/// extraction span/offsets). Keep `derived_from` non-empty for derived content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub status: ContentStatus,
    /// Source(s) this content was derived from. Empty only for genuinely
    /// original user-authored content.
    pub derived_from: Vec<SourceId>,
    pub created_by: Actor,
    pub created_at: Timestamp,
    // TODO(C5): originating_action: Option<AgentActionId>, extraction span.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_status_round_trips_as_snake_case() {
        for status in [
            ContentStatus::UserAuthored,
            ContentStatus::RawSource,
            ContentStatus::AiSummary,
            ContentStatus::ExtractedMetadata,
            ContentStatus::InferredLink,
            ContentStatus::Reviewed,
            ContentStatus::UnreviewedSuggestion,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: ContentStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }
}
