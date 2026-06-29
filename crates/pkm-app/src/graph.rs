use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use pkm_core::id::ObjectRef;
use pkm_core::ports::{EntityRepo, LinkRepo, NoteRepo, SourceRepo, ViewRepo};
use pkm_core::view::{TimelineGrouping, View, ViewParams};
use pkm_fs::{FsEntityRepo, FsLinkRepo, FsNoteRepo, FsSourceRepo, SharedVault};

use crate::commands;

type GraphNode = (String, String, f64, f64, String, String, String);

fn fmt_rfc3339(ts: &pkm_core::Timestamp) -> String {
    ts.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| ts.to_string())
}

/// Combined result of graph view data: nodes and edges.
pub struct GraphViewResult {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<commands::LinkNetworkEdge>,
}

pub fn get_graph_view_data(
    vault: SharedVault,
    _vault_path: PathBuf,
    view: &View,
) -> Option<GraphViewResult> {
    if let ViewParams::GraphView(params) = &view.params {
        let state = vault.read().unwrap();

        // Collect all link IDs from the vault
        let mut edges = Vec::new();
        let mut referenced_ids = HashSet::new();
        for link in state.links().values() {
            let source_id = match link.from {
                ObjectRef::Source(id) => id.to_string(),
                ObjectRef::Note(id) => id.to_string(),
                ObjectRef::Entity(id) => id.to_string(),
                ObjectRef::Block(id) => id.to_string(),
                _ => continue,
            };
            let target_id = match link.to {
                ObjectRef::Source(id) => id.to_string(),
                ObjectRef::Note(id) => id.to_string(),
                ObjectRef::Entity(id) => id.to_string(),
                ObjectRef::Block(id) => id.to_string(),
                _ => continue,
            };
            referenced_ids.insert(source_id.clone());
            referenced_ids.insert(target_id.clone());
            edges.push(commands::LinkNetworkEdge {
                id: link.id.to_string(),
                source: source_id,
                target: target_id,
                link_type: format!("{:?}", link.link_type).to_lowercase(),
                confidence: link.confidence,
            });
        }

        if !params.node_positions.is_empty() {
            let nodes: Vec<_> = params
                .node_positions
                .iter()
                .filter_map(|pos| {
                    if let Ok(source_id) = uuid::Uuid::parse_str(&pos.id) {
                        if let Some(source) = state.sources().get(&pkm_core::id::SourceId(source_id)) {
                            return Some((
                                pos.id.clone(),
                                source.title.clone().unwrap_or_else(|| "[untitled]".to_string()),
                                pos.x, pos.y,
                                format!("{:?}", source.origin),
                                format!("{:?}", source.ingestion_state),
                                "source".to_string(),
                            ));
                        }
                        if let Some(note) = state.notes().get(&pkm_core::id::NoteId(source_id)) {
                            return Some((
                                pos.id.clone(),
                                note.title.clone(),
                                pos.x, pos.y,
                                String::new(), String::new(),
                                "note".to_string(),
                            ));
                        }
                        if let Some(entity) = state.entities().get(&pkm_core::id::EntityId(source_id)) {
                            return Some((
                                pos.id.clone(),
                                entity.name.clone(),
                                pos.x, pos.y,
                                String::new(), String::new(),
                                "entity".to_string(),
                            ));
                        }
                    }
                    None
                })
                .collect();
            Some(GraphViewResult { nodes, edges })
        } else {
            let mut nodes = Vec::new();
            let mut seen_ids = HashSet::new();

            // Only include nodes that have links or are in referenced_ids
            for source in state.sources().values() {
                let id = source.id.to_string();
                if referenced_ids.contains(&id) || !edges.is_empty() {
                    seen_ids.insert(id.clone());
                    nodes.push((
                        id,
                        source.title.clone().unwrap_or_else(|| "[untitled]".to_string()),
                        0.0, 0.0,
                        format!("{:?}", source.origin),
                        format!("{:?}", source.ingestion_state),
                        "source".to_string(),
                    ));
                }
            }
            for note in state.notes().values() {
                let id = note.id.to_string();
                if referenced_ids.contains(&id) || !edges.is_empty() {
                    seen_ids.insert(id.clone());
                    nodes.push((
                        id,
                        note.title.clone(),
                        0.0, 0.0,
                        String::new(), String::new(),
                        "note".to_string(),
                    ));
                }
            }
            for entity in state.entities().values() {
                let id = entity.id.to_string();
                if referenced_ids.contains(&id) || !edges.is_empty() {
                    seen_ids.insert(id.clone());
                    nodes.push((
                        id,
                        entity.name.clone(),
                        0.0, 0.0,
                        String::new(), String::new(),
                        "entity".to_string(),
                    ));
                }
            }

            // If no links exist, include all nodes (scatter plot fallback)
            if nodes.is_empty() {
                for source in state.sources().values() {
                    nodes.push((
                        source.id.to_string(),
                        source.title.clone().unwrap_or_else(|| "[untitled]".to_string()),
                        0.0, 0.0,
                        format!("{:?}", source.origin),
                        format!("{:?}", source.ingestion_state),
                        "source".to_string(),
                    ));
                }
                for note in state.notes().values() {
                    nodes.push((
                        note.id.to_string(),
                        note.title.clone(),
                        0.0, 0.0,
                        String::new(), String::new(),
                        "note".to_string(),
                    ));
                }
                for entity in state.entities().values() {
                    nodes.push((
                        entity.id.to_string(),
                        entity.name.clone(),
                        0.0, 0.0,
                        String::new(), String::new(),
                        "entity".to_string(),
                    ));
                }
            }

            Some(GraphViewResult { nodes, edges })
        }
    } else {
        None
    }
}

