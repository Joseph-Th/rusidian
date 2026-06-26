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
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use crate::watcher::{watch_vault, NoteWatcherEvent};

type GraphNode = (String, f64, f64);

/// The application service: aggregates all repositories and provides
/// high-level operations for the Tauri frontend. The service manages
/// the database connection, vault directory, and creates repository instances on-demand.
pub struct AppService {
    conn: Arc<Mutex<Connection>>,
    vault_path: PathBuf,
}

impl AppService {
    /// Create a new AppService with the given database file and vault directory.
    /// If vault_path is not provided, it defaults to ./vault relative to the database.
    pub fn new(db_path: &str, vault_path: Option<&str>) -> Result<Self, String> {
        let conn = open(Path::new(db_path)).map_err(|e| format!("Failed to open db: {}", e))?;

        // Determine vault path
        let vault = if let Some(path) = vault_path {
            PathBuf::from(path)
        } else {
            let db = Path::new(db_path);
            let db_dir = db.parent().unwrap_or_else(|| Path::new("."));
            db_dir.join("vault")
        };

        // Create vault directory if it doesn't exist
        std::fs::create_dir_all(&vault).map_err(|e| format!("Failed to create vault dir: {}", e))?;

        Ok(AppService {
            conn: Arc::new(Mutex::new(conn)),
            vault_path: vault,
        })
    }

    /// Start watching the vault directory for external markdown file changes.
    /// When a user edits a file in an external editor (VS Code, Notepad, etc.),
    /// this method detects the change, parses the file, and syncs it to the database.
    ///
    /// The watcher runs in a background thread and continues until the receiver is dropped.
    pub fn start_vault_watcher(&self) -> Result<(), String> {
        let conn_clone = Arc::clone(&self.conn);
        let vault_path = self.vault_path.clone();

        // Start the file watcher
        let watcher_rx = watch_vault(&vault_path)
            .map_err(|e| format!("Failed to start vault watcher: {}", e))?;

        // Spawn a background task to process watcher events
        std::thread::spawn(move || {
            while let Ok(event) = watcher_rx.recv() {
                // Process the watcher event: sync to database
                if let Err(e) = Self::sync_external_note(&conn_clone, event) {
                    eprintln!("Failed to sync external note change: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Sync a note from an external file change into the database.
    /// This is called when the file watcher detects a change to a markdown file.
    fn sync_external_note(
        conn: &Arc<Mutex<Connection>>,
        event: NoteWatcherEvent,
    ) -> Result<(), String> {
        let conn = conn
            .lock()
            .map_err(|_| "Failed to acquire database lock".to_string())?;

        // Insert or replace the note in the database
        let note = &event.note;
        let created_at_str = note
            .created_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());
        let updated_at_str = note
            .updated_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());

        let metadata_json = serde_json::to_string(&note.metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
        let created_by_json = serde_json::to_string(&note.created_by)
            .map_err(|e| format!("Failed to serialize actor: {}", e))?;
        let file_path_str = event
            .file_path
            .to_str()
            .ok_or_else(|| "Invalid file path".to_string())?;

        // INSERT OR REPLACE the note
        conn.execute(
            "INSERT OR REPLACE INTO note (id, title, created_at, created_by, version, updated_at, metadata, file_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                note.id.to_string(),
                note.title,
                created_at_str,
                created_by_json,
                note.version as i64,
                updated_at_str,
                metadata_json,
                file_path_str,
            ],
        )
        .map_err(|e| format!("Failed to insert note: {}", e))?;

        println!(
            "✓ Synced external note: {} ({})",
            note.title, event.file_path.display()
        );

        Ok(())
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

        let note_repo = SqliteNoteRepo { conn: &conn, vault_path: self.vault_path.clone() };
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

        let note_repo = SqliteNoteRepo { conn: &conn, vault_path: self.vault_path.clone() };
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

        let note_repo = SqliteNoteRepo { conn: &conn, vault_path: self.vault_path.clone() };
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

        let note_repo = SqliteNoteRepo { conn: &conn, vault_path: self.vault_path.clone() };
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

        let note_repo = SqliteNoteRepo { conn: &conn, vault_path: self.vault_path.clone() };
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

        let note_repo = SqliteNoteRepo { conn: &conn, vault_path: self.vault_path.clone() };
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

        let note_repo = SqliteNoteRepo { conn: &conn, vault_path: self.vault_path.clone() };
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

    /// Get graph data for visualization: view details with positions.
    pub fn get_graph_view_data(&self, view_id: &str) -> Result<Option<Vec<GraphNode>>, String> {
        let view = self.get_view(view_id)?;

        match view {
            Some(view) => {
                if let ViewParams::GraphView(params) = &view.params {
                    // If positions are stored in params, use them
                    if !params.node_positions.is_empty() {
                        let nodes: Vec<_> = params
                            .node_positions
                            .iter()
                            .map(|pos| (pos.id.clone(), pos.x, pos.y))
                            .collect();
                        Ok(Some(nodes))
                    } else {
                        // Generate default positions for stored sources
                        let conn = self
                            .conn
                            .lock()
                            .map_err(|_| "Failed to acquire db lock".to_string())?;

                        let source_repo = SqliteSourceRepo { conn: &conn };
                        let sources = source_repo
                            .list(params.limit)
                            .map_err(|e| format!("Failed to list sources: {}", e))?;

                        // Simple circular layout for default positions
                        let count = sources.len() as f64;
                        let nodes: Vec<_> = sources
                            .iter()
                            .enumerate()
                            .map(|(i, source)| {
                                let angle = (i as f64) * 2.0 * std::f64::consts::PI / count;
                                let x = 200.0 + 150.0 * angle.cos();
                                let y = 200.0 + 150.0 * angle.sin();
                                (source.id.to_string(), x, y)
                            })
                            .collect();

                        Ok(Some(nodes))
                    }
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Get show_edges setting for a graph view (for link visualization).
    pub fn get_graph_show_edges(&self, view_id: &str) -> Result<bool, String> {
        let view = self
            .get_view(view_id)?
            .ok_or_else(|| format!("View not found: {}", view_id))?;

        if let ViewParams::GraphView(params) = view.params {
            Ok(params.show_edges)
        } else {
            Err("View is not a graph view".to_string())
        }
    }
}
