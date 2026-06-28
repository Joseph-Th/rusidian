//! View: a presentation-first rendering of structured information (AGENTS.md
//! "View"). Views are NOT saved searches and NOT one-off layouts — they are
//! reusable, structured renderings built from a shared view model.
//!
//! Red flag (AGENTS.md): "A view is built as a one-off instead of using the
//! view model." Every concrete view (dossier, timeline, ...) must be a
//! `ViewKind`, not bespoke UI code.

use std::result::Result as StdResult;

use serde::{Deserialize, Serialize};

use crate::id::{ObjectRef, SourceId, ViewId};
use crate::ingestion::IngestionState;
use crate::source::Source;
use crate::sync::SyncEligible;

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
    GraphView,
    CanvasView,
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

/// Parameters for SourceMap view: shows source citations and provenance chains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMapParams {
    /// Focus on a specific source ID to trace its derivations (if None, shows all sources).
    pub root_source_id: Option<String>,
    /// Maximum number of items to show.
    pub limit: Option<usize>,
}

impl SourceMapParams {
    pub fn new() -> Self {
        Self {
            root_source_id: None,
            limit: Some(50),
        }
    }

    pub fn with_source(mut self, source_id: String) -> Self {
        self.root_source_id = Some(source_id);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl Default for SourceMapParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameters for DecisionLog view: shows decisions and their context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionLogParams {
    /// Maximum number of items to show.
    pub limit: Option<usize>,
}

impl DecisionLogParams {
    pub fn new() -> Self {
        Self { limit: Some(50) }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl Default for DecisionLogParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameters for PersonProfile view: shows information about a person entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonProfileParams {
    /// The person entity to show.
    pub person_id: String,
    /// Maximum number of items to show.
    pub limit: Option<usize>,
}

impl PersonProfileParams {
    pub fn new(person_id: String) -> Self {
        Self {
            person_id,
            limit: Some(50),
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Parameters for EntityPage view: shows a specific entity and its relationships.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityPageParams {
    /// The entity to show.
    pub entity_id: String,
    /// Maximum number of items to show.
    pub limit: Option<usize>,
}

impl EntityPageParams {
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

/// Parameters for BriefingPage view: shows a summary/briefing on a topic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BriefingPageParams {
    /// Topic or entity to brief on.
    pub topic: String,
    /// Maximum number of items to show.
    pub limit: Option<usize>,
}

impl BriefingPageParams {
    pub fn new(topic: String) -> Self {
        Self {
            topic,
            limit: Some(50),
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Parameters for OpenQuestions view: shows unresolved questions and gaps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenQuestionsParams {
    /// Maximum number of items to show.
    pub limit: Option<usize>,
}

impl OpenQuestionsParams {
    pub fn new() -> Self {
        Self { limit: Some(50) }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl Default for OpenQuestionsParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameters for ActionList view: shows actionable items and tasks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionListParams {
    /// Maximum number of items to show.
    pub limit: Option<usize>,
}

impl ActionListParams {
    pub fn new() -> Self {
        Self { limit: Some(50) }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl Default for ActionListParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Layout type for graph visualization (Phase 7: H5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphLayoutType {
    /// Force-directed graph layout (nodes repel, edges attract).
    ForceDirected,
    /// Hierarchical/tree layout (top-down or left-right).
    Hierarchical,
    /// Circular layout (nodes arranged in rings).
    Circular,
    /// User-positioned custom layout.
    Custom,
}

/// Node position in a graph visualization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodePosition {
    /// Entity or source ID.
    pub id: String,
    /// X coordinate (0.0 to 1.0 normalized, or pixel absolute).
    pub x: f64,
    /// Y coordinate (0.0 to 1.0 normalized, or pixel absolute).
    pub y: f64,
}

/// Parameters for GraphView: spatial organization of entities and notes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphViewParams {
    /// Layout algorithm type.
    pub layout_type: GraphLayoutType,
    /// Saved node positions (used for custom layouts).
    pub node_positions: Vec<NodePosition>,
    /// Maximum number of nodes to show.
    pub limit: Option<usize>,
    /// Show/hide edges between nodes.
    pub show_edges: bool,
}

impl GraphViewParams {
    pub fn new() -> Self {
        Self {
            layout_type: GraphLayoutType::ForceDirected,
            node_positions: Vec::new(),
            limit: Some(100),
            show_edges: true,
        }
    }

    pub fn with_layout(mut self, layout: GraphLayoutType) -> Self {
        self.layout_type = layout;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_edges(mut self, show: bool) -> Self {
        self.show_edges = show;
        self
    }

    pub fn with_positions(mut self, positions: Vec<NodePosition>) -> Self {
        self.node_positions = positions;
        self
    }
}

impl Default for GraphViewParams {
    fn default() -> Self {
        Self::new()
    }
}

/// A node placed on the canvas. Points to an ObjectRef (Note, Block, Entity, Media, etc.)
/// at specific spatial coordinates with optional styling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasNode {
    /// What is placed on the canvas: a Note, Block, Entity, Media, or other object.
    pub target: ObjectRef,
    /// X coordinate (absolute pixels).
    pub x: f64,
    /// Y coordinate (absolute pixels).
    pub y: f64,
    /// Width of the node when rendered (absolute pixels).
    pub width: f64,
    /// Height of the node when rendered (absolute pixels).
    pub height: f64,
    /// Optional color theme/class for styling (e.g., "note-blue", "entity-red").
    pub color_theme: Option<String>,
}

impl CanvasNode {
    pub fn new(target: ObjectRef, x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            target,
            x,
            y,
            width,
            height,
            color_theme: None,
        }
    }

    pub fn with_color_theme(mut self, theme: String) -> Self {
        self.color_theme = Some(theme);
        self
    }
}

/// A visual grouping frame on the canvas. Used to organize nodes into sections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasFrame {
    /// Unique identifier for this frame within the canvas.
    pub id: String,
    /// Label displayed for this frame.
    pub label: String,
    /// X coordinate of top-left corner (absolute pixels).
    pub x: f64,
    /// Y coordinate of top-left corner (absolute pixels).
    pub y: f64,
    /// Width of the frame (absolute pixels).
    pub width: f64,
    /// Height of the frame (absolute pixels).
    pub height: f64,
    /// Optional background color (CSS hex or named color).
    pub background_color: Option<String>,
}

impl CanvasFrame {
    pub fn new(id: String, label: String, x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            id,
            label,
            x,
            y,
            width,
            height,
            background_color: None,
        }
    }

    pub fn with_background_color(mut self, color: String) -> Self {
        self.background_color = Some(color);
        self
    }
}

/// Visual routing information for edges (arrows) on the canvas.
/// The semantic meaning of the edge is stored in the Link system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasEdgeVisual {
    /// Source node's target ObjectRef.
    pub from: ObjectRef,
    /// Target node's target ObjectRef.
    pub to: ObjectRef,
    /// Edge routing style: "curved" or "straight".
    pub routing_style: String,
    /// Optional color for this edge (CSS hex or named color).
    pub color: Option<String>,
}

impl CanvasEdgeVisual {
    pub fn new(from: ObjectRef, to: ObjectRef, routing_style: String) -> Self {
        Self {
            from,
            to,
            routing_style,
            color: None,
        }
    }

    pub fn with_color(mut self, color: String) -> Self {
        self.color = Some(color);
        self
    }
}

/// Parameters for CanvasView: an infinite spatial canvas for organizing notes and entities.
///
/// The Canvas is a View, not a data type. Nodes point to real objects (via ObjectRef),
/// edges represent Links (semantic typed relationships), and spatial coordinates are
/// view-specific parameters stored as JSON. This ensures:
/// - No hidden data (all content is in real Notes/Blocks, searchable as markdown)
/// - AI-operable (agents can generate layouts through structured operations)
/// - Open format (canvas JSON exports alongside .md files)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasViewParams {
    /// Nodes placed on the canvas, each pointing to an object.
    pub nodes: Vec<CanvasNode>,
    /// Grouping frames that organize nodes visually.
    pub frames: Vec<CanvasFrame>,
    /// Visual routing data for edges/arrows (semantic meaning lives in Links).
    pub edge_visuals: Vec<CanvasEdgeVisual>,
    /// Maximum number of nodes to show (None = all).
    pub limit: Option<usize>,
}

impl CanvasViewParams {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            frames: Vec::new(),
            edge_visuals: Vec::new(),
            limit: Some(500),
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_nodes(mut self, nodes: Vec<CanvasNode>) -> Self {
        self.nodes = nodes;
        self
    }

    pub fn with_frames(mut self, frames: Vec<CanvasFrame>) -> Self {
        self.frames = frames;
        self
    }

    pub fn with_edge_visuals(mut self, visuals: Vec<CanvasEdgeVisual>) -> Self {
        self.edge_visuals = visuals;
        self
    }

    pub fn add_node(&mut self, node: CanvasNode) {
        self.nodes.push(node);
    }

    pub fn add_frame(&mut self, frame: CanvasFrame) {
        self.frames.push(frame);
    }

    pub fn add_edge_visual(&mut self, visual: CanvasEdgeVisual) {
        self.edge_visuals.push(visual);
    }
}

impl Default for CanvasViewParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Stub params for unimplemented views (task F1+).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StubViewParams;

/// All possible view parameters, one variant per ViewKind.
/// Tagged with `"type"` so the Tauri frontend can unambiguously specify which
/// variant to create (e.g. `{"type": "graph_view", "show_edges": false}`).
/// The storage layer reads params by dispatching on the stored kind string
/// directly (see `parse_view_params`), so DB data is unaffected by this tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewParams {
    ReadingQueue(ReadingQueueParams),
    ReviewQueue(ReviewQueueParams),
    Timeline(TimelineParams),
    Dossier(DossierParams),
    ProjectDashboard(ProjectDashboardParams),
    SourceMap(SourceMapParams),
    DecisionLog(DecisionLogParams),
    PersonProfile(PersonProfileParams),
    EntityPage(EntityPageParams),
    BriefingPage(BriefingPageParams),
    OpenQuestions(OpenQuestionsParams),
    ActionList(ActionListParams),
    GraphView(GraphViewParams),
    CanvasView(CanvasViewParams),
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

    pub fn source_map() -> Self {
        ViewParams::SourceMap(SourceMapParams::default())
    }

    pub fn decision_log() -> Self {
        ViewParams::DecisionLog(DecisionLogParams::default())
    }

    pub fn person_profile(person_id: String) -> Self {
        ViewParams::PersonProfile(PersonProfileParams::new(person_id))
    }

    pub fn entity_page(entity_id: String) -> Self {
        ViewParams::EntityPage(EntityPageParams::new(entity_id))
    }

    pub fn briefing_page(topic: String) -> Self {
        ViewParams::BriefingPage(BriefingPageParams::new(topic))
    }

    pub fn open_questions() -> Self {
        ViewParams::OpenQuestions(OpenQuestionsParams::default())
    }

    pub fn action_list() -> Self {
        ViewParams::ActionList(ActionListParams::default())
    }

    pub fn graph_view() -> Self {
        ViewParams::GraphView(GraphViewParams::default())
    }

    pub fn canvas_view() -> Self {
        ViewParams::CanvasView(CanvasViewParams::default())
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
    /// Who created this view (user or agent).
    pub created_by: crate::Actor,
    /// When this view was created.
    pub created_at: crate::Timestamp,
    /// Current version number (increments on each update).
    pub version: u32,
    /// When this version was created.
    pub updated_at: crate::Timestamp,
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

    fn render_source_map(
        params: &SourceMapParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;

    fn render_decision_log(
        params: &DecisionLogParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;

    fn render_person_profile(
        params: &PersonProfileParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;

    fn render_entity_page(
        params: &EntityPageParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;

    fn render_briefing_page(
        params: &BriefingPageParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;

    fn render_open_questions(
        params: &OpenQuestionsParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;

    fn render_action_list(
        params: &ActionListParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;

    fn render_graph_view(
        params: &GraphViewParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;

    fn render_canvas_view(
        params: &CanvasViewParams,
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

        // Sort by captured_at; direction depends on reverse_chronological flag.
        // In production, this would check note/entity metadata for semantic_date first,
        // falling back to captured_at if not available. Semantic date represents when
        // the event actually occurred, not when the source was captured.
        if params.reverse_chronological {
            filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));
        } else {
            filtered.sort_by(|a, b| a.captured_at.cmp(&b.captured_at));
        }

        let limit = params.limit.unwrap_or(100);

        // Group by TimelineGrouping (Day, Week, Month, Year) if needed.
        // This structure could be returned as hierarchical JSON to the frontend
        // for rendering as a Gantt chart or vertical timeline blocks.
        let source_ids: Vec<_> = filtered
            .into_iter()
            .take(limit)
            .map(|s| s.id)
            .collect();

        // TODO(Phase 6): Return hierarchical grouped structure instead of flat list
        // Example structure:
        // {
        //   "2024": {
        //     "March": [
        //       { "id": "source-1", "title": "Event 1", "date": "2024-03-15" }
        //     ]
        //   }
        // }
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

    fn render_source_map(
        params: &SourceMapParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        // Placeholder: until typed links with link chains are fully implemented,
        // source map shows all sources. In production, this would trace the provenance
        // chain from a root source through derived works and links to final notes.
        let mut filtered: Vec<_> = sources.iter().collect();

        // Sort by captured_at descending (most recent first)
        filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));

        let limit = params.limit.unwrap_or(50);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }

    fn render_decision_log(
        params: &DecisionLogParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        // Placeholder: decision log shows all sources. In production, this would filter
        // for sources and notes marked as decisions.
        let mut filtered: Vec<_> = sources.iter().collect();
        filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));

        let limit = params.limit.unwrap_or(50);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }

    fn render_person_profile(
        params: &PersonProfileParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        // Placeholder: person profile shows all sources. In production, this would filter
        // for sources and notes related to the specified person entity.
        let mut filtered: Vec<_> = sources.iter().collect();
        filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));

        let limit = params.limit.unwrap_or(50);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }

    fn render_entity_page(
        params: &EntityPageParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        // Placeholder: entity page shows all sources. In production, this would filter
        // for sources and notes related to the specified entity.
        let mut filtered: Vec<_> = sources.iter().collect();
        filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));

        let limit = params.limit.unwrap_or(50);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }

    fn render_briefing_page(
        params: &BriefingPageParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        // Placeholder: briefing page shows all sources. In production, this would filter
        // for sources and notes related to the specified topic.
        let mut filtered: Vec<_> = sources.iter().collect();
        filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));

        let limit = params.limit.unwrap_or(50);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }

    fn render_open_questions(
        params: &OpenQuestionsParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        // Placeholder: open questions shows all sources. In production, this would filter
        // for sources and notes marked as open questions or unresolved.
        let mut filtered: Vec<_> = sources.iter().collect();
        filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));

        let limit = params.limit.unwrap_or(50);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }

    fn render_action_list(
        params: &ActionListParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        // Placeholder: action list shows all sources. In production, this would filter
        // for sources and notes marked as actionable items or tasks.
        let mut filtered: Vec<_> = sources.iter().collect();
        filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));

        let limit = params.limit.unwrap_or(50);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }

    fn render_graph_view(
        params: &GraphViewParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        // Placeholder: graph view shows sources for spatial visualization.
        // In production, this would compute layout positions using the layout_type
        // (force-directed, hierarchical, etc.) and return nodes in rendering order.
        let mut filtered: Vec<_> = sources.iter().collect();
        filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));

