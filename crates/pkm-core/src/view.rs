//! View: a presentation-first rendering of structured information (AGENTS.md
//! "View"). Views are NOT saved searches and NOT one-off layouts — they are
//! reusable, structured renderings built from a shared view model.
//!
//! Red flag (AGENTS.md): "A view is built as a one-off instead of using the
//! view model." Every concrete view (dossier, timeline, ...) must be a
//! `ViewKind`, not bespoke UI code.

use std::result::Result as StdResult;

use serde::{Deserialize, Serialize};

use crate::id::{SourceId, ViewId};
use crate::ingestion::IngestionState;
use crate::source::Source;

/// The catalog of supported views. Phase 5 fills these in (STATUS.md F-series).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    Dossier,
    Timeline,
    ReadingQueue,
    ProjectDashboard,
    SourceMap,
    DecisionLog,
    PersonProfile,
    EntityPage,
    BriefingPage,
    ReviewQueue,
    OpenQuestions,
    ActionList,
}

/// Parameters for ReadingQueue view: shows sources awaiting review/reading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadingQueueParams {
    /// Filter by ingestion state (e.g., show only unreviewed sources).
    pub ingestion_state_filter: Option<IngestionState>,
    /// Maximum number of items to show.
    pub limit: Option<usize>,
}

impl ReadingQueueParams {
    pub fn new() -> Self {
        Self {
            ingestion_state_filter: None,
            limit: Some(50),
        }
    }

