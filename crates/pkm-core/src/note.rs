//! Note: a user-facing knowledge object (AGENTS.md "Note").
//!
//! A note is a structured record that can export to markdown — NOT a markdown
//! file with a database bolted on. It owns ordered [`crate::block::Block`]s and
//! carries typed metadata.

use serde::{Deserialize, Serialize};

use crate::id::{BlockId, NoteId};
use crate::{Actor, Timestamp};

/// Strongly-typed note metadata with known fields.
/// Avoids generic `serde_json::Value` — the compiler verifies this at compile time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct NoteMetadata {
    pub project: Option<String>,
    pub tags: Vec<String>,
    pub priority: Option<i32>,
    pub status: Option<String>,
}

impl NoteMetadata {
    pub fn is_empty(&self) -> bool {
        self.project.is_none()
            && self.tags.is_empty()
            && self.priority.is_none()
            && self.status.is_none()
    }
}

/// A durable knowledge object. Notes consist of ordered blocks and carry
/// metadata, provenance, and review state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub title: String,
    /// Ordered block ids. Resolving to full blocks happens in the storage/query
    /// layer, not here. Ordering is implicit in the vector position.
    pub blocks: Vec<BlockId>,
    /// Typed metadata. Use structured fields, not frontmatter blobs.
    /// Examples: project assignment, tags, custom fields.
    pub metadata: NoteMetadata,
    /// Who created this note (user or agent).
    pub created_by: Actor,
    /// When this note was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: Timestamp,
    /// Current version number (increments on each update).
    pub version: u32,
    /// When this version was created.
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: Timestamp,
}

impl Note {
    /// Generate a safe filename from the note title.
    /// Converts to lowercase, replaces spaces/graphic-punctuation with hyphens,
    /// appends UUID suffix for uniqueness, and adds .md extension.
    /// Example: "My Great Idea!" -> "my-great-idea-abc123de.md"
    pub fn file_name(&self) -> String {
        let uuid_str = self.id.0.to_string();
        let uuid_suffix = &uuid_str[uuid_str.len() - 8..];

        if self.title.is_empty() {
            return format!("untitled-{}.md", uuid_suffix);
        }

        let mut filename = String::new();
        let mut last_was_separator = true;

        for c in self.title.to_lowercase().chars() {
            if c.is_alphanumeric() {
                filename.push(c);
                last_was_separator = false;
            } else if !last_was_separator {
                // Treat any non-alphanumeric character (whitespace, punctuation,
                // symbols) as a separator, collapsing adjacent separators into one.
                filename.push('-');
                last_was_separator = true;
            }
        }

        let filename = filename.trim_matches('-').to_string();
        // Truncate to prevent ENAMETOOLONG errors
        let truncated: String = filename.chars().take(100).collect();

        if truncated.is_empty() {
            format!("untitled-{}.md", uuid_suffix)
        } else {
            format!("{}-{}.md", truncated, uuid_suffix)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_round_trips() {
        let metadata = NoteMetadata {
            project: Some("MyProject".to_string()),
            priority: Some(1),
            ..Default::default()
        };

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
            metadata: NoteMetadata::default(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let uuid_str = note.id.0.to_string();
        let suffix = &uuid_str[uuid_str.len() - 8..];
        assert_eq!(note.file_name(), format!("my-great-idea-{}.md", suffix));
    }

    #[test]
    fn file_name_with_special_characters() {
        let now = crate::Timestamp::now_utc();
        let note = Note {
            id: NoteId::new(),
            title: "My Great Idea!".to_string(),
            blocks: vec![],
            metadata: NoteMetadata::default(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let uuid_str = note.id.0.to_string();
        let suffix = &uuid_str[uuid_str.len() - 8..];
        assert_eq!(note.file_name(), format!("my-great-idea-{}.md", suffix));
    }

    #[test]
    fn file_name_with_only_whitespace() {
        let now = crate::Timestamp::now_utc();
        let note = Note {
            id: NoteId::new(),
            title: "   ".to_string(),
            blocks: vec![],
            metadata: NoteMetadata::default(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let uuid_str = note.id.0.to_string();
        let suffix = &uuid_str[uuid_str.len() - 8..];
        assert_eq!(note.file_name(), format!("untitled-{}.md", suffix));
    }

    #[test]
    fn file_name_with_hyphens_and_underscores() {
        let now = crate::Timestamp::now_utc();
        let note = Note {
            id: NoteId::new(),
            title: "My-Test_Note".to_string(),
            blocks: vec![],
            metadata: NoteMetadata::default(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let uuid_str = note.id.0.to_string();
        let suffix = &uuid_str[uuid_str.len() - 8..];
        assert_eq!(note.file_name(), format!("my-test-note-{}.md", suffix));
    }

    #[test]
    fn file_name_empty_title() {
        let now = crate::Timestamp::now_utc();
        let note = Note {
            id: NoteId::new(),
            title: "".to_string(),
            blocks: vec![],
            metadata: NoteMetadata::default(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let uuid_str = note.id.0.to_string();
        let suffix = &uuid_str[uuid_str.len() - 8..];
        assert_eq!(note.file_name(), format!("untitled-{}.md", suffix));
    }
}
