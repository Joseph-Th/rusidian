//! Block: a stable, addressable unit inside a note.
//!
//! Blocks exist so agents can make the *smallest correct change* (AGENTS.md
//! "Agent Editing Rules") instead of rewriting whole notes. Each block keeps a
//! stable `BlockId` across edits.

use serde::{Deserialize, Serialize};

use crate::id::{BlockId, NoteId};

/// What a block holds. Start markdown-compatible; richer block kinds (tables,
/// embeds, generated sections) come later per STATUS.md task C2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BlockContent {
    /// A markdown text span (paragraph, heading, list item, etc.).
    Markdown { text: String },
    // TODO(C2): Generated { text, provenance }, Embed { object_ref }, Table {..}.
}

/// STUB. Ordering within a note is task C2 (decide: explicit index vs. linked
/// list). Do not assume array position is the order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub note_id: NoteId,
    pub content: BlockContent,
    // TODO(C2): order key, created_by, created_at, source provenance ref.
}
