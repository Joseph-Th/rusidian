//! pkm-app: the Tauri desktop shell for the personal knowledge workbench.
//!
//! This crate provides the service layer and command API that the Tauri frontend
//! uses. It keeps business logic out of the UI layer and centralizes service
//! initialization and management.
//!
//! Architecture:
//! - NO business logic in this layer; all decisions go through service.rs
//! - Commands delegate to AppService; service delegates to ports/repositories
//! - UI-shell wiring only (Tauri setup, window management, command routing)

pub mod commands;
pub mod service;

pub use service::AppService;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_creates_and_retrieves_note() {
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().expect("invalid path");

        let service =
            AppService::new(db_path_str).expect("failed to create AppService with temp db");

        // Create a note
        let title = "Test Note".to_string();
        let note_id = service
            .create_note(title.clone())
            .expect("failed to create note");

        assert!(!note_id.is_empty(), "note_id should not be empty");

        // Retrieve the note
        let retrieved = service.get_note(&note_id).expect("failed to get note");

        assert!(retrieved.is_some(), "note should exist after creation");
        let (id, retrieved_title) = retrieved.unwrap();
        assert_eq!(
            id, note_id,
            "retrieved note id should match created note id"
        );
        assert_eq!(
            retrieved_title, title,
            "retrieved note title should match created title"
        );
    }
}
