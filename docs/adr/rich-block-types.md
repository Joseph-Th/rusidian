# ADR: Rich Block Types (C2)

**Status**: Implemented  
**Date**: 2026-06-26  
**Deciders**: Claude  
**Context**: Expanding BlockContent beyond simple markdown to support math, tables, media, and embeds while maintaining AI safety and markdown compatibility.

## Problem

The system needs to support rich content types (math equations, tables, images, video embeds, internal links) without violating core constraints:

1. **AI Safety**: Agents must use strongly-typed data structures, not fragile string manipulation that could corrupt syntax
2. **Markdown Compatibility**: All content must serialize to standard `.md` files readable in any editor
3. **Separation of Concerns**: Interactive UI (Kanban boards, dynamic dashboards) belongs in the View system, not embedded in raw note text

## Decision

Implement Rich Blocks as strongly-typed `BlockContent` enum variants, with standard markdown serialization and View-based embeds for complex UI.

### BlockContent Enum (Rust)

```rust
pub enum BlockContent {
    Markdown { text: String },
    
    Math {
        expression: String,
        display_mode: bool,
    },
    
    Media {
        hash_or_url: String,
        alt_text: String,
        media_type: MediaType, // Image, Audio, Video, Pdf
    },
    
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    
    InternalEmbed {
        target: ObjectRef,
        fallback_text: String,
    },
    
    ExternalEmbed {
        url: String,
        provider: EmbedProvider, // YouTube, Twitter, GoogleDrive, Generic
    },
}
```

### Markdown Serialization Contract

Each variant serializes to standard markdown that reads naturally in any editor:

| Type | Rust Struct | Markdown | Readable Outside App |
|------|------------|----------|---------------------|
| Math (display) | `Math { expr, display_mode: true }` | `$$\nE = mc^2\n$$` | ✓ Standard LaTeX |
| Math (inline) | `Math { expr, display_mode: false }` | `$x^2$` | ✓ Standard LaTeX |
| Table | `Table { headers, rows }` | GFM table with `\|` escaping | ✓ Readable table |
| Image | `Media { url, alt_text, Image }` | `![alt text](url)` | ✓ Standard markdown |
| Video (YouTube) | `ExternalEmbed { url, YouTube }` | `<!-- embed:external:youtube.com:url -->\n[url]() *[YouTube Video]*` | ✓ Clickable link |
| View Embed | `InternalEmbed { view_id, fallback }` | `<!-- embed:internal:View(...) -->\n> **[Embedded: View ...]**` | ✓ Readable fallback |

### Example Markdown Output

```markdown
---
id: note-uuid
created_by: User
created_at: 2026-06-26T10:30:00Z
---

# Q3 Earnings Analysis

Einstein's famous equation:

$$
E = mc^2
$$

<!-- block:uuid-1234 -->

Sales by department:

| Department | Revenue |
| --- | --- |
| Sales | $400k |
| Marketing | $150k |

<!-- block:uuid-5678 -->

<!-- embed:external:youtube.com:https://youtube.com/watch?v=dQw4w9WgXcQ -->

[https://youtube.com/watch?v=dQw4w9WgXcQ](https://youtube.com/watch?v=dQw4w9WgXcQ) *[YouTube Video]*

<!-- block:uuid-9012 -->

<!-- embed:internal:View(abcd-efgh) -->

> **[Embedded: View abcd-efgh]**
>
> Project Tasks: 12 active, 5 completed
>
> *[This is a dynamic view embedded here. Open in app to interact.]*
```

When opened in VS Code or Obsidian, this file reads perfectly:
- Math renders naturally if editor supports `$$`
- Tables are fully readable
- External embeds show as clickable links
- View embeds show as blockquotes with context

## AI Safety: How This Prevents Corruption

**Without strong types**, an agent might try to edit a table by string manipulation:
```rust
// ❌ FRAGILE - agent could corrupt pipes
let mut md = "| Name | Price |\n| Apple | 1 |".to_string();
md = md.replace("Apple", "Banana | $2"); // Breaks the table!
```

**With strong types**, the agent manipulates structured data:
```rust
// ✓ SAFE - agent cannot corrupt syntax
let mut table = Table {
    headers: vec!["Name".into(), "Price".into()],
    rows: vec![vec!["Apple".into(), "$1".into()]],
};
table.rows[0][0] = "Banana".into(); // Impossible to corrupt pipes
// On save: NoteRepo::update_block() converts back to markdown
```

## View System Integration

Complex interactive UI **does not belong in BlockContent**:

