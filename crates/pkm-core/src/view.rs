use std::result::Result as StdResult;

use serde::{Deserialize, Serialize};

use crate::id::{ObjectRef, SourceId, ViewId};
use crate::ingestion::IngestionState;
use crate::source::Source;
use crate::sync::SyncEligible;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    Timeline,
    ReadingQueue,
    ReviewQueue,
    GraphView,
    CanvasView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadingQueueParams {
    pub ingestion_state_filter: Option<IngestionState>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewQueueParams {
    pub ingestion_state_filter: Option<IngestionState>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineGrouping {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineParams {
    pub grouping: TimelineGrouping,
    pub limit: Option<usize>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphLayoutType {
    ForceDirected,
    Hierarchical,
    Circular,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodePosition {
    pub id: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphViewParams {
    pub layout_type: GraphLayoutType,
    pub node_positions: Vec<NodePosition>,
    pub limit: Option<usize>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasNode {
    pub target: ObjectRef,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasFrame {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasEdgeVisual {
    pub from: ObjectRef,
    pub to: ObjectRef,
    pub routing_style: String,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasViewParams {
    pub nodes: Vec<CanvasNode>,
    pub frames: Vec<CanvasFrame>,
    pub edge_visuals: Vec<CanvasEdgeVisual>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewParams {
    ReadingQueue(ReadingQueueParams),
    ReviewQueue(ReviewQueueParams),
    Timeline(TimelineParams),
    GraphView(GraphViewParams),
    CanvasView(CanvasViewParams),
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

    pub fn graph_view() -> Self {
        ViewParams::GraphView(GraphViewParams::default())
    }

    pub fn canvas_view() -> Self {
        ViewParams::CanvasView(CanvasViewParams::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    pub id: ViewId,
    pub kind: ViewKind,
    pub title: String,
    pub params: ViewParams,
    pub created_by: crate::Actor,
    pub created_at: crate::Timestamp,
    pub version: u32,
    pub updated_at: crate::Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewRenderResult {
    pub source_ids: Vec<SourceId>,
}

pub trait ViewModel {
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

    fn render_graph_view(
        params: &GraphViewParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;

    fn render_canvas_view(
        params: &CanvasViewParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String>;
}

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

        if params.reverse_chronological {
            filtered.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));
        } else {
            filtered.sort_by(|a, b| a.captured_at.cmp(&b.captured_at));
        }

        let limit = params.limit.unwrap_or(100);
        let source_ids: Vec<_> = filtered.into_iter().take(limit).map(|s| s.id).collect();

        Ok(ViewRenderResult { source_ids })
    }

    fn render_graph_view(
        params: &GraphViewParams,
        sources: &[Source],
    ) -> StdResult<ViewRenderResult, String> {
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
        let limit = params.limit.unwrap_or(500);

        let mut source_ids_set = std::collections::HashSet::new();
        let mut source_ids_ordered = Vec::new();

        for node in params.nodes.iter().take(limit) {
            if let ObjectRef::Source(source_id) = node.target {
                if !source_ids_set.contains(&source_id) {
                    source_ids_set.insert(source_id);
                    source_ids_ordered.push(source_id);
                }
            }
        }

        if source_ids_ordered.is_empty() {
            let mut sorted = _sources.to_vec();
            sorted.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));
            let source_ids: Vec<_> = sorted.into_iter().take(limit).map(|s| s.id).collect();
            return Ok(ViewRenderResult { source_ids });
        }

        let source_ids: Vec<_> = source_ids_ordered.into_iter().collect();

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
            ViewKind::Timeline,
            ViewKind::ReadingQueue,
            ViewKind::ReviewQueue,
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

        let params = TimelineParams::default();
        let result = DefaultViewModel::render_timeline(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 3);
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

        let params = ReviewQueueParams::default().with_state(IngestionState::AwaitingReview);
        let result = DefaultViewModel::render_review_queue(&params, &sources).unwrap();

        assert_eq!(result.source_ids.len(), 2);
        assert_eq!(result.source_ids[0], sources[0].id);
        assert_eq!(result.source_ids[1], sources[2].id);
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

        assert_eq!(params.nodes.len(), 2);
        assert_eq!(params.frames.len(), 1);
        assert_eq!(params.edge_visuals.len(), 1);

        assert_eq!(params.nodes[0].color_theme, Some("blue".to_string()));
        assert_eq!(params.nodes[1].color_theme, Some("red".to_string()));
        assert_eq!(params.frames[0].background_color, Some("#f5f5f5".to_string()));
        assert_eq!(params.edge_visuals[0].color, Some("#999999".to_string()));

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
