# Rich Blocks Implementation Guide

This guide explains how to work with Rich Block Types (Math, Tables, Media, Embeds) in the Rusidian system.

## Quick Reference

### Block Types and Their Markdown Representation

| Block Type | Rust Example | Markdown | Use Case |
|------------|--------------|----------|----------|
| **Markdown** | `Markdown { text: "Paragraph" }` | Plain text | Paragraphs, lists, quotes |
| **Math** | `Math { expr: "E=mc^2", display: true }` | `$$\nE=mc^2\n$$` | Equations, formulas |
| **Math (inline)** | `Math { expr: "x^2", display: false }` | `$x^2$` | Inline equations |
| **Table** | `Table { headers, rows }` | GFM table | Data, comparisons |
| **Image** | `Media { url: "img.png", alt: "...", Image }` | `![alt](img.png)` | Pictures, diagrams |
| **YouTube** | `ExternalEmbed { url: "...", YouTube }` | Link + comment | Video references |
| **View Embed** | `InternalEmbed { View(...), fallback }` | Comment + blockquote | Kanban, dashboards |

## Creating Rich Blocks

### Via API (Agent/Server)

Create a block using the strongly-typed enum:

```rust
// Creating a table block
let table_block = Block {
    id: BlockId::new(),
    note_id: my_note_id,
    content: BlockContent::Table {
        headers: vec!["Name".into(), "Score".into()],
        rows: vec![
            vec!["Alice".into(), "95".into()],
            vec!["Bob".into(), "87".into()],
        ],
    },
    order: 2.0,
    created_by: Actor::User,
    created_at: Timestamp::now_utc(),
    source_provenance_ref: None,
    version: 1,
    updated_at: Timestamp::now_utc(),
};

// Save via NoteRepo
note_repo.create_block(table_block)?;
```

### Via Markdown Import

Write markdown and let the parser detect rich blocks:

```markdown
# My Note

Here's a table:

| Name | Score |
| --- | --- |
| Alice | 95 |
| Bob | 87 |

<!-- block:uuid-123 -->

And an equation:

$$
E = mc^2
$$

<!-- block:uuid-456 -->
```

When imported:
1. Parser sees the GFM table → creates `BlockContent::Table { ... }`
2. Parser sees `$$ ... $$` → creates `BlockContent::Math { ..., display_mode: true }`
3. Block IDs from comments are preserved
4. Result is fully structured and type-safe

## Editing Rich Blocks

### Table Operations

**Goal**: Update a cell without corrupting the markdown syntax.

❌ **Wrong approach** (string manipulation):
```rust
let md = table_markdown_text;
let new_md = md.replace("Alice", "Alicia"); // Fragile!
```

✓ **Correct approach** (structured data):
```rust
let mut block: Block = note_repo.get_block(block_id)?;
if let BlockContent::Table { ref mut rows, .. } = block.content {
    rows[0][0] = "Alicia".into(); // Type-safe!
}
block.version += 1;
block.updated_at = Timestamp::now_utc();
note_repo.update_block(block)?; // Serialized back to markdown safely
```

The repo's `update_block()` function:
1. Takes the structured `Table { headers, rows }`
2. Calls `serialize_table()` to rebuild the GFM markdown
3. Escapes any pipes: `"if a | b"` → `"if a \| b"`
4. Saves the .md file
5. .md file remains readable in any editor

### Math Operations

**Goal**: Update an equation.

```rust
let mut block: Block = note_repo.get_block(block_id)?;
if let BlockContent::Math { ref mut expression, .. } = block.content {
    expression.push_str(" + 1"); // Safe: no syntax to corrupt
}
block.version += 1;
note_repo.update_block(block)?;
```

Result in .md file:
```markdown
$$
E = mc^2 + 1
$$
```

### Media Operations

**Goal**: Update image alt text or URL.

```rust
let mut block: Block = note_repo.get_block(block_id)?;
if let BlockContent::Media { ref mut alt_text, ref mut hash_or_url, .. } = block.content {
    alt_text.push_str(" (updated)");
    // hash_or_url might point to BlobStore or remote URL
}
block.version += 1;
note_repo.update_block(block)?;
```

Result in .md file:
```markdown
![My image (updated)](blob-hash-abc123.png)
```

## Creating Embeds

### Internal Embeds (Views)

Embed a View (Kanban, Dashboard, etc.) into a note:

