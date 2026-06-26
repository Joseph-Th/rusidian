//! Command API: the interface between the Tauri frontend and AppService.
//! Each command delegates to AppService; no business logic here.
//!
//! When integrated with Tauri, these functions can be wrapped with #[tauri::command]
//! and exposed through the Tauri command handler.

use crate::service::AppService;
use pkm_core::view::{ViewKind, ViewParams};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

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

pub async fn create_note(
    title: String,
    service: &Arc<Mutex<AppService>>,
) -> Result<CreateNoteResponse, String> {
    let svc = service
        .lock()
        .map_err(|_| "Failed to acquire service lock".to_string())?;

    let note_id = svc.create_note(title.clone())?;

    Ok(CreateNoteResponse { id: note_id, title })
}

pub async fn list_notes(
    limit: Option<usize>,
    service: &Arc<Mutex<AppService>>,
) -> Result<Vec<NoteInfo>, String> {
    let svc = service
        .lock()
        .map_err(|_| "Failed to acquire service lock".to_string())?;

    let notes = svc.list_notes(limit)?;

    Ok(notes
        .into_iter()
        .map(|(id, title)| NoteInfo { id, title })
        .collect())
}

pub async fn get_note(
    note_id: String,
    service: &Arc<Mutex<AppService>>,
) -> Result<GetNoteResponse, String> {
    let svc = service
        .lock()
        .map_err(|_| "Failed to acquire service lock".to_string())?;

    let note = svc
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

pub async fn update_note(
    note_id: String,
    title: String,
    metadata: BTreeMap<String, serde_json::Value>,
    service: &Arc<Mutex<AppService>>,
) -> Result<(), String> {
    let svc = service
        .lock()
        .map_err(|_| "Failed to acquire service lock".to_string())?;

    svc.update_note(&note_id, title, metadata)
}

pub async fn delete_note(
    note_id: String,
    service: &Arc<Mutex<AppService>>,
) -> Result<(), String> {
    let svc = service
        .lock()
        .map_err(|_| "Failed to acquire service lock".to_string())?;

    svc.delete_note(&note_id)
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

pub async fn create_view(
    kind: String,
    title: String,
    params: ViewParams,
    service: &Arc<Mutex<AppService>>,
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
        _ => return Err(format!("Unknown view kind: {}", kind)),
    };

    let svc = service
        .lock()
        .map_err(|_| "Failed to acquire service lock".to_string())?;

    let view_id = svc.create_view(view_kind, title.clone(), params)?;

    Ok(CreateViewResponse {
        id: view_id,
        kind,
        title,
    })
}

pub async fn list_views(
    limit: Option<usize>,
    service: &Arc<Mutex<AppService>>,
) -> Result<Vec<ViewInfo>, String> {
    let svc = service
        .lock()
        .map_err(|_| "Failed to acquire service lock".to_string())?;

    let views = svc.list_views(limit)?;

    Ok(views
        .into_iter()
        .map(|v| ViewInfo {
            id: v.id.to_string(),
            kind: format!("{:?}", v.kind).to_lowercase(),
            title: v.title,
        })
        .collect())
}

pub async fn get_view(
    view_id: String,
    service: &Arc<Mutex<AppService>>,
) -> Result<Option<ViewInfo>, String> {
    let svc = service
        .lock()
        .map_err(|_| "Failed to acquire service lock".to_string())?;

    let view = svc.get_view(&view_id)?;

    Ok(view.map(|v| ViewInfo {
        id: v.id.to_string(),
        kind: format!("{:?}", v.kind).to_lowercase(),
        title: v.title,
    }))
}

pub async fn render_view(
    view_id: String,
    service: &Arc<Mutex<AppService>>,
) -> Result<RenderViewResponse, String> {
    let svc = service
        .lock()
        .map_err(|_| "Failed to acquire service lock".to_string())?;

    let source_ids = svc.render_view(&view_id)?;

    Ok(RenderViewResponse { source_ids })
}

pub async fn create_graph_view(
    title: String,
    service: &Arc<Mutex<AppService>>,
) -> Result<CreateViewResponse, String> {
    let svc = service
        .lock()
        .map_err(|_| "Failed to acquire service lock".to_string())?;

    let view_id = svc.create_view(
        ViewKind::GraphView,
        title.clone(),
        ViewParams::graph_view(),
    )?;

    Ok(CreateViewResponse {
        id: view_id,
        kind: "graph_view".to_string(),
        title,
    })
}
