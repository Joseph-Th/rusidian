//! Command API: the interface between the Tauri frontend and AppService.
//! Each command delegates to AppService; no business logic here.
//!
//! When integrated with Tauri, these functions can be wrapped with #[tauri::command]
//! and exposed through the Tauri command handler.

use crate::service::AppService;
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
        metadata: note.metadata,
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
    service.update_note(&note_id, title, metadata)
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
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderViewResponse {
    pub source_ids: Vec<String>,
}

/// A single event in a timeline, grouped by date.
/// Used for chronological Gantt chart visualizations.
#[derive(Debug, Clone, Serialize)]
pub struct TimelineEventData {
    /// Unique identifier of this event's source (source ID).
    pub id: String,
    /// Display title of the event.
    pub title: String,
    /// Date when this event was captured/created (RFC3339 format).
    /// TODO: use semantic_date when available for historical events.
    pub date: String,
}

/// Hierarchical timeline data grouped by year and month/week/day.
/// Structure: {year: {period: [events]}}
/// Used for rendering chronological Gantt charts and timeline views.
#[derive(Debug, Clone, Serialize)]
pub struct TimelineRenderData {
    /// Title of the timeline view.
    pub title: String,
    /// Events grouped hierarchically: year → period → list of events.
    /// Period key depends on TimelineGrouping (e.g., "2024-03" for month, "2024-03-15" for day).
    pub events: std::collections::BTreeMap<String, std::collections::BTreeMap<String, Vec<TimelineEventData>>>,
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
        "dossier" => ViewKind::Dossier,
        "project_dashboard" => ViewKind::ProjectDashboard,
        "source_map" => ViewKind::SourceMap,
        "decision_log" => ViewKind::DecisionLog,
        "person_profile" => ViewKind::PersonProfile,
        "entity_page" => ViewKind::EntityPage,
        "briefing_page" => ViewKind::BriefingPage,
        "open_questions" => ViewKind::OpenQuestions,
        "action_list" => ViewKind::ActionList,
        "graph_view" => ViewKind::GraphView,
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
}

pub fn search_notes(
    query: String,
    limit: Option<usize>,
    service: &Arc<AppService>,
) -> Result<Vec<SearchResult>, String> {
    let results = service.search_notes(&query, limit)?;

    Ok(results
        .into_iter()
        .map(|(id, title)| SearchResult { id, title })
        .collect())
}

pub fn get_preview_card(
    entity_id: String,
    service: &Arc<AppService>,
) -> Result<PreviewCard, String> {
    service.get_preview_card(&entity_id)
}

/// A node in the graph view with spatial coordinates and metadata.
/// Used for "X-Ray Vision" to show provenance, ingestion state, and node types.
#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    /// Unique identifier for this node (source ID or entity ID).
    pub id: String,
    /// Display title of the node.
    pub title: String,
    /// X coordinate in the 2D layout (absolute pixels).
    pub x: f64,
    /// Y coordinate in the 2D layout (absolute pixels).
    pub y: f64,
    /// Provenance indicator (e.g., "user_authored", "ai_summary", "UserOrigin", etc.).
    pub provenance: String,
    /// Ingestion state (e.g., "Captured", "Promoted", "Ingested", etc.).
    pub ingestion_state: String,
    /// Type of node (e.g., "source", "entity", "note", "block").
    pub node_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphViewData {
    pub title: String,
    pub nodes: Vec<GraphNode>,
}

