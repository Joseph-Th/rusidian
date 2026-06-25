//! Source: a raw piece of captured information (AGENTS.md "Source").
//!
//! INVARIANTS (do not violate):
//! - Raw source content is immutable unless the user explicitly edits it.
//! - Never overwrite raw content with a summary; summaries are derived objects
//!   that link back via `LinkType::DerivedFrom` / `Summarizes`.
//! - Always preserve `origin` metadata.

use serde::{Deserialize, Serialize};

use crate::id::SourceId;

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

/// STUB. See STATUS.md task C1 for the full field set (raw bytes/text handling,
/// captured_at, content hash, ingestion status link).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub id: SourceId,
    pub origin: SourceOrigin,
    pub title: Option<String>,
    /// Raw captured text. Binary attachments are handled separately (task D4).
    pub raw_content: String,
    // TODO(C1): captured_at, content_hash, ingestion_state, byte_attachment_ref.
}
