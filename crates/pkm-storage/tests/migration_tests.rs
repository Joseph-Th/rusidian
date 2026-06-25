use pkm_storage::open;
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
        .query_row(
            "SELECT COUNT(*) as count FROM schema_version",
            [],
            |row| {
                let count: i32 = row.get(0)?;
                Ok(count.to_string())
            },
        )
        .unwrap();

    drop(conn1);

    // Second open (should not re-apply migration).
    let conn2 = open(&db_path).expect("second open should succeed");
    let version2: String = conn2
        .query_row(
            "SELECT COUNT(*) as count FROM schema_version",
            [],
            |row| {
                let count: i32 = row.get(0)?;
                Ok(count.to_string())
            },
        )
        .unwrap();

    // Should still have just one migration record.
    assert_eq!(version1, version2);
    assert_eq!(version1, "1");
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
