# Agent Interaction with Rich Blocks

This document specifies how AI agents safely interact with Rich Block Types. Read [AGENTS.md](AGENTS.md) first for general agent rules.

## Core Principle

**Agents manipulate structured data, never markdown strings.**

When an agent needs to update a block, it works with the Rust struct:
```rust
BlockContent::Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}
```

NOT with the markdown string representation. The system handles markdown serialization.

## Agent Operations on Blocks

### Reading Blocks

Agents can read any block type:

```rust
// Operation: RequestBlockRead
// Agent requests: "What's in block uuid-123?"

// System returns:
{
    "block_id": "uuid-123",
    "content_kind": "table",
    "content": {
        "headers": ["Name", "Score"],
        "rows": [
            ["Alice", "95"],
            ["Bob", "87"]
        ]
    }
}
```

Agent can now:
- Extract specific cells
- Compute aggregations (sum scores, count rows, find min/max)
- Detect patterns or anomalies
- Use the data in reasoning

### Creating Blocks

Agents create blocks via structured operations:

```rust
// Operation: CreateBlock
// Agent specifies:
{
    "note_id": "note-abc",
    "content": {
        "kind": "table",
        "headers": ["Item", "Quantity"],
        "rows": [
            ["Apples", "5"],
            ["Oranges", "3"]
        ]
    },
    "order": 2.5
}

// System returns:
{
    "block_id": "uuid-new",
    "status": "created",
    "markdown": "| Item | Quantity |\n| --- | --- |\n| Apples | 5 |\n| Oranges | 3 |"
}
```

The agent **never writes markdown**. It specifies `kind: "table"` and provides structured data.

### Updating Blocks

Agents propose updates via structured diffs:

```rust
// Operation: UpdateBlock
// Agent specifies what changed:
{
    "block_id": "uuid-123",
    "operation": "update_table_cell",
    "changes": [
        {
            "row": 0,
            "col": 1,
            "old_value": "95",
            "new_value": "96"
        }
    ]
}

// System processes:
// 1. Validates: row 0 exists, col 1 exists
// 2. Computes diff: Alice's score: 95 → 96
// 3. Marks as "Proposed"
// 4. Returns:
{
    "block_id": "uuid-123",
    "status": "proposed",
    "diff": {
        "kind": "table_cell_update",
        "row": 0,
        "col": 1,
        "old_value": "95",
        "new_value": "96"
    },
    "markdown_preview": "| Name | Score |\n| --- | --- |\n| Alice | 96 |\n| Bob | 87 |"
}
```

If the user approves:
- The markdown is regenerated with pipes properly escaped
- The .md file is updated
- The database is updated
- The operation is recorded in the audit log

If the user rejects:
- No changes are made
- The agent is informed and can try a different approach

### Deleting Blocks

Agents can request block deletion:

```rust
// Operation: DeleteBlock
{
    "block_id": "uuid-old"
}

// System returns:
{
    "block_id": "uuid-old",
    "status": "deleted",
    "markdown_removed": "[2 paragraphs of content removed]"
}
```

## Safety Guardrails

### 1. Schema Validation

Agents **cannot create invalid block structures**. The system validates:

```rust
// ❌ REJECTED: Table rows don't match header length
{
    "kind": "table",
    "headers": ["A", "B"],
    "rows": [["1", "2", "3"]]  // 3 cells, 2 headers
}

// ✓ ACCEPTED: Correct structure
{
    "kind": "table",
    "headers": ["A", "B"],
    "rows": [["1", "2"]]  // 2 cells, 2 headers
}
```

Error message guides the agent to fix the issue.

### 2. Type Constraints

Agents cannot specify arbitrary content:

```rust
// ❌ INVALID: Unknown block kind
{
    "kind": "custom_widget",
    "data": { "anything": "goes" }
}

// ✓ VALID: One of the known kinds
// - markdown, math, table, media, internal_embed, external_embed
```

### 3. Markdown Injection Prevention

Agents cannot embed markdown syntax:

```rust
// ❌ REJECTED: Agent tries to inject markdown
{
    "kind": "markdown",
    "text": "Click [here](http://malicious.com) to download"
}
// System: Parses, logs, stores plain text
// Output markdown: Safe, no active links without user intent

// ✓ ACCEPTED: Agent creates a link block explicitly
{
    "kind": "external_embed",
    "url": "http://example.com",
    "provider": "generic"
}
// User sees clearly it's a link, can inspect URL before visiting
```

### 4. Media Validation

Media blocks are checked:

```rust
// ❌ INVALID: No alt text
{
    "kind": "media",
    "hash_or_url": "blob-123.png",
    "alt_text": "",  // Empty!
    "media_type": "image"
}

// ✓ VALID: Descriptive alt text
{
    "kind": "media",
    "hash_or_url": "blob-123.png",
    "alt_text": "Architecture diagram showing service layers",
    "media_type": "image"
}
```

### 5. Audit Trail

Every block operation is logged:

```
[2026-06-26 10:30:45 UTC] Agent(claude-opus): Created table block uuid-123 in note note-abc
[2026-06-26 10:31:12 UTC] User: Approved UpdateBlock uuid-123 (Alice: 95 → 96)
[2026-06-26 10:31:13 UTC] System: Updated block uuid-123, version 2
[2026-06-26 10:32:00 UTC] Agent(claude-opus): Read block uuid-123
```

Agents cannot:
- Delete their own audit trail
- Forge timestamps
- Impersonate users

## Common Agent Workflows

### Workflow 1: Data Extraction and Summary

