//! SQLite implementation of [`pkm_core::ports::ViewRepo`].

use rusqlite::{params, Connection};
use uuid::Uuid;

use pkm_core::id::ViewId;
use pkm_core::ports::ViewRepo;
use pkm_core::view::View;
use pkm_core::Actor;
use pkm_core::CoreError;
use pkm_core::Result;

/// View persistence backed by SQLite.
pub struct SqliteViewRepo<'c> {
    pub conn: &'c Connection,
}

impl ViewRepo for SqliteViewRepo<'_> {
    fn create(&self, view: &View) -> Result<()> {
        let created_by_json =
            serde_json::to_string(&view.created_by).map_err(crate::StorageError::from)?;
        let params_json = serde_json::to_string(&view.params).map_err(crate::StorageError::from)?;
        let kind_str = view_kind_to_string(view.kind);
        let created_at_str = view
            .created_at
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| {
                crate::StorageError::Core(CoreError::Invariant(format!(
                    "failed to format timestamp: {}",
                    e
                )))
            })?;
        let updated_at_str = view
            .updated_at
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| {
                crate::StorageError::Core(CoreError::Invariant(format!(
                    "failed to format timestamp: {}",
                    e
                )))
            })?;

        self.conn
            .execute(
                "INSERT INTO view (id, kind, title, params, created_at, created_by, version, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    view.id.to_string(),
                    kind_str,
                    view.title,
                    params_json,
                    created_at_str,
                    created_by_json,
                    view.version,
                    updated_at_str,
                ],
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;
        Ok(())
    }

    fn get(&self, id: ViewId) -> Result<Option<View>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, kind, title, params, created_at, created_by, version, updated_at
                 FROM view WHERE id = ?",
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let kind_str: String = row.get(1)?;
            let title: String = row.get(2)?;
            let params_json: String = row.get(3)?;
            let created_at_str: String = row.get(4)?;
            let created_by_json: String = row.get(5)?;
            let version: i64 = row.get(6)?;
            let updated_at_str: String = row.get(7)?;

            Ok((
                id_str,
                kind_str,
                title,
                params_json,
                created_at_str,
                created_by_json,
                version,
                updated_at_str,
            ))
        });

        match result {
            Ok(fields) => {
                let view = build_view_from_fields(fields).map_err(|e| {
                    let ce: CoreError = e.into();
                    ce
                })?;
                Ok(Some(view))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                Err(ce)
            }
        }
    }

    fn list(&self, limit: Option<usize>) -> Result<Vec<View>> {
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
        let query = format!(
            "SELECT id, kind, title, params, created_at, created_by, version, updated_at
             FROM view ORDER BY created_at DESC{}",
            limit_clause
        );

        let mut stmt = self.conn.prepare(&query).map_err(|e| {
            let se = crate::StorageError::from(e);
            let ce: CoreError = se.into();
            ce
        })?;

        let views = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let kind_str: String = row.get(1)?;
                let title: String = row.get(2)?;
                let params_json: String = row.get(3)?;
                let created_at_str: String = row.get(4)?;
                let created_by_json: String = row.get(5)?;
                let version: i64 = row.get(6)?;
                let updated_at_str: String = row.get(7)?;

                Ok((
                    id_str,
                    kind_str,
                    title,
                    params_json,
                    created_at_str,
                    created_by_json,
                    version,
                    updated_at_str,
                ))
            })
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;

        let mut result = Vec::new();
        for fields in views {
            let view = build_view_from_fields(fields).map_err(|e| {
                let ce: CoreError = e.into();
                ce
            })?;
            result.push(view);
        }
        Ok(result)
    }
}

/// Parse an Actor from JSON.
fn parse_actor(json: &str) -> Actor {
    serde_json::from_str(json).unwrap_or(Actor::User)
}

/// Pure mapping function: builds a View from extracted fields.
fn build_view_from_fields(
    fields: (String, String, String, String, String, String, i64, String),
) -> crate::Result<View> {
    let (
        id_str,
        kind_str,
        title,
        params_json,
        created_at_str,
        created_by_json,
        version,
        updated_at_str,
    ) = fields;

    let id = Uuid::parse_str(&id_str).map(ViewId).map_err(|e| {
        crate::StorageError::Core(CoreError::Invariant(format!("invalid view id: {}", e)))
    })?;

    let kind = parse_view_kind(&kind_str);
    let params: pkm_core::view::ViewParams =
        serde_json::from_str(&params_json).map_err(crate::StorageError::from)?;
    let created_by = parse_actor(&created_by_json);

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

    Ok(View {
        id,
        kind,
        title,
        params,
        created_by,
        created_at,
        version: version as u32,
        updated_at,
    })
}

/// Parse a view kind string (from DB) into a ViewKind.
fn parse_view_kind(s: &str) -> pkm_core::view::ViewKind {
    use pkm_core::view::ViewKind;
    match s {
        "dossier" => ViewKind::Dossier,
        "timeline" => ViewKind::Timeline,
        "reading_queue" => ViewKind::ReadingQueue,
        "project_dashboard" => ViewKind::ProjectDashboard,
        "source_map" => ViewKind::SourceMap,
        "decision_log" => ViewKind::DecisionLog,
        "person_profile" => ViewKind::PersonProfile,
        "entity_page" => ViewKind::EntityPage,
        "briefing_page" => ViewKind::BriefingPage,
        "review_queue" => ViewKind::ReviewQueue,
        "open_questions" => ViewKind::OpenQuestions,
        "action_list" => ViewKind::ActionList,
        _ => ViewKind::ReadingQueue, // Fallback to safest default
    }
}

