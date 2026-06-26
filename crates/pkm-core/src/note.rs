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

impl Note {
    /// Generate a safe filename from the note title.
    /// Converts to lowercase, replaces spaces with hyphens, removes special characters,
    /// and appends .md extension.
    /// Example: "My Great Idea!" -> "my-great-idea.md"
    pub fn file_name(&self) -> String {
        if self.title.is_empty() {
            return "untitled.md".to_string();
        }

        let mut filename = String::new();
        let mut last_was_separator = true;

        for c in self.title.to_lowercase().chars() {
            if c.is_alphanumeric() {
                filename.push(c);
                last_was_separator = false;
            } else if (c.is_whitespace() || c == '_' || c == '-') && !last_was_separator {
                filename.push('-');
                last_was_separator = true;
            }
            // Skip all other special characters
        }

        let filename = filename.trim_matches('-').to_string();

        if filename.is_empty() {
            "untitled.md".to_string()
        } else {
            format!("{}.md", filename)
        }
    }
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

    #[test]
    fn file_name_basic() {
        let now = crate::Timestamp::now_utc();
        let note = Note {
            id: NoteId::new(),
            title: "My Great Idea".to_string(),
            blocks: vec![],
            metadata: BTreeMap::new(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        assert_eq!(note.file_name(), "my-great-idea.md");
    }

    #[test]
    fn file_name_with_special_characters() {
        let now = crate::Timestamp::now_utc();
        let note = Note {
            id: NoteId::new(),
            title: "My Great Idea!".to_string(),
            blocks: vec![],
            metadata: BTreeMap::new(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        assert_eq!(note.file_name(), "my-great-idea.md");
    }

    #[test]
    fn file_name_with_multiple_special_chars() {
        let now = crate::Timestamp::now_utc();
        let note = Note {
            id: NoteId::new(),
            title: "Test@#$%^&*() Note???".to_string(),
            blocks: vec![],
            metadata: BTreeMap::new(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        assert_eq!(note.file_name(), "test-note.md");
    }

    #[test]
    fn file_name_with_only_whitespace() {
        let now = crate::Timestamp::now_utc();
        let note = Note {
            id: NoteId::new(),
            title: "   ".to_string(),
            blocks: vec![],
            metadata: BTreeMap::new(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        assert_eq!(note.file_name(), "untitled.md");
    }

    #[test]
    fn file_name_empty_title() {
        let now = crate::Timestamp::now_utc();
        let note = Note {
            id: NoteId::new(),
            title: "".to_string(),
            blocks: vec![],
            metadata: BTreeMap::new(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        assert_eq!(note.file_name(), "untitled.md");
    }

    #[test]
    fn file_name_with_hyphens_and_underscores() {
        let now = crate::Timestamp::now_utc();
        let note = Note {
            id: NoteId::new(),
            title: "My-Test_Note".to_string(),
            blocks: vec![],
            metadata: BTreeMap::new(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        assert_eq!(note.file_name(), "my-test-note.md");
    }

    #[test]
    fn file_name_with_numbers() {
        let now = crate::Timestamp::now_utc();
        let note = Note {
            id: NoteId::new(),
            title: "Test 123 Note 456".to_string(),
            blocks: vec![],
            metadata: BTreeMap::new(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        assert_eq!(note.file_name(), "test-123-note-456.md");
    }
}