pub fn get_graph_show_edges(vault: SharedVault, vault_path: PathBuf, view_id: &str) -> Result<bool, String> {
    let uuid = uuid::Uuid::parse_str(view_id).map_err(|_| format!("Invalid view ID: {}", view_id))?;
    let parsed_id = pkm_core::id::ViewId(uuid);

    let view_repo = pkm_fs::FsViewRepo { state: vault, vault_path };
    let view = view_repo
        .get(parsed_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("View not found: {}", view_id))?;

    if let ViewParams::GraphView(params) = view.params {
        Ok(params.show_edges)
    } else {
        Err("View is not a graph view".to_string())
    }
}

pub fn get_preview_card(
    vault: SharedVault,
    vault_path: PathBuf,
    entity_id: &str,
) -> Result<commands::PreviewCard, String> {
    let uuid = uuid::Uuid::parse_str(entity_id).map_err(|_| format!("Invalid entity ID: {}", entity_id))?;
    let parsed_id = pkm_core::id::EntityId(uuid);

    let entity_repo = FsEntityRepo { state: vault, vault_path };
    let entity = entity_repo
        .get(parsed_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Entity not found: {}", entity_id))?;

    let aliases_str = if entity.aliases.is_empty() {
        String::new()
    } else {
        format!(" Also known as {}.", entity.aliases.join(", "))
    };

    Ok(commands::PreviewCard {
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

pub fn get_link_network(
    vault: SharedVault,
    vault_path: PathBuf,
    root_entity_id: &str,
    depth: usize,
) -> Result<commands::LinkNetworkData, String> {
    let uuid = uuid::Uuid::parse_str(root_entity_id).map_err(|_| format!("Invalid entity ID: {}", root_entity_id))?;
    let root_id = pkm_core::id::EntityId(uuid);

    let entity_repo = FsEntityRepo { state: vault.clone(), vault_path: vault_path.clone() };
    let link_repo = FsLinkRepo { state: vault.clone(), vault_path: vault_path.clone() };

    let root = entity_repo
        .get(root_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Entity not found: {}", root_entity_id))?;

    let mut nodes = HashMap::new();
    let mut edges = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut processed_links = std::collections::HashSet::new();
    let mut queue = VecDeque::new();
    let mut node_depth = HashMap::new();

    let root_ref = ObjectRef::Entity(root_id);
    queue.push_back((root_ref, 0));
    visited.insert(format!("entity:{}", root_id));
    node_depth.insert(format!("entity:{}", root_id), 0);

    nodes.insert(
        root_entity_id.to_string(),
        commands::LinkNetworkNode {
            id: root_entity_id.to_string(),
            title: root.name.clone(),
            kind: format!("{:?}", root.kind).to_lowercase(),
        },
    );

    let max_nodes = 300;

    while let Some((current_ref, current_depth)) = queue.pop_front() {
        if current_depth >= depth || nodes.len() >= max_nodes {
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
                                    commands::LinkNetworkNode {
                                        id: to_id.clone(),
                                        title: entity.name,
                                        kind: format!("{:?}", entity.kind).to_lowercase(),
                                    },
                                );
                            }
                        }
                    } else if to_type == "note" {
                        if let Ok(uuid) = uuid::Uuid::parse_str(&to_id) {
                            let note_repo = FsNoteRepo { state: vault.clone(), vault_path: vault_path.clone() };
                            if let Ok(Some(note)) = note_repo.get(pkm_core::id::NoteId(uuid)) {
                                nodes.insert(
                                    to_id.clone(),
                                    commands::LinkNetworkNode {
                                        id: to_id.clone(),
                                        title: note.title,
                                        kind: "note".to_string(),
                                    },
                                );
                            }
                        }
                    } else if to_type == "source" {
                        if let Ok(uuid) = uuid::Uuid::parse_str(&to_id) {
                            let source_repo = FsSourceRepo { state: vault.clone(), vault_path: vault_path.clone() };
                            if let Ok(Some(source)) = source_repo.get(pkm_core::id::SourceId(uuid)) {
                                nodes.insert(
                                    to_id.clone(),
                                    commands::LinkNetworkNode {
                                        id: to_id.clone(),
                                        title: source.title.unwrap_or_else(|| "Raw Source".into()),
                                        kind: "source".to_string(),
                                    },
                                );
                            }
                        }
                    } else if to_type == "block" {
                        if let Ok(block_uuid) = uuid::Uuid::parse_str(&to_id) {
                            let note_repo = FsNoteRepo { state: vault.clone(), vault_path: vault_path.clone() };
                            if let Ok(Some(note_id)) = note_repo.get_note_id_for_block(pkm_core::id::BlockId(block_uuid)) {
                                if let Ok(Some(note)) = note_repo.get(note_id) {
                                    nodes.insert(
                                        to_id.clone(),
                                        commands::LinkNetworkNode {
                                            id: to_id.clone(),
                                            title: format!("[block] {}", note.title),
                                            kind: "block".to_string(),
                                        },
                                    );
                                }
                            }
                        }
                    }
                }

                if processed_links.insert(link.id) {
                    edges.push(commands::LinkNetworkEdge {
                        id: link.id.to_string(),
                        source: from_id_str.clone(),
                        target: to_id.clone(),
                        link_type: format!("{:?}", link.link_type).to_lowercase(),
                        confidence: link.confidence,
                    });
                }
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
                                    commands::LinkNetworkNode {
                                        id: from_id.clone(),
                                        title: entity.name,
                                        kind: format!("{:?}", entity.kind).to_lowercase(),
                                    },
                                );
                            }
                        }
                    } else if from_type == "note" {
                        if let Ok(uuid) = uuid::Uuid::parse_str(&from_id) {
                            let note_repo = FsNoteRepo { state: vault.clone(), vault_path: vault_path.clone() };
                            if let Ok(Some(note)) = note_repo.get(pkm_core::id::NoteId(uuid)) {
                                nodes.insert(
                                    from_id.clone(),
                                    commands::LinkNetworkNode {
                                        id: from_id.clone(),
                                        title: note.title,
                                        kind: "note".to_string(),
                                    },
                                );
                            }
                        }
                    } else if from_type == "source" {
                        if let Ok(uuid) = uuid::Uuid::parse_str(&from_id) {
                            let source_repo = FsSourceRepo { state: vault.clone(), vault_path: vault_path.clone() };
                            if let Ok(Some(source)) = source_repo.get(pkm_core::id::SourceId(uuid)) {
                                nodes.insert(
                                    from_id.clone(),
                                    commands::LinkNetworkNode {
                                        id: from_id.clone(),
                                        title: source.title.unwrap_or_else(|| "Raw Source".into()),
                                        kind: "source".to_string(),
                                    },
                                );
                            }
                        }
                    } else if from_type == "block" {
                        if let Ok(block_uuid) = uuid::Uuid::parse_str(&from_id) {
                            let note_repo = FsNoteRepo { state: vault.clone(), vault_path: vault_path.clone() };
                            if let Ok(Some(note_id)) = note_repo.get_note_id_for_block(pkm_core::id::BlockId(block_uuid)) {
                                if let Ok(Some(note)) = note_repo.get(note_id) {
                                    nodes.insert(
                                        from_id.clone(),
                                        commands::LinkNetworkNode {
                                            id: from_id.clone(),
                                            title: format!("[block] {}", note.title),
                                            kind: "block".to_string(),
                                        },
                                    );
                                }
                            }
                        }
                    }
                }

                if processed_links.insert(link.id) {
                    edges.push(commands::LinkNetworkEdge {
                        id: link.id.to_string(),
                        source: from_id,
                        target: from_id_str.clone(),
                        link_type: format!("{:?}", link.link_type).to_lowercase(),
                        confidence: link.confidence,
                    });
                }
            }
        }
    }

    Ok(commands::LinkNetworkData {
        nodes: nodes.into_values().collect(),
        edges,
    })
}