        let limit = params.limit.unwrap_or(100);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }

    fn render_canvas_view(
        params: &CanvasViewParams,
        _sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
        // Canvas view renders nodes placed on an infinite spatial canvas.
        // Nodes are AI-generated placements referencing real objects (Notes, Blocks, Entities, Media).
        // Return sources referenced in the canvas nodes plus any referenced sources,
        // ordered by the canvas node list for proper rendering.

        let limit = params.limit.unwrap_or(500);

        // Collect unique source IDs referenced by nodes
        let mut source_ids_set = std::collections::HashSet::new();
        let mut source_ids_ordered = Vec::new();

        // Extract source IDs from canvas nodes (only Source ObjectRefs)
        for node in params.nodes.iter().take(limit) {
            if let ObjectRef::Source(source_id) = node.target {
                if !source_ids_set.contains(&source_id) {
                    source_ids_set.insert(source_id);
                    source_ids_ordered.push(source_id);
                }
            }
        }

        if source_ids_ordered.is_empty() {
            // Fallback to all sources sorted by captured_at descending up to limit
            let mut sorted = _sources.to_vec();
            sorted.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));
            let source_ids: Vec<_> = sorted.into_iter().take(limit).map(|s| s.id).collect();
            return Ok(ViewRenderResult { source_ids });
        }

        // Convert to string representation and return in rendering order
        let source_ids: Vec<_> = source_ids_ordered
            .into_iter()
            .collect();

        Ok(ViewRenderResult { source_ids })
    }
}

