# Autonomous Bulk Link Ingestion System

## Overview

The autonomous bulk link ingestion system enables you to paste large blocks of URLs into your PKM app and have them automatically processed, scraped, analyzed, and promoted into your knowledge base—all without human review.

This system leverages:
- **Tokio** for concurrent processing (30+ URLs processed in parallel)
- **Jina AI** for markdown extraction from web pages
- **AI reasoning** (Gemini 3.5 Flash / Claude Haiku 4.5) for summarization
- **Agent Safety Layer** for auditable, reversible operations
- **Autonomous Promotion** to bypass the "waiting room" and create durable notes instantly

## Architecture: Producer-Consumer with Rate Limiting

```
PRODUCER LAYER:
┌─────────────────────────────────────────────────────────────┐
│ User pastes raw text (50 URLs) → ingest_bulk_links          │
│ Regex extracts URLs instantly                               │
│ Each URL sent to unbounded mpsc channel (fire-and-forget)   │
│ Returns immediately: "Processing 50 links in background"    │
└─────────────────────┬───────────────────────────────────────┘
                      │
QUEUE LAYER (Unbounded):
                      │ 50 URLs queued
                      │ (no buffering limit)
                      │
FETCHER LAYER (Rate Limited):
        ┌─────────────▼──────────────────────────────────────┐
        │ SINGLE-THREADED 3-SECOND TICKER                    │
        │ (20 RPM = 1 request every 3 seconds)               │
        │ Ensures Jina free tier compliance                   │
        │                                                     │
        │ ⏰ ticker.tick().await → waits 3 seconds           │
        │ 🔗 URL dequeued from mpsc                          │
        │ 🌐 Jina fetch (500-1500ms) synchronously          │
        │ ✓ Markdown obtained → handoff                      │
        └─────────────┬──────────────────────────────────────┘
                      │
                      │ (Markdown + URL handed off)
                      │
PROCESSOR LAYER (Concurrent):
        ┌─────────────▼────────────────────────────────────────────┐
        │ tokio::spawn() SEPARATE TASK (non-blocking)             │
        │ • LLM reasoning (5-10+ seconds) ← doesn't block fetcher  │
        │ • DB writes (create Source, Note, Link)                 │
        │ • Agent auditing (agent_action)                         │
        │                                                          │
        │ Multiple processors run concurrently                     │
        │ Slow LLM doesn't starve the 3-sec Jina rhythm          │
        └─────────────┬────────────────────────────────────────────┘
                      │
UI LAYER:
        ┌─────────────▼──────────────────────┐
        │ File watcher detects new Notes      │
        │ UI automatically refreshes          │
        │ 50 new notes visible                │
        └──────────────────────────────────────┘

PERFORMANCE:
• 50 URLs: 150 seconds sequential (3 sec × 50) + ~500ms LLM overhead
• Real bottleneck: Jina rate limit (not CPU)
• LLM latency hidden (concurrent processing)
• Peak throughput: 20 URLs/minute (Jina free tier cap)
```

## API

### Command: `ingest_bulk_links`

**Input:**
```typescript
raw_text: string  // Pasted text block containing URLs
```

**Output:**
```typescript
{
  count: number,           // Number of URLs extracted
  message: string          // "Processing N links in background..."
}
```

**Example:**

```rust
// From Tauri frontend
await invoke('ingest_bulk_links', {
  rawText: `
    Here are some important articles:
    https://example.com/article-1
    https://research.org/paper
    https://news.site/story
  `
})
```

The command returns immediately. Behind the scenes, Tokio spawns concurrent tasks that:
1. Extract markdown from each URL
2. Create Source objects
3. Call AI for summarization
4. Autonomously create Notes
5. Record everything in agent_action audit trail

### Command: `rollback_autonomous_ingestion`

**Input:**
```typescript
minutes: i64  // Rollback actions from the past N minutes
```

**Output:**
```typescript
{
  rolled_back: number,     // Number of actions rolled back
  message: string          // "Rolled back N autonomous ingestion actions"
}
```

**Example:**

```rust
// Nuclear undo button: revert last 5 minutes of autonomous ingestion
await invoke('rollback_autonomous_ingestion', { minutes: 5 })
```

This surgically removes:
- All created Notes
- All created Links
- All Source state changes
- All agent_action entries

Everything is still logged (rollback actions create new audit entries), so you can re-trace what happened.

