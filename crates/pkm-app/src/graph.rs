use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;

use pkm_core::id::ObjectRef;
use pkm_core::ports::{EntityRepo, LinkRepo, NoteRepo, SourceRepo, ViewRepo};
use pkm_core::view::{TimelineGrouping, View, ViewParams};
use pkm_fs::{FsEntityRepo, FsLinkRepo, FsNoteRepo, FsSourceRepo, SharedVault};

use crate::commands;

type GraphNode = (String, String, f64, f64, String, String, String);

pub fn get_graph_view_data(
    vault: SharedVault,
    vault_path: PathBuf,
    view: &View,
) -> Option<Vec<GraphNode>> {
    if let ViewParams::GraphView(params) = &view.params {
        let source_repo = FsSourceRepo { state: vault, vault_path };
        let sources = source_repo.list(None).ok()?;

        let mut source_map = HashMap::new();
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
            Some(nodes)
        } else {
            let nodes: Vec<_> = sources
                .iter()
                .map(|source| {
                    (
                        source.id.to_string(),
                        source.title.clone().unwrap_or_else(|| "[untitled]".to_string()),
                        0.0,
                        0.0,
                        format!("{:?}", source.origin),
                        format!("{:?}", source.ingestion_state),
                        "source".to_string(),
                    )
                })
                .collect();

            Some(nodes)
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
                    }
                }

                edges.push(commands::LinkNetworkEdge {
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
                    }
                }

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
            }

            edges.push(commands::LinkNetworkEdge {
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
            }

            edges.push(commands::LinkNetworkEdge {
                id: link.id.to_string(),
                source: from_id,
                target: target_id.to_string(),
                link_type: format!("{:?}", link.link_type).to_lowercase(),
                confidence: link.confidence,
            });
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
                _ => {}
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

        Some(commands::CanvasViewRenderData {
            title: view.title.clone(),
            nodes,
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

        let mut grouped: BTreeMap<String, BTreeMap<String, Vec<commands::TimelineEventData>>> = BTreeMap::new();

        for source in sources.iter().take(per_type_limit) {
            let date_str = source.captured_at.to_string();
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
                    TimelineGrouping::Week |
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

            grouped
                .entry(year)
                .or_insert_with(BTreeMap::new)
                .entry(month_key)
                .or_insert_with(Vec::new)
                .push(event);
        }

        for entity in timeline_entities.iter().take(per_type_limit) {
            let date = entity.semantic_date.unwrap_or(entity.created_at);
            let date_str = date.to_string();
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
                    TimelineGrouping::Week |
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

            grouped
                .entry(year)
                .or_insert_with(BTreeMap::new)
                .entry(month_key)
                .or_insert_with(Vec::new)
                .push(event);
        }

        Some(commands::TimelineRenderData {
            title: view.title.clone(),
            events: grouped,
        })
    } else {
        None
    }
}
