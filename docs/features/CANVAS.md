# Infinite Canvas (Spatial Reasoning) Feature

## Architecture

The Infinite Canvas is implemented as a **View**, not a new data type. This design preserves the core safety invariants while enabling rich spatial organization of knowledge.

### Core Principle

A Canvas is a presentation layer built from existing primitives:
- **Canvas Nodes** → Real objects (Notes, Blocks, Entities, Media) via `ObjectRef`
- **Canvas Edges** → Semantic Links with typed relationships
- **Canvas Frames** → Visual grouping constructs (no semantic meaning)
- **Spatial Layout** → View parameters stored as JSON

This means:
✅ No hidden data trapped in canvas JSON
✅ All content remains searchable as markdown
✅ AI agents can manipulate layouts through the audit system
✅ Export format is open (JSON alongside .md files)

## Data Model

### CanvasNode

Represents a single object placed on the canvas at specific spatial coordinates.

```rust
pub struct CanvasNode {
    pub target: ObjectRef,          // What is placed: Note, Block, Entity, Media, etc.
    pub x: f64,                     // X coordinate (absolute pixels)
    pub y: f64,                     // Y coordinate (absolute pixels)
    pub width: f64,                 // Node width (pixels)
    pub height: f64,                // Node height (pixels)
    pub color_theme: Option<String>,// Optional styling class
}
```

### CanvasFrame

A visual grouping construct for organizing nodes into logical sections.

```rust
pub struct CanvasFrame {
    pub id: String,                     // Unique identifier
    pub label: String,                  // Display label
    pub x: f64,                         // Top-left X
    pub y: f64,                         // Top-left Y
    pub width: f64,                     // Frame width
    pub height: f64,                    // Frame height
    pub background_color: Option<String>,// Optional background color
}
```

### CanvasEdgeVisual

Visual routing information for edges. The semantic meaning lives in the Link system.

```rust
pub struct CanvasEdgeVisual {
    pub from: ObjectRef,            // Source node target
    pub to: ObjectRef,              // Target node target
    pub routing_style: String,      // "curved" or "straight"
    pub color: Option<String>,      // Optional edge color
}
```

### CanvasViewParams

The complete spatial layout specification.

```rust
pub struct CanvasViewParams {
    pub nodes: Vec<CanvasNode>,                // Positioned objects
    pub frames: Vec<CanvasFrame>,              // Visual groupings
    pub edge_visuals: Vec<CanvasEdgeVisual>,   // Visual routing
    pub limit: Option<usize>,                  // Max nodes to render
}
```

## Usage Patterns

### 1. Creating a Canvas View

```rust
use pkm_core::view::*;
use pkm_core::id::{NoteId, ObjectRef};

// Create nodes pointing to real objects
let note1 = NoteId::new();
let node1 = CanvasNode::new(
    ObjectRef::Note(note1),
    100.0, 100.0,    // x, y
    200.0, 200.0,    // width, height
);

// Create grouping frames
let frame = CanvasFrame::new(
    "ideas".to_string(),
    "Research Ideas".to_string(),
    0.0, 0.0,
    400.0, 300.0,
).with_background_color("#f5f5f5".to_string());

// Create visual edge routing
let edge = CanvasEdgeVisual::new(
    ObjectRef::Note(note1),
    ObjectRef::Note(note2),
    "curved".to_string(),
).with_color("#999999".to_string());

// Assemble into canvas
let mut canvas = CanvasViewParams::new();
canvas.add_node(node1);
canvas.add_frame(frame);
canvas.add_edge_visual(edge);
```

### 2. Storing a Canvas

Canvas views are persisted like any other view:

```rust
let now = Timestamp::now_utc();
let view = View {
    id: ViewId::new(),
    kind: ViewKind::CanvasView,
    title: "Project Overview".to_string(),
    params: ViewParams::CanvasView(canvas),
    created_by: Actor::User,
    created_at: now,
    version: 1,
    updated_at: now,
};

view_repo.create(&view)?;
```

The `CanvasViewParams` is serialized as JSON in the database, alongside the view metadata.

### 3. AI-Assisted Canvas Organization

Agents can propose canvas reorganizations through the `UpdateView` operation:

```rust
// Agent reads 50 notes and proposes clustering
let proposed_layout = compute_layout(&notes);  // AI logic

// Wrap in AgentAction
let action = AgentAction {
    id: AgentActionId::new(),
    actor: Actor::Agent { name: "claude-sonnet".to_string() },
    operation: OperationKind::UpdateView,
    target: ObjectRef::View(canvas_view_id),
    status: AgentActionStatus::Proposed,
    rationale: "Clustered by concept similarity".to_string(),
    diff: serde_json::to_value(&proposed_layout)?,
    rollback_of: None,
};
```