## URL Extraction Logic

The system uses a simple regex to extract URLs from pasted text:

```regex
https?://[^\s]+
```

This matches:
- `https://example.com/path` ✓
- `http://site.org` ✓
- `https://link.com/path?query=value#hash` ✓

It automatically trims trailing punctuation:
- `https://example.com.` → `https://example.com` ✓
- `(https://example.com)` → `https://example.com` ✓

## Data Flow

### Step 1: Scrape (Jina AI)

Each URL is fetched via Jina's markdown extraction API:

```bash
GET https://r.jina.ai/https://example.com
```

Jina returns clean markdown of the page. We store this in the `raw_content` field of the Source.

### Step 2: Save Raw Source

A Source object is created with:
- `origin: WebArticle { url }`
- `raw_content: markdown`
- `ingestion_state: Captured`
- `created_by: System`

This is the immutable historical record of what was scraped.

### Step 3: AI Reasoning

The markdown is sent to your AI model (Gemini 3.5 Flash / Claude Haiku 4.5) to extract:
- **Title**: The main topic (from first heading or URL domain)
- **Summary**: A 200-char summary of key points

Currently, this is mocked (reads first heading or URL). In production, call:
```rust
async fn call_gemini_3_5_flash(markdown: &str) -> AiAnalysisResult
async fn call_claude_haiku(markdown: &str) -> AiAnalysisResult
```

### Step 4: Autonomous Promotion

Using the pkm-agent safety layer, we:

**ACTION A: Create Note**
```rust
Operation::CreateNote {
    note_id: NoteId::new(),
    title: ai_results.title,
}
```

This creates a durable Note in the vault and database. The operation is executed and applied immediately (no human review).

**ACTION B: Create DerivedFrom Link**
```rust
Operation::CreateTypedLink {
    from: ObjectRef::Note(note_id),
    to: ObjectRef::Source(source_id),
    link_type: LinkType::DerivedFrom,
}
```

This links the Note back to its original Source, preserving provenance.

**ACTION C: Update Source State**
```sql
UPDATE source SET ingestion_state = 'promoted' WHERE id = source_id
```

The Source transitions from `Captured` → `Promoted`.

All three operations are logged to `agent_action` table for auditability.

## Concurrency Model: Rate-Limited Fetching

The system uses a **Producer-Consumer architecture** that respects Jina's free tier limits:

### Fetcher Loop (Single-threaded, 3-second ticker)
```rust
let mut ticker = interval(Duration::from_secs(3)); // 20 RPM limit

while let Some(url) = rx.recv().await {
    ticker.tick().await; // CRITICAL: Wait 3 seconds before next Jina request
    
    // Fetch markdown (blocking, but fast: 500-1500ms)
    let markdown = client.get(&format!("https://r.jina.ai/{}", url)).send().await;
    
    // Hand off to processor (non-blocking spawn)
    tokio::spawn(async move {
        process_and_promote_source(pool, vault, client, url, markdown).await
    });
}
```

### Processor Tasks (Unlimited concurrent)
```rust
// Each spawned task runs LLM reasoning + DB writes independently
// Slow LLMs (5-10+ seconds) don't block the fetcher's 3-second rhythm
```

This means:
- **Fetcher:** 1 request per 3 seconds = 20 URLs/minute (hard limit)
- **Processors:** Unlimited concurrent tasks (limited by tokio runtime)
- **Total throughput:** ~20 URLs/minute (Jina-bottlenecked, not CPU-bottlenecked)
- **Peak memory:** Multiple LLM responses buffered (each ~1KB), not a concern

**Why this works:**
1. The 3-second ticker ensures zero HTTP 429 (rate limit) errors from Jina
2. Unbounded queue means URLs don't get dropped (they queue up)
3. Concurrent processor tasks mean a 10-second LLM response doesn't delay the next Jina fetch
4. Perfect for fire-and-forget ingestion: paste 50 URLs, they process one per 3 seconds

## Automation Mode

The system runs in **full automation mode**:

- `pkm_agent::requires_review()` returns `false` for all operations
- Operations execute immediately as `Applied` (not `Proposed`)
- Human review is bypassed entirely
- Rollback is your safety net

This is safe because:
1. Every operation is logged in `agent_action` (append-only audit trail)
2. Rollback is surgical and traceable
3. Sources are immutable (you can always re-analyze them later)
4. Notes can be reviewed/edited afterward

