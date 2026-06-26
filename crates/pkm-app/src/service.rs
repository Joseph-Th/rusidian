//! Service layer: wires concrete implementations and provides a unified API
//! for the Tauri frontend. This is the only place where business-logic coordination
//! should happen. NO business logic in the UI layer.

use pkm_core::note::Note;
use pkm_core::ports::{NoteRepo, Retriever, SearchMode, SourceRepo, ViewRepo};
use pkm_core::view::{DefaultViewModel, View, ViewKind, ViewModel, ViewParams};
use pkm_core::{Actor, Timestamp};
use pkm_search::parse_query;
use pkm_storage::{open, SqliteNoteRepo, SqliteRetriever, SqliteSourceRepo, SqliteViewRepo};
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

    /// Create a new view and return its ID.
    pub fn create_view(
        &self,
        kind: ViewKind,
        title: String,
        params: ViewParams,
    ) -> Result<String, String> {
        let now = Timestamp::now_utc();
        let view = View {
            id: pkm_core::id::ViewId::new(),
            kind,
            title,
            params,
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let view_id = view.id.to_string();

        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to acquire db lock".to_string())?;

        let view_repo = SqliteViewRepo { conn: &conn };
        view_repo
            .create(&view)
            .map_err(|e| format!("Failed to create view: {}", e))?;

        Ok(view_id)
    }

    /// List all views with optional limit.
    pub fn list_views(&self, limit: Option<usize>) -> Result<Vec<View>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to acquire db lock".to_string())?;

        let view_repo = SqliteViewRepo { conn: &conn };
        view_repo
            .list(limit)
            .map_err(|e| format!("Failed to list views: {}", e))
    }

    /// Get a view by ID.
    pub fn get_view(&self, view_id: &str) -> Result<Option<View>, String> {
        let uuid =
            uuid::Uuid::parse_str(view_id).map_err(|_| format!("Invalid view ID: {}", view_id))?;
        let parsed_id = pkm_core::id::ViewId(uuid);

        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to acquire db lock".to_string())?;

        let view_repo = SqliteViewRepo { conn: &conn };
        view_repo
            .get(parsed_id)
            .map_err(|e| format!("Failed to get view: {}", e))
    }

    /// Render a view by ID, returning matching source IDs in order.
    pub fn render_view(&self, view_id: &str) -> Result<Vec<String>, String> {
        let view = self
            .get_view(view_id)?
            .ok_or_else(|| format!("View not found: {}", view_id))?;

        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to acquire db lock".to_string())?;

        let source_repo = SqliteSourceRepo { conn: &conn };
        let sources = source_repo
            .list(None)
            .map_err(|e| format!("Failed to list sources: {}", e))?;

        let result = match &view.params {
            ViewParams::ReadingQueue(params) => {
                DefaultViewModel::render_reading_queue(params, &sources)
            }
            ViewParams::ReviewQueue(params) => {
                DefaultViewModel::render_review_queue(params, &sources)
            }
            ViewParams::Timeline(params) => DefaultViewModel::render_timeline(params, &sources),
            ViewParams::Dossier(params) => DefaultViewModel::render_dossier(params, &sources),
            ViewParams::ProjectDashboard(params) => {
                DefaultViewModel::render_project_dashboard(params, &sources)
            }
            ViewParams::SourceMap(params) => DefaultViewModel::render_source_map(params, &sources),
            ViewParams::DecisionLog(params) => {
                DefaultViewModel::render_decision_log(params, &sources)
            }
            ViewParams::PersonProfile(params) => {
                DefaultViewModel::render_person_profile(params, &sources)
            }
            ViewParams::EntityPage(params) => {
                DefaultViewModel::render_entity_page(params, &sources)
            }
            ViewParams::BriefingPage(params) => {
                DefaultViewModel::render_briefing_page(params, &sources)
            }
            ViewParams::OpenQuestions(params) => {
                DefaultViewModel::render_open_questions(params, &sources)
            }
            ViewParams::ActionList(params) => {
                DefaultViewModel::render_action_list(params, &sources)
            }
            ViewParams::GraphView(params) => DefaultViewModel::render_graph_view(params, &sources),
            ViewParams::Stub(_) => Err("Stub view not yet implemented".to_string()),
        }
        .map_err(|e| format!("Failed to render view: {}", e))?;

        Ok(result.source_ids.iter().map(|id| id.to_string()).collect())
    }

    /// Search notes by query text using fuzzy text search.
    pub fn search_notes(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<(String, String)>, String> {
        use pkm_core::id::ObjectRef;

        let search_query = parse_query(SearchMode::FuzzyText, query);
        let retriever = SqliteRetriever::new(self.conn.clone());

        let hits = retriever
            .search(&search_query)
            .map_err(|e| format!("Search failed: {}", e))?;

        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to acquire db lock".to_string())?;

        let note_repo = SqliteNoteRepo { conn: &conn };
        let mut results = Vec::new();

        for hit in hits.iter().take(limit.unwrap_or(50)) {
            match hit.object {
                ObjectRef::Note(note_id) => {
                    if let Ok(Some(note)) = note_repo.get(note_id) {
                        results.push((note.id.to_string(), note.title));
                    } else if let Some(snippet) = &hit.snippet {
                        results.push((format!("{:?}", hit.object), snippet.clone()));
                    }
                }
                _ => {
                    if let Some(snippet) = &hit.snippet {
                        results.push((format!("{:?}", hit.object), snippet.clone()));
                    }
                }
            }
        }

        Ok(results)
    }
}
