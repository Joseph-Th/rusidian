//! SQLite implementation of [`pkm_core::ports::SourceRepo`].

use rusqlite::{params, Connection};
use uuid::Uuid;

use pkm_core::id::SourceId;
use pkm_core::ingestion::IngestionState;
use pkm_core::ports::SourceRepo;
use pkm_core::source::{Source, SourceOrigin};
use pkm_core::Result;
use pkm_core::{Actor, CoreError};

/// Source persistence backed by SQLite.
pub struct SqliteSourceRepo<'c> {
    pub conn: &'c Connection,
}

impl SourceRepo for SqliteSourceRepo<'_> {
    fn create(&self, source: &Source) -> Result<()> {
        let origin_json =
            serde_json::to_string(&source.origin).map_err(crate::StorageError::from)?;
        let created_by_json =
            serde_json::to_string(&source.created_by).map_err(crate::StorageError::from)?;
        let state_str = ingestion_state_to_string(source.ingestion_state);

        // Format timestamps as RFC3339 for storage and parsing consistency.
        let captured_at_str = source
            .captured_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());

        self.conn
            .execute(
                "INSERT INTO source (id, origin, title, raw_content, created_at, created_by,
                                captured_at, content_hash, ingestion_state)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    source.id.to_string(),
                    origin_json,
                    source.title,
                    source.raw_content,
                    captured_at_str.clone(), // created_at and captured_at are the same for now
                    created_by_json,
                    captured_at_str,
                    source.content_hash,
                    state_str,
                ],
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;
        Ok(())
    }

    fn get(&self, id: SourceId) -> Result<Option<Source>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, origin, title, raw_content, created_at, created_by,
                    captured_at, content_hash, ingestion_state, version, updated_at FROM source WHERE id = ?",
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;

        // Note: query_row's closure must return rusqlite::Error, so we manually
        // unwrap the mapping result and propagate errors later.
        let result = stmt.query_row(params![id.to_string()], |row| {
            // Extract all columns first (rusqlite errors are OK here)
            let id_str: String = row.get(0)?;
            let origin_json: String = row.get(1)?;
            let title: Option<String> = row.get(2)?;
            let raw_content: String = row.get(3)?;
            let created_at_str: String = row.get(4)?;
            let created_by_json: String = row.get(5)?;
            let captured_at_str: String = row.get(6)?;
            let content_hash: String = row.get(7)?;
            let ingestion_state_str: String = row.get(8)?;
            let version: i64 = row.get(9)?;
            let updated_at_str: String = row.get(10)?;

            // Parse and return as a tuple to avoid nesting errors
            Ok((
                id_str,
                origin_json,
                title,
                raw_content,
                created_at_str,
                created_by_json,
                captured_at_str,
                content_hash,
                ingestion_state_str,
                version,
                updated_at_str,
            ))
        });

        match result {
            Ok(fields) => {
                let source = build_source_from_fields(fields).map_err(|e| {
                    let ce: CoreError = e.into();
                    ce
                })?;
                Ok(Some(source))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                Err(ce)
            }
        }
    }
}

/// Pure mapping function: builds a Source from extracted fields.
/// Separated for clarity and testability.
fn build_source_from_fields(
    fields: (
        String,
        String,
        Option<String>,
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        String,
    ),
) -> crate::Result<Source> {
    let (
        id_str,
        origin_json,
        title,
        raw_content,
        created_at_str,
        created_by_json,
        captured_at_str,
        content_hash,
        ingestion_state_str,
        version,
        updated_at_str,
    ) = fields;

    let id = Uuid::parse_str(&id_str).map(SourceId).map_err(|e| {
        crate::StorageError::Core(CoreError::Invariant(format!("invalid source id: {}", e)))
    })?;

    let origin: SourceOrigin = serde_json::from_str(&origin_json)?;
    let created_by = parse_actor(&created_by_json);
    let ingestion_state = parse_ingestion_state(&ingestion_state_str);

    let created_at = time::OffsetDateTime::parse(
        &created_at_str,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| {
        crate::StorageError::Core(CoreError::Invariant(format!(
            "invalid timestamp: {}: {}",
            created_at_str, e
        )))
    })?;

    let captured_at = time::OffsetDateTime::parse(
        &captured_at_str,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| {
        crate::StorageError::Core(CoreError::Invariant(format!(
            "invalid timestamp: {}: {}",
            captured_at_str, e
        )))
    })?;

    let updated_at = time::OffsetDateTime::parse(
        &updated_at_str,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| {
        crate::StorageError::Core(CoreError::Invariant(format!(
            "invalid timestamp: {}: {}",
            updated_at_str, e
        )))
    })?;

    Ok(Source {
        id,
        origin,
        title,
        raw_content,
        captured_at,
        content_hash,
        ingestion_state,
        created_by,
        created_at,
        version: version as u32,
        updated_at,
    })
}

