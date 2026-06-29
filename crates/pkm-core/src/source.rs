//! Source: a raw piece of captured information (AGENTS.md "Source").
//!
//! INVARIANTS (do not violate):
//! - Raw source content is immutable unless the user explicitly edits it.
//! - Never overwrite raw content with a summary; summaries are derived objects
//!   that link back via `LinkType::DerivedFrom` / `Summarizes`.
//! - Always preserve `origin` metadata.

use serde::{Deserialize, Serialize};

use crate::id::SourceId;
use crate::ingestion::IngestionState;
use crate::{Actor, Timestamp};

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
    #[serde(skip, default)]
    pub raw_content: String,
    #[serde(with = "time::serde::rfc3339")]
    pub captured_at: Timestamp,
    pub content_hash: String,
    pub ingestion_state: IngestionState,
    pub created_by: Actor,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: Timestamp,
    pub version: u32,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: Timestamp,
    // TODO(D4): byte_attachment_ref for binary content.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_round_trips() {
        let now = crate::Timestamp::now_utc();
        let source = Source {
            id: SourceId::new(),
            origin: SourceOrigin::WebArticle {
                url: "https://example.com".to_string(),
            },
            title: Some("Example Article".to_string()),
            raw_content: "Some content".to_string(),
            captured_at: now,
            content_hash: "abc123def456".to_string(),
            ingestion_state: IngestionState::Captured,
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let json = serde_json::to_string(&source).expect("serialize");
        let back: Source = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back.id, source.id);
        assert_eq!(back.title, source.title);
        // raw_content is skipped in serde — rebuilt from .md files on load
        assert_eq!(back.raw_content, "");
        assert_eq!(back.content_hash, source.content_hash);
        assert_eq!(back.ingestion_state, source.ingestion_state);
        assert_eq!(back.created_by, source.created_by);
        assert_eq!(back.version, source.version);
    }
}
