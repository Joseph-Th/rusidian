//! Command API: the interface between the Tauri frontend and AppService.
//! Each command delegates to AppService; no business logic here.
//!
//! When integrated with Tauri, these functions can be wrapped with #[tauri::command]
//! and exposed through the Tauri command handler.

use crate::service::AppService;
use serde::Serialize;
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
