use pkm_core::note::{Note, NoteMetadata};
use pkm_core::ports::{NoteRepo, Retriever, SearchMode, ViewRepo};
use pkm_core::view::{DefaultViewModel, View, ViewKind, ViewModel, ViewParams};
use pkm_core::{Actor, Timestamp};
use pkm_search::parse_query;
use pkm_fs::{SharedVault, FsNoteRepo, FsViewRepo, FsRetriever, load_vault, persist_metadata};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use crate::watcher::IgnoreNextEvent;
use crate::ingestion::IngestPayload;
use tokio::sync::mpsc;

static WATCHER_IGNORE_HANDLE: OnceLock<IgnoreNextEvent> = OnceLock::new();

pub fn get_watcher_ignore_handle() -> Option<IgnoreNextEvent> {
    WATCHER_IGNORE_HANDLE.get().cloned()
}

pub struct AppService {
    pub vault: SharedVault,
    pub vault_path: PathBuf,
    pub ingestion_tx: Option<mpsc::UnboundedSender<IngestPayload>>,
}

impl AppService {
    pub fn new(vault_path: &str) -> Result<Self, String> {
        let vault_dir = PathBuf::from(vault_path);

        std::fs::create_dir_all(&vault_dir).map_err(|e| format!("Failed to create vault dir: {}", e))?;
        let vault_dir = std::fs::canonicalize(vault_dir).map_err(|e| format!("Failed to canonicalize vault path: {}", e))?;

        let vault = load_vault(&vault_dir);

        let ingestion_tx = crate::ingestion::start_ingestion_worker(vault.clone(), vault_dir.clone());

        // Background metadata flush: batches disk writes every 5 seconds
        let vault_flush = vault.clone();
        let vault_path_flush = vault_dir.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let data = {
                    let state = vault_flush.read().unwrap();
                    if state.take_dirty() {
                        Some(state.extract_save_data())
                    } else {
                        None
                    }
                };
                if let Some(data) = data {
                    if let Err(e) = persist_metadata(&vault_path_flush, &data) {
                        eprintln!("Failed to persist metadata: {}", e);
                        vault_flush.read().unwrap().mark_dirty();
                    }
                }
            }
        });

        Ok(AppService {
            vault,
            vault_path: vault_dir,
            ingestion_tx: Some(ingestion_tx),
        })
    }

    pub fn create_note(&self, title: String) -> Result<String, String> {
        let now = Timestamp::now_utc();
        let note = Note {
            id: pkm_core::id::NoteId::new(),
            title,
            blocks: vec![],
            metadata: NoteMetadata::default(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let file_path = self.vault_path.join("notes").join(note.file_name());

        let note_repo = FsNoteRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        note_repo
            .create(&note)
            .map_err(|e| format!("Failed to create note: {}", e))?;

        // Skip next AFTER the write completes to avoid race with slow filesystems
        if let Some(ignore_handle) = get_watcher_ignore_handle() {
            ignore_handle.skip_next(file_path);
        }

        Ok(note.id.to_string())
    }

    pub fn get_note(&self, note_id: &str) -> Result<Option<(String, String)>, String> {
        let uuid = uuid::Uuid::parse_str(note_id).map_err(|_| format!("Invalid note ID: {}", note_id))?;
        let parsed_id = pkm_core::id::NoteId(uuid);

        let note_repo = FsNoteRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        let note = note_repo.get(parsed_id).map_err(|e| e.to_string())?;
        Ok(note.map(|n| (n.id.to_string(), n.title)))
    }

    pub fn list_notes(&self, limit: Option<usize>) -> Result<Vec<(String, String)>, String> {
        let note_repo = FsNoteRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        let notes = note_repo.list(limit).map_err(|e| e.to_string())?;
        Ok(notes.into_iter().map(|n| (n.id.to_string(), n.title)).collect())
    }

    pub fn get_note_full(&self, note_id: &str) -> Result<Option<Note>, String> {
        let uuid = uuid::Uuid::parse_str(note_id).map_err(|_| format!("Invalid note ID: {}", note_id))?;
        let parsed_id = pkm_core::id::NoteId(uuid);

        let note_repo = FsNoteRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        let note = note_repo.get(parsed_id).map_err(|e| e.to_string())?;
        Ok(note)
    }

    pub fn update_note(
        &self,
        note_id: &str,
        title: String,
        metadata: NoteMetadata,
    ) -> Result<(), String> {
        let uuid = uuid::Uuid::parse_str(note_id).map_err(|_| format!("Invalid note ID: {}", note_id))?;
        let parsed_id = pkm_core::id::NoteId(uuid);

        let note_repo = FsNoteRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        let mut note = note_repo
            .get(parsed_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Note not found: {}", note_id))?;

        note.title = title;
        note.metadata = metadata;
        note.version += 1;
        note.updated_at = Timestamp::now_utc();

        let file_path = self.vault_path.join("notes").join(note.file_name());

        note_repo.update(&note).map_err(|e| e.to_string())?;

        // Skip next AFTER the write completes to avoid race with slow filesystems
        if let Some(ignore_handle) = get_watcher_ignore_handle() {
            ignore_handle.skip_next(file_path);
        }
        Ok(())
    }

    pub fn delete_note(&self, note_id: &str) -> Result<(), String> {
        let uuid = uuid::Uuid::parse_str(note_id).map_err(|_| format!("Invalid note ID: {}", note_id))?;
        let parsed_id = pkm_core::id::NoteId(uuid);

        let note_repo = FsNoteRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };

        let file_path = if let Ok(Some(note)) = note_repo.get(parsed_id) {
            Some(self.vault_path.join("notes").join(note.file_name()))
        } else {
            None
        };

        note_repo.delete(parsed_id).map_err(|e| e.to_string())?;

        // Skip next AFTER the delete completes to avoid race with slow filesystems
        if let Some(fp) = file_path {
            if let Some(ignore_handle) = get_watcher_ignore_handle() {
                ignore_handle.skip_next(fp);
            }
        }
        Ok(())
    }

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
        let view_repo = FsViewRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        view_repo.create(&view).map_err(|e| e.to_string())?;
        Ok(view_id)
    }

    pub fn list_views(&self, limit: Option<usize>) -> Result<Vec<View>, String> {
        let view_repo = FsViewRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        view_repo.list(limit).map_err(|e| e.to_string())
    }

    pub fn get_view(&self, view_id: &str) -> Result<Option<View>, String> {
        let uuid = uuid::Uuid::parse_str(view_id).map_err(|_| format!("Invalid view ID: {}", view_id))?;
        let parsed_id = pkm_core::id::ViewId(uuid);

        let view_repo = FsViewRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        view_repo.get(parsed_id).map_err(|e| e.to_string())
    }

    pub fn render_view(&self, view_id: &str) -> Result<Vec<String>, String> {
        let view = self
            .get_view(view_id)?
            .ok_or_else(|| format!("View not found: {}", view_id))?;

        // Hold the read lock during the render operation — the lock duration
        // is brief (just iterating and filtering IDs), and it avoids cloning
        // all Source values into a new Vec.
        let state = self.vault.read().unwrap();
        let sources: Vec<&pkm_core::source::Source> = state.sources().values().collect();

        let result = match &view.params {
            ViewParams::ReadingQueue(params) => {
                DefaultViewModel::render_reading_queue(params, &sources)
            }
            ViewParams::ReviewQueue(params) => {
                DefaultViewModel::render_review_queue(params, &sources)
            }
            ViewParams::Timeline(params) => DefaultViewModel::render_timeline(params, &sources),
            ViewParams::GraphView(params) => DefaultViewModel::render_graph_view(params, &sources),
            ViewParams::CanvasView(params) => DefaultViewModel::render_canvas_view(params, &sources),
        }
        .map_err(|e| format!("Failed to render view: {}", e))?;

        drop(state);
        Ok(result.object_refs.iter().map(|r| r.to_string()).collect())
    }

    pub fn search_notes(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<(String, String, String)>, String> {
        use pkm_core::id::ObjectRef;

        let search_query = parse_query(SearchMode::FuzzyText, query);
        let retriever = FsRetriever { state: self.vault.clone() };

        let hits = retriever
            .search(&search_query)
            .map_err(|e| format!("Search failed: {}", e))?;

        let note_repo = FsNoteRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        let mut results = Vec::new();

        for hit in hits.iter().take(limit.unwrap_or(50)) {
            let (object_type, id) = match hit.object {
                ObjectRef::Note(id) => ("note", id.0.to_string()),
                ObjectRef::Block(id) => ("block", id.0.to_string()),
                ObjectRef::Source(id) => ("source", id.0.to_string()),
                ObjectRef::Entity(id) => ("entity", id.0.to_string()),
                ObjectRef::Link(id) => ("link", id.0.to_string()),
                ObjectRef::View(id) => ("view", id.0.to_string()),
            };

            match hit.object {
                ObjectRef::Note(note_id) => {
                    if let Ok(Some(note)) = note_repo.get(note_id) {
                        results.push((id, object_type.to_string(), note.title));
                    } else if let Some(snippet) = &hit.snippet {
                        results.push((id, object_type.to_string(), snippet.clone()));
                    }
                }
                ObjectRef::Block(block_id) => {
                    if let Ok(Some(note_id)) = note_repo.get_note_id_for_block(block_id) {
                        if let Ok(Some(note)) = note_repo.get(note_id) {
                            let snippet = hit.snippet.clone().unwrap_or_default();
                            results.push((id, object_type.to_string(), format!("{} - {}", note.title, snippet)));
                        }
                    }
                }
                _ => {
                    if let Some(snippet) = &hit.snippet {
                        results.push((id, object_type.to_string(), snippet.clone()));
                    }
                }
            }
        }

        Ok(results)
    }

    pub fn start_vault_watcher(self: &Arc<Self>) -> Result<(), String> {
        let (mut rx, ignore_handle) = crate::watcher::watch_vault(&self.vault_path, self.vault.clone())
            .map_err(|e| e.to_string())?;

        WATCHER_IGNORE_HANDLE.set(ignore_handle).ok();

        let service = self.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    crate::watcher::NoteWatcherEvent::Modified { ref note, ref blocks, ref file_path } => {
                        let service_clone = service.clone();
                        let note = note.clone();
                        let blocks = blocks.clone();
                        let file_path = file_path.clone();
                        tokio::task::spawn_blocking(move || {
                            let repo = FsNoteRepo {
                                state: service_clone.vault.clone(),
                                vault_path: service_clone.vault_path.clone(),
                            };
                            let _ = repo.upsert_from_external(&note, &blocks, &file_path);
                            // Skip next AFTER the write completes to avoid race with slow disks
                            if let Some(handle) = crate::service::get_watcher_ignore_handle() {
                                handle.skip_next(file_path);
                            }
                        });
                    }
                    crate::watcher::NoteWatcherEvent::Deleted { file_path } => {
                        let service_clone = service.clone();
                        tokio::spawn(async move {
                            // Wait briefly to allow a coupled 'Create' (rename) event to process
                            tokio::time::sleep(std::time::Duration::from_millis(250)).await;

                            tokio::task::spawn_blocking(move || {
                                let repo = FsNoteRepo {
                                    state: service_clone.vault.clone(),
                                    vault_path: service_clone.vault_path.clone(),
                                };
                                let filename = match file_path.file_name().and_then(|s| s.to_str()) {
                                    Some(f) => f.to_string(),
                                    None => return,
                                };

                                // If NO note in memory maps to this filename anymore, it's safe to delete.
                                // (If it was renamed, the note in memory already has a new title/filename).
                                let state = service_clone.vault.read().unwrap();
                                let actual_note: Option<pkm_core::note::Note> = state.notes().values().find(|n| n.file_name() == filename).cloned();
                                drop(state);

                                if let Some(note) = actual_note {
                                    // Double check the file is actually gone from disk
                                    if !file_path.exists() {
                                        let _ = repo.delete(note.id);
                                    }
                                }
                            });
                        });
                    }
                }
            }
        });
        Ok(())
    }

    /// Force a final synchronous flush of metadata to disk.
    /// Called during graceful shutdown to prevent data loss.
    pub fn flush_metadata(&self) -> Result<(), String> {
        let state = self.vault.read().unwrap();
        if state.take_dirty() {
            let data = state.extract_save_data();
            drop(state);
            persist_metadata(&self.vault_path, &data).map_err(|e| format!("Flush failed: {}", e))
        } else {
            Ok(())
        }
    }

    pub fn ingest_bulk_links(&self, raw_text: String) -> Result<usize, String> {
        let urls = crate::ingestion::extract_urls(&raw_text);
        let count = urls.len();

        if count == 0 {
            return Ok(0);
        }

        if let Some(ref tx) = self.ingestion_tx {
            for url in urls {
                let _ = tx.send(IngestPayload { url, retries: 0 });
            }
        } else {
            return Err("Ingestion worker not available".to_string());
        }

        Ok(count)
    }
}