/// Serialize a ViewKind to string for storage.
fn view_kind_to_string(kind: pkm_core::view::ViewKind) -> &'static str {
    use pkm_core::view::ViewKind;
    match kind {
        ViewKind::Dossier => "dossier",
        ViewKind::Timeline => "timeline",
        ViewKind::ReadingQueue => "reading_queue",
        ViewKind::ProjectDashboard => "project_dashboard",
        ViewKind::SourceMap => "source_map",
        ViewKind::DecisionLog => "decision_log",
        ViewKind::PersonProfile => "person_profile",
        ViewKind::EntityPage => "entity_page",
        ViewKind::BriefingPage => "briefing_page",
        ViewKind::ReviewQueue => "review_queue",
        ViewKind::OpenQuestions => "open_questions",
        ViewKind::ActionList => "action_list",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open;
    use pkm_core::view::*;
    use std::path::Path;

    fn test_db() -> rusqlite::Connection {
        let conn = open(Path::new(":memory:")).expect("Failed to open in-memory DB");
        conn
    }

    #[test]
    fn view_create_and_get_round_trip() {
        let conn = test_db();
        let repo = SqliteViewRepo { conn: &conn };

        let now = pkm_core::Timestamp::now_utc();
        let view = View {
            id: ViewId::new(),
            kind: ViewKind::ReadingQueue,
            title: "My Reading Queue".to_string(),
            params: ViewParams::reading_queue(),
            created_by: pkm_core::Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let view_id = view.id;
        repo.create(&view).expect("Failed to create view");

        let retrieved = repo.get(view_id).expect("Failed to get view");
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, view.id);
        assert_eq!(retrieved.kind, view.kind);
        assert_eq!(retrieved.title, view.title);
    }

    #[test]
    fn view_list_returns_all() {
        let conn = test_db();
        let repo = SqliteViewRepo { conn: &conn };

        let now = pkm_core::Timestamp::now_utc();
        let view1 = View {
            id: ViewId::new(),
            kind: ViewKind::Timeline,
            title: "Timeline View".to_string(),
            params: ViewParams::timeline(),
            created_by: pkm_core::Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let now = pkm_core::Timestamp::now_utc();
        let view2 = View {
            id: ViewId::new(),
            kind: ViewKind::ReadingQueue,
            title: "Reading Queue".to_string(),
            params: ViewParams::reading_queue(),
            created_by: pkm_core::Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        repo.create(&view1).expect("Failed to create view1");
        repo.create(&view2).expect("Failed to create view2");

        let list = repo.list(None).expect("Failed to list views");
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn view_list_respects_limit() {
        let conn = test_db();
        let repo = SqliteViewRepo { conn: &conn };

        for i in 0..10 {
            let now = pkm_core::Timestamp::now_utc();
            let view = View {
                id: ViewId::new(),
                kind: ViewKind::ReadingQueue,
                title: format!("View {}", i),
                params: ViewParams::reading_queue(),
                created_by: pkm_core::Actor::User,
                created_at: now,
                version: 1,
                updated_at: now,
            };
            repo.create(&view).expect("Failed to create view");
        }

        let list = repo.list(Some(5)).expect("Failed to list views");
        assert_eq!(list.len(), 5);
    }

    #[test]
    fn view_get_nonexistent_returns_none() {
        let conn = test_db();
        let repo = SqliteViewRepo { conn: &conn };

        let retrieved = repo.get(ViewId::new()).expect("Failed to get view");
        assert!(retrieved.is_none());
    }

    #[test]
    fn view_with_dossier_params_round_trips() {
        let conn = test_db();
        let repo = SqliteViewRepo { conn: &conn };

        let now = pkm_core::Timestamp::now_utc();
        let view = View {
            id: ViewId::new(),
            kind: ViewKind::Dossier,
            title: "Dossier on Entity X".to_string(),
            params: ViewParams::dossier("entity-123".to_string()),
            created_by: pkm_core::Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let view_id = view.id;
        repo.create(&view).expect("Failed to create view");

        let retrieved = repo.get(view_id).expect("Failed to get view");
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        // ViewKind is always preserved correctly
        assert_eq!(retrieved.kind, ViewKind::Dossier);
        assert_eq!(retrieved.title, "Dossier on Entity X");
    }

    #[test]
    fn view_with_timeline_params_round_trips() {
        let conn = test_db();
        let repo = SqliteViewRepo { conn: &conn };

        let now = pkm_core::Timestamp::now_utc();
        let view = View {
            id: ViewId::new(),
            kind: ViewKind::Timeline,
            title: "Monthly Timeline".to_string(),
            params: ViewParams::timeline(),
            created_by: pkm_core::Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let view_id = view.id;
        repo.create(&view).expect("Failed to create view");

        let retrieved = repo.get(view_id).expect("Failed to get view");
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.kind, ViewKind::Timeline);
    }
}