pub fn get_neighbors(
    vault: SharedVault,
    vault_path: PathBuf,
    target_id: &str,
    _depth: usize,
) -> Result<commands::LinkNetworkData, String> {
    let uuid = uuid::Uuid::parse_str(target_id).map_err(|_| format!("Invalid ID: {}", target_id))?;

    let entity_repo = FsEntityRepo { state: vault.clone(), vault_path: vault_path.clone() };
    let link_repo = FsLinkRepo { state: vault.clone(), vault_path: vault_path.clone() };

    let mut nodes = HashMap::new();
    let mut edges = Vec::new();
    let mut processed_links = std::collections::HashSet::new();

    let target_ref = if let Ok(entity) = entity_repo.get(pkm_core::id::EntityId(uuid)) {
        if let Some(ent) = entity {
            nodes.insert(
                target_id.to_string(),
                commands::LinkNetworkNode {
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
                            commands::LinkNetworkNode {
                                id: to_id.clone(),
                                title: entity.name,
                                kind: format!("{:?}", entity.kind).to_lowercase(),
                            },
                        );
                    }
                }
            } else if to_type == "note" {
                if let Ok(uuid) = uuid::Uuid::parse_str(&to_id) {
                    let note_repo = FsNoteRepo { state: vault.clone(), vault_path: vault_path.clone() };
                    if let Ok(Some(note)) = note_repo.get(pkm_core::id::NoteId(uuid)) {
                        nodes.insert(
                            to_id.clone(),
                            commands::LinkNetworkNode {
                                id: to_id.clone(),
                                title: note.title,
                                kind: "note".to_string(),
                            },
                        );
                    }
                }
            } else if to_type == "source" {
                if let Ok(uuid) = uuid::Uuid::parse_str(&to_id) {
                    let source_repo = FsSourceRepo { state: vault.clone(), vault_path: vault_path.clone() };
                    if let Ok(Some(source)) = source_repo.get(pkm_core::id::SourceId(uuid)) {
                        nodes.insert(
                            to_id.clone(),
                            commands::LinkNetworkNode {
                                id: to_id.clone(),
                                title: source.title.unwrap_or_else(|| "Raw Source".into()),
                                kind: "source".to_string(),
                            },
                        );
                    }
                }
            } else if to_type == "block" {
                if let Ok(block_uuid) = uuid::Uuid::parse_str(&to_id) {
                    let note_repo = FsNoteRepo { state: vault.clone(), vault_path: vault_path.clone() };
                    if let Ok(Some(note_id)) = note_repo.get_note_id_for_block(pkm_core::id::BlockId(block_uuid)) {
                        if let Ok(Some(note)) = note_repo.get(note_id) {
                            nodes.insert(
                                to_id.clone(),
                                commands::LinkNetworkNode {
                                    id: to_id.clone(),
                                    title: format!("[block] {}", note.title),
                                    kind: "block".to_string(),
                                },
                            );
                        }
                    }
                }
            }

            if processed_links.insert(link.id) {
                edges.push(commands::LinkNetworkEdge {
                    id: link.id.to_string(),
                    source: target_id.to_string(),
                    target: to_id,
                    link_type: format!("{:?}", link.link_type).to_lowercase(),
                    confidence: link.confidence,
                });
            }
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
                            commands::LinkNetworkNode {
                                id: from_id.clone(),
                                title: entity.name,
                                kind: format!("{:?}", entity.kind).to_lowercase(),
                            },
                        );
                    }
                }
            } else if from_type == "note" {
                if let Ok(uuid) = uuid::Uuid::parse_str(&from_id) {
                    let note_repo = FsNoteRepo { state: vault.clone(), vault_path: vault_path.clone() };
                    if let Ok(Some(note)) = note_repo.get(pkm_core::id::NoteId(uuid)) {
                        nodes.insert(
                            from_id.clone(),
                            commands::LinkNetworkNode {
                                id: from_id.clone(),
                                title: note.title,
                                kind: "note".to_string(),
                            },
                        );
                    }
                }
            } else if from_type == "source" {
                if let Ok(uuid) = uuid::Uuid::parse_str(&from_id) {
                    let source_repo = FsSourceRepo { state: vault.clone(), vault_path: vault_path.clone() };
                    if let Ok(Some(source)) = source_repo.get(pkm_core::id::SourceId(uuid)) {
                        nodes.insert(
                            from_id.clone(),
                            commands::LinkNetworkNode {
                                id: from_id.clone(),
                                title: source.title.unwrap_or_else(|| "Raw Source".into()),
                                kind: "source".to_string(),
                            },
                        );
                    }
                }
            } else if from_type == "block" {
                if let Ok(block_uuid) = uuid::Uuid::parse_str(&from_id) {
                    let note_repo = FsNoteRepo { state: vault.clone(), vault_path: vault_path.clone() };
                    if let Ok(Some(note_id)) = note_repo.get_note_id_for_block(pkm_core::id::BlockId(block_uuid)) {
                        if let Ok(Some(note)) = note_repo.get(note_id) {
                            nodes.insert(
                                from_id.clone(),
                                commands::LinkNetworkNode {
                                    id: from_id.clone(),
                                    title: format!("[block] {}", note.title),
                                    kind: "block".to_string(),
                                },
                            );
                        }
                    }
                }
            }

            if processed_links.insert(link.id) {
                edges.push(commands::LinkNetworkEdge {
                    id: link.id.to_string(),
                    source: from_id,
                    target: target_id.to_string(),
                    link_type: format!("{:?}", link.link_type).to_lowercase(),
                    confidence: link.confidence,
                });
            }
        }
    }

    Ok(commands::LinkNetworkData {
        nodes: nodes.into_values().collect(),
        edges,
    })
}

