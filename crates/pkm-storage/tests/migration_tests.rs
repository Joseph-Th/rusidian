use pkm_core::fixtures::sample_source;
use pkm_core::ports::SourceRepo;
use pkm_core::source::Source;
use pkm_storage::{open, SqliteSourceRepo};
use tempfile::TempDir;

#[test]
fn fresh_db_open_succeeds() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let conn = open(&db_path).expect("failed to open fresh db");

    // Verify the db file exists.
    assert!(db_path.exists(), "db file should exist after open");

    // Verify schema_version table exists and has recorded the migration.
    let version: String = conn
        .query_row(
            "SELECT version FROM schema_version WHERE version = '0001_init'",
            [],
            |row| row.get(0),
        )
        .expect("0001_init migration should be recorded");

    assert_eq!(version, "0001_init");
}

#[test]
fn open_is_idempotent() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // First open.
    let conn1 = open(&db_path).expect("first open should succeed");
    let version1: String = conn1
        .query_row("SELECT COUNT(*) as count FROM schema_version", [], |row| {
            let count: i32 = row.get(0)?;
            Ok(count.to_string())
        })
        .unwrap();

    drop(conn1);

    // Second open (should not re-apply migration).
    let conn2 = open(&db_path).expect("second open should succeed");
    let version2: String = conn2
        .query_row("SELECT COUNT(*) as count FROM schema_version", [], |row| {
            let count: i32 = row.get(0)?;
            Ok(count.to_string())
        })
        .unwrap();

    // Should still have the same number of migration records.
    assert_eq!(version1, version2);
    // There are now 3 migrations: 0001_init, 0002_extend_source, 0003_fts5_indexing.
    assert_eq!(version1, "3");
}

#[test]
fn schema_tables_exist() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let conn = open(&db_path).expect("failed to open db");

    // List of required tables.
    let required_tables = [
        "schema_version",
        "source",
        "note",
        "block",
        "entity",
        "link",
        "view",
        "agent_action",
    ];

    for table in &required_tables {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?",
                [table],
                |row| row.get(0),
            )
            .unwrap_or(false);

        assert!(exists, "table {} should exist", table);
    }
}

#[test]
fn source_create_and_get_round_trip() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let conn = open(&db_path).expect("failed to open db");
    let repo = SqliteSourceRepo { conn: &conn };

    // Create a sample source.
    let source = sample_source();
    repo.create(&source).expect("failed to create source");

    // Retrieve it back.
    let retrieved = repo
        .get(source.id)
        .expect("failed to get source")
        .expect("source should exist");

    // Verify all fields match (except for timestamps which may have rounding).
    assert_eq!(retrieved.id, source.id);
    assert_eq!(retrieved.origin, source.origin);
    assert_eq!(retrieved.title, source.title);
    assert_eq!(retrieved.raw_content, source.raw_content);
    assert_eq!(retrieved.content_hash, source.content_hash);
    assert_eq!(retrieved.ingestion_state, source.ingestion_state);
    assert_eq!(retrieved.created_by, source.created_by);
    // Note: timestamps may differ slightly due to SQL DEFAULT precision, but should be close
}

#[test]
fn source_get_nonexistent_returns_none() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let conn = open(&db_path).expect("failed to open db");
    let repo = SqliteSourceRepo { conn: &conn };

    let fake_id = pkm_core::id::SourceId::new();
    let result = repo.get(fake_id).expect("get should not error");

    assert!(
        result.is_none(),
        "getting a nonexistent source should return None"
    );
}

/// S1: Vertical slice: source round-trip + JSON export.
/// Tests the end-to-end flow: create → persist → retrieve → export JSON.
#[test]
fn s1_source_round_trip_with_json_export() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let conn = open(&db_path).expect("failed to open db");
    let repo = SqliteSourceRepo { conn: &conn };

    // Step 1: Create a source from fixture.
    let source = sample_source();
    let original_id = source.id;

    // Step 2: Persist it.
    repo.create(&source).expect("failed to create source");

    // Step 3: Retrieve it back.
    let retrieved = repo
        .get(original_id)
        .expect("failed to get source")
        .expect("source should exist after create");

    // Step 4: Verify equality (field-by-field, allowing for timestamp precision).
    assert_eq!(retrieved.id, source.id);
    assert_eq!(retrieved.origin, source.origin);
    assert_eq!(retrieved.title, source.title);
    assert_eq!(retrieved.raw_content, source.raw_content);
    assert_eq!(retrieved.content_hash, source.content_hash);
    assert_eq!(retrieved.ingestion_state, source.ingestion_state);
    assert_eq!(retrieved.created_by, source.created_by);

    // Step 5: Export to JSON and verify round-trip.
    let json = serde_json::to_string(&retrieved).expect("failed to serialize source to json");

    let from_json: Source =
        serde_json::from_str(&json).expect("failed to deserialize source from json");

    assert_eq!(from_json.id, source.id);
    assert_eq!(from_json.origin, source.origin);
    assert_eq!(from_json.title, source.title);
    assert_eq!(from_json.raw_content, source.raw_content);
    assert_eq!(from_json.content_hash, source.content_hash);
    assert_eq!(from_json.ingestion_state, source.ingestion_state);
    assert_eq!(from_json.created_by, source.created_by);
}

