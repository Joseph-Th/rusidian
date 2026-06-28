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
pub mod watcher;
pub mod ingestion;

pub use service::AppService;
pub use watcher::{watch_vault, NoteWatcherEvent};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn service_creates_and_retrieves_note() {
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().expect("invalid path");

        let service =
            AppService::new(db_path_str, None).expect("failed to create AppService with temp db");

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
        use std::collections::BTreeMap;

        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("crud_test.db");
        let db_path_str = db_path.to_str().expect("invalid path");

        let service =
            AppService::new(db_path_str, None).expect("failed to create AppService with temp db");

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
        let mut metadata = BTreeMap::new();
        metadata.insert("tag".to_string(), serde_json::json!("important"));
        metadata.insert("project".to_string(), serde_json::json!("myproject"));

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
            updated_note.metadata.len(),
            2,
            "metadata should have 2 entries"
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

    #[tokio::test]
    async fn vault_watcher_syncs_external_markdown_changes() {
        use std::thread;
        use std::time::Duration;

        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("watcher_test.db");
        let db_path_str = db_path.to_str().expect("invalid path");
        let vault_path = temp_dir.path().to_str().expect("invalid path");

        let service =
            AppService::new(db_path_str, Some(vault_path)).expect("failed to create AppService");

        // Start the vault watcher
        service
            .start_vault_watcher()
            .expect("failed to start watcher");

        // Create a note through the app
        let note_id = service.create_note("External Edit Test".to_string())
            .expect("failed to create note");

        // Get the note to verify it exists
        let original_note = service
            .get_note_full(&note_id)
            .expect("failed to get note")
            .expect("note should exist");

        // Wait for ignore TTL to expire before writing the external edit
        tokio::time::sleep(Duration::from_millis(1200)).await;

        let file_path = temp_dir.path().join("notes").join(original_note.file_name());
        let markdown_content = "---\nid: ".to_string()
            + &original_note.id.to_string()
            + "\ncreated_by: User\ncreated_at: "
            + &original_note.created_at.to_string()
            + "\nmetadata:\n  external: true\n---\n\n# External Edit Test\n\nContent added by external editor!";

        std::fs::write(&file_path, &markdown_content)
            .expect("failed to write external edit");

        // Give the watcher time to detect and process the change
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Verify the database was synced with the external change
        let synced_note = service
            .get_note_full(&note_id)
            .expect("failed to get note after external edit")
            .expect("note should still exist");

        // Check that the metadata was synced
        assert!(
            synced_note.metadata.contains_key("external"),
            "external metadata should have been synced from markdown file"
        );
    }
}
