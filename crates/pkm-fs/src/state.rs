use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::Path;
use std::fs;
use pkm_core::id::{NoteId, BlockId, EntityId, LinkId, ObjectRef, SourceId, ViewId};
use pkm_core::{note::Note, block::Block, entity::Entity, link::Link, source::Source, view::View};
use serde::Deserialize;

/// In-memory vault state.
///
/// All data fields are `pub(crate)` — only `pkm-fs` internals (repositories,
/// retriever, loader) may mutate them directly. External code MUST go through
/// the repository traits defined in `pkm_core::ports`.
pub struct VaultState {
    pub(crate) notes: HashMap<NoteId, Note>,
    pub(crate) blocks: HashMap<BlockId, Block>,
    pub(crate) sources: HashMap<SourceId, Source>,
    pub(crate) entities: HashMap<EntityId, Entity>,
    pub(crate) links: HashMap<LinkId, Link>,
    pub(crate) views: HashMap<ViewId, View>,
    pub(crate) ingestion_queue: Vec<IngestionItem>,

    pub(crate) links_by_source: HashMap<ObjectRef, Vec<LinkId>>,
    pub(crate) links_by_target: HashMap<ObjectRef, Vec<LinkId>>,

    pub(crate) dirty: AtomicBool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IngestionItem {
    pub id: i64,
    pub url: String,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
    pub processed_at: Option<String>,
}

impl VaultState {
    pub fn new() -> Self {
        Self {
            notes: HashMap::new(),
            blocks: HashMap::new(),
            sources: HashMap::new(),
            entities: HashMap::new(),
            links: HashMap::new(),
            views: HashMap::new(),
            ingestion_queue: Vec::new(),
            links_by_source: HashMap::new(),
            links_by_target: HashMap::new(),
            dirty: AtomicBool::new(false),
        }
    }

    /// Clone data needed for metadata persistence (must be called under lock).
    pub fn extract_save_data(&self) -> MetadataSaveData {
        MetadataSaveData {
            entities: self.entities.clone(),
            links: self.links.clone(),
            views: self.views.clone(),
            sources: self.sources.clone(),
            ingestion_queue: self.ingestion_queue.clone(),
        }
    }

    pub fn rebuild_indexes(&mut self) {
        self.links_by_source.clear();
        self.links_by_target.clear();

        for link in self.links.values() {
            self.links_by_source
                .entry(link.from)
                .or_default()
                .push(link.id);
            self.links_by_target
                .entry(link.to)
                .or_default()
                .push(link.id);
        }
    }

    pub fn index_link_add(&mut self, link: &pkm_core::link::Link) {
        self.links_by_source
            .entry(link.from)
            .or_default()
            .push(link.id);
        self.links_by_target
            .entry(link.to)
            .or_default()
            .push(link.id);
    }

    pub fn index_link_remove(&mut self, link: &pkm_core::link::Link) {
        if let Some(vec) = self.links_by_source.get_mut(&link.from) {
            vec.retain(|&id| id != link.id);
        }
        if let Some(vec) = self.links_by_target.get_mut(&link.to) {
            vec.retain(|&id| id != link.id);
        }
    }

    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Release);
    }

    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::AcqRel)
    }

    // ── Public read-only accessors ──────────────────────────────────────
    // External code reads through these; mutation goes through repository traits.

    pub fn notes(&self) -> &HashMap<NoteId, Note> { &self.notes }
    pub fn blocks(&self) -> &HashMap<BlockId, Block> { &self.blocks }
    pub fn sources(&self) -> &HashMap<SourceId, Source> { &self.sources }
    pub fn entities(&self) -> &HashMap<EntityId, Entity> { &self.entities }
    pub fn links(&self) -> &HashMap<LinkId, Link> { &self.links }
    pub fn views(&self) -> &HashMap<ViewId, View> { &self.views }
}

impl Default for VaultState {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of metadata needed for persistence. Cloned under lock, written outside.
pub struct MetadataSaveData {
    pub entities: HashMap<EntityId, pkm_core::entity::Entity>,
    pub links: HashMap<LinkId, Link>,
    pub views: HashMap<ViewId, pkm_core::view::View>,
    pub sources: HashMap<SourceId, Source>,
    pub ingestion_queue: Vec<IngestionItem>,
}

/// Write JSON to a temporary file first, then atomically rename to prevent
/// corruption on crash (avoids truncate-then-write race).
fn write_json_atomic<T: serde::Serialize>(path: &Path, data: &T) -> std::io::Result<()> {
    let tmp_path = path.with_extension("tmp");
    let file = std::fs::File::create(&tmp_path)?;
    serde_json::to_writer_pretty(&file, data)?;
    file.sync_all()?;
    std::fs::rename(tmp_path, path)?;
    Ok(())
}

/// Write metadata files to disk. No lock should be held when calling this.
pub fn persist_metadata(vault_path: &Path, data: &MetadataSaveData) -> std::io::Result<()> {
    let pkm_dir = vault_path.join(".pkm");
    std::fs::create_dir_all(&pkm_dir)?;

    write_json_atomic(&pkm_dir.join("entities.json"), &data.entities)?;
    write_json_atomic(&pkm_dir.join("links.json"), &data.links)?;
    write_json_atomic(&pkm_dir.join("views.json"), &data.views)?;
    write_json_atomic(&pkm_dir.join("sources.json"), &data.sources)?;
    write_json_atomic(&pkm_dir.join("ingestion_queue.json"), &data.ingestion_queue)?;

    Ok(())
}

pub type SharedVault = Arc<RwLock<VaultState>>;

/// Try to extract a SourceId from YAML frontmatter in a source markdown file.
/// Returns None if no frontmatter or no valid id field.
fn extract_source_id_from_frontmatter(text: &str) -> Option<SourceId> {
    let text = if text.starts_with("---\r\n") {
        text.replace("\r\n", "\n")
    } else {
        text.to_string()
    };
    if !text.starts_with("---\n") {
        return None;
    }
    if let Some(end_pos) = text[4..].find("\n---\n") {
        let frontmatter_str = &text[4..4 + end_pos];
        #[derive(Deserialize)]
        struct Fm { id: Option<String> }
        if let Ok(fm) = serde_yaml::from_str::<Fm>(frontmatter_str) {
            if let Some(id_str) = fm.id {
                return uuid::Uuid::parse_str(&id_str).ok().map(SourceId);
            }
        }
    }
    None
}

/// Load vault from Markdown files + JSON metadata on disk.
pub fn load_vault(vault_path: &Path) -> SharedVault {
    let mut state = VaultState::new();

    let notes_dir = vault_path.join("notes");
    let sources_dir = vault_path.join("sources");
    let media_dir = vault_path.join("media");
    let pkm_dir = vault_path.join(".pkm");

    let _ = fs::create_dir_all(&notes_dir);
    let _ = fs::create_dir_all(&sources_dir);
    let _ = fs::create_dir_all(&media_dir);
    let _ = fs::create_dir_all(&pkm_dir);

    // 1. Load Notes from Markdown files (recursive subdirectory support)
    for entry in walkdir::WalkDir::new(&notes_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Ok(markdown_text) = fs::read_to_string(&path) {
                let dummy_id = NoteId::new();
                let now = pkm_core::Timestamp::now_utc();
                if let Ok((note, blocks)) = pkm_core::markdown::markdown_to_note(
                    &markdown_text,
                    dummy_id,
                    pkm_core::Actor::User,
                    now,
                ) {
                    state.notes.insert(note.id, note);
                    for block in blocks {
                        state.blocks.insert(block.id, block);
                    }
                }
            }
        }
    }