## Safety & Auditability

### Audit Trail

Every bulk ingestion creates entries in the `agent_action` table:

```sql
SELECT * FROM agent_action 
WHERE actor LIKE '%Autonomous-Ingestor%' 
ORDER BY created_at DESC;
```

Each action has:
- `id`: Unique action ID (for rollback)
- `actor`: "Autonomous-Ingestor-Haiku"
- `operation`: CreateNote, CreateTypedLink, etc.
- `target`: The object affected (Note ID, Link ID, Source ID)
- `status`: Applied
- `rationale`: "Autonomous bulk ingestion"
- `created_at`: Timestamp
- `diff`: Minimal diffs (for larger ops)
- `rollback_of`: (set to None initially, populated if rolled back)

### Rollback Mechanism

The `rollback_recent_autonomous_ingestion` command:

1. Queries `agent_action` for all actions by `Autonomous-Ingestor-*` in the past N minutes
2. For each action, calls `pkm_agent::rollback_action(action_id, ...)`
3. Each rollback creates a new `RollbackAction` entry pointing to the original

**Example rollback:**

```sql
-- Before rollback:
agent_action:
  id=abc, operation=CreateNote, status=Applied, target=Note(xyz)
  id=def, operation=CreateTypedLink, status=Applied, target=Link(uvw)

-- After rollback_recent_autonomous_ingestion(5):
agent_action:
  id=abc, operation=CreateNote, status=Reverted, target=Note(xyz)
  id=def, operation=CreateTypedLink, status=Reverted, target=Link(uvw)
  id=ghi, operation=RollbackAction, status=Applied, rollback_of=abc
  id=jkl, operation=RollbackAction, status=Applied, rollback_of=def
```

The original actions are marked `Reverted`, and new `RollbackAction` entries are created for traceability.

## Integration with pkm-agent

The bulk ingestion system uses pkm-agent's core operations:

```rust
// Agent layer (no persistence)
pkm_agent::execute(req, &action_repo)  // Creates audit entry
→ AgentAction { status: Applied, ... }

// Apply (persists the operation)
pkm_agent::apply_action(action.id, &action_repo, &note_repo, Some(&link_repo))
→ Creates Note, Link, etc. in database

// Rollback (reverses the operation)
pkm_agent::rollback_action(action_id, &action_repo, &note_repo, Some(&entity_repo), Some(&link_repo))
→ Deletes Note, unlinks Link, etc.
```

All three operations are idempotent and fully reversible.

## Configuration & Tuning

### Jina AI Rate Limits

Jina's free tier allows ~100 requests/min. For higher throughput, upgrade your plan or batch requests over time.

Current batch size: **50 URLs per mpsc message**. Adjust in `crates/pkm-app/src/ingestion.rs`:

```rust
let (tx, mut rx) = mpsc::channel::<Vec<String>>(50);  // Buffer size
```

### Concurrency Level

Currently, all URLs in a batch spawn Tokio tasks concurrently. To limit parallelism (e.g., to respect rate limits):

```rust
// Limit to 10 concurrent tasks
while let Some(urls) = rx.recv().await {
    for url in urls.chunks(10) {
        for batch_url in url {
            tokio::spawn(async move { ... });
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
```

### AI Model Selection

Mock analysis (currently enabled):
```rust
async fn analyze_content_mock(markdown: &str, url: &str) -> AiAnalysisResult {
    // Extract title from first heading or URL
    // Return dummy summary
}
```

To use real AI, replace with:
```rust
async fn analyze_content_real(markdown: &str, url: &str) -> AiAnalysisResult {
    let client = reqwest::Client::new();
    
    // Option A: Gemini 3.5 Flash
    let response = client
        .post("https://generativelanguage.googleapis.com/v1/models/gemini-3.5-flash:generateContent")
        .json(&json!({
            "contents": [{
                "parts": [{
                    "text": format!("Summarize this markdown in one sentence:\n{}", markdown)
                }]
            }]
        }))
        .send()
        .await?;
    
    // Option B: Claude Haiku 4.5
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", std::env::var("ANTHROPIC_API_KEY")?)
        .json(&json!({
            "model": "claude-haiku-4.5-20251001",
            "max_tokens": 200,
            "messages": [{
                "role": "user",
                "content": format!("Summarize this markdown in one sentence:\n{}", markdown)
            }]
        }))
        .send()
        .await?;
    
    // Parse response and return AiAnalysisResult
}
```

