// Tauri desktop application entry point.
// Wires AppService commands to the Tauri frontend via RPC.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use pkm_app::{commands, AppService};
use pkm_core::view::ViewParams;
use std::collections::BTreeMap;
use std::sync::Arc;
use tauri::menu::Menu;

#[tauri::command]
fn create_note(
    title: String,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<commands::CreateNoteResponse, String> {
    let service = state.inner();
    commands::create_note(title, service)
}

#[tauri::command]
fn list_notes(
    limit: Option<usize>,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<Vec<commands::NoteInfo>, String> {
    let service = state.inner();
    commands::list_notes(limit, service)
}

#[tauri::command]
fn get_note(
    note_id: String,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<commands::GetNoteResponse, String> {
    let service = state.inner();
    commands::get_note(note_id, service)
}

#[tauri::command]
fn update_note(
    note_id: String,
    title: String,
    metadata: BTreeMap<String, serde_json::Value>,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<(), String> {
    let service = state.inner();
    commands::update_note(note_id, title, metadata, service)
}

#[tauri::command]
fn delete_note(
    note_id: String,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<(), String> {
    let service = state.inner();
    commands::delete_note(note_id, service)
}

#[tauri::command]
fn search_notes(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<Vec<commands::SearchResult>, String> {
    let service = state.inner();
    commands::search_notes(query, limit, service)
}

#[tauri::command]
fn get_graph_view_data(
    view_id: String,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<Option<commands::GraphViewData>, String> {
    let service = state.inner();
    commands::get_graph_view_data(view_id, service)
}

#[tauri::command]
fn create_view(
    kind: String,
    title: String,
    params: ViewParams,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<commands::CreateViewResponse, String> {
    let service = state.inner();
    commands::create_view(kind, title, params, service)
}

#[tauri::command]
fn list_views(
    limit: Option<usize>,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<Vec<commands::ViewInfo>, String> {
    let service = state.inner();
    commands::list_views(limit, service)
}

#[tauri::command]
fn get_view(
    view_id: String,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<Option<commands::ViewInfo>, String> {
    let service = state.inner();
    commands::get_view(view_id, service)
}

#[tauri::command]
fn render_view(
    view_id: String,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<commands::RenderViewResponse, String> {
    let service = state.inner();
    commands::render_view(view_id, service)
}

#[tauri::command]
fn create_graph_view(
    title: String,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<commands::CreateViewResponse, String> {
    let service = state.inner();
    commands::create_graph_view(title, service)
}

#[tauri::command]
fn get_preview_card(
    entity_id: String,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<commands::PreviewCard, String> {
    let service = state.inner();
    commands::get_preview_card(entity_id, service)
}

#[tauri::command]
fn get_link_network(
    root_id: String,
    depth: Option<usize>,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<commands::LinkNetworkData, String> {
    let service = state.inner();
    commands::get_link_network(root_id, depth, service)
}

#[tauri::command]
fn get_neighbors(
    target_id: String,
    depth: Option<usize>,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<commands::LinkNetworkData, String> {
    let service = state.inner();
    commands::get_neighbors(target_id, depth, service)
}

#[tauri::command]
fn ingest_bulk_links(
    raw_text: String,
    state: tauri::State<'_, Arc<AppService>>,
) -> Result<commands::BulkIngestionResponse, String> {
    let service = state.inner();
    commands::ingest_bulk_links(raw_text, service)
}

fn main() {
    // Use platform-standard data directory for the vault
    let vault_path = {
        let data_dir = if cfg!(target_os = "windows") {
            std::env::var("LOCALAPPDATA")
                .ok()
                .and_then(|path| Some(std::path::PathBuf::from(path)))
                .unwrap_or_else(|| {
                    std::env::var("APPDATA")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|_| std::path::PathBuf::from("."))
                })
        } else if cfg!(target_os = "macos") {
            let home = std::env::var("HOME")
                .unwrap_or_else(|_| ".".to_string());
            std::path::PathBuf::from(home)
                .join("Library")
                .join("Application Support")
        } else {
            std::env::var("XDG_DATA_HOME")
                .ok()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    let home = std::env::var("HOME")
                        .unwrap_or_else(|_| ".".to_string());
                    std::path::PathBuf::from(home)
                        .join(".local")
                        .join("share")
                })
        };

        let vault_dir = data_dir.join("pkm").join("vault");
        let _ = std::fs::create_dir_all(&vault_dir);
        vault_dir
            .to_str()
            .expect("invalid vault path")
            .to_string()
    };

    let service = Arc::new(
        AppService::new(&vault_path).expect("failed to create AppService")
    );

    // File watcher is disabled by default. To enable live external markdown sync,
    // uncomment the start_vault_watcher() call below and add `pub mod watcher` in lib.rs.
    // service.start_vault_watcher().expect("failed to start vault watcher");

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
            create_graph_view,
            get_preview_card,
            get_link_network,
            get_neighbors,
            ingest_bulk_links
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_, _| {})
}