Agent reads a complex table and creates a summary:

```
1. RequestBlockRead uuid-123 (table with 100 rows)
2. Agent processes: compute sum, average, count, etc.
3. CreateBlock (new Markdown block with summary)
   "Q3 Revenue: $450k total, avg $4.5k/customer, 92% growth"
4. System saves, returns uuid-new
```

### Workflow 2: Collaborative Table Edit

Agent and user iteratively refine data:

```
1. Agent reads table (sales forecast)
2. Agent computes: "Q4 looks low, 30% below trend"
3. Agent proposes UpdateBlock:
   - Q4 forecast: $100k → $130k
4. System marks as "Proposed", user reviews
5. User approves → system applies, regenerates markdown
6. Agent reads updated table, provides analysis
```

### Workflow 3: Auto-Generated Content

Agent creates sections with rich blocks:

```
1. Agent reads sources, extracts key facts
2. Agent creates table:
   - headers: ["Fact", "Source", "Confidence"]
   - rows: [["X", "source-1", "high"], ...]
3. Agent creates math block: "Formula for prediction"
4. Agent creates embeds: internal links to related notes
5. All blocks marked with source_provenance_ref pointing to sources
```

### Workflow 4: Content Validation

Agent checks data consistency:

```
1. Agent reads all table blocks in note
2. For each: "Are header names consistent across tables?"
3. If inconsistent: proposes UpdateBlock to standardize
4. System validates, user approves
5. Markdown regenerated with consistent headers
```

## Error Handling

### When an agent makes a mistake

```rust
// Agent tries to update a cell that doesn't exist
{
    "block_id": "uuid-123",
    "operation": "update_table_cell",
    "changes": [
        {"row": 999, "col": 0, "new_value": "X"}
    ]
}

// System responds:
{
    "error": "INDEX_OUT_OF_BOUNDS",
    "detail": "Table has 10 rows, row 999 does not exist",
    "hint": "Use operation: list_block_contents to see valid indices"
}

// Agent can then:
// 1. Request list of valid rows
// 2. Correct the index
// 3. Resubmit the operation
```

### When markdown becomes corrupted

If somehow the markdown file gets corrupted (e.g., manual edit gone wrong):

```
1. User opens note in app
2. Parser encounters malformed GFM table
3. Falls back to treating as Markdown block
4. Logs warning: "Table parsing failed on block uuid-123"
5. Content is preserved as-is
6. User can use app UI to fix it (table editor creates valid markdown)
```

## Best Practices for Agents

### DO ✓

1. **Use structured operations** — Always specify `kind` and structured data
2. **Validate before proposing** — Check cell counts, header lengths, etc.
3. **Provide context** — When proposing changes, explain why (in operation comment)
4. **Request approval** — Mark operations as "Proposed" for user review
5. **Read after write** — Verify your created block has the expected structure
6. **Use source_provenance_ref** — Link generated blocks back to their sources

### DON'T ✗

1. **Never write markdown strings** — Always use structured BlockContent types
2. **Never assume block structure** — Always validate before accessing cells
3. **Never create unreadable markdown** — The system does that; you provide data
4. **Never delete without approval** — Mark for deletion, wait for user confirmation
5. **Never craft HTML/JS injection** — You can't; blocks don't support it
6. **Never bypass schema validation** — The system rejects invalid structures

## Integration with Operations

Rich block updates are part of the broader Operation system (see [AGENTS.md](AGENTS.md)):

```rust
pub enum Operation {
    CreateBlock { ... },
    UpdateBlock { ... },
    DeleteBlock { ... },
    // Other operations (links, views, etc.)
}

pub enum OperationStatus {
    Proposed,    // Waiting for user approval
    Approved,    // User approved, ready to execute
    Executed,    // Applied to database and markdown
    Rejected,    // User rejected, no changes made
}
```

All operations follow the same workflow:
1. Agent proposes (status = Proposed)
2. System validates and shows diff
3. User approves/rejects
4. System executes and updates audit trail

## Future Extensions

### Phase C3: Richer Agent Capabilities
- **Batch operations**: Update multiple blocks atomically
- **Conditional operations**: "If table has X rows, then do Y"
- **Template expansion**: "Create a table from template T"
- **AI-generated images**: Create image blocks from descriptions

### Phase D1: Agent-Driven Views
- **Query blocks**: Agents propose Views that query related blocks
- **Derived blocks**: Auto-generated summaries that update on source changes
- **Agent-owned blocks**: Blocks created by agents are marked with `created_by: Agent`

## Testing Agent Interactions

Use fixtures to test agent workflows:

```rust
#[test]
fn agent_can_update_table_cells() {
    let table = sample_table_block();
    let mut updated = table.clone();
    
    if let BlockContent::Table { ref mut rows, .. } = updated.content {
        rows[0][0] = "Updated".into();
    }
    
    // Verify markdown is valid
    let md = blocks_to_markdown(&[updated]);
    assert!(md.contains("| Updated |"));
    assert!(!md.contains("| Name |")); // Old value replaced
}
```

See [crates/pkm-core/src/markdown.rs](../crates/pkm-core/src/markdown.rs) for comprehensive block tests.

## See Also

- [AGENTS.md](AGENTS.md) — General agent rules and operations
- [ADR: Rich Block Types](adr/rich-block-types.md) — Block design and rationale
- [Rich Blocks Guide](rich-blocks-guide.md) — Practical examples and tutorials
- [Agent Action ADR](adr/agent-action.md) — Operation and safety model
