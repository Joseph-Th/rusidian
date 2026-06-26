//! Note: a user-facing knowledge object (AGENTS.md "Note").
//!
//! A note is a structured record that can export to markdown — NOT a markdown
//! file with a database bolted on. It owns ordered [`crate::block::Block`]s and
//! carries typed metadata.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::id::{BlockId, NoteId};
use crate::sync::SyncEligible;
use crate::{Actor, Timestamp};

/// A durable knowledge object. Notes consist of ordered blocks and carry
/// metadata, provenance, and review state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub title: String,
    /// Ordered block ids. Resolving to full blocks happens in the storage/query
    /// layer, not here. Blocks maintain their own `order` key for stable reordering.
    pub blocks: Vec<BlockId>,
    /// Typed metadata. Use structured fields, not frontmatter blobs.
    /// Examples: project assignment, tags, custom fields.
    pub metadata: BTreeMap<String, serde_json::Value>,
    /// Who created this note (user or agent).
    pub created_by: Actor,
    /// When this note was created.
    pub created_at: Timestamp,
    /// Current version number (increments on each update).
    pub version: u32,
    /// When this version was created.
    pub updated_at: Timestamp,
}

impl SyncEligible for Note {
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
    fn note_round_trips() {
        let mut metadata = BTreeMap::new();
        metadata.insert("project".to_string(), serde_json::json!("MyProject"));
        metadata.insert("priority".to_string(), serde_json::json!(1));

        let now = crate::Timestamp::now_utc();
        let note = Note {
            id: NoteId::new(),
            title: "Test Note".to_string(),
            blocks: vec![BlockId::new(), BlockId::new()],
            metadata,
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let json = serde_json::to_string(&note).expect("serialize");
        let back: Note = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back.id, note.id);
        assert_eq!(back.title, note.title);
        assert_eq!(back.blocks, note.blocks);
        assert_eq!(back.metadata, note.metadata);
        assert_eq!(back.created_by, note.created_by);
        assert_eq!(back.version, note.version);
    }
}