    pub fn with_state(mut self, state: IngestionState) -> Self {
        self.ingestion_state_filter = Some(state);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl Default for ReadingQueueParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameters for ReviewQueue view: shows items awaiting review/approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewQueueParams {
    /// Filter by ingestion state (e.g., show only awaiting_review sources).
    pub ingestion_state_filter: Option<IngestionState>,
    /// Maximum number of items to show.
    pub limit: Option<usize>,
}

impl ReviewQueueParams {
    pub fn new() -> Self {
        Self {
            ingestion_state_filter: None,
            limit: Some(50),
        }
    }

    pub fn with_state(mut self, state: IngestionState) -> Self {
        self.ingestion_state_filter = Some(state);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl Default for ReviewQueueParams {
    fn default() -> Self {
        Self::new()
    }
}

/// How to group timeline events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineGrouping {
    Day,
    Week,
    Month,
    Year,
}

/// Parameters for Timeline view: shows notes in chronological order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineParams {
    /// How to group events in the timeline.
    pub grouping: TimelineGrouping,
    /// Maximum number of items to show.
    pub limit: Option<usize>,
    /// If true, show newest first; if false, show oldest first.
    pub reverse_chronological: bool,
}

impl TimelineParams {
    pub fn new() -> Self {
        Self {
            grouping: TimelineGrouping::Month,
            limit: Some(100),
            reverse_chronological: true,
        }
    }

    pub fn with_grouping(mut self, grouping: TimelineGrouping) -> Self {
        self.grouping = grouping;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_chronological(mut self, reverse: bool) -> Self {
        self.reverse_chronological = reverse;
        self
    }
}

impl Default for TimelineParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameters for Dossier view: shows notes focused on a particular entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DossierParams {
    /// The entity that is the focus of this dossier.
    pub entity_id: String,
    /// Maximum number of related sources/notes to show.
    pub limit: Option<usize>,
}

impl DossierParams {
    pub fn new(entity_id: String) -> Self {
        Self {
            entity_id,
            limit: Some(50),
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Parameters for ProjectDashboard view: shows status aggregated by project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDashboardParams {
    /// Filter to a specific project name (if None, shows all projects).
    pub project_name: Option<String>,
    /// Maximum number of items to show.
    pub limit: Option<usize>,
}

impl ProjectDashboardParams {
    pub fn new() -> Self {
        Self {
            project_name: None,
            limit: Some(100),
        }
    }

    pub fn with_project(mut self, project_name: String) -> Self {
        self.project_name = Some(project_name);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl Default for ProjectDashboardParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Stub params for unimplemented views (task F1+).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StubViewParams;

/// All possible view parameters, one variant per ViewKind.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ViewParams {
    ReadingQueue(ReadingQueueParams),
    ReviewQueue(ReviewQueueParams),
    Timeline(TimelineParams),
    Dossier(DossierParams),
    ProjectDashboard(ProjectDashboardParams),
    // Stub: other views will be implemented in F1+
    Stub(StubViewParams),
}

impl ViewParams {
    pub fn reading_queue() -> Self {
        ViewParams::ReadingQueue(ReadingQueueParams::default())
    }

    pub fn review_queue() -> Self {
        ViewParams::ReviewQueue(ReviewQueueParams::default())
    }

    pub fn timeline() -> Self {
        ViewParams::Timeline(TimelineParams::default())
    }

    pub fn dossier(entity_id: String) -> Self {
        ViewParams::Dossier(DossierParams::new(entity_id))
    }

    pub fn project_dashboard() -> Self {
        ViewParams::ProjectDashboard(ProjectDashboardParams::default())
    }
}

/// A view is a saved spec (kind + typed parameters) that the render layer turns
/// into a presentation. Each ViewKind has its own parameter type defined above.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    pub id: ViewId,
    pub kind: ViewKind,
    pub title: String,
    /// Typed, kind-specific parameters (filters, entity focus, date range).
    pub params: ViewParams,
}

/// Result of rendering a view: a list of source IDs in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewRenderResult {
    /// Ordered list of source IDs matching the view criteria.
    pub source_ids: Vec<SourceId>,
}

/// ViewModel trait: renders a view given its parameters and access to data repos.
/// Every view type goes through this shared interface (AGENTS.md red flag prevention).
pub trait ViewModel {
    /// Render a view with given parameters, returning a list of matching source IDs.
    /// Implementations should filter/sort according to the view's parameters.
    fn render_reading_queue(
        params: &ReadingQueueParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;

    fn render_review_queue(
        params: &ReviewQueueParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;

    fn render_timeline(
        params: &TimelineParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;

    fn render_dossier(
        params: &DossierParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;

    fn render_project_dashboard(
        params: &ProjectDashboardParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;
}

/// Default ViewModel implementation.
pub struct DefaultViewModel;

impl ViewModel for DefaultViewModel {
    fn render_reading_queue(
        params: &ReadingQueueParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        let mut filtered: Vec<_> = sources
            .iter()
            .filter(|s| {
                if let Some(state) = params.ingestion_state_filter {
                    s.ingestion_state == state
                } else {
                    true
                }
            })
            .collect();

        // Sort by captured_at descending (most recent first)
        filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));

        let limit = params.limit.unwrap_or(50);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }

    fn render_review_queue(
        params: &ReviewQueueParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        let mut filtered: Vec<_> = sources
            .iter()
            .filter(|s| {
                if let Some(state) = params.ingestion_state_filter {
                    s.ingestion_state == state
                } else {
                    true
                }
            })
            .collect();

        // For review queue, sort by captured_at descending (most recent first)
        filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));

        let limit = params.limit.unwrap_or(50);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }

    fn render_timeline(
        params: &TimelineParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        let mut filtered: Vec<_> = sources.iter().collect();

        // Sort by captured_at; direction depends on reverse_chronological flag
        if params.reverse_chronological {
            filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));
        } else {
            filtered.sort_by(|a, b| a.captured_at.cmp(&b.captured_at));
        }

        let limit = params.limit.unwrap_or(100);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }

    fn render_dossier(
        params: &DossierParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        // Placeholder: until entity-source linking is implemented in B2/E-series,
        // dossier shows all sources. In production, this would filter by entity references.
        let mut filtered: Vec<_> = sources.iter().collect();

        // Sort by captured_at descending (most recent first)
        filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));

        let limit = params.limit.unwrap_or(50);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }

    fn render_project_dashboard(
        params: &ProjectDashboardParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        // Placeholder: until note metadata with project assignments is accessible through sources,
        // project dashboard shows all sources. In production, this would filter by project names
        // stored in note metadata and aggregated by project status.
        let mut filtered: Vec<_> = sources.iter().collect();

        // Sort by captured_at descending (most recent first)
        filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));

        let limit = params.limit.unwrap_or(100);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SourceId;
    use crate::ingestion::IngestionState;
    use crate::source::SourceOrigin;
    use crate::{Actor, Timestamp};

    #[test]
    fn view_kind_round_trips_as_snake_case() {
        for kind in [
            ViewKind::Dossier,
            ViewKind::Timeline,
            ViewKind::ReadingQueue,
            ViewKind::ProjectDashboard,
            ViewKind::SourceMap,
            ViewKind::DecisionLog,
            ViewKind::PersonProfile,
            ViewKind::EntityPage,
            ViewKind::BriefingPage,
            ViewKind::ReviewQueue,
            ViewKind::OpenQuestions,
            ViewKind::ActionList,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: ViewKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, kind);
        }
    }

