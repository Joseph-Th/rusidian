//! Service layer: wires concrete implementations and provides a unified API
//! for the Tauri frontend. This is the only place where business-logic coordination
//! should happen. NO business logic in the UI layer.

use pkm_core::note::Note;
use pkm_core::ports::NoteRepo;
use pkm_core::{Actor, Timestamp};
use pkm_storage::{open, SqliteNoteRepo};
use rusqlite::Connection;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// The application service: aggregates all repositories and provides
/// high-level operations for the Tauri frontend. The service manages
/// the database connection and creates repository instances on-demand.
pub struct AppService {
    conn: Arc<Mutex<Connection>>,
}

impl AppService {
    /// Create a new AppService with the given database file.
    pub fn new(db_path: &str) -> Result<Self, String> {
        let conn = open(Path::new(db_path)).map_err(|e| format!("Failed to open db: {}", e))?;

        Ok(AppService {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create a new note and return its ID.
    pub fn create_note(&self, title: String) -> Result<String, String> {
        let now = Timestamp::now_utc();
        let note = Note {
            id: pkm_core::id::NoteId::new(),
            title,
            blocks: vec![],
            metadata: BTreeMap::new(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let note_id = note.id.to_string();

        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to acquire db lock".to_string())?;

        let note_repo = SqliteNoteRepo { conn: &conn };
        note_repo
            .create(&note)
            .map_err(|e| format!("Failed to create note: {}", e))?;

        Ok(note_id)
    }

    /// Get a single note by ID.
    pub fn get_note(&self, note_id: &str) -> Result<Option<(String, String)>, String> {
        let uuid =
            uuid::Uuid::parse_str(note_id).map_err(|_| format!("Invalid note ID: {}", note_id))?;
        let parsed_id = pkm_core::id::NoteId(uuid);

        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to acquire db lock".to_string())?;

        let note_repo = SqliteNoteRepo { conn: &conn };
        let note = note_repo
            .get(parsed_id)
            .map_err(|e| format!("Failed to get note: {}", e))?;

        Ok(note.map(|n| (n.id.to_string(), n.title)))
    }

    /// List all notes with optional limit.
    pub fn list_notes(&self, limit: Option<usize>) -> Result<Vec<(String, String)>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to acquire db lock".to_string())?;

        let note_repo = SqliteNoteRepo { conn: &conn };
        let notes = note_repo
            .list(limit)
            .map_err(|e| format!("Failed to list notes: {}", e))?;

        Ok(notes
            .into_iter()
            .map(|n| (n.id.to_string(), n.title))
            .collect())
    }

    /// Get a full note by ID, including all metadata and block count.
    pub fn get_note_full(&self, note_id: &str) -> Result<Option<Note>, String> {
        let uuid =
            uuid::Uuid::parse_str(note_id).map_err(|_| format!("Invalid note ID: {}", note_id))?;
        let parsed_id = pkm_core::id::NoteId(uuid);

        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to acquire db lock".to_string())?;

        let note_repo = SqliteNoteRepo { conn: &conn };
        let note = note_repo
            .get(parsed_id)
            .map_err(|e| format!("Failed to get note: {}", e))?;

        Ok(note)
    }

    /// Update a note's title and metadata.
    pub fn update_note(
        &self,
        note_id: &str,
        title: String,
        metadata: BTreeMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        let uuid =
            uuid::Uuid::parse_str(note_id).map_err(|_| format!("Invalid note ID: {}", note_id))?;
        let parsed_id = pkm_core::id::NoteId(uuid);

        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to acquire db lock".to_string())?;

        let note_repo = SqliteNoteRepo { conn: &conn };
        let mut note = note_repo
            .get(parsed_id)
            .map_err(|e| format!("Failed to get note: {}", e))?
            .ok_or_else(|| format!("Note not found: {}", note_id))?;

        note.title = title;
        note.metadata = metadata;
        note.version += 1;
        note.updated_at = Timestamp::now_utc();

        note_repo
            .update(&note)
            .map_err(|e| format!("Failed to update note: {}", e))?;

        Ok(())
    }

    /// Delete a note by ID.
    pub fn delete_note(&self, note_id: &str) -> Result<(), String> {
        let uuid =
            uuid::Uuid::parse_str(note_id).map_err(|_| format!("Invalid note ID: {}", note_id))?;
        let parsed_id = pkm_core::id::NoteId(uuid);

        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to acquire db lock".to_string())?;

        let note_repo = SqliteNoteRepo { conn: &conn };
        note_repo
            .delete(parsed_id)
            .map_err(|e| format!("Failed to delete note: {}", e))?;

        Ok(())
    }
}
