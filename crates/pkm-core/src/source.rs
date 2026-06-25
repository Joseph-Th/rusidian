//! Source: a raw piece of captured information (AGENTS.md "Source").
//!
//! INVARIANTS (do not violate):
//! - Raw source content is immutable unless the user explicitly edits it.
//! - Never overwrite raw content with a summary; summaries are derived objects
//!   that link back via `LinkType::DerivedFrom` / `Summarizes`.
//! - Always preserve `origin` metadata.

use serde::{Deserialize, Serialize};

use crate::id::SourceId;
use crate::{Actor, Timestamp};
use crate::ingestion::IngestionState;

/// Where a source came from. Extend this enum as new ingestion kinds land
/// (AGENTS.md lists web article, pdf, email, screenshot, transcript, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceOrigin {
    WebArticle { url: String },
    Pdf { path: String },
    Email,
    Screenshot,
    AudioTranscript,
    PastedText,
    ImportedMarkdown { path: String },
    ManualCapture,
}

/// A captured piece of raw information. Raw content is immutable unless the
/// user explicitly edits it; summaries and derived artifacts link back via
/// `LinkType::DerivedFrom` / `Summarizes` (AGENTS.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub id: SourceId,
    pub origin: SourceOrigin,
    pub title: Option<String>,
    pub raw_content: String,
    pub captured_at: Timestamp,
    pub content_hash: String,
    pub ingestion_state: IngestionState,
    pub created_by: Actor,
    // TODO(D4): byte_attachment_ref for binary content.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_round_trips() {
        let source = Source {
            id: SourceId::new(),
            origin: SourceOrigin::WebArticle {
                url: "https://example.com".to_string(),
            },
            title: Some("Example Article".to_string()),
            raw_content: "Some content".to_string(),
            captured_at: crate::Timestamp::now_utc(),
            content_hash: "abc123def456".to_string(),
            ingestion_state: IngestionState::Captured,
            created_by: Actor::User,
        };

        let json = serde_json::to_string(&source).expect("serialize");
        let back: Source = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back.id, source.id);
        assert_eq!(back.title, source.title);
        assert_eq!(back.raw_content, source.raw_content);
        assert_eq!(back.content_hash, source.content_hash);
        assert_eq!(back.ingestion_state, source.ingestion_state);
        assert_eq!(back.created_by, source.created_by);
    }
}
