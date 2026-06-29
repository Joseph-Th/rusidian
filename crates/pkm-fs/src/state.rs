use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::path::Path;
use std::fs;
use pkm_core::id::{NoteId, BlockId, EntityId, LinkId, SourceId, ViewId};
use pkm_core::agent_action::AgentAction;
use pkm_core::{note::Note, block::Block, entity::Entity, link::Link, source::Source, view::View};

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
    pub(crate) actions: Vec<AgentAction>,
    pub(crate) ingestion_queue: Vec<IngestionItem>,

    pub(crate) links_by_source: HashMap<String, Vec<LinkId>>,
    pub(crate) links_by_target: HashMap<String, Vec<LinkId>>,
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

    pub fn save_metadata(&self, vault_path: &Path) -> std::io::Result<()> {
        let pkm_dir = vault_path.join(".pkm");
        std::fs::create_dir_all(&pkm_dir)?;

        let entities_file = std::fs::File::create(pkm_dir.join("entities.json"))?;
        serde_json::to_writer_pretty(entities_file, &self.entities)?;

        let links_file = std::fs::File::create(pkm_dir.join("links.json"))?;
        serde_json::to_writer_pretty(links_file, &self.links)?;

        let views_file = std::fs::File::create(pkm_dir.join("views.json"))?;
        serde_json::to_writer_pretty(views_file, &self.views)?;

        let actions_file = std::fs::File::create(pkm_dir.join("actions.json"))?;
        serde_json::to_writer_pretty(actions_file, &self.actions)?;

        Ok(())
    }

    pub fn rebuild_indexes(&mut self) {
        self.links_by_source.clear();
        self.links_by_target.clear();

        for link in self.links.values() {
            let from_key = format!("{:?}", link.from);
            let to_key = format!("{:?}", link.to);

            self.links_by_source
                .entry(from_key)
                .or_default()
                .push(link.id);
            self.links_by_target
                .entry(to_key)
                .or_default()
                .push(link.id);
        }
    }

    pub fn index_link_add(&mut self, link: &pkm_core::link::Link) {
        let from_key = format!("{:?}", link.from);
        let to_key = format!("{:?}", link.to);
        self.links_by_source
            .entry(from_key)
            .or_default()
            .push(link.id);
        self.links_by_target
            .entry(to_key)
            .or_default()
            .push(link.id);
    }

    pub fn index_link_remove(&mut self, link: &pkm_core::link::Link) {
        let from_key = format!("{:?}", link.from);
        if let Some(vec) = self.links_by_source.get_mut(&from_key) {
            vec.retain(|&id| id != link.id);
        }
        let to_key = format!("{:?}", link.to);
        if let Some(vec) = self.links_by_target.get_mut(&to_key) {
            vec.retain(|&id| id != link.id);
        }
    }

    // ── Public read-only accessors ──────────────────────────────────────
    // External code reads through these; mutation goes through repository traits.

    pub fn notes(&self) -> &HashMap<NoteId, Note> { &self.notes }
    pub fn blocks(&self) -> &HashMap<BlockId, Block> { &self.blocks }
    pub fn sources(&self) -> &HashMap<SourceId, Source> { &self.sources }
    pub fn entities(&self) -> &HashMap<EntityId, Entity> { &self.entities }
    pub fn links(&self) -> &HashMap<LinkId, Link> { &self.links }
    pub fn views(&self) -> &HashMap<ViewId, View> { &self.views }
    pub fn actions(&self) -> &[AgentAction] { &self.actions }
}

impl Default for VaultState {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedVault = Arc<RwLock<VaultState>>;

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

    // 1. Load Notes from Markdown files
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

    // 2. Load Sources from disk
    if let Ok(entries) = fs::read_dir(&sources_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                // Sources are stored as raw markdown files; metadata is re-created from path
                let id_str = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if let Ok(id) = uuid::Uuid::parse_str(id_str) {
                    if let Ok(raw_content) = fs::read_to_string(&path) {
                        let now = pkm_core::Timestamp::now_utc();
                        let source = Source {
                            id: SourceId(id),
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

    // 3. Load Entities from JSON
    let entities_path = pkm_dir.join("entities.json");
    if entities_path.exists() {
        if let Ok(file) = std::fs::File::open(&entities_path) {
            if let Ok(entities) = serde_json::from_reader::<_, HashMap<EntityId, Entity>>(file) {
                state.entities = entities;
            }
        }
    }

    // 4. Load Links from JSON
    let links_path = pkm_dir.join("links.json");
    if links_path.exists() {
        if let Ok(file) = std::fs::File::open(&links_path) {
            if let Ok(links) = serde_json::from_reader::<_, HashMap<LinkId, Link>>(file) {
                state.links = links;
            }
        }
    }

    // 5. Load Views from JSON
    let views_path = pkm_dir.join("views.json");
    if views_path.exists() {
        if let Ok(file) = std::fs::File::open(&views_path) {
            if let Ok(views) = serde_json::from_reader::<_, HashMap<ViewId, View>>(file) {
                state.views = views;
            }
        }
    }

    // 6. Load Agent Actions from JSON
    let actions_path = pkm_dir.join("actions.json");
    if actions_path.exists() {
        if let Ok(file) = std::fs::File::open(&actions_path) {
            if let Ok(actions) = serde_json::from_reader::<_, Vec<AgentAction>>(file) {
                state.actions = actions;
            }
        }
    }

    state.rebuild_indexes();

    Arc::new(RwLock::new(state))
}
