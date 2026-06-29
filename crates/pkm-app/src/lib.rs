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
pub mod graph;
pub mod service;
pub mod watcher;
pub mod ingestion;

pub use service::AppService;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn service_creates_and_retrieves_note() {
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let vault_path = temp_dir.path().join("vault");
        let vault_path_str = vault_path.to_str().expect("invalid path");

        let service =
            AppService::new(vault_path_str).expect("failed to create AppService");

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

    #[tokio::test]
    async fn crud_workflow_end_to_end() {
        use pkm_core::note::NoteMetadata;

        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let vault_path = temp_dir.path().join("vault");
        let vault_path_str = vault_path.to_str().expect("invalid path");

        let service =
            AppService::new(vault_path_str).expect("failed to create AppService");

        // CREATE: Create a note
        let original_title = "My Note".to_string();
        let note_id = service
            .create_note(original_title.clone())
            .expect("failed to create note");
        assert!(!note_id.is_empty(), "note_id should not be empty");

        // READ: Get the note
        let full_note = service
            .get_note_full(&note_id)
            .expect("failed to get note")
            .expect("note should exist");
        assert_eq!(full_note.title, original_title, "title should match");
        assert_eq!(full_note.version, 1, "initial version should be 1");

        // UPDATE: Update the note title and metadata
        let new_title = "Updated Note".to_string();
        let metadata = NoteMetadata {
            project: Some("myproject".to_string()),
            tags: vec!["important".to_string()],
            priority: None,
            status: None,
        };

        service
            .update_note(&note_id, new_title.clone(), metadata.clone())
            .expect("failed to update note");

        // Verify update worked
        let updated_note = service
            .get_note_full(&note_id)
            .expect("failed to get note")
            .expect("note should exist after update");
        assert_eq!(updated_note.title, new_title, "title should be updated");
        assert_eq!(updated_note.version, 2, "version should increment");
        assert_eq!(
            updated_note.metadata.project.as_deref(),
            Some("myproject"),
            "project should be set"
        );
        assert!(
            updated_note.metadata.tags.contains(&"important".to_string()),
            "tags should contain important"
        );

        // LIST: List all notes
        let notes = service.list_notes(Some(10)).expect("failed to list notes");
        assert!(!notes.is_empty(), "list should contain the created note");

        // DELETE: Delete the note
        service
            .delete_note(&note_id)
            .expect("failed to delete note");

        // Verify delete worked
        let deleted = service
            .get_note(&note_id)
            .expect("failed to check if note exists");
        assert!(deleted.is_none(), "note should not exist after deletion");

        // List should be empty
        let final_list = service.list_notes(Some(10)).expect("failed to list notes");
        assert!(final_list.is_empty(), "list should be empty after deletion");
    }


}
