//! Block: a stable, addressable unit inside a note.
//!
//! Blocks exist so agents can make the *smallest correct change* (AGENTS.md
//! "Agent Editing Rules") instead of rewriting whole notes. Each block keeps a
//! stable `BlockId` across edits.
//!
//! BLOCK ORDERING: blocks are ordered implicitly by their position in the
//! parent note's `blocks: Vec<BlockId>` array. Reordering is done by
//! manipulating the vector (remove + insert at new index).

use serde::{Deserialize, Serialize};

use crate::id::{BlockId, NoteId, ObjectRef};
use crate::media::{EmbedProvider, MediaType};
use crate::{Actor, Timestamp};

/// What a block holds. Each variant is strongly typed to prevent AI agents from
/// generating malformed syntax. Markdown serialization is handled in the markdown module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BlockContent {
    /// A markdown text span (paragraph, heading, list item, etc.).
    Markdown { text: String },

    /// LaTeX math equation.
    Math {
        expression: String,
        /// True for display mode ($$), false for inline ($)
        display_mode: bool,
    },

    /// Local media (images, PDFs) backed by BlobStore, or remote URLs.
    Media {
        /// Either a local blob hash (from BlobStore) or remote URL.
        hash_or_url: String,
        /// Descriptive alt text for accessibility.
        alt_text: String,
        /// Type of media (image, audio, video, PDF).
        media_type: MediaType,
    },

    /// Strictly structured table. Rows must all have the same length as headers.
    Table {
        /// Column headers.
        headers: Vec<String>,
        /// Table rows. Each row is a list of cells.
        rows: Vec<Vec<String>>,
    },

    /// Internal transclusion (embedding another Note, View, or Block).
    InternalEmbed {
        /// Reference to the embedded object.
        target: ObjectRef,
        /// Text snapshot for markdown fallback (shown when opened outside app).
        fallback_text: String,
    },

    /// External iFrame embed (YouTube, Twitter, Google Drive, etc.).
    ExternalEmbed {
        /// URL to embed.
        url: String,
        /// Provider (used to detect embed type and construct embed code).
        provider: EmbedProvider,
    },
}

/// A stable, addressable unit inside a note. Blocks support fine-grained edits
/// and maintain order for insertion and reordering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub note_id: NoteId,
    pub content: BlockContent,
    // RIP: pub order: String, <- DELETED!
    /// Who created this block (user or agent).
    pub created_by: Actor,
    /// When this block was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: Timestamp,
    /// If this block was extracted/derived from a source, reference it here.
    pub source_provenance_ref: Option<ObjectRef>,
    /// Current version number (increments on each update).
    pub version: u32,
    /// When this version was created.
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_content_variants_are_serializable() {
        use crate::id::ViewId;

        // Test that all BlockContent variants serialize correctly
        let variants = vec![
            BlockContent::Markdown {
                text: "Test".to_string(),
            },
            BlockContent::Math {
                expression: "x^2".to_string(),
                display_mode: true,
            },
            BlockContent::Math {
                expression: "x".to_string(),
                display_mode: false,
            },
            BlockContent::Table {
                headers: vec!["A".to_string(), "B".to_string()],
                rows: vec![vec!["1".to_string(), "2".to_string()]],
            },
            BlockContent::Media {
                hash_or_url: "hash123".to_string(),
                alt_text: "Image".to_string(),
                media_type: crate::media::MediaType::Image,
            },
            BlockContent::InternalEmbed {
                target: ObjectRef::View(ViewId::new()),
                fallback_text: "View content".to_string(),
            },
            BlockContent::ExternalEmbed {
                url: "https://example.com".to_string(),
                provider: crate::media::EmbedProvider::Generic,
            },
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).expect("serialize");
            let back: BlockContent = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, variant);
        }
    }
}
