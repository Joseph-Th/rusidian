use crate::graph;
use crate::service::AppService;
use pkm_core::note::NoteMetadata;
use pkm_core::view::{ViewKind, ViewParams};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateNoteResponse {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NoteInfo {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GetNoteResponse {
    pub id: String,
    pub title: String,
    pub blocks: Vec<String>,
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub created_by: String,
    pub created_at: String,
    pub version: u32,
    pub updated_at: String,
    pub block_count: usize,
}

pub fn create_note(
    title: String,
    service: &Arc<AppService>,
) -> Result<CreateNoteResponse, String> {
    let note_id = service.create_note(title.clone())?;
    Ok(CreateNoteResponse { id: note_id, title })
}

pub fn list_notes(
    limit: Option<usize>,
    service: &Arc<AppService>,
) -> Result<Vec<NoteInfo>, String> {
    let notes = service.list_notes(limit)?;
    Ok(notes
        .into_iter()
        .map(|(id, title)| NoteInfo { id, title })
        .collect())
}

fn metadata_to_map(m: &NoteMetadata) -> BTreeMap<String, serde_json::Value> {
    let mut map = BTreeMap::new();
    if let Some(ref p) = m.project {
        map.insert("project".into(), serde_json::Value::String(p.clone()));
    }
    if !m.tags.is_empty() {
        map.insert("tags".into(), serde_json::Value::Array(m.tags.iter().map(|t| serde_json::Value::String(t.clone())).collect()));
    }
    if let Some(ref s) = m.status {
        map.insert("status".into(), serde_json::Value::String(s.clone()));
    }
    if let Some(p) = m.priority {
        map.insert("priority".into(), serde_json::Value::Number(p.into()));
    }
    map
}

fn map_to_metadata(map: BTreeMap<String, serde_json::Value>) -> NoteMetadata {
    let project = map.get("project").and_then(|v| v.as_str().map(String::from));
    let tags = map.get("tags").map(|v| {
        v.as_array().map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default()
    }).unwrap_or_default();
    let status = map.get("status").and_then(|v| v.as_str().map(String::from));
    let priority = map.get("priority").and_then(|v| v.as_i64().map(|n| n as i32));
    NoteMetadata { project, tags, priority, status }
}

pub fn get_note(
    note_id: String,
    service: &Arc<AppService>,
) -> Result<GetNoteResponse, String> {
    let note = service
        .get_note_full(&note_id)?
        .ok_or_else(|| format!("Note not found: {}", note_id))?;

    Ok(GetNoteResponse {
        id: note.id.to_string(),
        title: note.title,
        blocks: note.blocks.iter().map(|b| b.to_string()).collect(),
        metadata: metadata_to_map(&note.metadata),
        created_by: format!("{:?}", note.created_by),
        created_at: note.created_at.to_string(),
        version: note.version,
        updated_at: note.updated_at.to_string(),
        block_count: note.blocks.len(),
    })
}

pub fn update_note(
    note_id: String,
    title: String,
    metadata: BTreeMap<String, serde_json::Value>,
    service: &Arc<AppService>,
) -> Result<(), String> {
    service.update_note(&note_id, title, map_to_metadata(metadata))
}

pub fn delete_note(note_id: String, service: &Arc<AppService>) -> Result<(), String> {
    service.delete_note(&note_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateViewResponse {
    pub id: String,
    pub kind: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewInfo {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderViewResponse {
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineEventData {
    pub id: String,
    pub title: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineMonthGroup {
    pub key: String,
    pub events: Vec<TimelineEventData>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineGroup {
    pub year: String,
    pub months: Vec<TimelineMonthGroup>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineRenderData {
    pub title: String,
    pub groups: Vec<TimelineGroup>,
}

pub fn create_view(
    kind: String,
    title: String,
    params: ViewParams,
    service: &Arc<AppService>,
) -> Result<CreateViewResponse, String> {
    let view_kind = match kind.as_str() {
        "reading_queue" => ViewKind::ReadingQueue,
        "review_queue" => ViewKind::ReviewQueue,
        "timeline" => ViewKind::Timeline,
        "graph_view" => ViewKind::GraphView,
        "canvas_view" => ViewKind::CanvasView,
        _ => return Err(format!("Unknown view kind: {}", kind)),
    };

    let view_id = service.create_view(view_kind, title.clone(), params)?;

    Ok(CreateViewResponse {
        id: view_id,
        kind,
        title,
    })
}

pub fn list_views(
    limit: Option<usize>,
    service: &Arc<AppService>,
) -> Result<Vec<ViewInfo>, String> {
    let views = service.list_views(limit)?;

    Ok(views
        .into_iter()
        .map(|v| ViewInfo {
            id: v.id.to_string(),
            kind: serde_json::to_value(v.kind)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| format!("{:?}", v.kind).to_lowercase()),
            title: v.title,
            params: serde_json::to_value(&v.params).unwrap_or_default(),
        })
        .collect())
}

pub fn get_view(
    view_id: String,
    service: &Arc<AppService>,
) -> Result<Option<ViewInfo>, String> {
    let view = service.get_view(&view_id)?;

    Ok(view.map(|v| ViewInfo {
        id: v.id.to_string(),
        kind: format!("{:?}", v.kind).to_lowercase(),
        title: v.title,
        params: serde_json::to_value(&v.params).unwrap_or_default(),
    }))
}

pub fn render_view(
    view_id: String,
    service: &Arc<AppService>,
) -> Result<RenderViewResponse, String> {
    let source_ids = service.render_view(&view_id)?;

    Ok(RenderViewResponse { source_ids })
}

pub fn create_graph_view(
    title: String,
    service: &Arc<AppService>,
) -> Result<CreateViewResponse, String> {
    let view_id = service.create_view(ViewKind::GraphView, title.clone(), ViewParams::graph_view())?;

    Ok(CreateViewResponse {
        id: view_id,
        kind: "graph_view".to_string(),
        title,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewCard {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub aliases: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub object_type: String,
}

pub fn search_notes(
    query: String,
    limit: Option<usize>,
    service: &Arc<AppService>,
) -> Result<Vec<SearchResult>, String> {
    let results = service.search_notes(&query, limit)?;

    Ok(results
        .into_iter()
        .map(|(id, object_type, title)| SearchResult { id, object_type, title })
        .collect())
}

pub fn get_preview_card(
    entity_id: String,
    service: &Arc<AppService>,
) -> Result<PreviewCard, String> {
    graph::get_preview_card(service.vault.clone(), service.vault_path.clone(), &entity_id)
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub provenance: String,
    pub ingestion_state: String,
    pub node_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphViewData {
    pub title: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<LinkNetworkEdge>,
}

pub fn get_graph_view_data(
    view_id: String,
    service: &Arc<AppService>,
) -> Result<Option<GraphViewData>, String> {
    let view = service
        .get_view(&view_id)?
        .ok_or_else(|| format!("View not found: {}", view_id))?;

    match graph::get_graph_view_data(service.vault.clone(), service.vault_path.clone(), &view) {
        Some(result) => Ok(Some(GraphViewData {
            title: view.title,
            nodes: result
                .nodes
                .into_iter()
                .map(|(id, title, x, y, provenance, ingestion_state, node_type)| GraphNode {
                    id,
                    title,
                    x,
                    y,
                    provenance,
                    ingestion_state,
                    node_type,
                })
                .collect(),
            edges: result.edges,
        })),
        None => Ok(None),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkNetworkNode {
    pub id: String,
    pub title: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkNetworkEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub link_type: String,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkNetworkData {
    pub nodes: Vec<LinkNetworkNode>,
    pub edges: Vec<LinkNetworkEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CanvasNodeData {
    pub id: String,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color_theme: Option<String>,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CanvasFrameData {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub background_color: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CanvasEdgeData {
    pub id: String,
    pub from_type: String,
    pub from_id: String,
    pub to_type: String,
    pub to_id: String,
    pub routing_style: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CanvasViewRenderData {
    pub title: String,
    pub nodes: Vec<CanvasNodeData>,
    pub edges: Vec<CanvasEdgeData>,
    pub frames: Vec<CanvasFrameData>,
}

pub fn get_link_network(
    root_id: String,
    depth: Option<usize>,
    service: &Arc<AppService>,
) -> Result<LinkNetworkData, String> {
    graph::get_link_network(service.vault.clone(), service.vault_path.clone(), &root_id, depth.unwrap_or(2))
}

pub fn get_neighbors(
    target_id: String,
    depth: Option<usize>,
    service: &Arc<AppService>,
) -> Result<LinkNetworkData, String> {
    graph::get_neighbors(service.vault.clone(), service.vault_path.clone(), &target_id, depth.unwrap_or(1))
}

pub fn get_canvas_view_data(
    view_id: String,
    service: &Arc<AppService>,
) -> Result<Option<CanvasViewRenderData>, String> {
    let view = service
        .get_view(&view_id)?
        .ok_or_else(|| format!("View not found: {}", view_id))?;

    Ok(graph::get_canvas_view_data(service.vault.clone(), service.vault_path.clone(), &view))
}

pub fn get_timeline_view_data(
    view_id: String,
    service: &Arc<AppService>,
) -> Result<Option<TimelineRenderData>, String> {
    let view = service
        .get_view(&view_id)?
        .ok_or_else(|| format!("View not found: {}", view_id))?;

    Ok(graph::get_timeline_view_data(service.vault.clone(), service.vault_path.clone(), &view))
}

#[derive(Debug, Clone, Serialize)]
pub struct BulkIngestionResponse {
    pub count: usize,
    pub message: String,
}

pub fn ingest_bulk_links(
    raw_text: String,
    service: &Arc<AppService>,
) -> Result<BulkIngestionResponse, String> {
    let count = service.ingest_bulk_links(raw_text)?;

    let message = if count == 0 {
        "No URLs found in text".to_string()
    } else {
        format!("Processing {} links in background...", count)
    };

    Ok(BulkIngestionResponse { count, message })
}
