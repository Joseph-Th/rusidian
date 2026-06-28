//! Service layer: wires concrete implementations and provides a unified API
//! for the Tauri frontend. This is the only place where business-logic coordination
//! should happen. NO business logic in the UI layer.

use pkm_core::note::Note;
use pkm_core::ports::{EntityRepo, LinkRepo, NoteRepo, Retriever, SearchMode, SourceRepo, ViewRepo};
use pkm_core::view::{DefaultViewModel, View, ViewKind, ViewModel, ViewParams};
use pkm_core::{Actor, Timestamp};
use pkm_search::parse_query;
use pkm_storage::{SqliteEntityRepo, SqliteNoteRepo, SqliteRetriever, SqliteSourceRepo, SqliteViewRepo};
use pkm_storage::repositories::SqliteLinkRepo;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use crate::db_pool::{create_pool, DbPool};
use crate::watcher::{watch_vault, NoteWatcherEvent, IgnoreNextEvent};
use tokio::sync::mpsc;
use rusqlite::params;

type GraphNode = (String, String, f64, f64, String, String, String);

// Global ignore handle for file watcher (set when watcher starts)
static WATCHER_IGNORE_HANDLE: OnceLock<IgnoreNextEvent> = OnceLock::new();

/// Get the global watcher ignore handle (if watcher has started).
pub fn get_watcher_ignore_handle() -> Option<IgnoreNextEvent> {
    WATCHER_IGNORE_HANDLE.get().cloned()
}

/// The application service: aggregates all repositories and provides
/// high-level operations for the Tauri frontend. The service manages
/// the database connection pool, vault directory, and creates repository instances on-demand.
pub struct AppService {
    pool: DbPool,
    db_path: PathBuf,
    vault_path: PathBuf,
    /// Bounded sender for ingestion URLs (rate-limited in fetcher)
    pub ingestion_tx: Option<mpsc::Sender<String>>,
}

impl AppService {
    /// Create a new AppService with the given database file and vault directory.
    /// If vault_path is not provided, it defaults to ./vault relative to the database.
    pub fn new(db_path: &str, vault_path: Option<&str>) -> Result<Self, String> {
        let db_path_obj = PathBuf::from(db_path);
        let pool = create_pool(&db_path_obj)?;

        // Determine vault path
        let vault = if let Some(path) = vault_path {
            PathBuf::from(path)
        } else {
            let db_dir = db_path_obj.parent().unwrap_or_else(|| Path::new("."));
            db_dir.join("vault")
        };

        // Create vault directory if it doesn't exist
        std::fs::create_dir_all(&vault).map_err(|e| format!("Failed to create vault dir: {}", e))?;

        // Canonicalize the vault path immediately to ensure consistent, absolute paths
        let vault = std::fs::canonicalize(vault).map_err(|e| format!("Failed to canonicalize vault path: {}", e))?;

        // Start the rate-limited background ingestion worker
        let ingestion_tx = crate::ingestion::start_ingestion_worker(pool.clone(), vault.clone());

        Ok(AppService {
            pool,
            db_path: db_path_obj,
            vault_path: vault,
            ingestion_tx: Some(ingestion_tx),
        })
    }