## Workflow Examples

### Example 1: Bulk Ingest from Research Session

**Scenario:** You just finished a 2-hour AI research session and have 30 URLs from OpenAI Research:

1. Copy the entire text block (URLs + notes mixed in)
2. Paste into app's "Bulk Ingest" textarea
3. Click "Ingest"
4. **App responds:** "Processing 30 links in background... Go back to browsing!"
5. **Behind the scenes:** Tokio spawns 30 concurrent tasks over the next 30-60 seconds
6. **Your UI updates automatically:** 30 new Notes appear in your default view
7. **You browse the new notes:** Click titles, read summaries, add tags/links

### Example 2: Disaster Recovery (Bad AI Output)

**Scenario:** You ingested 50 URLs, but the AI generated terrible summaries (hallucinations, missing context, etc.):

1. Click "Revert Last Batch" button
2. Choose "Last 5 minutes"
3. **App responds:** "Rolled back 50 autonomous ingestion actions"
4. **Your database:** All 50 Notes deleted, all Links removed, all Sources back to Captured state
5. **Next steps:** Re-analyze the Sources later with a better model

### Example 3: Manual Review After Ingestion

**Scenario:** You ingested 50 URLs, but you want to selectively accept/reject them:

1. Ingest 50 URLs → 50 Notes created automatically
2. Create a **ReviewQueue View** to filter by ingestion date
3. Manually click "Accept" on the ones you want to keep
4. Manually delete the garbage ones
5. The audit trail still shows all actions (for compliance/audit)

## Performance Metrics

**With Rate Limiting (Jina Free Tier):**

| Metric | Value |
|--------|-------|
| URLs extracted (regex) | <1ms |
| Jina API rate | **1 request per 3 seconds** (20 RPM free tier) |
| Jina API latency (per URL) | 500-1500ms |
| AI analysis latency (per URL) | 100ms (mock); 1000-5000ms (real API, concurrent) |
| DB insert latency (per URL) | 50-100ms (concurrent with LLM) |
| **Per URL throughput** | 3+ seconds (Jina-limited) |
| **50 URLs total time** | ~150 seconds (3s × 50 URLs) + LLM overhead hidden |
| **100 URLs total time** | ~300 seconds (5 minutes) + LLM overhead hidden |

**Key insights:**
- Bottleneck is Jina (1 request/3 seconds), NOT CPU or LLM
- LLM latency is **hidden** because it runs in separate tokio::spawn tasks
- Example: If LLM takes 10 seconds but Jina takes 3 seconds, you don't see the 10 seconds—it overlaps
- Memory usage is minimal (unbounded queue buffers URLs, not full responses)
- Upgrade Jina plan → proportionally faster throughput

## Future Enhancements

1. **Real AI Integration:** Replace mock analysis with Gemini 3.5 Flash / Claude Haiku 4.5
2. **Entity Extraction:** Automatically extract people, organizations, concepts from summaries
3. **Relationship Mining:** Infer connections between ingested sources using AI reasoning
4. **Batch Scheduling:** Queue large ingestions over time to respect rate limits
5. **Duplicate Detection:** Skip URLs already in the database (content hash check)
6. **Progressive Feedback:** Emit Tauri events as URLs complete (show real-time progress bar)

## Troubleshooting

### "No URLs found in text"

Check that your URLs start with `http://` or `https://`. The regex doesn't match `www.example.com` without the protocol.

### "Queue full" error

The mpsc channel buffer is full (unlikely in practice). Increase the buffer size in `ingestion.rs`:

```rust
let (tx, mut rx) = mpsc::channel::<Vec<String>>(100);  // Was 50
```

### Notes aren't appearing

The background worker is still processing. Wait 30-60 seconds and refresh your view. Check the server logs for errors.

### Jina API errors

Jina might be rate-limited or down. Check the logs for HTTP errors. You can retry later or skip problematic URLs.

### Rollback isn't working

Rollback only works on actions from the past N minutes. If you ingested 2 hours ago and try to rollback 5 minutes, it won't find any recent actions.

## See Also

- [pkm-agent Architecture](crates/pkm-agent/README.md) — Agent safety layer
- [Source Model](crates/pkm-core/src/source.rs) — Source object definition
- [Agent Action Audit Trail](crates/pkm-storage/src/migrations/agent_action.sql) — Schema
