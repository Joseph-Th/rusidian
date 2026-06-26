// Tauri desktop application entry point.
// Wires AppService commands to the Tauri frontend via RPC.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use pkm_app::{commands, AppService};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tauri::menu::Menu;

#[tauri::command]
async fn create_note(
    title: String,
    state: tauri::State<'_, Arc<Mutex<AppService>>>,
) -> Result<commands::CreateNoteResponse, String> {
    let service = state.inner();
    commands::create_note(title, service).await
}

#[tauri::command]
async fn list_notes(
    limit: Option<usize>,
    state: tauri::State<'_, Arc<Mutex<AppService>>>,
) -> Result<Vec<commands::NoteInfo>, String> {
    let service = state.inner();
    commands::list_notes(limit, service).await
}

#[tauri::command]
async fn get_note(
    note_id: String,
    state: tauri::State<'_, Arc<Mutex<AppService>>>,
) -> Result<commands::GetNoteResponse, String> {
    let service = state.inner();
    commands::get_note(note_id, service).await
}

#[tauri::command]
async fn update_note(
    note_id: String,
    title: String,
    metadata: BTreeMap<String, serde_json::Value>,
    state: tauri::State<'_, Arc<Mutex<AppService>>>,
) -> Result<(), String> {
    let service = state.inner();
    commands::update_note(note_id, title, metadata, service).await
}

fn main() {
    let db_path = {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home)
            .join(".pkm")
            .join("pkm.db")
            .to_str()
            .expect("invalid db path")
            .to_string()
    };

    // Ensure the directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let service = Arc::new(Mutex::new(
        AppService::new(&db_path).expect("failed to create AppService"),
    ));

    tauri::Builder::default()
        .menu(Menu::new)
        .manage(service)
        .invoke_handler(tauri::generate_handler![
            create_note,
            list_notes,
            get_note,
            update_note
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_, _| {})
}