    #[test]
    fn reading_queue_params_serialize_and_deserialize() {
        let params = ReadingQueueParams::default()
            .with_state(IngestionState::Captured)
            .with_limit(25);
        let json = serde_json::to_string(&params).unwrap();
        let back: ReadingQueueParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back, params);
    }

    #[test]
    fn view_with_typed_params_round_trips() {
        let view = View {
            id: ViewId::new(),
            kind: ViewKind::ReadingQueue,
            title: "My Reading Queue".to_string(),
            params: ViewParams::reading_queue(),
        };
        let json = serde_json::to_string(&view).unwrap();
        let back: View = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, view.kind);
        assert_eq!(back.title, view.title);
    }

    #[test]
    fn default_view_model_filters_by_ingestion_state() {
        // Create test sources with different ingestion states
        let now = Timestamp::now_utc();
        let sources = vec![
            Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some("Source 1".to_string()),
                raw_content: "content1".to_string(),
                captured_at: now,
                content_hash: "hash1".to_string(),
                ingestion_state: IngestionState::Captured,
                created_by: Actor::User,
            },
            Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some("Source 2".to_string()),
                raw_content: "content2".to_string(),
                captured_at: now,
                content_hash: "hash2".to_string(),
                ingestion_state: IngestionState::Indexed,
                created_by: Actor::User,
            },
        ];

        // Render with filter for Captured state
        let params = ReadingQueueParams::default().with_state(IngestionState::Captured);
        let result = DefaultViewModel::render_reading_queue(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 1);
        assert_eq!(result.source_ids[0], sources[0].id);
    }

    #[test]
    fn default_view_model_respects_limit() {
        let now = Timestamp::now_utc();
        let sources: Vec<_> = (0..100)
            .map(|i| Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some(format!("Source {}", i)),
                raw_content: format!("content{}", i),
                captured_at: now,
                content_hash: format!("hash{}", i),
                ingestion_state: IngestionState::Captured,
                created_by: Actor::User,
            })
            .collect();

        let params = ReadingQueueParams::default().with_limit(25);
        let result = DefaultViewModel::render_reading_queue(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 25);
    }

    #[test]
    fn reading_queue_view_renders_correctly() {
        let now = Timestamp::now_utc();
        let sources = vec![Source {
            id: SourceId::new(),
            origin: SourceOrigin::PastedText,
            title: Some("Reading Item".to_string()),
            raw_content: "content".to_string(),
            captured_at: now,
            content_hash: "hash".to_string(),
            ingestion_state: IngestionState::Captured,
            created_by: Actor::User,
        }];

        let view = View {
            id: ViewId::new(),
            kind: ViewKind::ReadingQueue,
            title: "My Reading Queue".to_string(),
            params: ViewParams::reading_queue(),
        };

        // Verify the view can be created with typed params
        assert_eq!(view.kind, ViewKind::ReadingQueue);
        match &view.params {
            ViewParams::ReadingQueue(params) => {
                let result = DefaultViewModel::render_reading_queue(params, &sources).unwrap();
                assert_eq!(result.source_ids.len(), 1);
            }
            _ => panic!("Expected ReadingQueue params"),
        }
    }

    #[test]
    fn timeline_params_serialize_and_deserialize() {
        let params = TimelineParams::default()
            .with_grouping(TimelineGrouping::Week)
            .with_limit(75)
            .with_chronological(false);
        let json = serde_json::to_string(&params).unwrap();
        let back: TimelineParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back, params);
    }

    #[test]
    fn timeline_view_renders_in_reverse_chronological_order() {
        let base = Timestamp::now_utc();
        // Create sources with different timestamps
        let source1 = Source {
            id: SourceId::new(),
            origin: SourceOrigin::PastedText,
            title: Some("Oldest".to_string()),
            raw_content: "content1".to_string(),
            captured_at: base - time::Duration::days(2),
            content_hash: "hash1".to_string(),
            ingestion_state: IngestionState::Captured,
            created_by: Actor::User,
        };
        let source2 = Source {
            id: SourceId::new(),
            origin: SourceOrigin::PastedText,
            title: Some("Middle".to_string()),
            raw_content: "content2".to_string(),
            captured_at: base - time::Duration::days(1),
            content_hash: "hash2".to_string(),
            ingestion_state: IngestionState::Captured,
            created_by: Actor::User,
        };
        let source3 = Source {
            id: SourceId::new(),
            origin: SourceOrigin::PastedText,
            title: Some("Newest".to_string()),
            raw_content: "content3".to_string(),
            captured_at: base,
            content_hash: "hash3".to_string(),
            ingestion_state: IngestionState::Captured,
            created_by: Actor::User,
        };

        let sources = vec![source1.clone(), source2.clone(), source3.clone()];

        // Render with reverse chronological (default)
        let params = TimelineParams::default();
        let result = DefaultViewModel::render_timeline(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 3);
        // Should be newest first
        assert_eq!(result.source_ids[0], source3.id);
        assert_eq!(result.source_ids[1], source2.id);
        assert_eq!(result.source_ids[2], source1.id);
    }

    #[test]
    fn timeline_view_respects_limit() {
        let base = Timestamp::now_utc();
        let sources: Vec<_> = (0..50)
            .map(|i| Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some(format!("Item {}", i)),
                raw_content: format!("content{}", i),
                captured_at: base - time::Duration::days(i as i64),
                content_hash: format!("hash{}", i),
                ingestion_state: IngestionState::Captured,
                created_by: Actor::User,
            })
            .collect();

        let params = TimelineParams::default().with_limit(10);
        let result = DefaultViewModel::render_timeline(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 10);
    }

    #[test]
    fn dossier_params_serialize_and_deserialize() {
        let entity_id = "550e8400-e29b-41d4-a716-446655440000".to_string();
        let params = DossierParams::new(entity_id.clone()).with_limit(25);
        let json = serde_json::to_string(&params).unwrap();
        let back: DossierParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entity_id, entity_id);
        assert_eq!(back.limit, Some(25));
    }

    #[test]
    fn dossier_view_renders_with_limit() {
        let now = Timestamp::now_utc();
        let sources: Vec<_> = (0..100)
            .map(|i| Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some(format!("Related Item {}", i)),
                raw_content: format!("content{}", i),
                captured_at: now - time::Duration::days(i as i64),
                content_hash: format!("hash{}", i),
                ingestion_state: IngestionState::Captured,
                created_by: Actor::User,
            })
            .collect();

        let entity_id = "550e8400-e29b-41d4-a716-446655440000".to_string();
        let params = DossierParams::new(entity_id).with_limit(20);
        let result = DefaultViewModel::render_dossier(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 20);
    }

    #[test]
    fn review_queue_params_serialize_and_deserialize() {
        let params = ReviewQueueParams::default()
            .with_state(IngestionState::AwaitingReview)
            .with_limit(25);
        let json = serde_json::to_string(&params).unwrap();
        let back: ReviewQueueParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back, params);
    }

    #[test]
    fn review_queue_filters_by_ingestion_state() {
        let now = Timestamp::now_utc();
        let sources = vec![
            Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some("Awaiting Review 1".to_string()),
                raw_content: "content1".to_string(),
                captured_at: now,
                content_hash: "hash1".to_string(),
                ingestion_state: IngestionState::AwaitingReview,
                created_by: Actor::User,
            },
            Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some("Promoted".to_string()),
                raw_content: "content2".to_string(),
                captured_at: now,
                content_hash: "hash2".to_string(),
                ingestion_state: IngestionState::Promoted,
                created_by: Actor::User,
            },
            Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some("Awaiting Review 2".to_string()),
                raw_content: "content3".to_string(),
                captured_at: now - time::Duration::days(1),
                content_hash: "hash3".to_string(),
                ingestion_state: IngestionState::AwaitingReview,
                created_by: Actor::User,
            },
        ];

        // Render with filter for AwaitingReview state
        let params = ReviewQueueParams::default().with_state(IngestionState::AwaitingReview);
        let result = DefaultViewModel::render_review_queue(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 2);
        // Should be sorted by captured_at descending (most recent first)
        assert_eq!(result.source_ids[0], sources[0].id);
        assert_eq!(result.source_ids[1], sources[2].id);
    }

    #[test]
    fn project_dashboard_params_serialize_and_deserialize() {
        let params = ProjectDashboardParams::default()
            .with_project("MyProject".to_string())
            .with_limit(75);
        let json = serde_json::to_string(&params).unwrap();
        let back: ProjectDashboardParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back, params);
    }

    #[test]
    fn project_dashboard_aggregates_and_displays_status() {
        let now = Timestamp::now_utc();
        let sources: Vec<_> = (0..50)
            .map(|i| Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some(format!("Item {}", i)),
                raw_content: format!("content{}", i),
                captured_at: now - time::Duration::days(i as i64),
                content_hash: format!("hash{}", i),
                ingestion_state: if i % 2 == 0 {
                    IngestionState::Promoted
                } else {
                    IngestionState::AwaitingReview
                },
                created_by: Actor::User,
            })
            .collect();

        // Render project dashboard (placeholder: shows all sources)
        let params = ProjectDashboardParams::default().with_limit(20);
        let result = DefaultViewModel::render_project_dashboard(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 20);
        // Should be sorted by captured_at descending (most recent first)
        assert_eq!(result.source_ids[0], sources[0].id);
    }
}