```rust
// Create or load a View (e.g., ProjectDashboard with tasks)
let view = View {
    id: ViewId::new(),
    kind: ViewKind::ProjectDashboard {
        params: DashboardParams {
            filtered_by_project: "MyProject".into(),
        },
    },
    // ... other fields
};

// Embed it in a note
let embed_block = Block {
    id: BlockId::new(),
    note_id: my_note_id,
    content: BlockContent::InternalEmbed {
        target: ObjectRef::View(view.id),
        fallback_text: "Project Dashboard: 12 tasks active, 5 completed".into(),
    },
    order: 3.0,
    created_by: Actor::User,
    created_at: Timestamp::now_utc(),
    source_provenance_ref: None,
    version: 1,
    updated_at: Timestamp::now_utc(),
};

note_repo.create_block(embed_block)?;
```

Result in .md file:
```markdown
<!-- embed:internal:View(abcd-efgh) -->

> **[Embedded: View abcd-efgh]**
>
> Project Dashboard: 12 tasks active, 5 completed
>
> *[This is a dynamic view embedded here. Open in app to interact.]*

<!-- block:uuid-789 -->
```

When the note opens in the app:
1. The app sees `InternalEmbed` pointing to the View
2. Looks up the View by ID
3. Queries SQLite for related data (tasks, entities, etc.)
4. Renders the interactive dashboard in-place
5. In markdown-only contexts, users see the fallback text

### External Embeds (YouTube, Twitter, etc.)

Embed a video or tweet:

```rust
let embed_block = Block {
    id: BlockId::new(),
    note_id: my_note_id,
    content: BlockContent::ExternalEmbed {
        url: "https://youtube.com/watch?v=dQw4w9WgXcQ".into(),
        provider: EmbedProvider::YouTube,
    },
    order: 4.0,
    created_by: Actor::User,
    created_at: Timestamp::now_utc(),
    source_provenance_ref: None,
    version: 1,
    updated_at: Timestamp::now_utc(),
};

note_repo.create_block(embed_block)?;
```

Result in .md file:
```markdown
<!-- embed:external:youtube.com:https://youtube.com/watch?v=dQw4w9WgXcQ -->

[https://youtube.com/watch?v=dQw4w9WgXcQ](https://youtube.com/watch?v=dQw4w9WgXcQ) *[YouTube Video]*

<!-- block:uuid-890 -->
```

When the note opens in the app:
1. The app recognizes the URL and provider
2. Embeds the YouTube player inline
3. In markdown-only contexts, users see a clickable link

## Querying and Listing Blocks

### Get all blocks of a certain type

```rust
let note: Note = note_repo.get_note(note_id)?;
let blocks: Vec<Block> = note_repo.get_blocks(&note.blocks)?;

let tables: Vec<&Block> = blocks.iter()
    .filter(|b| matches!(b.content, BlockContent::Table { .. }))
    .collect();

for table_block in tables {
    println!("Table: {:?}", table_block.id);
}
```

### Count specific block types

```rust
let counts = blocks.iter().fold(
    (0, 0, 0, 0, 0, 0),
    |(md, math, table, media, internal, external), b| {
        match &b.content {
            BlockContent::Markdown { .. } => (md + 1, math, table, media, internal, external),
            BlockContent::Math { .. } => (md, math + 1, table, media, internal, external),
            BlockContent::Table { .. } => (md, math, table + 1, media, internal, external),
            BlockContent::Media { .. } => (md, math, table, media + 1, internal, external),
            BlockContent::InternalEmbed { .. } => (md, math, table, media, internal + 1, external),
            BlockContent::ExternalEmbed { .. } => (md, math, table, media, internal, external + 1),
        }
    },
);

println!("Blocks: {} text, {} math, {} tables, {} media, {} internal, {} external",
    counts.0, counts.1, counts.2, counts.3, counts.4, counts.5);
```

## Agent Safety: Block Update Workflow

When an agent (Claude) wants to update a block:

### Step 1: Propose (Agent calls Operation::UpdateBlock)
```rust
Operation::UpdateBlock {
    block_id: uuid-123,
    new_content: BlockContent::Table {
        headers: vec!["Name", "Score"],
        rows: vec![["Alice", "96"], ["Bob", "88"]],
    },
}
```

### Step 2: Review (NoteRepo verifies)
- Validates the new content is well-formed (no unescaped pipes, valid markdown syntax, etc.)
- Computes a JSON diff showing exactly what changed
- Marks the operation as "Proposed"

### Step 3: User Approval
- User sees a diff UI showing the change
- "Accept" button applies the operation
- "Reject" button discards it

### Step 4: Execute (NoteRepo applies)
1. Retrieves current block
2. Validates schema (rows match header length, etc.)
3. Calls `serialize_table()` to convert to markdown
4. Increments `block.version`
5. Updates `block.updated_at`
6. Saves to .md file
7. Updates database

