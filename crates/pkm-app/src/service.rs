use pkm_core::note::Note;
use pkm_core::ports::{EntityRepo, LinkRepo, NoteRepo, Retriever, SearchMode, SourceRepo, ViewRepo};
use pkm_core::view::{DefaultViewModel, View, ViewKind, ViewModel, ViewParams};
use pkm_core::{Actor, Timestamp};
use pkm_search::parse_query;
use pkm_fs::{
    SharedVault, FsNoteRepo, FsSourceRepo, FsEntityRepo, FsLinkRepo, FsViewRepo,
    FsAgentActionRepo, FsRetriever, load_vault, IngestionItem
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use crate::watcher::{watch_vault, NoteWatcherEvent, IgnoreNextEvent};
use tokio::sync::mpsc;

type GraphNode = (String, String, f64, f64, String, String, String);

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
    pub fn new(db_path: &str, vault_path: Option<&str>) -> Result<Self, String> {
        let vault_dir = if let Some(path) = vault_path {
            PathBuf::from(path)
        } else {
            let db_path_obj = PathBuf::from(db_path);
            let db_dir = db_path_obj.parent().unwrap_or_else(|| Path::new("."));
            db_dir.join("vault")
        };

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

    pub fn start_vault_watcher(&self) -> Result<(), String> {
        let vault = self.vault.clone();
        let vault_path = self.vault_path.clone();

        let (watcher_rx, ignore_handle) = watch_vault(&vault_path)
            .map_err(|e| format!("Failed to start vault watcher: {}", e))?;

        let _ = WATCHER_IGNORE_HANDLE.set(ignore_handle);

        tokio::spawn(async move {
            let mut rx = watcher_rx;
            while let Some(event) = rx.recv().await {
                match event {
                    NoteWatcherEvent::Modified { .. } => {
                        if let Err(e) = Self::sync_external_note(&vault, &vault_path, event) {
                            eprintln!("Failed to sync external note change: {}", e);
                        }
                    }
                    NoteWatcherEvent::Deleted { file_path } => {
                        Self::sync_external_note_delete(&file_path);
                    }
                }
            }
        });

        Ok(())
    }

    fn sync_external_note(
        vault: &SharedVault,
        vault_path: &Path,
        event: NoteWatcherEvent,
    ) -> Result<(), String> {
        let (file_path, note, blocks) = match event {
            NoteWatcherEvent::Modified { file_path, note, blocks } => (file_path, note, blocks),
            _ => return Err("Expected Modified event".to_string()),
        };

        let file_path_canonical = std::fs::canonicalize(file_path.clone())
            .unwrap_or_else(|_| file_path.clone());

        let mut state = vault.write().unwrap();
        state.notes.insert(note.id, note.clone());

        // Update blocks for note
        state.blocks.retain(|_, b| b.note_id != note.id);
        for block in &blocks {
            state.blocks.insert(block.id, block.clone());
        }

        println!("✓ Synced external note: {} ({} blocks)", note.title, blocks.len());
        Ok(())
    }

    fn sync_external_note_delete(file_path: &Path) {
        println!(
            "[Watcher] Ignored external deletion of {} to prevent data loss. Delete via the app UI to safely remove notes.", 
            file_path.display()
        );
    }

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
        metadata: BTreeMap<String, serde_json::Value>,
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

    pub fn get_graph_view_data(&self, view_id: &str) -> Result<Option<Vec<GraphNode>>, String> {
        let view = self.get_view(view_id)?;

        match view {
            Some(view) => {
                if let ViewParams::GraphView(params) = &view.params {
                    let source_repo = FsSourceRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
                    let sources = source_repo.list(None).map_err(|e| e.to_string())?;

                    let mut source_map = std::collections::HashMap::new();
                    for source in &sources {
                        source_map.insert(source.id.to_string(), source);
                    }

                    if !params.node_positions.is_empty() {
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
                        // NO MORE RUST MATH!
                        // Just map the nodes and let the Frontend JS physics engine position them.
                        let nodes: Vec<_> = sources
                            .iter()
                            .map(|source| {
                                (
                                    source.id.to_string(),
                                    source.title.clone().unwrap_or_else(|| "[untitled]".to_string()),
                                    0.0, // x: Frontend handles layout
                                    0.0, // y: Frontend handles layout
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

    pub fn get_preview_card(&self, entity_id: &str) -> Result<crate::commands::PreviewCard, String> {
        let uuid = uuid::Uuid::parse_str(entity_id).map_err(|_| format!("Invalid entity ID: {}", entity_id))?;
        let parsed_id = pkm_core::id::EntityId(uuid);

        let entity_repo = FsEntityRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        let entity = entity_repo
            .get(parsed_id)
            .map_err(|e| e.to_string())?
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

    pub fn get_link_network(&self, root_entity_id: &str, depth: usize) -> Result<crate::commands::LinkNetworkData, String> {
        use pkm_core::id::ObjectRef;
        use std::collections::{HashMap, VecDeque};

        let uuid = uuid::Uuid::parse_str(root_entity_id).map_err(|_| format!("Invalid entity ID: {}", root_entity_id))?;
        let root_id = pkm_core::id::EntityId(uuid);

        let entity_repo = FsEntityRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        let link_repo = FsLinkRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };

        let root = entity_repo
            .get(root_id)
            .map_err(|e| e.to_string())?
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

        nodes.insert(
            root_entity_id.to_string(),
            crate::commands::LinkNetworkNode {
                id: root_entity_id.to_string(),
                title: root.name.clone(),
                kind: format!("{:?}", root.kind).to_lowercase(),
            },
        );

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

                    if !visited.contains(&node_key) {
                        visited.insert(node_key.clone());
                        node_depth.insert(node_key.clone(), current_depth + 1);
                        queue.push_back((link.to, current_depth + 1));

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

                    edges.push(crate::commands::LinkNetworkEdge {
                        id: link.id.to_string(),
                        source: from_id_str.clone(),
                        target: to_id.clone(),
                        link_type: format!("{:?}", link.link_type).to_lowercase(),
                        confidence: link.confidence,
                    });
                }
            }

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

                    if !visited.contains(&node_key) {
                        visited.insert(node_key.clone());
                        node_depth.insert(node_key.clone(), current_depth + 1);
                        queue.push_back((link.from, current_depth + 1));

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

    pub fn get_canvas_view_data(&self, view_id: &str) -> Result<Option<crate::commands::CanvasViewRenderData>, String> {
        let view = self.get_view(view_id)?;

        match view {
            Some(view) => {
                if let ViewParams::CanvasView(params) = &view.params {
                    let source_repo = FsSourceRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
                    let sources = source_repo.list(None).map_err(|e| e.to_string())?;

                    let mut source_map = std::collections::HashMap::new();
                    for source in sources {
                        source_map.insert(source.id.to_string(), source);
                    }

                    let limit = params.limit.unwrap_or(500);

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
                            pkm_core::id::ObjectRef::Note(note_id) => {
                                let note_repo = FsNoteRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
                                if let Ok(Some(note)) = note_repo.get(*note_id) {
                                    nodes.push(crate::commands::CanvasNodeData {
                                        id: note.id.to_string(),
                                        title: note.title,
                                        x: node.x, y: node.y, width: node.width, height: node.height,
                                        color_theme: node.color_theme.clone(),
                                        kind: "note".to_string(),
                                    });
                                }
                            }
                            pkm_core::id::ObjectRef::Entity(entity_id) => {
                                let entity_repo = FsEntityRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
                                if let Ok(Some(entity)) = entity_repo.get(*entity_id) {
                                    nodes.push(crate::commands::CanvasNodeData {
                                        id: entity.id.to_string(),
                                        title: entity.name,
                                        x: node.x, y: node.y, width: node.width, height: node.height,
                                        color_theme: node.color_theme.clone(),
                                        kind: "entity".to_string(),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }

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

    pub fn get_timeline_view_data(&self, view_id: &str) -> Result<Option<crate::commands::TimelineRenderData>, String> {
        use std::collections::BTreeMap;

        let view = self.get_view(view_id)?;

        match view {
            Some(view) => {
                if let ViewParams::Timeline(params) = &view.params {
                    let source_repo = FsSourceRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
                    let mut sources = source_repo.list(None).map_err(|e| e.to_string())?;

                    if params.reverse_chronological {
                        sources.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));
                    } else {
                        sources.sort_by(|a, b| a.captured_at.cmp(&b.captured_at));
                    }

                    let limit = params.limit.unwrap_or(100);

                    let mut grouped: BTreeMap<String, BTreeMap<String, Vec<crate::commands::TimelineEventData>>> = BTreeMap::new();

                    for source in sources.iter().take(limit) {
                        let date_str = source.captured_at.to_string();
                        let year = date_str.split('-').next().unwrap_or("unknown").to_string();

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

    pub fn get_neighbors(&self, target_id: &str, _depth: usize) -> Result<crate::commands::LinkNetworkData, String> {
        use pkm_core::id::ObjectRef;

        let uuid = uuid::Uuid::parse_str(target_id).map_err(|_| format!("Invalid ID: {}", target_id))?;

        let entity_repo = FsEntityRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        let link_repo = FsLinkRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };

        let mut nodes = std::collections::HashMap::new();
        let mut edges = Vec::new();

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

        if let Ok(outgoing) = link_repo.get_by_from(target_ref) {
            for link in outgoing.iter().take(20) {
                let (to_type, to_id) = match link.to {
                    ObjectRef::Entity(id) => ("entity", id.to_string()),
                    ObjectRef::Note(id) => ("note", id.to_string()),
                    ObjectRef::Source(id) => ("source", id.to_string()),
                    ObjectRef::Block(id) => ("block", id.to_string()),
                    _ => continue,
                };

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

        if let Ok(incoming) = link_repo.get_by_to(target_ref) {
            for link in incoming.iter().take(20) {
                let (from_type, from_id) = match link.from {
                    ObjectRef::Entity(id) => ("entity", id.to_string()),
                    ObjectRef::Note(id) => ("note", id.to_string()),
                    ObjectRef::Source(id) => ("source", id.to_string()),
                    ObjectRef::Block(id) => ("block", id.to_string()),
                    _ => continue,
                };

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

    pub fn get_provenance_chain(&self, _block_id: &str) -> Result<crate::commands::ProvenanceChainData, String> {
        Err("Provenance chain traversal is not yet implemented.".to_string())
    }

    pub fn get_entity_matrix(
        &self,
        _row_kind: &str,
        _col_kind: &str,
        _min_confidence: Option<f32>,
    ) -> Result<crate::commands::EntityMatrixData, String> {
        Err("Entity matrix queries are not yet implemented.".to_string())
    }

    pub fn handle_ai_action(&self, operation: pkm_agent::Operation) -> Result<String, String> {
        let action_repo = FsAgentActionRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        let note_repo = FsNoteRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        let link_repo = FsLinkRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };

        let req = pkm_agent::OperationRequest {
            actor: Actor::Agent { name: "Auto-Agent".to_string() },
            operation,
            rationale: "Automated AI task".to_string(),
        };

        let action = pkm_agent::execute(req, &action_repo, &note_repo)
            .map_err(|e| format!("Failed to execute operation: {}", e))?;

        pkm_agent::apply_action(
            action.id,
            &action_repo,
            &note_repo,
            Some(&link_repo)
        ).map_err(|e| format!("Failed to apply action: {}", e))?;

        Ok(action.id.to_string())
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

    pub fn rollback_recent_autonomous_ingestion(
        &self,
        minutes: i64,
    ) -> Result<usize, String> {
        let state = self.vault.read().unwrap();
        let cutoff = Timestamp::now_utc() - time::Duration::minutes(minutes);

        // Find all autonomous action IDs after cutoff
        let action_ids: Vec<pkm_core::id::AgentActionId> = state.actions.iter()
            .filter(|a| {
                let is_ingestor = match &a.actor {
                    Actor::Agent { name } => name.contains("Autonomous-Ingestor"),
                    _ => false,
                };
                is_ingestor && a.created_at > cutoff
            })
            .map(|a| a.id)
            .collect();
        drop(state);

        let count = action_ids.len();

        let action_repo = FsAgentActionRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };
        let note_repo = FsNoteRepo { state: self.vault.clone(), vault_path: self.vault_path.clone() };

        for action_id in action_ids {
            let _ = pkm_agent::rollback_action(
                action_id,
                &action_repo,
                &note_repo,
                None,
                None,
            );
        }

        Ok(count)
    }
}