/// Convert IngestionState to the persisted snake_case string representation.
fn ingestion_state_to_string(state: IngestionState) -> &'static str {
    match state {
        IngestionState::Captured => "captured",
        IngestionState::Parsed => "parsed",
        IngestionState::Cleaned => "cleaned",
        IngestionState::Indexed => "indexed",
        IngestionState::Summarized => "summarized",
        IngestionState::Classified => "classified",
        IngestionState::Linked => "linked",
        IngestionState::AwaitingReview => "awaiting_review",
        IngestionState::Promoted => "promoted",
        IngestionState::Archived => "archived",
        IngestionState::Rejected => "rejected",
        IngestionState::Failed => "failed",
    }
}

/// Parse ingestion state from string. Defaults to Captured if unrecognized.
fn parse_ingestion_state(s: &str) -> IngestionState {
    match s {
        "captured" => IngestionState::Captured,
        "parsed" => IngestionState::Parsed,
        "cleaned" => IngestionState::Cleaned,
        "indexed" => IngestionState::Indexed,
        "summarized" => IngestionState::Summarized,
        "classified" => IngestionState::Classified,
        "linked" => IngestionState::Linked,
        "awaiting_review" => IngestionState::AwaitingReview,
        "promoted" => IngestionState::Promoted,
        "archived" => IngestionState::Archived,
        "rejected" => IngestionState::Rejected,
        "failed" => IngestionState::Failed,
        _ => IngestionState::Captured,
    }
}

/// Parse actor from JSON. Defaults to User if unrecognized or malformed.
fn parse_actor(json: &str) -> Actor {
    serde_json::from_str(json).unwrap_or(Actor::User)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ingestion_states() {
        assert_eq!(parse_ingestion_state("captured"), IngestionState::Captured);
        assert_eq!(parse_ingestion_state("parsed"), IngestionState::Parsed);
        assert_eq!(
            parse_ingestion_state("awaiting_review"),
            IngestionState::AwaitingReview
        );
        assert_eq!(parse_ingestion_state("invalid"), IngestionState::Captured);
    }

    #[test]
    fn ingestion_state_round_trip() {
        let states = vec![
            IngestionState::Captured,
            IngestionState::Parsed,
            IngestionState::Cleaned,
            IngestionState::Indexed,
            IngestionState::Summarized,
            IngestionState::Classified,
            IngestionState::Linked,
            IngestionState::AwaitingReview,
            IngestionState::Promoted,
            IngestionState::Archived,
            IngestionState::Rejected,
            IngestionState::Failed,
        ];
        for state in states {
            let str = ingestion_state_to_string(state);
            assert_eq!(parse_ingestion_state(str), state);
        }
    }

    #[test]
    fn parse_actors() {
        let user_json = serde_json::to_string(&Actor::User).unwrap();
        assert_eq!(parse_actor(&user_json), Actor::User);

        let agent_json = serde_json::to_string(&Actor::Agent {
            name: "test".to_string(),
        })
        .unwrap();
        let parsed = parse_actor(&agent_json);
        if let Actor::Agent { name } = parsed {
            assert_eq!(name, "test");
        } else {
            panic!("Expected Agent variant");
        }

        // Invalid JSON should default to User
        assert_eq!(parse_actor("invalid json"), Actor::User);
    }
}
