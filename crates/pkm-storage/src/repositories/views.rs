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
    let params = parse_view_params(&kind_str, &params_json)?;
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

/// Deserialize ViewParams using the stored kind string as a discriminator.
/// This avoids the `#[serde(untagged)]` ambiguity where all-optional param
/// structs (ReadingQueueParams) would match any variant's JSON first.
fn parse_view_params(kind_str: &str, params_json: &str) -> crate::Result<pkm_core::view::ViewParams> {
    use pkm_core::view::*;
    match kind_str {
        "reading_queue" => serde_json::from_str::<ReadingQueueParams>(params_json)
            .map(ViewParams::ReadingQueue)
            .map_err(crate::StorageError::from),
        "review_queue" => serde_json::from_str::<ReviewQueueParams>(params_json)
            .map(ViewParams::ReviewQueue)
            .map_err(crate::StorageError::from),
        "timeline" => serde_json::from_str::<TimelineParams>(params_json)
            .map(ViewParams::Timeline)
            .map_err(crate::StorageError::from),
        "dossier" => serde_json::from_str::<DossierParams>(params_json)
            .map(ViewParams::Dossier)
            .map_err(crate::StorageError::from),
        "project_dashboard" => serde_json::from_str::<ProjectDashboardParams>(params_json)
            .map(ViewParams::ProjectDashboard)
            .map_err(crate::StorageError::from),
        "source_map" => serde_json::from_str::<SourceMapParams>(params_json)
            .map(ViewParams::SourceMap)
            .map_err(crate::StorageError::from),
        "decision_log" => serde_json::from_str::<DecisionLogParams>(params_json)
            .map(ViewParams::DecisionLog)
            .map_err(crate::StorageError::from),
        "person_profile" => serde_json::from_str::<PersonProfileParams>(params_json)
            .map(ViewParams::PersonProfile)
            .map_err(crate::StorageError::from),
        "entity_page" => serde_json::from_str::<EntityPageParams>(params_json)
            .map(ViewParams::EntityPage)
            .map_err(crate::StorageError::from),
        "briefing_page" => serde_json::from_str::<BriefingPageParams>(params_json)
            .map(ViewParams::BriefingPage)
            .map_err(crate::StorageError::from),
        "open_questions" => serde_json::from_str::<OpenQuestionsParams>(params_json)
            .map(ViewParams::OpenQuestions)
            .map_err(crate::StorageError::from),
        "action_list" => serde_json::from_str::<ActionListParams>(params_json)
            .map(ViewParams::ActionList)
            .map_err(crate::StorageError::from),
        "graph_view" => serde_json::from_str::<GraphViewParams>(params_json)
            .map(ViewParams::GraphView)
            .map_err(crate::StorageError::from),
        "canvas_view" => serde_json::from_str::<CanvasViewParams>(params_json)
            .map(ViewParams::CanvasView)
            .map_err(crate::StorageError::from),
        _ => Ok(ViewParams::Stub(StubViewParams)),
    }
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
        "graph_view" => ViewKind::GraphView,
        "canvas_view" => ViewKind::CanvasView,
        other => {
            // Unknown kind stored in DB — this indicates schema drift or corruption.
            // Log and fall back rather than silently serving wrong data.
            eprintln!("pkm-storage: unknown view kind {:?}, defaulting to ReadingQueue", other);
            ViewKind::ReadingQueue
        }
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
        ViewKind::GraphView => "graph_view",
        ViewKind::CanvasView => "canvas_view",
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
    fn graph_view_params_round_trip_through_db() {
        let conn = test_db();
        let repo = SqliteViewRepo { conn: &conn };

        let now = pkm_core::Timestamp::now_utc();
        let positions = vec![NodePosition { id: "n1".to_string(), x: 10.0, y: 20.0 }];
        let params = GraphViewParams::default()
            .with_layout(GraphLayoutType::Circular)
            .with_positions(positions)
            .with_edges(false);
        let view = View {
            id: ViewId::new(),
            kind: ViewKind::GraphView,
            title: "My Graph".to_string(),
            params: ViewParams::GraphView(params.clone()),
            created_by: pkm_core::Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let view_id = view.id;
        repo.create(&view).expect("Failed to create graph view");

        let retrieved = repo.get(view_id).expect("Failed to get graph view").unwrap();
        assert_eq!(retrieved.kind, ViewKind::GraphView);
        match retrieved.params {
            ViewParams::GraphView(p) => {
                assert_eq!(p.layout_type, GraphLayoutType::Circular);
                assert_eq!(p.node_positions.len(), 1);
                assert!(!p.show_edges);
            }
            other => panic!("Expected GraphView params, got {:?}", other),
        }
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

    #[test]
    fn canvas_view_params_round_trip_through_db() {
        use pkm_core::id::NoteId;
        use pkm_core::id::ObjectRef;

        let conn = test_db();
        let repo = SqliteViewRepo { conn: &conn };

        let now = pkm_core::Timestamp::now_utc();
        let note1 = NoteId::new();
        let note2 = NoteId::new();

        let node1 = CanvasNode::new(ObjectRef::Note(note1), 0.0, 0.0, 200.0, 200.0);
        let node2 = CanvasNode::new(ObjectRef::Note(note2), 300.0, 0.0, 200.0, 200.0);

        let frame = CanvasFrame::new(
            "frame-1".to_string(),
            "Research Ideas".to_string(),
            -50.0,
            -50.0,
            600.0,
            350.0,
        )
        .with_background_color("#f5f5f5".to_string());

        let edge =
            CanvasEdgeVisual::new(ObjectRef::Note(note1), ObjectRef::Note(note2), "curved".to_string())
                .with_color("#999999".to_string());

        let params = CanvasViewParams::default()
            .with_nodes(vec![node1, node2])
            .with_frames(vec![frame])
            .with_edge_visuals(vec![edge])
            .with_limit(100);

        let view = View {
            id: ViewId::new(),
            kind: ViewKind::CanvasView,
            title: "My Infinite Canvas".to_string(),
            params: ViewParams::CanvasView(params.clone()),
            created_by: pkm_core::Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let view_id = view.id;
        repo.create(&view).expect("Failed to create canvas view");

        let retrieved = repo.get(view_id).expect("Failed to get canvas view").unwrap();
        assert_eq!(retrieved.kind, ViewKind::CanvasView);
        assert_eq!(retrieved.title, "My Infinite Canvas");

        match retrieved.params {
            ViewParams::CanvasView(p) => {
                assert_eq!(p.nodes.len(), 2);
                assert_eq!(p.frames.len(), 1);
                assert_eq!(p.edge_visuals.len(), 1);
                assert_eq!(p.limit, Some(100));

                // Verify nodes
                assert_eq!(p.nodes[0].x, 0.0);
                assert_eq!(p.nodes[1].x, 300.0);

                // Verify frame
                assert_eq!(p.frames[0].id, "frame-1");
                assert_eq!(p.frames[0].label, "Research Ideas");
                assert_eq!(p.frames[0].background_color, Some("#f5f5f5".to_string()));

                // Verify edge
                assert_eq!(p.edge_visuals[0].routing_style, "curved");
                assert_eq!(p.edge_visuals[0].color, Some("#999999".to_string()));
            }
            other => panic!("Expected CanvasView params, got {:?}", other),
        }
    }
}
