//! Block: a stable, addressable unit inside a note.
//!
//! Blocks exist so agents can make the *smallest correct change* (AGENTS.md
//! "Agent Editing Rules") instead of rewriting whole notes. Each block keeps a
//! stable `BlockId` across edits.
//!
//! BLOCK ORDERING: blocks are ordered within a note using an `order` key (f32).
//! This allows stable reordering without re-indexing: insert a new block at
//! order 1.5 between blocks at 1.0 and 2.0. Clients maintain order invariant.

use serde::{Deserialize, Serialize};

use crate::id::{BlockId, NoteId, ObjectRef};
use crate::media::{EmbedProvider, MediaType};
use crate::sync::SyncEligible;
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
    /// Fractional order key for stable insertion/reordering (not array position).
    /// Insert between blocks by choosing an intermediate value.
    pub order: f32,
    /// Who created this block (user or agent).
    pub created_by: Actor,
    /// When this block was created.
    pub created_at: Timestamp,
    /// If this block was extracted/derived from a source, reference it here.
    pub source_provenance_ref: Option<ObjectRef>,
    /// Current version number (increments on each update).
    pub version: u32,
    /// When this version was created.
    pub updated_at: Timestamp,
}

impl SyncEligible for Block {
    fn version(&self) -> u32 {
        self.version
    }

    fn updated_at(&self) -> Timestamp {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_ordering_stable_with_fractional_keys() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        // Create three blocks with fractional order keys.
        let mut blocks = [
            Block {
                id: BlockId::new(),
                note_id,
                content: BlockContent::Markdown {
                    text: "First".to_string(),
                },
                order: 1.0,
                created_by: Actor::User,
                created_at: now,
                source_provenance_ref: None,
                version: 1,
                updated_at: now,
            },
            Block {
                id: BlockId::new(),
                note_id,
                content: BlockContent::Markdown {
                    text: "Third".to_string(),
                },
                order: 3.0,
                created_by: Actor::User,
                created_at: now,
                source_provenance_ref: None,
                version: 1,
                updated_at: now,
            },
            Block {
                id: BlockId::new(),
                note_id,
                content: BlockContent::Markdown {
                    text: "Second".to_string(),
                },
                order: 2.0,
                created_by: Actor::User,
                created_at: now,
                source_provenance_ref: None,
                version: 1,
                updated_at: now,
            },
        ];

        // Sort by order key.
        blocks.sort_by(|a, b| a.order.partial_cmp(&b.order).unwrap());

        // Verify order is correct.
        assert_eq!(blocks[0].order, 1.0);
        assert_eq!(blocks[1].order, 2.0);
        assert_eq!(blocks[2].order, 3.0);
    }

    #[test]
    fn block_ordering_allows_insert_between() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        // Create a block between two existing blocks.
        let first = Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::Markdown {
                text: "First".to_string(),
            },
            order: 1.0,
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        };

        let last = Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::Markdown {
                text: "Third".to_string(),
            },
            order: 3.0,
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        };

        // Insert a new block between them (order = 2.0, which is between 1.0 and 3.0).
        let middle = Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::Markdown {
                text: "Inserted".to_string(),
            },
            order: 2.0,
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        };

        // Verify ordering: 1.0 < 2.0 < 3.0
        assert!(first.order < middle.order);
        assert!(middle.order < last.order);
    }

    #[test]
    fn block_content_variants_are_serializable() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        // Test that all BlockContent variants serialize correctly
        let variants = vec![
            BlockContent::Markdown {
                text: "Test".to_string(),
            },
            BlockContent::Math {
                expression: "x^2".to_string(),
                display_mode: true,
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