    /// Start watching the vault directory for external markdown file changes.
    /// When a user edits a file in an external editor (VS Code, Notepad, etc.),
    /// this method detects the change, parses the file, and syncs it to the database.
    ///
    /// The watcher runs in a background thread and continues until the receiver is dropped.
    pub fn start_vault_watcher(&self) -> Result<(), String> {
        let pool = self.pool.clone();
        let vault_path = self.vault_path.clone();

        // Start the file watcher
        let (watcher_rx, ignore_handle) = watch_vault(&vault_path)
            .map_err(|e| format!("Failed to start vault watcher: {}", e))?;

        // Store the ignore handle globally for use when writing files
        let _ = WATCHER_IGNORE_HANDLE.set(ignore_handle);

        // Spawn a background task to process watcher events
        tokio::spawn(async move {
            let mut rx = watcher_rx;
            while let Some(event) = rx.recv().await {
                match event {
                    NoteWatcherEvent::Modified { .. } => {
                        if let Err(e) = Self::sync_external_note(&pool, event) {
                            eprintln!("Failed to sync external note change: {}", e);
                        }
                    }
                    NoteWatcherEvent::Deleted { file_path } => {
                        if let Err(e) = Self::sync_external_note_delete(&pool, &file_path) {
                            eprintln!("Failed to sync external note deletion: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Sync a note from an external file change into the database.
    /// This is called when the file watcher detects a change to a markdown file.
    /// Also handles block ID regeneration: if any block lacked a persistent ID comment,
    /// the markdown file is rewritten to bake in the newly assigned UUIDs.
    fn sync_external_note(
        pool: &DbPool,
        event: NoteWatcherEvent,
    ) -> Result<(), String> {
        let (file_path, note, blocks, needs_rewrite) = match event {
            NoteWatcherEvent::Modified { file_path, note, blocks, needs_rewrite } => (file_path, note, blocks, needs_rewrite),
            _ => return Err("Expected Modified event".to_string()),
        };

        let conn = pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        // Insert or replace the note in the database
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
        let file_path_canonical = std::fs::canonicalize(file_path.clone())
            .unwrap_or_else(|_| file_path.clone());
        let file_path_str = file_path_canonical
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

        // BUG FIX: Fetch old metadata to preserve it before deleting
        let mut stmt = conn
            .prepare("SELECT id, content, created_at, created_by, version, updated_at FROM block WHERE note_id = ?1")
            .map_err(|e| format!("Failed to prepare select block stmt: {}", e))?;
        let existing_blocks: std::collections::HashMap<String, (String, String, String, i64, String)> = stmt
            .query_map(rusqlite::params![note.id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?, 
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?, 
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?
                ))
            })
            .map_err(|e| format!("Failed to query blocks: {}", e))?
            .filter_map(Result::ok)
            .map(|(id, c, ca, cb, v, ua)| (id, (c, ca, cb, v, ua)))
            .collect();

        conn.execute(
            "DELETE FROM block WHERE note_id = ?1",
            rusqlite::params![note.id.to_string()],
        )
        .map_err(|e| format!("Failed to delete old blocks: {}", e))?;

        // Insert the new blocks from the parsed markdown
        for block in &blocks {
            let mut created_at_str = block
                .created_at
                .format(&time::format_description::well_known::Rfc3339)
                .map_err(|e| format!("Failed to format created_at: {}", e))?;
            let mut updated_at_str = block
                .updated_at
                .format(&time::format_description::well_known::Rfc3339)
                .map_err(|e| format!("Failed to format updated_at: {}", e))?;
            let mut created_by_json = serde_json::to_string(&block.created_by)
                .map_err(|e| format!("Failed to serialize actor: {}", e))?;
            let mut version = block.version as i64;
            let content_json = serde_json::to_string(&block.content)
                .map_err(|e| format!("Failed to serialize block content: {}", e))?;

            // Merge metadata
            if let Some((old_content, old_ca, old_cb, old_v, old_ua)) = existing_blocks.get(&block.id.to_string()) {
                created_at_str = old_ca.clone();
                created_by_json = old_cb.clone();
                if &content_json != old_content {
                    version = old_v + 1; // Content changed, increment version. updated_at is `now`
                } else {
                    version = *old_v;
                    updated_at_str = old_ua.clone(); // Keep original updated_at
                }
            }

            conn.execute(
                "INSERT INTO block (id, note_id, block_type, content, \"order\", created_at, created_by, version, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    block.id.to_string(),
                    note.id.to_string(),
                    "markdown",
                    content_json,
                    block.order,
                    created_at_str,
                    created_by_json,
                    version,
                    updated_at_str,
                ],
            )
            .map_err(|e| format!("Failed to insert block: {}", e))?;
        }

        // If blocks had regenerated IDs, rewrite the markdown file to persist them
        if needs_rewrite {
            // Register the upcoming write with the watcher cache to prevent infinite loop
            if let Some(ignore_handle) = get_watcher_ignore_handle() {
                ignore_handle.skip_next(file_path_canonical.clone());
            }
            let markdown_text = pkm_core::markdown::note_to_markdown(&note, &blocks);
            if let Err(e) = std::fs::write(file_path_str, markdown_text) {
                eprintln!("Warning: Failed to persist block IDs to file: {}", e);
            } else {
                println!("✓ Persisted block IDs to {}", file_path.display());
            }
        }

        println!(
            "✓ Synced external note: {} ({} blocks from {})",
            note.title, blocks.len(), file_path.display()
        );

        Ok(())
    }

    fn sync_external_note_delete(
        pool: &DbPool,
        file_path: &Path,
    ) -> Result<(), String> {
        let conn = pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let file_path_canonical = std::fs::canonicalize(file_path)
            .unwrap_or_else(|_| file_path.to_path_buf());
        let file_path_str = file_path_canonical
            .to_str()
            .ok_or_else(|| "Invalid file path".to_string())?;

        let note_id_str: Option<String> = conn.query_row(
            "SELECT id FROM note WHERE file_path = ?1",
            params![file_path_str],
            |row| row.get(0),
        ).ok();

        if let Some(note_id) = note_id_str {
            let note_uuid = uuid::Uuid::parse_str(&note_id)
                .map_err(|_| format!("Invalid note ID in DB: {}", note_id))?;
            let vault_path = file_path_canonical.parent()
                .ok_or_else(|| "Invalid vault path".to_string())?
                .to_path_buf();
            let note_repo = SqliteNoteRepo {
                conn: &conn,
                vault_path,
            };
            note_repo.delete(pkm_core::id::NoteId(note_uuid))
                .map_err(|e| format!("Failed to delete note: {}", e))?;
            println!("[Watcher] Deleted note from DB: {}", note_id);
        }

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

        // Skip the next watcher event for this file to prevent infinite loop
        let file_path = self.vault_path.join(note.file_name());
        if let Some(ignore_handle) = get_watcher_ignore_handle() {
            ignore_handle.skip_next(file_path);
        }

        let conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

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
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let note_repo = SqliteNoteRepo { conn: &conn, vault_path: self.vault_path.clone() };
        let note = note_repo
            .get(parsed_id)
            .map_err(|e| format!("Failed to get note: {}", e))?;

        Ok(note.map(|n| (n.id.to_string(), n.title)))
    }

    /// List all notes with optional limit.
    pub fn list_notes(&self, limit: Option<usize>) -> Result<Vec<(String, String)>, String> {
        let conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

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
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

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
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let note_repo = SqliteNoteRepo { conn: &conn, vault_path: self.vault_path.clone() };
        let mut note = note_repo
            .get(parsed_id)
            .map_err(|e| format!("Failed to get note: {}", e))?
            .ok_or_else(|| format!("Note not found: {}", note_id))?;

        note.title = title;
        note.metadata = metadata;
        note.version += 1;
        note.updated_at = Timestamp::now_utc();

        // Skip the next watcher event for this file to prevent infinite loop
        let file_path = self.vault_path.join(note.file_name());
        if let Some(ignore_handle) = get_watcher_ignore_handle() {
            ignore_handle.skip_next(file_path);
        }

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
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;
 
        // Retrieve file path to ignore in watcher
        let file_path_str: Option<String> = conn.query_row(
            "SELECT file_path FROM note WHERE id = ?1",
            params![note_id],
            |row| row.get(0),
        ).ok();

        if let Some(path_str) = file_path_str {
            if let Some(ignore_handle) = get_watcher_ignore_handle() {
                ignore_handle.skip_next(PathBuf::from(path_str));
            }
        }

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
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let view_repo = SqliteViewRepo { conn: &conn };
        view_repo
            .create(&view)
            .map_err(|e| format!("Failed to create view: {}", e))?;

        Ok(view_id)
    }

    /// List all views with optional limit.
    pub fn list_views(&self, limit: Option<usize>) -> Result<Vec<View>, String> {
        let conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

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
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

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
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

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
            ViewParams::CanvasView(params) => DefaultViewModel::render_canvas_view(params, &sources),
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

        let search_conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;
        let retriever = SqliteRetriever::new(&search_conn);

        let hits = retriever
            .search(&search_query)
            .map_err(|e| format!("Search failed: {}", e))?;

        // Get a fresh connection for note lookups
        let conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

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
                    let conn = self
                        .pool
                        .get()
                        .map_err(|e| format!("Failed to get database connection: {}", e))?;

                    // If positions are stored in params, use them
                    if !params.node_positions.is_empty() {
                        let source_repo = SqliteSourceRepo { conn: &conn };
                        let sources = source_repo
                            .list(None)
                            .map_err(|e| format!("Failed to list sources: {}", e))?;

                        let mut source_map = std::collections::HashMap::new();
                        for source in sources {
                            source_map.insert(source.id.to_string(), source);
                        }

                        let nodes: Vec<_> = params
                            .node_positions
                            .iter()
                            .filter_map(|pos| {
                                source_map.get(&pos.id).map(|source| {
                                    (
                                        pos.id.clone(),
                                        source.title.clone().unwrap_or_else(|| "[untitled]".to_string()),
                                        pos.x,
                                        pos.y,
                                        format!("{:?}", source.origin),
                                        format!("{:?}", source.ingestion_state),
                                        "source".to_string(),
                                    )
                                })
                            })
                            .collect();
                        Ok(Some(nodes))
                    } else {
                        // Generate default positions for stored sources
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
                                (
                                    source.id.to_string(),
                                    source.title.clone().unwrap_or_else(|| "[untitled]".to_string()),
                                    x,
                                    y,
                                    format!("{:?}", source.origin),
                                    format!("{:?}", source.ingestion_state),
                                    "source".to_string(),
                                )
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

    /// Get a preview card for an entity (used for hover tooltips in the frontend).
    pub fn get_preview_card(&self, entity_id: &str) -> Result<crate::commands::PreviewCard, String> {
        let uuid = uuid::Uuid::parse_str(entity_id)
            .map_err(|_| format!("Invalid entity ID: {}", entity_id))?;
        let parsed_id = pkm_core::id::EntityId(uuid);

        let conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let entity_repo = SqliteEntityRepo { conn: &conn };
        let entity = entity_repo
            .get(parsed_id)
            .map_err(|e| format!("Failed to get entity: {}", e))?
            .ok_or_else(|| format!("Entity not found: {}", entity_id))?;

        let aliases_str = if entity.aliases.is_empty() {
            String::new()
        } else {
            format!(" Also known as {}.", entity.aliases.join(", "))
        };

        Ok(crate::commands::PreviewCard {
            id: entity.id.to_string(),
            name: entity.name,
            kind: format!("{:?}", entity.kind).to_lowercase(),
            aliases: entity.aliases,
            summary: format!(
                "Created {} by {:?}. Version {}.{}",
                entity.created_at, entity.created_by, entity.version, aliases_str
            ),
        })
    }

    /// Get a hierarchical network of links starting from a root entity.
    /// Used by Argument Trees (Priority 2) to visualize link relationships.
    pub fn get_link_network(&self, root_entity_id: &str, depth: usize) -> Result<crate::commands::LinkNetworkData, String> {
        use pkm_core::id::ObjectRef;
        use pkm_storage::repositories::SqliteLinkRepo;
        use std::collections::{HashMap, VecDeque};

        let uuid = uuid::Uuid::parse_str(root_entity_id)
            .map_err(|_| format!("Invalid entity ID: {}", root_entity_id))?;
        let root_id = pkm_core::id::EntityId(uuid);

        let conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let entity_repo = SqliteEntityRepo { conn: &conn };
        let link_repo = SqliteLinkRepo { conn: &conn };

        // Verify the root entity exists
        let root = entity_repo
            .get(root_id)
            .map_err(|e| format!("Failed to get root entity: {}", e))?
            .ok_or_else(|| format!("Entity not found: {}", root_entity_id))?;

        let mut nodes = HashMap::new();
        let mut edges = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = VecDeque::new();
        let mut node_depth = HashMap::new();

        let root_ref = ObjectRef::Entity(root_id);
        queue.push_back((root_ref, 0));
        visited.insert(format!("entity:{}", root_id));
        node_depth.insert(format!("entity:{}", root_id), 0);

        // Add root node
        nodes.insert(
            root_entity_id.to_string(),
            crate::commands::LinkNetworkNode {
                id: root_entity_id.to_string(),
                title: root.name.clone(),
                kind: format!("{:?}", root.kind).to_lowercase(),
            },
        );

        // BFS traversal
        while let Some((current_ref, current_depth)) = queue.pop_front() {
            if current_depth >= depth {
                continue;
            }

            let (_from_type, from_id_str) = match &current_ref {
                ObjectRef::Entity(id) => ("entity", id.to_string()),
                ObjectRef::Note(id) => ("note", id.to_string()),
                ObjectRef::Source(id) => ("source", id.to_string()),
                ObjectRef::Block(id) => ("block", id.to_string()),
                _ => continue,
            };

            // Get outgoing links
            if let Ok(outgoing) = link_repo.get_by_from(current_ref) {
                for link in outgoing {
                    let (to_type, to_id) = match link.to {
                        ObjectRef::Entity(id) => ("entity", id.to_string()),
                        ObjectRef::Note(id) => ("note", id.to_string()),
                        ObjectRef::Source(id) => ("source", id.to_string()),
                        ObjectRef::Block(id) => ("block", id.to_string()),
                        _ => continue,
                    };

                    let node_key = format!("{}:{}", to_type, to_id);

                    // Add node if not already visited
                    if !visited.contains(&node_key) {
                        visited.insert(node_key.clone());
                        node_depth.insert(node_key.clone(), current_depth + 1);
                        queue.push_back((link.to, current_depth + 1));

                        // Fetch entity info if it's an entity
                        if to_type == "entity" {
                            if let Ok(entity_uuid) = uuid::Uuid::parse_str(&to_id) {
                                if let Ok(Some(entity)) = entity_repo.get(pkm_core::id::EntityId(entity_uuid)) {
                                    nodes.insert(
                                        to_id.clone(),
                                        crate::commands::LinkNetworkNode {
                                            id: to_id.clone(),
                                            title: entity.name,
                                            kind: format!("{:?}", entity.kind).to_lowercase(),
                                        },
                                    );
                                }
                            }
                        }
                    }

                    // Add edge
                    edges.push(crate::commands::LinkNetworkEdge {
                        id: link.id.to_string(),
                        source: from_id_str.clone(),
                        target: to_id.clone(),
                        link_type: format!("{:?}", link.link_type).to_lowercase(),
                        confidence: link.confidence,
                    });
                }
            }

            // Get incoming links
            if let Ok(incoming) = link_repo.get_by_to(current_ref) {
                for link in incoming {
                    let (from_type, from_id) = match link.from {
                        ObjectRef::Entity(id) => ("entity", id.to_string()),
                        ObjectRef::Note(id) => ("note", id.to_string()),
                        ObjectRef::Source(id) => ("source", id.to_string()),
                        ObjectRef::Block(id) => ("block", id.to_string()),
                        _ => continue,
                    };

                    let node_key = format!("{}:{}", from_type, from_id);

                    // Add node if not already visited
                    if !visited.contains(&node_key) {
                        visited.insert(node_key.clone());
                        node_depth.insert(node_key.clone(), current_depth + 1);
                        queue.push_back((link.from, current_depth + 1));

                        // Fetch entity info if it's an entity
                        if from_type == "entity" {
                            if let Ok(entity_uuid) = uuid::Uuid::parse_str(&from_id) {
                                if let Ok(Some(entity)) = entity_repo.get(pkm_core::id::EntityId(entity_uuid)) {
                                    nodes.insert(
                                        from_id.clone(),
                                        crate::commands::LinkNetworkNode {
                                            id: from_id.clone(),
                                            title: entity.name,
                                            kind: format!("{:?}", entity.kind).to_lowercase(),
                                        },
                                    );
                                }
                            }
                        }
                    }

                    // Add edge
                    edges.push(crate::commands::LinkNetworkEdge {
                        id: link.id.to_string(),
                        source: from_id,
                        target: from_id_str.clone(),
                        link_type: format!("{:?}", link.link_type).to_lowercase(),
                        confidence: link.confidence,
                    });
                }
            }
        }

        Ok(crate::commands::LinkNetworkData {
            nodes: nodes.into_values().collect(),
            edges,
        })
    }

    /// Get canvas view data with resolved node content.
    /// Returns the canvas layout with nodes enriched with titles and metadata from the database.
    pub fn get_canvas_view_data(&self, view_id: &str) -> Result<Option<crate::commands::CanvasViewRenderData>, String> {
        let view = self.get_view(view_id)?;

        match view {
            Some(view) => {
                if let ViewParams::CanvasView(params) = &view.params {
                    let conn = self
                        .pool
                        .get()
                        .map_err(|e| format!("Failed to get database connection: {}", e))?;

                    let source_repo = SqliteSourceRepo { conn: &conn };
                    let sources = source_repo
                        .list(None)
                        .map_err(|e| format!("Failed to list sources: {}", e))?;

                    let mut source_map = std::collections::HashMap::new();
                    for source in sources {
                        source_map.insert(source.id.to_string(), source);
                    }

                    let limit = params.limit.unwrap_or(500);

                    // Resolve canvas nodes to include content metadata
                    let mut nodes = Vec::new();
                    for node in params.nodes.iter().take(limit) {
                        match &node.target {
                            pkm_core::id::ObjectRef::Source(source_id) => {
                                if let Some(source) = source_map.get(&source_id.to_string()) {
                                    nodes.push(crate::commands::CanvasNodeData {
                                        id: source.id.to_string(),
                                        title: source.title.clone().unwrap_or_else(|| "[untitled]".to_string()),
                                        x: node.x,
                                        y: node.y,
                                        width: node.width,
                                        height: node.height,
                                        color_theme: node.color_theme.clone(),
                                        kind: "source".to_string(),
                                    });
                                }
                            }
                            _ => {
                                // For now, only support Source nodes
                                // In the future, extend to support Note, Entity, Block, Media
                            }
                        }
                    }

                    // Copy frames as-is
                    let frames = params.frames.iter().map(|f| {
                        crate::commands::CanvasFrameData {
                            id: f.id.clone(),
                            label: f.label.clone(),
                            x: f.x,
                            y: f.y,
                            width: f.width,
                            height: f.height,
                            background_color: f.background_color.clone(),
                        }
                    }).collect();

                    Ok(Some(crate::commands::CanvasViewRenderData {
                        title: view.title,
                        nodes,
                        frames,
                    }))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Get hierarchical timeline data grouped by year/month/week/day.
    /// Used for Gantt charts and chronological visualizations.
    pub fn get_timeline_view_data(&self, view_id: &str) -> Result<Option<crate::commands::TimelineRenderData>, String> {
        use std::collections::BTreeMap;

        let view = self.get_view(view_id)?;

        match view {
            Some(view) => {
                if let ViewParams::Timeline(params) = &view.params {
                    let conn = self
                        .pool
                        .get()
                        .map_err(|e| format!("Failed to get database connection: {}", e))?;

                    let source_repo = SqliteSourceRepo { conn: &conn };
                    let mut sources = source_repo
                        .list(None)
                        .map_err(|e| format!("Failed to list sources: {}", e))?;

                    // Sort by captured_at; direction depends on reverse_chronological flag
                    if params.reverse_chronological {
                        sources.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));
                    } else {
                        sources.sort_by(|a, b| a.captured_at.cmp(&b.captured_at));
                    }

                    let limit = params.limit.unwrap_or(100);

                    // Group by TimelineGrouping
                    let mut grouped: BTreeMap<String, BTreeMap<String, Vec<crate::commands::TimelineEventData>>> = BTreeMap::new();

                    for source in sources.iter().take(limit) {
                        let date_str = source.captured_at.to_string();

                        // Parse date for grouping (expect RFC3339 format: YYYY-MM-DDTHH:MM:SS...)
                        let year = date_str.split('-').next().unwrap_or("unknown").to_string();

                        // Extract month-day safely; RFC3339 is at least "YYYY-MM-DD" (10 chars)
                        let month_key = if date_str.len() >= 7 {
                            match params.grouping {
                                pkm_core::view::TimelineGrouping::Day => {
                                    if date_str.len() >= 10 {
                                        date_str[0..10].to_string()
                                    } else {
                                        date_str[0..7].to_string()
                                    }
                                }
                                pkm_core::view::TimelineGrouping::Week |
                                pkm_core::view::TimelineGrouping::Month => date_str[0..7].to_string(),
                                pkm_core::view::TimelineGrouping::Year => year.clone(),
                            }
                        } else {
                            "unknown".to_string()
                        };

                        let event = crate::commands::TimelineEventData {
                            id: source.id.to_string(),
                            title: source.title.clone().unwrap_or_else(|| "[untitled]".to_string()),
                            date: date_str,
                        };

                        grouped
                            .entry(year)
                            .or_insert_with(BTreeMap::new)
                            .entry(month_key)
                            .or_insert_with(Vec::new)
                            .push(event);
                    }

                    Ok(Some(crate::commands::TimelineRenderData {
                        title: view.title,
                        events: grouped,
                    }))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Compute a 2D semantic layout for a graph view using embeddings and PCA.
    /// Returns node positions optimized for conceptual clustering rather than explicit links.
    pub fn get_semantic_layout(&self, view_id: &str) -> Result<Option<Vec<(String, f64, f64)>>, String> {
        use pkm_storage::embeddings;

        let view = self.get_view(view_id)?;

        match view {
            Some(view) => {
                if let ViewParams::GraphView(_params) = &view.params {
                    let conn = self
                        .pool
                        .get()
                        .map_err(|e| format!("Failed to get database connection: {}", e))?;

                    // Get embeddings for all sources
                    let embeddings = embeddings::get_embeddings_by_type(&conn, "source")
                        .map_err(|e| format!("Failed to get embeddings: {}", e))?;

                    if embeddings.is_empty() {
                        return Ok(None);
                    }

                    // Compute 2D layout via PCA
                    let layout = embeddings::compute_2d_layout(&embeddings);

                    // Convert to tuple format matching GraphNode
                    let nodes: Vec<_> = layout
                        .into_iter()
                        .map(|(id, (x, y))| (id, x, y))
                        .collect();

                    Ok(Some(nodes))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Immediate neighbors of a node (depth 1) for progressive disclosure graphs.
    /// Used by Progressive Graphs (Priority 3) to expand nodes on double-click.
    pub fn get_neighbors(&self, target_id: &str, _depth: usize) -> Result<crate::commands::LinkNetworkData, String> {
        use pkm_core::id::ObjectRef;
        use pkm_storage::repositories::SqliteLinkRepo;

        let uuid = uuid::Uuid::parse_str(target_id)
            .map_err(|_| format!("Invalid ID: {}", target_id))?;

        let conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let entity_repo = SqliteEntityRepo { conn: &conn };
        let link_repo = SqliteLinkRepo { conn: &conn };

        let mut nodes = std::collections::HashMap::new();
        let mut edges = Vec::new();

        // Try to parse as entity first
        let target_ref = if let Ok(entity) = entity_repo.get(pkm_core::id::EntityId(uuid)) {
            if let Some(ent) = entity {
                nodes.insert(
                    target_id.to_string(),
                    crate::commands::LinkNetworkNode {
                        id: target_id.to_string(),
                        title: ent.name,
                        kind: format!("{:?}", ent.kind).to_lowercase(),
                    },
                );
                ObjectRef::Entity(pkm_core::id::EntityId(uuid))
            } else {
                return Err(format!("Entity not found: {}", target_id));
            }
        } else {
            return Err(format!("Invalid entity ID: {}", target_id));
        };

        // Get outgoing neighbors (up to depth levels)
        if let Ok(outgoing) = link_repo.get_by_from(target_ref) {
            for link in outgoing.iter().take(20) {
                let (to_type, to_id) = match link.to {
                    ObjectRef::Entity(id) => ("entity", id.to_string()),
                    ObjectRef::Note(id) => ("note", id.to_string()),
                    ObjectRef::Source(id) => ("source", id.to_string()),
                    ObjectRef::Block(id) => ("block", id.to_string()),
                    _ => continue,
                };

                // Fetch entity info if it's an entity
                if to_type == "entity" {
                    if let Ok(entity_uuid) = uuid::Uuid::parse_str(&to_id) {
                        if let Ok(Some(entity)) = entity_repo.get(pkm_core::id::EntityId(entity_uuid)) {
                            nodes.insert(
                                to_id.clone(),
                                crate::commands::LinkNetworkNode {
                                    id: to_id.clone(),
                                    title: entity.name,
                                    kind: format!("{:?}", entity.kind).to_lowercase(),
                                },
                            );
                        }
                    }
                }

                edges.push(crate::commands::LinkNetworkEdge {
                    id: link.id.to_string(),
                    source: target_id.to_string(),
                    target: to_id,
                    link_type: format!("{:?}", link.link_type).to_lowercase(),
                    confidence: link.confidence,
                });
            }
        }

        // Get incoming neighbors (up to depth levels)
        if let Ok(incoming) = link_repo.get_by_to(target_ref) {
            for link in incoming.iter().take(20) {
                let (from_type, from_id) = match link.from {
                    ObjectRef::Entity(id) => ("entity", id.to_string()),
                    ObjectRef::Note(id) => ("note", id.to_string()),
                    ObjectRef::Source(id) => ("source", id.to_string()),
                    ObjectRef::Block(id) => ("block", id.to_string()),
                    _ => continue,
                };

                // Fetch entity info if it's an entity
                if from_type == "entity" {
                    if let Ok(entity_uuid) = uuid::Uuid::parse_str(&from_id) {
                        if let Ok(Some(entity)) = entity_repo.get(pkm_core::id::EntityId(entity_uuid)) {
                            nodes.insert(
                                from_id.clone(),
                                crate::commands::LinkNetworkNode {
                                    id: from_id.clone(),
                                    title: entity.name,
                                    kind: format!("{:?}", entity.kind).to_lowercase(),
                                },
                            );
                        }
                    }
                }

                edges.push(crate::commands::LinkNetworkEdge {
                    id: link.id.to_string(),
                    source: from_id,
                    target: target_id.to_string(),
                    link_type: format!("{:?}", link.link_type).to_lowercase(),
                    confidence: link.confidence,
                });
            }
        }

        Ok(crate::commands::LinkNetworkData {
            nodes: nodes.into_values().collect(),
            edges,
        })
    }

    /// Retrieve the complete provenance chain for a block or entity.
    /// Shows the full derivation history: how this content traces back to original sources.
    /// Returns a chain from the root block back through all DerivedFrom links to the original source.
    pub fn get_provenance_chain(&self, _block_id: &str) -> Result<crate::commands::ProvenanceChainData, String> {
        Err("Provenance chain traversal (recursive DerivedFrom links) is not yet implemented. \
             Full implementation requires recursive CTE queries to traverse the link graph. \
             See task D2 in STATUS.md.".to_string())
    }

    /// Get a matrix of entities showing relationships between two entity kinds.
    /// Returns all entities of row_kind as rows and col_kind as columns, with links as cells.
    /// Useful for "Organizations vs Products" or "People vs Projects" comparisons.
    pub fn get_entity_matrix(
        &self,
        _row_kind: &str,
        _col_kind: &str,
        _min_confidence: Option<f32>,
    ) -> Result<crate::commands::EntityMatrixData, String> {
        Err("Entity matrix queries are not yet implemented. \
             Full implementation requires querying entities by kind and finding direct links between them. \
             See task D2 in STATUS.md.".to_string())
    }

    /// Handle an AI action by executing it and immediately applying it to the database.
    /// This bypasses the human review step in the "Waiting Room" pattern.
    /// All changes are still recorded in the agent_action audit log for potential rollback.
    pub fn handle_ai_action(&self, operation: pkm_agent::Operation) -> Result<String, String> {
        let conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let action_repo = pkm_storage::SqliteAgentActionRepo { conn: &conn };
        let note_repo = SqliteNoteRepo { conn: &conn, vault_path: self.vault_path.clone() };
        let link_repo = SqliteLinkRepo { conn: &conn };

        let req = pkm_agent::OperationRequest {
            actor: Actor::Agent { name: "Auto-Agent".to_string() },
            operation,
            rationale: "Automated AI task".to_string(),
        };

        // Step 1: Generate the audit log entry (execute)
        let action = pkm_agent::execute(req, &action_repo, &note_repo)
            .map_err(|e| format!("Failed to execute operation: {}", e))?;

        // Step 2: Immediately apply it (bypassing human review)
        pkm_agent::apply_action(
            action.id,
            &action_repo,
            &note_repo,
            Some(&link_repo)
        ).map_err(|e| format!("Failed to apply action: {}", e))?;

        Ok(action.id.to_string())
    }

    /// Ingest bulk links from raw pasted text.
    /// Extracts URLs, queues them for rate-limited background processing.
    /// Returns immediately with the count of URLs found.
    pub fn ingest_bulk_links(&self, raw_text: String) -> Result<usize, String> {
        // Extract all URLs from the pasted text
        let urls = crate::ingestion::extract_urls(&raw_text);
        let count = urls.len();

        if count == 0 {
            return Ok(0);
        }

        // Queue each URL individually to the bounded fetcher queue
        if let Some(ref tx) = self.ingestion_tx {
            for url in urls {
                // Ignore send errors if channel is closed (shouldn't happen)
                let _ = tx.blocking_send(url);
            }
        } else {
            return Err("Ingestion worker not available".to_string());
        }

        Ok(count)
    }

    /// Rollback all autonomous ingestion actions from the past N minutes.
    /// This is the "nuclear undo" for when bulk ingestion produces bad data.
    pub fn rollback_recent_autonomous_ingestion(
        &self,
        minutes: i64,
    ) -> Result<usize, String> {
        let conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        // Find all actions by Autonomous-Ingestor in the last X minutes
        let query = "
            SELECT id FROM agent_action
            WHERE actor LIKE ?1
              AND created_at > datetime('now', ?2 || ' minutes')
            ORDER BY created_at DESC
        ";

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let action_ids: Vec<String> = stmt
            .query_map(rusqlite::params!["%Autonomous-Ingestor%", format!("-{}", minutes)], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to query actions: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect action IDs: {}", e))?;

        let count = action_ids.len();

        // Rollback each action
        let action_repo = pkm_storage::SqliteAgentActionRepo { conn: &conn };
        let note_repo = SqliteNoteRepo { conn: &conn, vault_path: self.vault_path.clone() };

        for id_str in action_ids {
            if let Ok(uuid) = uuid::Uuid::parse_str(&id_str) {
                let action_id = pkm_core::id::AgentActionId(uuid);
                let _ = pkm_agent::rollback_action(
                    action_id,
                    &action_repo,
                    &note_repo,
                    None,
                    None,
                );
            }
        }

        Ok(count)
    }
}