pub fn get_graph_view_data(
    view_id: String,
    service: &Arc<AppService>,
) -> Result<Option<GraphViewData>, String> {
    let view = service
        .get_view(&view_id)?
        .ok_or_else(|| format!("View not found: {}", view_id))?;

    match service.get_graph_view_data(&view_id)? {
        Some(nodes) => Ok(Some(GraphViewData {
            title: view.title,
            nodes: nodes
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

/// An edge (directed link) in the link network graph.
/// Used for Argument Trees to show relationships between entities with confidence scores.
#[derive(Debug, Clone, Serialize)]
pub struct LinkNetworkEdge {
    /// Unique identifier for this link.
    pub id: String,
    /// Source entity/node ID.
    pub source: String,
    /// Target entity/node ID.
    pub target: String,
    /// Type of link (e.g., "supports", "contradicts", "derives_from", "mentions").
    /// Frontend uses this to color-code edges (supports=green, contradicts=red).
    pub link_type: String,
    /// Optional confidence score (0.0-1.0) for AI-inferred links.
    /// Frontend can use this to vary line thickness or opacity.
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkNetworkData {
    pub nodes: Vec<LinkNetworkNode>,
    pub edges: Vec<LinkNetworkEdge>,
}

/// A single entry in the provenance chain showing how content was derived.
/// Shows the full lineage: Original Source ← AI Processing ← Summary ← Final Note
#[derive(Debug, Clone, Serialize)]
pub struct ProvenanceEntry {
    /// Unique ID of the source/object in the provenance chain.
    pub id: String,
    /// Display title of this entry.
    pub title: String,
    /// Type of content ("source", "note", "block", "entity").
    pub object_type: String,
    /// Current status of this content ("UserAuthored", "AiSummary", "Reviewed", etc.).
    pub status: String,
    /// Who created/generated this content (user email, agent name, system).
    pub created_by: String,
    /// When this entry was created (RFC3339 format).
    pub created_at: String,
    /// Optional byte range in source that was extracted (if applicable).
    /// Format: "start:end" (inclusive:exclusive)
    pub extraction_span: Option<String>,
}

/// Complete provenance chain for a block, showing its derivation history.
/// Used to visualize "Supply Chain of Truth" - how content traces back to sources.
#[derive(Debug, Clone, Serialize)]
pub struct ProvenanceChainData {
    /// The block/entity being traced.
    pub root_id: String,
    /// Display title of the root.
    pub root_title: String,
    /// Full chain from root back to original sources, in chronological order.
    /// First entry is the root, last entry is the original source.
    pub chain: Vec<ProvenanceEntry>,
}

/// A single link relationship in an entity matrix cell.
/// Shows how two entities are connected with their relationship type and confidence.
#[derive(Debug, Clone, Serialize)]
pub struct MatrixCellLink {
    /// Type of link between row and column entity (e.g., "supports", "part_of").
    pub link_type: String,
    /// Confidence score for this link (0.0-1.0) for AI-inferred relationships.
    pub confidence: f32,
    /// Link ID for potential follow-up queries.
    pub link_id: String,
}

/// Complete entity matrix data showing multi-dimensional relationships.
/// Used for visualizing comparisons like "Organizations vs Products".
#[derive(Debug, Clone, Serialize)]
pub struct EntityMatrixData {
    /// Entity names for rows (left axis).
    pub row_entities: Vec<(String, String)>, // (id, name)
    /// Entity names for columns (top axis).
    pub col_entities: Vec<(String, String)>, // (id, name)
    /// 2D matrix of optional links. matrix[row][col] is Some if link exists between row and col entities.
    /// Indexed as: matrix[row_idx][col_idx]
    pub matrix: Vec<Vec<Option<MatrixCellLink>>>,
}

/// A node placed on the canvas with resolved content data.
/// Used for AI Canvas Views with AI-generated spatial layouts.
#[derive(Debug, Clone, Serialize)]
pub struct CanvasNodeData {
    /// Unique identifier for this node's target object (source ID, entity ID, etc.).
    pub id: String,
    /// Display title of the target object.
    pub title: String,
    /// X coordinate (absolute pixels).
    pub x: f64,
    /// Y coordinate (absolute pixels).
    pub y: f64,
    /// Width when rendered (absolute pixels).
    pub width: f64,
    /// Height when rendered (absolute pixels).
    pub height: f64,
    /// Optional CSS color theme or class for styling.
    pub color_theme: Option<String>,
    /// Type of object ("source", "entity", "note", "block", etc.).
    pub kind: String,
}

/// A visual frame/grouping on the canvas for organizing nodes.
#[derive(Debug, Clone, Serialize)]
pub struct CanvasFrameData {
    /// Unique identifier for this frame.
    pub id: String,
    /// Label displayed for this frame.
    pub label: String,
    /// X coordinate of top-left corner.
    pub x: f64,
    /// Y coordinate of top-left corner.
    pub y: f64,
    /// Width of the frame.
    pub width: f64,
    /// Height of the frame.
    pub height: f64,
    /// Optional background color (CSS hex or named color).
    pub background_color: Option<String>,
}

/// Complete canvas view data ready for frontend rendering.
/// Includes AI-generated node placements and grouping frames.
#[derive(Debug, Clone, Serialize)]
pub struct CanvasViewRenderData {
    /// Title of the canvas view.
    pub title: String,
    /// Nodes placed on the canvas, with resolved content.
    pub nodes: Vec<CanvasNodeData>,
    /// Grouping frames that organize nodes.
    pub frames: Vec<CanvasFrameData>,
}

pub fn get_link_network(
    root_id: String,
    depth: Option<usize>,
    service: &Arc<AppService>,
) -> Result<LinkNetworkData, String> {
    service.get_link_network(&root_id, depth.unwrap_or(2))
}

pub fn get_neighbors(
    target_id: String,
    depth: Option<usize>,
    service: &Arc<AppService>,
) -> Result<LinkNetworkData, String> {
    service.get_neighbors(&target_id, depth.unwrap_or(1))
}

pub fn get_canvas_view_data(
    view_id: String,
    service: &Arc<AppService>,
) -> Result<Option<CanvasViewRenderData>, String> {
    service.get_canvas_view_data(&view_id)
}

pub fn get_timeline_view_data(
    view_id: String,
    service: &Arc<AppService>,
) -> Result<Option<TimelineRenderData>, String> {
    service.get_timeline_view_data(&view_id)
}

pub fn get_provenance_chain(
    block_id: String,
    service: &Arc<AppService>,
) -> Result<ProvenanceChainData, String> {
    service.get_provenance_chain(&block_id)
}

pub fn get_entity_matrix(
    row_kind: String,
    col_kind: String,
    min_confidence: Option<f32>,
    service: &Arc<AppService>,
) -> Result<EntityMatrixData, String> {
    service.get_entity_matrix(&row_kind, &col_kind, min_confidence)
}

#[derive(Debug, Clone, Serialize)]
pub struct BulkIngestionResponse {
    /// Number of URLs extracted and queued for processing.
    pub count: usize,
    /// Human-friendly status message.
    pub message: String,
}

/// Ingest bulk links from pasted text.
/// Extracts all URLs, queues them for concurrent processing in the background.
/// Returns immediately with a count of URLs found.
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

#[derive(Debug, Clone, Serialize)]
pub struct RollbackResponse {
    /// Number of actions rolled back.
    pub rolled_back: usize,
    /// Human-friendly status message.
    pub message: String,
}

/// Rollback recent autonomous ingestion actions.
/// Undoes all agent actions from the past N minutes.
pub fn rollback_autonomous_ingestion(
    minutes: i64,
    service: &Arc<AppService>,
) -> Result<RollbackResponse, String> {
    let rolled_back = service.rollback_recent_autonomous_ingestion(minutes)?;

    let message = if rolled_back == 0 {
        format!("No actions found in the past {} minutes", minutes)
    } else {
        format!("Rolled back {} autonomous ingestion actions", rolled_back)
    };

    Ok(RollbackResponse {
        rolled_back,
        message,
    })
}