impl SyncEligible for View {
    fn version(&self) -> u32 {
        self.version
    }

    fn updated_at(&self) -> crate::Timestamp {
        self.updated_at
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
            ViewKind::GraphView,
            ViewKind::CanvasView,
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
        let now = crate::Timestamp::now_utc();
        let view = View {
            id: ViewId::new(),
            kind: ViewKind::ReadingQueue,
            title: "My Reading Queue".to_string(),
            params: ViewParams::reading_queue(),
            created_by: crate::Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };
        let json = serde_json::to_string(&view).unwrap();
        let back: View = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, view.kind);
        assert_eq!(back.title, view.title);
        assert_eq!(back.version, view.version);
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
                created_at: now,
                version: 1,
                updated_at: now,
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
                created_at: now,
                version: 1,
                updated_at: now,
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
                created_at: now,
                version: 1,
                updated_at: now,
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
            created_at: now,
            version: 1,
            updated_at: now,
        }];

        let now = crate::Timestamp::now_utc();
        let view = View {
            id: ViewId::new(),
            kind: ViewKind::ReadingQueue,
            title: "My Reading Queue".to_string(),
            params: ViewParams::reading_queue(),
            created_by: crate::Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
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
            created_at: base,
            version: 1,
            updated_at: base,
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
            created_at: base,
            version: 1,
            updated_at: base,
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
            created_at: base,
            version: 1,
            updated_at: base,
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
                created_at: base,
                version: 1,
                updated_at: base,
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
                created_at: now,
                version: 1,
                updated_at: now,
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
                created_at: now,
                version: 1,
                updated_at: now,
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
                created_at: now,
                version: 1,
                updated_at: now,
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
                created_at: now,
                version: 1,
                updated_at: now,
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
                created_at: now,
                version: 1,
                updated_at: now,
            })
            .collect();

        // Render project dashboard (placeholder: shows all sources)
        let params = ProjectDashboardParams::default().with_limit(20);
        let result = DefaultViewModel::render_project_dashboard(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 20);
        // Should be sorted by captured_at descending (most recent first)
        assert_eq!(result.source_ids[0], sources[0].id);
    }

    #[test]
    fn source_map_params_serialize_and_deserialize() {
        let params = SourceMapParams::default()
            .with_source("550e8400-e29b-41d4-a716-446655440000".to_string())
            .with_limit(25);
        let json = serde_json::to_string(&params).unwrap();
        let back: SourceMapParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back, params);
    }

    #[test]
    fn source_map_displays_link_chain() {
        let now = Timestamp::now_utc();
        let sources: Vec<_> = (0..30)
            .map(|i| Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some(format!("Source {}", i)),
                raw_content: format!("content{}", i),
                captured_at: now - time::Duration::days(i as i64),
                content_hash: format!("hash{}", i),
                ingestion_state: IngestionState::Promoted,
                created_by: Actor::User,
                created_at: now,
                version: 1,
                updated_at: now,
            })
            .collect();

        // Render source map (placeholder: shows all sources)
        let params = SourceMapParams::default().with_limit(15);
        let result = DefaultViewModel::render_source_map(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 15);
        // Should be sorted by captured_at descending (most recent first)
        assert_eq!(result.source_ids[0], sources[0].id);
    }

    #[test]
    fn all_remaining_views_render_successfully() {
        let now = Timestamp::now_utc();
        let sources: Vec<_> = (0..20)
            .map(|i| Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some(format!("Item {}", i)),
                raw_content: format!("content{}", i),
                captured_at: now - time::Duration::days(i as i64),
                content_hash: format!("hash{}", i),
                ingestion_state: IngestionState::Promoted,
                created_by: Actor::User,
                created_at: now,
                version: 1,
                updated_at: now,
            })
            .collect();

        // Test DecisionLog
        let params = DecisionLogParams::default().with_limit(10);
        let result = DefaultViewModel::render_decision_log(&params, &sources).unwrap();
        assert_eq!(result.source_ids.len(), 10);

        // Test PersonProfile
        let params = PersonProfileParams::new("person-123".to_string()).with_limit(10);
        let result = DefaultViewModel::render_person_profile(&params, &sources).unwrap();
        assert_eq!(result.source_ids.len(), 10);

        // Test EntityPage
        let params = EntityPageParams::new("entity-456".to_string()).with_limit(10);
        let result = DefaultViewModel::render_entity_page(&params, &sources).unwrap();
        assert_eq!(result.source_ids.len(), 10);

        // Test BriefingPage
        let params = BriefingPageParams::new("topic-789".to_string()).with_limit(10);
        let result = DefaultViewModel::render_briefing_page(&params, &sources).unwrap();
        assert_eq!(result.source_ids.len(), 10);

        // Test OpenQuestions
        let params = OpenQuestionsParams::default().with_limit(10);
        let result = DefaultViewModel::render_open_questions(&params, &sources).unwrap();
        assert_eq!(result.source_ids.len(), 10);

        // Test ActionList
        let params = ActionListParams::default().with_limit(10);
        let result = DefaultViewModel::render_action_list(&params, &sources).unwrap();
        assert_eq!(result.source_ids.len(), 10);
    }

    #[test]
    fn graph_view_params_serialize_and_deserialize() {
        let positions = vec![
            NodePosition {
                id: "node-1".to_string(),
                x: 100.0,
                y: 200.0,
            },
            NodePosition {
                id: "node-2".to_string(),
                x: 300.0,
                y: 150.0,
            },
        ];
        let params = GraphViewParams::default()
            .with_layout(GraphLayoutType::Hierarchical)
            .with_positions(positions.clone())
            .with_limit(50)
            .with_edges(false);

        let json = serde_json::to_string(&params).unwrap();
        let back: GraphViewParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back.layout_type, GraphLayoutType::Hierarchical);
        assert_eq!(back.node_positions.len(), 2);
        assert_eq!(back.limit, Some(50));
        assert!(!back.show_edges);
    }

    #[test]
    fn graph_view_renders_successfully() {
        let now = Timestamp::now_utc();
        let sources: Vec<_> = (0..30)
            .map(|i| Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some(format!("Node {}", i)),
                raw_content: format!("content{}", i),
                captured_at: now - time::Duration::days(i as i64),
                content_hash: format!("hash{}", i),
                ingestion_state: IngestionState::Promoted,
                created_by: Actor::User,
                created_at: now,
                version: 1,
                updated_at: now,
            })
            .collect();

        let params = GraphViewParams::default()
            .with_layout(GraphLayoutType::ForceDirected)
            .with_limit(15);
        let result = DefaultViewModel::render_graph_view(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 15);
        // Should be sorted by captured_at descending (most recent first)
        assert_eq!(result.source_ids[0], sources[0].id);
    }

    #[test]
    fn graph_layout_types_serialize_and_deserialize() {
        let layouts = vec![
            GraphLayoutType::ForceDirected,
            GraphLayoutType::Hierarchical,
            GraphLayoutType::Circular,
            GraphLayoutType::Custom,
        ];

        for layout in layouts {
            let json = serde_json::to_string(&layout).unwrap();
            let back: GraphLayoutType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, layout);
        }
    }

    #[test]
    fn canvas_node_creation_and_styling() {
        use crate::id::NoteId;

        let note_id = NoteId::new();
        let node = CanvasNode::new(ObjectRef::Note(note_id), 100.0, 200.0, 300.0, 150.0);

        assert_eq!(node.x, 100.0);
        assert_eq!(node.y, 200.0);
        assert_eq!(node.width, 300.0);
        assert_eq!(node.height, 150.0);
        assert_eq!(node.target, ObjectRef::Note(note_id));
        assert_eq!(node.color_theme, None);

        let styled = node.with_color_theme("note-blue".to_string());
        assert_eq!(styled.color_theme, Some("note-blue".to_string()));
    }

    #[test]
    fn canvas_frame_creation() {
        let frame = CanvasFrame::new(
            "frame-1".to_string(),
            "Research Section".to_string(),
            0.0,
            0.0,
            500.0,
            400.0,
        );

        assert_eq!(frame.id, "frame-1");
        assert_eq!(frame.label, "Research Section");
        assert_eq!(frame.x, 0.0);
        assert_eq!(frame.y, 0.0);
        assert_eq!(frame.width, 500.0);
        assert_eq!(frame.height, 400.0);
        assert_eq!(frame.background_color, None);

        let colored = frame.with_background_color("#f0f0f0".to_string());
        assert_eq!(colored.background_color, Some("#f0f0f0".to_string()));
    }

    #[test]
    fn canvas_edge_visual_creation() {
        use crate::id::NoteId;

        let from = ObjectRef::Note(NoteId::new());
        let to = ObjectRef::Note(NoteId::new());

        let edge = CanvasEdgeVisual::new(from, to, "curved".to_string());
        assert_eq!(edge.from, from);
        assert_eq!(edge.to, to);
        assert_eq!(edge.routing_style, "curved");
        assert_eq!(edge.color, None);

        let colored = edge.with_color("#ff0000".to_string());
        assert_eq!(colored.color, Some("#ff0000".to_string()));
    }

    #[test]
    fn canvas_view_params_creation() {
        use crate::id::NoteId;

        let mut params = CanvasViewParams::new();
        assert_eq!(params.nodes.len(), 0);
        assert_eq!(params.frames.len(), 0);
        assert_eq!(params.edge_visuals.len(), 0);
        assert_eq!(params.limit, Some(500));

        let node = CanvasNode::new(ObjectRef::Note(NoteId::new()), 0.0, 0.0, 100.0, 100.0);
        params.add_node(node);
        assert_eq!(params.nodes.len(), 1);
    }

    #[test]
    fn canvas_view_params_serialize_and_deserialize() {
        use crate::id::NoteId;

        let note1 = NoteId::new();
        let note2 = NoteId::new();

        let node1 = CanvasNode::new(ObjectRef::Note(note1), 0.0, 0.0, 200.0, 200.0);
        let node2 = CanvasNode::new(ObjectRef::Note(note2), 300.0, 0.0, 200.0, 200.0);

        let frame = CanvasFrame::new(
            "group-1".to_string(),
            "Ideas".to_string(),
            0.0,
            250.0,
            600.0,
            300.0,
        );

        let edge = CanvasEdgeVisual::new(ObjectRef::Note(note1), ObjectRef::Note(note2), "curved".to_string());

        let params = CanvasViewParams::default()
            .with_nodes(vec![node1, node2])
            .with_frames(vec![frame])
            .with_edge_visuals(vec![edge])
            .with_limit(100);

        let json = serde_json::to_string(&params).unwrap();
        let back: CanvasViewParams = serde_json::from_str(&json).unwrap();

        assert_eq!(back.nodes.len(), 2);
        assert_eq!(back.frames.len(), 1);
        assert_eq!(back.edge_visuals.len(), 1);
        assert_eq!(back.limit, Some(100));
        assert_eq!(back.nodes[0].x, 0.0);
        assert_eq!(back.nodes[1].x, 300.0);
    }

    #[test]
    fn canvas_view_kind_round_trips() {
        let json = serde_json::to_string(&ViewKind::CanvasView).unwrap();
        assert_eq!(json, "\"canvas_view\"");
        let back: ViewKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ViewKind::CanvasView);
    }

    #[test]
    fn view_params_canvas_variant_round_trips() {
        use crate::id::NoteId;

        let node = CanvasNode::new(ObjectRef::Note(NoteId::new()), 10.0, 20.0, 100.0, 100.0);
        let params = ViewParams::CanvasView(CanvasViewParams::default().with_nodes(vec![node]));

        let json = serde_json::to_string(&params).unwrap();
        let back: ViewParams = serde_json::from_str(&json).unwrap();

        match back {
            ViewParams::CanvasView(cv_params) => {
                assert_eq!(cv_params.nodes.len(), 1);
                assert_eq!(cv_params.nodes[0].x, 10.0);
                assert_eq!(cv_params.nodes[0].y, 20.0);
            }
            _ => panic!("Expected CanvasView variant"),
        }
    }

    #[test]
    fn canvas_view_renders_successfully() {
        let now = Timestamp::now_utc();
        let sources: Vec<_> = (0..50)
            .map(|i| Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some(format!("Canvas Item {}", i)),
                raw_content: format!("content{}", i),
                captured_at: now - time::Duration::days(i as i64),
                content_hash: format!("hash{}", i),
                ingestion_state: IngestionState::Promoted,
                created_by: Actor::User,
                created_at: now,
                version: 1,
                updated_at: now,
            })
            .collect();

        let params = CanvasViewParams::default().with_limit(25);
        let result = DefaultViewModel::render_canvas_view(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 25);
        // Should be sorted by captured_at descending (most recent first)
        assert_eq!(result.source_ids[0], sources[0].id);
    }

    #[test]
    fn canvas_view_with_all_components() {
        use crate::id::NoteId;

        let note1 = NoteId::new();
        let note2 = NoteId::new();

        let nodes = vec![
            CanvasNode::new(ObjectRef::Note(note1), 0.0, 0.0, 200.0, 200.0)
                .with_color_theme("blue".to_string()),
            CanvasNode::new(ObjectRef::Note(note2), 250.0, 0.0, 200.0, 200.0)
                .with_color_theme("red".to_string()),
        ];

        let frames = vec![CanvasFrame::new(
            "main".to_string(),
            "Main Concept".to_string(),
            -50.0,
            -50.0,
            600.0,
            350.0,
        )
        .with_background_color("#f5f5f5".to_string())];

        let edges = vec![
            CanvasEdgeVisual::new(ObjectRef::Note(note1), ObjectRef::Note(note2), "curved".to_string())
                .with_color("#999999".to_string()),
        ];

        let params = CanvasViewParams::default()
            .with_nodes(nodes)
            .with_frames(frames)
            .with_edge_visuals(edges)
            .with_limit(100);

        // Verify structure
        assert_eq!(params.nodes.len(), 2);
        assert_eq!(params.frames.len(), 1);
        assert_eq!(params.edge_visuals.len(), 1);

        // Verify styling is preserved
        assert_eq!(params.nodes[0].color_theme, Some("blue".to_string()));
        assert_eq!(params.nodes[1].color_theme, Some("red".to_string()));
        assert_eq!(params.frames[0].background_color, Some("#f5f5f5".to_string()));
        assert_eq!(params.edge_visuals[0].color, Some("#999999".to_string()));

        // Verify serialization roundtrip
        let json = serde_json::to_string(&params).unwrap();
        let back: CanvasViewParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back.nodes.len(), 2);
        assert_eq!(back.frames.len(), 1);
        assert_eq!(back.edge_visuals.len(), 1);
    }

    #[test]
    fn canvas_view_respects_limit() {
        let now = Timestamp::now_utc();
        let sources: Vec<_> = (0..100)
            .map(|i| Source {
                id: SourceId::new(),
                origin: SourceOrigin::PastedText,
                title: Some(format!("Item {}", i)),
                raw_content: format!("content{}", i),
                captured_at: now - time::Duration::days(i as i64),
                content_hash: format!("hash{}", i),
                ingestion_state: IngestionState::Promoted,
                created_by: Actor::User,
                created_at: now,
                version: 1,
                updated_at: now,
            })
            .collect();

        let params = CanvasViewParams::default().with_limit(30);
        let result = DefaultViewModel::render_canvas_view(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 30);
    }
}
