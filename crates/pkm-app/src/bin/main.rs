// Tauri desktop application entry point.
// Wires AppService commands to the Tauri frontend via RPC.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use pkm_app::{commands, AppService};
use pkm_core::view::ViewParams;
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

#[tauri::command]
async fn delete_note(
    note_id: String,
    state: tauri::State<'_, Arc<Mutex<AppService>>>,
) -> Result<(), String> {
    let service = state.inner();
    commands::delete_note(note_id, service).await
}

#[tauri::command]
async fn search_notes(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, Arc<Mutex<AppService>>>,
) -> Result<Vec<commands::SearchResult>, String> {
    let service = state.inner();
    commands::search_notes(query, limit, service).await
}

#[tauri::command]
async fn get_graph_view_data(
    view_id: String,
    state: tauri::State<'_, Arc<Mutex<AppService>>>,
) -> Result<Option<commands::GraphViewData>, String> {
    let service = state.inner();
    commands::get_graph_view_data(view_id, service).await
}

#[tauri::command]
async fn create_view(
    kind: String,
    title: String,
    params: ViewParams,
    state: tauri::State<'_, Arc<Mutex<AppService>>>,
) -> Result<commands::CreateViewResponse, String> {
    let service = state.inner();
    commands::create_view(kind, title, params, service).await
}

#[tauri::command]
async fn list_views(
    limit: Option<usize>,
    state: tauri::State<'_, Arc<Mutex<AppService>>>,
) -> Result<Vec<commands::ViewInfo>, String> {
    let service = state.inner();
    commands::list_views(limit, service).await
}

#[tauri::command]
async fn get_view(
    view_id: String,
    state: tauri::State<'_, Arc<Mutex<AppService>>>,
) -> Result<Option<commands::ViewInfo>, String> {
    let service = state.inner();
    commands::get_view(view_id, service).await
}

#[tauri::command]
async fn render_view(
    view_id: String,
    state: tauri::State<'_, Arc<Mutex<AppService>>>,
) -> Result<commands::RenderViewResponse, String> {
    let service = state.inner();
    commands::render_view(view_id, service).await
}

#[tauri::command]
async fn create_graph_view(
    title: String,
    state: tauri::State<'_, Arc<Mutex<AppService>>>,
) -> Result<commands::CreateViewResponse, String> {
    let service = state.inner();
    commands::create_graph_view(title, service).await
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
            update_note,
            delete_note,
            search_notes,
            get_graph_view_data,
            create_view,
            list_views,
            get_view,
            render_view,
            create_graph_view
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_, _| {})
}