pub fn get_canvas_view_data(
    vault: SharedVault,
    vault_path: PathBuf,
    view: &View,
) -> Option<commands::CanvasViewRenderData> {
    if let ViewParams::CanvasView(params) = &view.params {
        let source_repo = FsSourceRepo { state: vault.clone(), vault_path: vault_path.clone() };
        let sources = source_repo.list(None).ok()?;

        let mut source_map = HashMap::new();
        for source in sources {
            source_map.insert(source.id.to_string(), source);
        }

        let limit = params.limit.unwrap_or(500);

        let mut nodes = Vec::new();
        for node in params.nodes.iter().take(limit) {
            match &node.target {
                ObjectRef::Source(source_id) => {
                    if let Some(source) = source_map.get(&source_id.to_string()) {
                        nodes.push(commands::CanvasNodeData {
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
                ObjectRef::Note(note_id) => {
                    let note_repo = FsNoteRepo { state: vault.clone(), vault_path: vault_path.clone() };
                    if let Ok(Some(note)) = note_repo.get(*note_id) {
                        nodes.push(commands::CanvasNodeData {
                            id: note.id.to_string(),
                            title: note.title,
                            x: node.x, y: node.y, width: node.width, height: node.height,
                            color_theme: node.color_theme.clone(),
                            kind: "note".to_string(),
                        });
                    }
                }
                ObjectRef::Entity(entity_id) => {
                    let entity_repo = FsEntityRepo { state: vault.clone(), vault_path: vault_path.clone() };
                    if let Ok(Some(entity)) = entity_repo.get(*entity_id) {
                        nodes.push(commands::CanvasNodeData {
                            id: entity.id.to_string(),
                            title: entity.name,
                            x: node.x, y: node.y, width: node.width, height: node.height,
                            color_theme: node.color_theme.clone(),
                            kind: "entity".to_string(),
                        });
                    }
                }
                ObjectRef::Block(block_id) => {
                    let note_repo = FsNoteRepo { state: vault.clone(), vault_path: vault_path.clone() };
                    if let Ok(Some(note_id)) = note_repo.get_note_id_for_block(*block_id) {
                        if let Ok(Some(note)) = note_repo.get(note_id) {
                            nodes.push(commands::CanvasNodeData {
                                id: block_id.to_string(),
                                title: format!("[block] {}", note.title),
                                x: node.x, y: node.y, width: node.width, height: node.height,
                                color_theme: node.color_theme.clone(),
                                kind: "block".to_string(),
                            });
                        }
                    }
                }
                ObjectRef::Link(link_id) => {
                    nodes.push(commands::CanvasNodeData {
                        id: link_id.to_string(),
                        title: format!("[link] {}", link_id),
                        x: node.x, y: node.y, width: node.width, height: node.height,
                        color_theme: node.color_theme.clone(),
                        kind: "link".to_string(),
                    });
                }
                ObjectRef::View(view_id) => {
                    nodes.push(commands::CanvasNodeData {
                        id: view_id.to_string(),
                        title: format!("[view] {}", view_id),
                        x: node.x, y: node.y, width: node.width, height: node.height,
                        color_theme: node.color_theme.clone(),
                        kind: "view".to_string(),
                    });
                }
            }
        }

        let frames = params.frames.iter().map(|f| {
            commands::CanvasFrameData {
                id: f.id.clone(),
                label: f.label.clone(),
                x: f.x,
                y: f.y,
                width: f.width,
                height: f.height,
                background_color: f.background_color.clone(),
            }
        }).collect();

        let edges = params.edge_visuals.iter().map(|e| {
            let (from_type, from_id) = match &e.from {
                ObjectRef::Source(id) => ("source", id.to_string()),
                ObjectRef::Note(id) => ("note", id.to_string()),
                ObjectRef::Block(id) => ("block", id.to_string()),
                ObjectRef::Entity(id) => ("entity", id.to_string()),
                ObjectRef::Link(id) => ("link", id.to_string()),
                ObjectRef::View(id) => ("view", id.to_string()),
            };
            let (to_type, to_id) = match &e.to {
                ObjectRef::Source(id) => ("source", id.to_string()),
                ObjectRef::Note(id) => ("note", id.to_string()),
                ObjectRef::Block(id) => ("block", id.to_string()),
                ObjectRef::Entity(id) => ("entity", id.to_string()),
                ObjectRef::Link(id) => ("link", id.to_string()),
                ObjectRef::View(id) => ("view", id.to_string()),
            };
            commands::CanvasEdgeData {
                id: format!("{}->{}", from_id, to_id),
                from_type: from_type.to_string(),
                from_id,
                to_type: to_type.to_string(),
                to_id,
                routing_style: e.routing_style.clone(),
                color: e.color.clone(),
            }
        }).collect();

        Some(commands::CanvasViewRenderData {
            title: view.title.clone(),
            nodes,
            edges,
            frames,
        })
    } else {
        None
    }
}

pub fn get_timeline_view_data(
    vault: SharedVault,
    vault_path: PathBuf,
    view: &View,
) -> Option<commands::TimelineRenderData> {
    if let ViewParams::Timeline(params) = &view.params {
        let source_repo = FsSourceRepo { state: vault.clone(), vault_path: vault_path.clone() };
        let mut sources = source_repo.list(None).ok()?;

        if params.reverse_chronological {
            sources.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));
        } else {
            sources.sort_by(|a, b| a.captured_at.cmp(&b.captured_at));
        }

        // Load entities with semantic dates for timeline
        let entities: Vec<_> = {
            let state = vault.read().unwrap();
            state.entities().values().cloned().collect()
        };
        let mut timeline_entities: Vec<_> = entities
            .into_iter()
            .filter(|e| e.semantic_date.is_some())
            .collect();

        if params.reverse_chronological {
            timeline_entities.sort_by(|a, b| {
                let a_date = a.semantic_date.unwrap_or(a.created_at);
                let b_date = b.semantic_date.unwrap_or(b.created_at);
                b_date.cmp(&a_date)
            });
        } else {
            timeline_entities.sort_by(|a, b| {
                let a_date = a.semantic_date.unwrap_or(a.created_at);
                let b_date = b.semantic_date.unwrap_or(b.created_at);
                a_date.cmp(&b_date)
            });
        }

        let limit = params.limit.unwrap_or(100);
        let per_type_limit = limit;

        let mut year_map: std::collections::HashMap<String, std::collections::HashMap<String, Vec<commands::TimelineEventData>>> = std::collections::HashMap::new();

        for source in sources.iter().take(per_type_limit) {
            let date_str = fmt_rfc3339(&source.captured_at);
            let year = date_str.split('-').next().unwrap_or("unknown").to_string();

            let month_key = if date_str.len() >= 7 {
                match params.grouping {
                    TimelineGrouping::Day => {
                        if date_str.len() >= 10 {
                            date_str[0..10].to_string()
                        } else {
                            date_str[0..7].to_string()
                        }
                    }
                    TimelineGrouping::Week => {
                        if let Ok(dt) = time::OffsetDateTime::parse(&date_str, &time::format_description::well_known::Rfc3339) {
                            let date = dt.date();
                            let iso_week = date.iso_week();
                            format!("{}-W{:02}", date.year(), iso_week)
                        } else {
                            date_str[0..7].to_string()
                        }
                    }
                    TimelineGrouping::Month => date_str[0..7].to_string(),
                    TimelineGrouping::Year => year.clone(),
                }
            } else {
                "unknown".to_string()
            };

            let event = commands::TimelineEventData {
                id: source.id.to_string(),
                title: source.title.clone().unwrap_or_else(|| "[untitled]".to_string()),
                date: date_str,
            };

            year_map
                .entry(year)
                .or_insert_with(std::collections::HashMap::new)
                .entry(month_key)
                .or_insert_with(Vec::new)
                .push(event);
        }

        for entity in timeline_entities.iter().take(per_type_limit) {
            let date = entity.semantic_date.unwrap_or(entity.created_at);
            let date_str = fmt_rfc3339(&date);
            let year = date_str.split('-').next().unwrap_or("unknown").to_string();

            let month_key = if date_str.len() >= 7 {
                match params.grouping {
                    TimelineGrouping::Day => {
                        if date_str.len() >= 10 {
                            date_str[0..10].to_string()
                        } else {
                            date_str[0..7].to_string()
                        }
                    }
                    TimelineGrouping::Week => {
                        if let Ok(dt) = time::OffsetDateTime::parse(&date_str, &time::format_description::well_known::Rfc3339) {
                            let date = dt.date();
                            let iso_week = date.iso_week();
                            format!("{}-W{:02}", date.year(), iso_week)
                        } else {
                            date_str[0..7].to_string()
                        }
                    }
                    TimelineGrouping::Month => date_str[0..7].to_string(),
                    TimelineGrouping::Year => year.clone(),
                }
            } else {
                "unknown".to_string()
            };

            let event = commands::TimelineEventData {
                id: entity.id.to_string(),
                title: entity.name.clone(),
                date: date_str,
            };

            year_map
                .entry(year)
                .or_insert_with(std::collections::HashMap::new)
                .entry(month_key)
                .or_insert_with(Vec::new)
                .push(event);
        }

        // Convert HashMap to ordered Vec preserving the intended sort direction
        let mut years: Vec<(&str, &std::collections::HashMap<String, Vec<commands::TimelineEventData>>)> = year_map.iter().map(|(k, v)| (k.as_str(), v)).collect();
        if params.reverse_chronological {
            years.sort_by(|a, b| b.0.cmp(a.0));
        } else {
            years.sort_by(|a, b| a.0.cmp(b.0));
        }

        let groups: Vec<commands::TimelineGroup> = years.iter().map(|(year, month_map)| {
            let mut months: Vec<(&str, &Vec<commands::TimelineEventData>)> = month_map.iter().map(|(k, v)| (k.as_str(), v)).collect();
            if params.reverse_chronological {
                months.sort_by(|a, b| b.0.cmp(a.0));
            } else {
                months.sort_by(|a, b| a.0.cmp(b.0));
            }
            commands::TimelineGroup {
                year: year.to_string(),
                months: months.iter().map(|(key, events)| {
                    let mut sorted_events = (*events).clone();
                    if params.reverse_chronological {
                        sorted_events.sort_by(|a, b| b.date.cmp(&a.date));
                    } else {
                        sorted_events.sort_by(|a, b| a.date.cmp(&b.date));
                    }
                    commands::TimelineMonthGroup {
                        key: key.to_string(),
                        events: sorted_events,
                    }
                }).collect(),
            }
        }).collect();

        Some(commands::TimelineRenderData {
            title: view.title.clone(),
            groups,
        })
    } else {
        None
    }
}
