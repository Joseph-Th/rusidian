//! Command API: the interface between the Tauri frontend and AppService.
//! Each command delegates to AppService; no business logic here.
//!
//! When integrated with Tauri, these functions can be wrapped with #[tauri::command]
//! and exposed through the Tauri command handler.

use crate::service::AppService;
use serde::Serialize;
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