The UI can then preview the ghosted layout and let the user accept/reject.

## Integration Points

### Storage Layer (`pkm-storage`)

- `SqliteViewRepo` handles `CanvasViewParams` serialization/deserialization
- `parse_view_params()` dispatches on kind string → "canvas_view" → `CanvasViewParams`
- `view_kind_to_string()` serializes `ViewKind::CanvasView` → "canvas_view"

### Rendering Layer (`pkm-app`)

- `PkmService::render_view()` calls `DefaultViewModel::render_canvas_view()`
- Returns source IDs for all nodes on the canvas (in canvas order)

### Agent Layer (`pkm-agent`)

- `UpdateView` operation can modify `CanvasViewParams`
- Agent Bouncer audits layout changes before applying
- User gets preview + accept/reject UI

## Semantic Links

When a user drags an edge on the canvas, the app creates a real `Link`:

```
Canvas UI: User drags from Note A to Note B
    ↓
App prompts: "How are these related?"
    ↓
User selects: "Supports"
    ↓
App fires: Operation::CreateTypedLink {
    from: ObjectRef::Note(A),
    to: ObjectRef::Note(B),
    link_type: LinkType::Supports,
    ...
}
    ↓
Link is stored, appears in backlinks, searchable
```

The `CanvasEdgeVisual` is purely visual routing; the semantic meaning lives in `Link`.

## File Format

When exported, a canvas view is stored as:

```
my-vault/
  notes/
    note-001.md
    note-002.md
    ...
  views/
    Project Overview.canvas.json
```

The `.canvas.json` contains:

```json
{
  "type": "canvas_view",
  "nodes": [
    {
      "target": {"type": "note", "id": "..."},
      "x": 100.0,
      "y": 200.0,
      "width": 200.0,
      "height": 200.0,
      "color_theme": "note-blue"
    },
    ...
  ],
  "frames": [...],
  "edge_visuals": [...],
  "limit": 500
}
```

This format is:
- **Open**: human-readable, tool-parseable JSON
- **Portable**: no vendor lock-in
- **Linkable**: `ObjectRef` pointers to real objects

## Testing

Comprehensive test coverage includes:

### Core (pkm-core)

- `canvas_node_creation_and_styling()` — node structure
- `canvas_frame_creation()` — frame structure
- `canvas_edge_visual_creation()` — edge routing
- `canvas_view_params_serialize_and_deserialize()` — JSON round-trip
- `canvas_view_with_all_components()` — full integration
- `canvas_view_renders_successfully()` — rendering logic

### Storage (pkm-storage)

- `canvas_view_params_round_trip_through_db()` — persistence
  - Creates a canvas with nodes, frames, edges
  - Stores in SQLite
  - Retrieves and verifies all components

## Future Enhancements

### Layout Computation

- Force-directed auto-layout (repel/attract physics)
- Hierarchical tree layout
- Treemap packing for large node sets
- User-guided manual layout refinement

### Canvas as Knowledge Graph

- Real-time backlink inference
- Automatic clustering by entity co-occurrence
- Spatial search ("find notes near this position")
- Mini-maps and pan/zoom navigation

### Collaborative Canvases

- Multi-user simultaneous editing with OT/CRDT
- Presence awareness (who's moving what)
- Conflict resolution on layout disputes

### AI-Driven Features

- "Explain this cluster" → AI summarizes group relationships
- "Find related notes" → AI suggests nodes to add
- "Layout by [criteria]" → AI groups by time, tag, entity, sentiment
- Interactive refinement feedback loop

## Invariants

These properties are maintained by design:

1. **No Hidden Data**: All canvas content is in real Notes/Blocks, not in JSON blobs.
2. **Searchability**: Every node's text is indexed and searchable via vault search.
3. **Auditability**: Layout changes go through `AgentAction` with diffs and rollbacks.
4. **Portability**: Canvas JSON is open format, exportable with markdown.
5. **Semantics Precision**: Link types are explicit (not implicit from visual proximity).

---

**See Also:**
- [AGENTS.md](../AGENTS.md) - Agent action audit model
- [Link Types](../model/LINKS.md) - Semantic relationship taxonomy
- [Views Architecture](../architecture/VIEWS.md) - View system design
