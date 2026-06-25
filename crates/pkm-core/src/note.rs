//! Note: a user-facing knowledge object (AGENTS.md "Note").
//!
//! A note is a structured record that can export to markdown — NOT a markdown
//! file with a database bolted on. It owns ordered [`crate::block::Block`]s and
//! carries typed metadata.

use serde::{Deserialize, Serialize};

use crate::id::{BlockId, NoteId};

/// STUB. The block list here is just ids; blocks are stored as their own
/// objects (see storage task B2). Markdown round-trip is task C2/E1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub title: String,
    /// Ordered block ids. Resolving to full blocks happens in the storage/query
    /// layer, not here.
    pub blocks: Vec<BlockId>,
    // TODO(C2): metadata map (typed, not frontmatter), created/updated, author.
}
