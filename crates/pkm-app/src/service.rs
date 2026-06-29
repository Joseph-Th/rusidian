use pkm_core::note::{Note, NoteMetadata};
use pkm_core::ports::{NoteRepo, Retriever, SearchMode, SourceRepo, ViewRepo};
use pkm_core::view::{DefaultViewModel, View, ViewKind, ViewModel, ViewParams};
use pkm_core::{Actor, Timestamp};
use pkm_search::parse_query;
use pkm_fs::{SharedVault, FsNoteRepo, FsSourceRepo, FsViewRepo, FsRetriever, load_vault};
use std::path::PathBuf;
use std::sync::OnceLock;
use crate::watcher::{IgnoreNextEvent};
use tokio::sync::mpsc;

static WATCHER_IGNORE_HANDLE: OnceLock<IgnoreNextEvent> = OnceLock::new();

pub fn get_watcher_ignore_handle() -> Option<IgnoreNextEvent> {
    WATCHER_IGNORE_HANDLE.get().cloned()
}

pub struct AppService {
    pub vault: SharedVault,
    pub vault_path: PathBuf,
    pub ingestion_tx: Option<mpsc::UnboundedSender<String>>,
}

impl AppService {
    pub fn new(vault_path: &str) -> Result<Self, String> {
        let vault_dir = PathBuf::from(vault_path);

        std::fs::create_dir_all(&vault_dir).map_err(|e| format!("Failed to create vault dir: {}", e))?;
        let vault_dir = std::fs::canonicalize(vault_dir).map_err(|e| format!("Failed to canonicalize vault path: {}", e))?;

        let vault = load_vault(&vault_dir);

        let ingestion_tx = crate::ingestion::start_ingestion_worker(vault.clone(), vault_dir.clone());

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

        let note_id = note.id.to_string();
        let file_path = self.vault_path.join("notes").join(note.file_name());
        if let Some(ignore_handle) = get_watcher_ignore_handle() {
            ignore_handle.skip_next(file_path);
        }

        let note_repo = FsNoteRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        note_repo
            .create(&note)
            .map_err(|e| format!("Failed to create note: {}", e))?;

        Ok(note_id)
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
        if let Some(ignore_handle) = get_watcher_ignore_handle() {
            ignore_handle.skip_next(file_path);
        }

        note_repo.update(&note).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_note(&self, note_id: &str) -> Result<(), String> {
        let uuid = uuid::Uuid::parse_str(note_id).map_err(|_| format!("Invalid note ID: {}", note_id))?;
        let parsed_id = pkm_core::id::NoteId(uuid);

        let note_repo = FsNoteRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };

        if let Ok(Some(note)) = note_repo.get(parsed_id) {
            let file_path = self.vault_path.join("notes").join(note.file_name());
            if let Some(ignore_handle) = get_watcher_ignore_handle() {
                ignore_handle.skip_next(file_path);
            }
        }

        note_repo.delete(parsed_id).map_err(|e| e.to_string())?;
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

        let source_repo = FsSourceRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        let sources = source_repo.list(None).map_err(|e| e.to_string())?;

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

        Ok(result.source_ids.iter().map(|id| id.to_string()).collect())
    }

    pub fn search_notes(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<(String, String)>, String> {
        use pkm_core::id::ObjectRef;

        let search_query = parse_query(SearchMode::FuzzyText, query);
        let retriever = FsRetriever { state: self.vault.clone() };

        let hits = retriever
            .search(&search_query)
            .map_err(|e| format!("Search failed: {}", e))?;

        let note_repo = FsNoteRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
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

    pub fn ingest_bulk_links(&self, raw_text: String) -> Result<usize, String> {
        let urls = crate::ingestion::extract_urls(&raw_text);
        let count = urls.len();

        if count == 0 {
            return Ok(0);
        }

        if let Some(ref tx) = self.ingestion_tx {
            for url in urls {
                let _ = tx.send(url);
            }
        } else {
            return Err("Ingestion worker not available".to_string());
        }

        Ok(count)
    }
}
