use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::path::{Path, PathBuf};
use std::fs;
use pkm_core::id::{NoteId, BlockId, EntityId, LinkId, SourceId, ViewId};
use pkm_core::{note::Note, block::Block, entity::Entity, link::Link, source::Source, view::View};
use pkm_core::agent_action::AgentAction;

pub struct VaultState {
    // Core Collections
    pub notes: HashMap<NoteId, Note>,
    pub blocks: HashMap<BlockId, Block>, // Flattened from notes for fast O(1) lookup
    pub sources: HashMap<SourceId, Source>,
    pub entities: HashMap<EntityId, Entity>,
    pub links: HashMap<LinkId, Link>,
    pub views: HashMap<ViewId, View>,
    pub actions: Vec<AgentAction>, // Append-only Agent Actions log
    pub ingestion_queue: Vec<IngestionItem>, // Ingestion queue items
    
    // Fast Lookup Indexes (Built on boot)
    pub links_by_source: HashMap<String, Vec<LinkId>>,
    pub links_by_target: HashMap<String, Vec<LinkId>>,
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
            actions: Vec::new(),
            ingestion_queue: Vec::new(),
            links_by_source: HashMap::new(),
            links_by_target: HashMap::new(),
        }
    }

    /// Rebuilds fast-lookup indexes (called on boot or after a link changes)
    pub fn rebuild_indexes(&mut self) {
        self.links_by_source.clear();
        self.links_by_target.clear();
        
        for link in self.links.values() {
            let from_key = format!("{:?}", link.from);
            let to_key = format!("{:?}", link.to);
            
            self.links_by_source.entry(from_key).or_default().push(link.id);
            self.links_by_target.entry(to_key).or_default().push(link.id);
        }
    }
}

// Wrap it in an Arc<RwLock> so Tauri commands and background agents can share it.
pub type SharedVault = Arc<RwLock<VaultState>>;

/// Load vault from filesystem into memory
pub fn load_vault(vault_path: &Path) -> SharedVault {
    let mut state = VaultState::new();

    // 1. Create necessary directories
    let notes_dir = vault_path.join("notes");
    let sources_dir = vault_path.join("sources");
    let media_dir = vault_path.join("media");
    let pkm_dir = vault_path.join(".pkm");

    let _ = fs::create_dir_all(&notes_dir);
    let _ = fs::create_dir_all(&sources_dir);
    let _ = fs::create_dir_all(&media_dir);
    let _ = fs::create_dir_all(&pkm_dir);

    // 2. Load Notes from vault/notes/*.md
    if let Ok(entries) = fs::read_dir(&notes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
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
    }

    // 3. Load relational metadata from .pkm/
    if let Ok(entities_data) = fs::read_to_string(pkm_dir.join("entities.json")) {
        if let Ok(entities) = serde_json::from_str::<HashMap<EntityId, Entity>>(&entities_data) {
            state.entities = entities;
        }
    }
    if let Ok(links_data) = fs::read_to_string(pkm_dir.join("links.json")) {
        if let Ok(links) = serde_json::from_str::<HashMap<LinkId, Link>>(&links_data) {
            state.links = links;
        }
    }
    if let Ok(views_data) = fs::read_to_string(pkm_dir.join("views.json")) {
        if let Ok(views) = serde_json::from_str::<HashMap<ViewId, View>>(&views_data) {
            state.views = views;
        }
    }
    if let Ok(sources_data) = fs::read_to_string(pkm_dir.join("sources.json")) {
        if let Ok(sources) = serde_json::from_str::<HashMap<SourceId, Source>>(&sources_data) {
            state.sources = sources;
        }
    }
    if let Ok(queue_data) = fs::read_to_string(pkm_dir.join("ingestion_queue.json")) {
        if let Ok(queue) = serde_json::from_str::<Vec<IngestionItem>>(&queue_data) {
            state.ingestion_queue = queue;
        }
    }

    // 4. Load agent actions from .pkm/actions.jsonl
    if let Ok(actions_file) = fs::read_to_string(pkm_dir.join("actions.jsonl")) {
        for line in actions_file.lines() {
            if !line.trim().is_empty() {
                if let Ok(action) = serde_json::from_str::<AgentAction>(line) {
                    state.actions.push(action);
                }
            }
        }
    }

    state.rebuild_indexes();

    Arc::new(RwLock::new(state))
}
