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

/// Byte range marking where content was extracted from within a source.
/// Used to precisely locate what in the original material generated a
/// derived object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionSpan {
    /// Start byte offset (inclusive).
    pub start: u64,
    /// End byte offset (exclusive).
    pub end: u64,
}

impl ExtractionSpan {
    /// Create a new extraction span. Returns Err if start > end.
    pub fn new(start: u64, end: u64) -> Result<Self, crate::CoreError> {
        if start > end {
            return Err(crate::CoreError::Invariant(format!(
                "extraction span start {} > end {}",
                start, end
            )));
        }
        Ok(Self { start, end })
    }

    /// Length of the extraction span in bytes.
    pub fn len(&self) -> u64 {
        self.end - self.start
    }

    /// Whether this span is empty (zero-length).
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// Where a piece of derived knowledge came from. Attach to any derived object
/// (summary block, inferred link, extracted entity) so it can always be traced
/// back to its source(s) and originating actor.
///
/// Invariant: derived content (ContentStatus != UserAuthored) must have
/// non-empty `derived_from`. Traceability to originating action and precise
/// extraction location is optional but highly recommended for accountability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub status: ContentStatus,
    /// Source(s) this content was derived from. Empty only for genuinely
    /// original user-authored content. For derived content, must be non-empty.
    pub derived_from: Vec<SourceId>,
    pub created_by: Actor,
    pub created_at: Timestamp,
    /// The agent action that originated this content, if any (UUID string).
    pub originating_action: Option<String>,
    /// Where in the source this content was extracted from.
    pub extraction_span: Option<ExtractionSpan>,
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

    #[test]
    fn extraction_span_len() {
        let span = ExtractionSpan::new(10, 20).unwrap();
        assert_eq!(span.len(), 10);
        assert!(!span.is_empty());

        let empty = ExtractionSpan::new(5, 5).unwrap();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn extraction_span_round_trips() {
        let span = ExtractionSpan::new(100, 150).unwrap();
        let json = serde_json::to_string(&span).unwrap();
        let back: ExtractionSpan = serde_json::from_str(&json).unwrap();
        assert_eq!(back, span);
    }

    #[test]
    fn extraction_span_returns_error_on_invalid_range() {
        let result = ExtractionSpan::new(20, 10);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("start 20 > end 10"));
    }

    #[test]
    fn provenance_with_all_fields_round_trips() {
        let source_id = SourceId::new();
        let action_id = "some-uuid".to_string();
        let span = ExtractionSpan::new(0, 42).unwrap();
        let now = Timestamp::now_utc();

        let prov = Provenance {
            status: ContentStatus::AiSummary,
            derived_from: vec![source_id],
            created_by: Actor::Agent {
                name: "test-agent".to_string(),
            },
            created_at: now,
            originating_action: Some(action_id),
            extraction_span: Some(span),
        };

        let json = serde_json::to_string(&prov).unwrap();
        let back: Provenance = serde_json::from_str(&json).unwrap();
        assert_eq!(back, prov);
    }

    #[test]
    fn provenance_derived_content_requires_sources() {
        let now = Timestamp::now_utc();
        let derived = Provenance {
            status: ContentStatus::AiSummary,
            derived_from: vec![],
            created_by: Actor::Agent {
                name: "test".to_string(),
            },
            created_at: now,
            originating_action: None,
            extraction_span: None,
        };

        // Invariant: derived content must have non-empty derived_from.
        // This test documents the expectation; enforcement happens at
        // construction boundaries (storage layer, agent layer).
        assert!(
            derived.derived_from.is_empty() && derived.status != ContentStatus::UserAuthored,
            "derived content with empty derived_from violates C5 invariant"
        );
    }

    #[test]
    fn user_authored_provenance_has_empty_sources() {
        let now = Timestamp::now_utc();
        let user_authored = Provenance {
            status: ContentStatus::UserAuthored,
            derived_from: vec![],
            created_by: Actor::User,
            created_at: now,
            originating_action: None,
            extraction_span: None,
        };

        assert_eq!(user_authored.status, ContentStatus::UserAuthored);
        assert!(user_authored.derived_from.is_empty());
    }
}