❌ **WRONG**: `BlockContent::Kanban { columns: Vec<Column>, cards: Vec<Card> }`
- Mixing data (note content) with presentation (UI layout)
- Fragile when columns/cards schema changes
- Hard to query/index

✓ **RIGHT**: Create a `View` (e.g., `ViewKind::ProjectDashboard`), then embed it:
```rust
BlockContent::InternalEmbed {
    target: ObjectRef::View(view_id),
    fallback_text: "Project Tasks Kanban: 12 active, 5 completed".into(),
}
```

When the app loads the note:
1. It sees `InternalEmbed` and looks up the View
2. The View queries SQLite to find related tasks
3. The app renders a drag-drop Kanban board in the document
4. Markdown file contains only the fallback text (stays human-readable)
5. If the .md file is opened outside the app, users still see meaningful context

## Implementation Details

### `crates/pkm-core/src/media.rs` (new)

Strongly-typed enums for media safety:

```rust
pub enum MediaType {
    Image,   // PNG, JPEG, SVG, etc.
    Audio,   // MP3, WAV, FLAC
    Video,   // MP4, WebM
    Pdf,     // PDF documents
}

pub enum EmbedProvider {
    YouTube,
    Twitter,
    GoogleDrive,
    Generic,
}
```

### `crates/pkm-core/src/block.rs`

Expanded `BlockContent` enum with all variants. Serde JSON handles serialization to/from the database.

### `crates/pkm-core/src/markdown.rs`

Core functions:
- `block_content_to_markdown()`: Converts BlockContent → standard markdown
- `markdown_to_blocks()`: Parses markdown → BlockContent (with fallback to Markdown for unrecognized formats)
- `serialize_table()`: GFM table with pipe escaping
- `try_parse_image()`, `try_parse_inline_math()`: Detect rich blocks in markdown

## Testing

- **Round-trip serialization**: Each BlockContent variant → markdown → BlockContent
- **Pipe escaping**: Tables with `|` in cells escape correctly
- **Block ID preservation**: HTML comments survive round-trip
- **Fallback parsing**: Unrecognized blocks fall back to Markdown
- **Fixture examples**: `sample_math_block()`, `sample_table_block()`, etc. for testing

All 88 tests pass, including 16 new tests for rich blocks.

## Migration Path

Existing notes with only `Markdown` blocks are unaffected:
1. Markdown files import as before
2. Each paragraph becomes a `Markdown` block
3. If a paragraph looks like a table, math, or image, it's parsed as the appropriate type
4. On export, everything serializes back to readable markdown

Rich blocks are opt-in:
- Users/agents create them intentionally (via structured operations)
- Import from external sources detects and converts them
- Fallback behavior is always safe

## Future Extensions

### Phase C3+
- Syntax highlighting for code blocks: `CodeBlock { language, code }`
- Inline widgets for time pickers, checkboxes, etc. (still using InternalEmbed+View)
- Collaborative editing awareness (track edit locks per block)

### View System
- **ProjectDashboard**: Kanban, Gantt, timeline views (already implemented)
- **EntityDossier**: Linked entity cards with properties
- **ReadingQueue**: Sorted/filtered list of sources pending review

Kanban boards, complex dashboards, and any interactive UI stay in Views and embed via `InternalEmbed`.

## Consequences

### ✓ Benefits
1. **Type Safety**: AI agents cannot corrupt markdown syntax
2. **Compatibility**: Notes remain readable in Obsidian, VS Code, GitHub
3. **Clean Separation**: Data (blocks) vs. presentation (views)
4. **Extensible**: Easy to add new block types (just a new enum variant + markdown handlers)
5. **Future-Proof**: Markdown is a stable, well-documented format

### ⚠ Tradeoffs
1. **Parser Complexity**: `markdown_to_blocks()` must recognize multiple formats
   - *Mitigated*: Best-effort parsing with safe fallback to Markdown
2. **View Latency**: Embedded views require DB lookups on load
   - *Mitigated*: Views are cached; lazy-load on demand
3. **Edit Granularity**: Can't edit individual table cells in the .md file directly
   - *Expected*: Rich edits happen in the app UI; .md files are canonical storage

## Related ADRs
- [ADR: Markdown-First Architecture](markdown-first.md)
- [ADR: View System](view-system.md)
- [ADR: Block-Level Editing](block-editing.md)

## References
- GitHub-Flavored Markdown Tables: https://github.github.com/gfm/#tables-extension-
- LaTeX in Markdown: Pandoc, CommonMark Math extensions
- Internal Transclusion: Obsidian wiki links, LogSeq embed syntax
