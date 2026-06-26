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
use crate::{Actor, Timestamp};

/// What a block holds. Start markdown-compatible; richer block kinds (tables,
/// embeds, generated sections) come later per STATUS.md task C2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BlockContent {
    /// A markdown text span (paragraph, heading, list item, etc.).
    Markdown { text: String },
    // TODO(C2+): Generated { text, provenance }, Embed { object_ref }, Table {..}.
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
                created_at: now.clone(),
                source_provenance_ref: None,
                version: 1,
                updated_at: now.clone(),
            },
            Block {
                id: BlockId::new(),
                note_id,
                content: BlockContent::Markdown {
                    text: "Third".to_string(),
                },
                order: 3.0,
                created_by: Actor::User,
                created_at: now.clone(),
                source_provenance_ref: None,
                version: 1,
                updated_at: now.clone(),
            },
            Block {
                id: BlockId::new(),
                note_id,
                content: BlockContent::Markdown {
                    text: "Second".to_string(),
                },
                order: 2.0,
                created_by: Actor::User,
                created_at: now.clone(),
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
            created_at: now.clone(),
            source_provenance_ref: None,
            version: 1,
            updated_at: now.clone(),
        };

        let last = Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::Markdown {
                text: "Third".to_string(),
            },
            order: 3.0,
            created_by: Actor::User,
            created_at: now.clone(),
            source_provenance_ref: None,
            version: 1,
            updated_at: now.clone(),
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
            created_at: now.clone(),
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        };

        // Verify ordering: 1.0 < 2.0 < 3.0
        assert!(first.order < middle.order);
        assert!(middle.order < last.order);
    }
}