This workflow ensures:
- **Type safety**: Agent cannot pass malformed data
- **Auditability**: Every change is versioned and attributed to the agent
- **User control**: Agents cannot silently corrupt data
- **Markdown preservation**: .md file always remains readable

## Media Management

### Local Blobs (BlobStore)

For images, PDFs, etc. stored locally:

```rust
let block = Block {
    id: BlockId::new(),
    note_id: my_note_id,
    content: BlockContent::Media {
        // hash_or_url points to BlobStore
        hash_or_url: "blob-abc123def456.png".into(),
        alt_text: "Architecture diagram".into(),
        media_type: MediaType::Image,
    },
    // ...
};
```

In markdown:
```markdown
![Architecture diagram](blob-abc123def456.png)
```

The BlobStore manages:
- Content-addressed storage (hash-based filenames prevent duplicates)
- Deduplication across notes
- Retention policies
- Lazy loading/CDN

### Remote URLs

For external images, videos, etc.:

```rust
let block = Block {
    id: BlockId::new(),
    note_id: my_note_id,
    content: BlockContent::Media {
        // hash_or_url points to a remote URL
        hash_or_url: "https://example.com/diagram.png".into(),
        alt_text: "External diagram".into(),
        media_type: MediaType::Image,
    },
    // ...
};
```

In markdown:
```markdown
![External diagram](https://example.com/diagram.png)
```

## Markdown Compatibility

All rich blocks serialize to **readable, standard markdown**. This means:

✓ **Works in**:
- GitHub (tables, math with LaTeX support, images, links)
- Obsidian (all formats)
- VS Code with Markdown extensions
- Logseq, Roam Research
- Pandoc, Sphinx

✓ **Is preserved by**:
- Git diff/merge
- Grep, sed, other text tools
- Email, Slack (text-only fallback)

## Examples

### Example 1: Research Note with Mixed Content

```markdown
# Quantum Entanglement Study

Einstein called it "spooky action at a distance". Here's the math:

$$
|\psi\rangle = \frac{1}{\sqrt{2}}(|00\rangle + |11\rangle)
$$

<!-- block:uuid-111 -->

## Experimental Results

| Configuration | Success Rate | Notes |
| --- | --- | --- |
| Polarization A | 92.3% | Optimal setup |
| Polarization B | 87.1% | Requires recalibration |
| Polarization C | 94.8% | **Best result** |

<!-- block:uuid-222 -->

## Video Reference

[Quantum Entanglement Explained](https://youtube.com/watch?v=xyz) *[YouTube Video]*

<!-- block:uuid-333 -->
```

### Example 2: Project Tracker

```markdown
# Project Aurora

High-level status and task breakdown.

<!-- embed:internal:View(proj-dash-1) -->

> **[Embedded: View proj-dash-1]**
>
> Project Aurora Dashboard
> - Tasks: 24 active, 8 completed
> - Timeline: 45% through sprint
> - Risk: 3 open issues
>
> *[This is a dynamic view embedded here. Open in app to interact.]*

<!-- block:uuid-444 -->

## Budget Breakdown

| Category | Q1 | Q2 | Q3 | Q4 |
| --- | --- | --- | --- | --- |
| Dev | $120k | $120k | $140k | $150k |
| Ops | $40k | $40k | $50k | $60k |
| Marketing | $30k | $40k | $50k | $60k |

<!-- block:uuid-555 -->
```

## Troubleshooting

### Q: I updated a table but the markdown looks weird
**A**: Check that all rows have the same number of columns as headers. The parser validates this on import.

### Q: Can I edit a .md file in VS Code and have changes sync?
**A**: Yes! The system uses markdown as the source of truth. Edit in your favorite editor, and on next load:
1. File is parsed into blocks
2. Rich block types are detected (tables, math, etc.)
3. Structures are updated in the database
4. App reflects changes

### Q: What happens if I manually break a table in the .md file?
**A**: The parser falls back to treating it as a `Markdown` block (plain text). No data loss—it's preserved as-is. When you open the note in the app, you can fix it using the table editor.

### Q: Can I have a Kanban board in my note?
**A**: Yes! Create a `ProjectDashboard` View with task filters, then embed it via `InternalEmbed`. The view stays a View (queryable, interactive), and the note embeds it with a fallback summary.

## See Also

- [ADR: Rich Block Types](../adr/rich-block-types.md) — Design decisions and rationale
- [ADR: Markdown-First Architecture](../adr/markdown-first.md) — Overall markdown strategy
- [AGENTS.md](../../AGENTS.md) — How agents interact with blocks
- [Block Tests](../../crates/pkm-core/src/markdown.rs) — Comprehensive test examples