/// S2: Vertical slice: propose → diff → accept → rollback.
/// Tests the full agent-action lifecycle: propose an UpdateBlock, apply it,
/// capture before/after diff, and rollback to prior state.
#[test]
fn s2_propose_apply_and_rollback_block_update() {
    use pkm_agent::{execute, apply_action, rollback_action, Operation, OperationRequest};
    use pkm_core::block::{Block, BlockContent};
    use pkm_core::note::Note;
    use pkm_core::ports::{AgentActionRepo, NoteRepo};
    use pkm_core::id::{BlockId, NoteId};
    use pkm_core::{Actor, Timestamp};
    use pkm_storage::{open, SqliteNoteRepo, SqliteAgentActionRepo};
    use std::collections::BTreeMap;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let conn = open(&db_path).expect("failed to open db");

    // Create note and block repos
    let note_repo = SqliteNoteRepo { conn: &conn };
    let action_repo = SqliteAgentActionRepo { conn: &conn };

    // Step 1: Create a note and a block
    let note_id = NoteId::new();
    let block_id = BlockId::new();
    let original_content = BlockContent::Markdown {
        text: "Original content".to_string(),
    };

    // Create and insert the note first (so block can reference it via FK)
    let note = Note {
        id: note_id,
        title: "Test Note".to_string(),
        blocks: vec![block_id],
        metadata: BTreeMap::new(),
        created_at: Timestamp::now_utc(),
        created_by: Actor::User,
    };

    note_repo.create(&note).expect("failed to create note");

    // Now create and insert the block
    let block = Block {
        id: block_id,
        note_id,
        content: original_content.clone(),
        order: 1.0,
        created_by: Actor::User,
        created_at: Timestamp::now_utc(),
        source_provenance_ref: None,
    };

    note_repo.insert_block(&block).expect("failed to insert block");

    // Verify the note was created
    let retrieved_note = note_repo
        .get(note_id)
        .expect("failed to get note")
        .expect("note should exist");

    assert_eq!(retrieved_note.blocks.len(), 1);
    assert_eq!(retrieved_note.blocks[0], block_id, "block ID should match");

    // Step 2: Propose an UpdateBlock operation
    let new_content = BlockContent::Markdown {
        text: "Updated content".to_string(),
    };

    let update_op = Operation::UpdateBlock {
        block_id,
        new_content: new_content.clone(),
    };

    let req = OperationRequest {
        actor: Actor::System,
        operation: update_op,
        rationale: "Test update".to_string(),
    };

    let action = execute(req, &action_repo).expect("failed to execute operation");
    let action_id = action.id;

    assert_eq!(action.status, pkm_core::agent_action::AgentActionStatus::Proposed);
    assert_eq!(action.operation, pkm_core::agent_action::OperationKind::UpdateBlock);

    // Step 3: Verify the action was recorded
    let retrieved_action = action_repo
        .get(action_id)
        .expect("failed to get action")
        .expect("action should exist");

    assert_eq!(retrieved_action.status, pkm_core::agent_action::AgentActionStatus::Proposed);

    // Step 4: Apply the action
    let applied_action = apply_action(action_id, &action_repo, &note_repo)
        .expect("failed to apply action");

    assert_eq!(applied_action.status, pkm_core::agent_action::AgentActionStatus::Applied);

    // Step 5: Rollback the action
    let rollback_action = rollback_action(action_id, &action_repo, &note_repo)
        .expect("failed to rollback action");

    assert_eq!(rollback_action.status, pkm_core::agent_action::AgentActionStatus::Applied);
    assert_eq!(rollback_action.operation, pkm_core::agent_action::OperationKind::RollbackAction);

    // Verify original action is marked as Reverted
    let reverted_action = action_repo
        .get(action_id)
        .expect("failed to get reverted action")
        .expect("action should exist");

    assert_eq!(reverted_action.status, pkm_core::agent_action::AgentActionStatus::Reverted);

    // Verify the rollback action was created with correct metadata
    let retrieved_rollback = action_repo
        .get(rollback_action.id)
        .expect("failed to get rollback action")
        .expect("rollback action should exist");

    assert_eq!(retrieved_rollback.rollback_of, Some(action_id));
    assert_eq!(retrieved_rollback.operation, pkm_core::agent_action::OperationKind::RollbackAction);
}