    // 2. Load Sources metadata from JSON (if available)
    let sources_path = pkm_dir.join("sources.json");
    if sources_path.exists() {
        if let Ok(file) = std::fs::File::open(&sources_path) {
            if let Ok(sources) = serde_json::from_reader::<_, HashMap<SourceId, Source>>(file) {
                state.sources = sources;
            }
        }
    }

    // 3. Load Sources from disk – use JSON metadata if present, otherwise fallback.
    //    Supports reading SourceId from YAML frontmatter (not just filename),
    //    so renaming a source file doesn't cause data loss.
    let mut source_ids_from_disk = std::collections::HashSet::new();
    for entry in walkdir::WalkDir::new(&sources_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Ok(raw_content) = fs::read_to_string(&path) {
                // Try frontmatter ID first, then fall back to filename
                let source_id = extract_source_id_from_frontmatter(&raw_content)
                    .or_else(|| {
                        let id_str = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                        uuid::Uuid::parse_str(id_str).ok().map(SourceId)
                    });
                if let Some(sid) = source_id {
                    source_ids_from_disk.insert(sid);
                    if let Some(existing) = state.sources.get_mut(&sid) {
                        // Preserve metadata, refresh raw content from disk
                        existing.raw_content = raw_content;
                    } else {
                        // Fallback: new source with default metadata
                        let now = pkm_core::Timestamp::now_utc();
                        let source = Source {
                            id: sid,
                            origin: pkm_core::source::SourceOrigin::ManualCapture,
                            title: None,
                            raw_content,
                            captured_at: now,
                            content_hash: String::new(),
                            ingestion_state: pkm_core::ingestion::IngestionState::Captured,
                            created_by: pkm_core::Actor::User,
                            created_at: now,
                            version: 1,
                            updated_at: now,
                        };
                        state.sources.insert(source.id, source);
                    }
                }
            }
        }
    }

    // Warn about and remove orphaned source entries whose .md files have been
    // deleted from disk, since they have no raw_content to operate on.
    let orphaned: Vec<SourceId> = state
        .sources
        .keys()
        .filter(|id| !source_ids_from_disk.contains(id))
        .copied()
        .collect();
    for id in &orphaned {
        eprintln!("[load_vault] Warning: source {} has no .md file on disk, removing from state", id.0);
    }
    for id in orphaned {
        state.sources.remove(&id);
    }

    // 4. Load Entities from JSON
    let entities_path = pkm_dir.join("entities.json");
    if entities_path.exists() {
        if let Ok(file) = std::fs::File::open(&entities_path) {
            if let Ok(entities) = serde_json::from_reader::<_, HashMap<EntityId, Entity>>(file) {
                state.entities = entities;
            }
        }
    }

    // 5. Load Links from JSON
    let links_path = pkm_dir.join("links.json");
    if links_path.exists() {
        if let Ok(file) = std::fs::File::open(&links_path) {
            if let Ok(links) = serde_json::from_reader::<_, HashMap<LinkId, Link>>(file) {
                state.links = links;
            }
        }
    }

    // 6. Load Views from JSON
    let views_path = pkm_dir.join("views.json");
    if views_path.exists() {
        if let Ok(file) = std::fs::File::open(&views_path) {
            if let Ok(views) = serde_json::from_reader::<_, HashMap<ViewId, View>>(file) {
                state.views = views;
            }
        }
    }

    // 7. Load Ingestion Queue from JSON
    let ingestion_queue_path = pkm_dir.join("ingestion_queue.json");
    if ingestion_queue_path.exists() {
        if let Ok(file) = std::fs::File::open(&ingestion_queue_path) {
            if let Ok(queue) = serde_json::from_reader::<_, Vec<IngestionItem>>(file) {
                state.ingestion_queue = queue;
            }
        }
    }

    state.rebuild_indexes();

    Arc::new(RwLock::new(state))
}
