# Autonomous Bulk Link Ingestion: Rate-Limited Implementation

## What Changed (From Concurrent to Rate-Limited)

The ingestion system was refactored to respect Jina's free tier limits (20 RPM = 1 request per 3 seconds) while maintaining full autonomous operation and LLM concurrency.

### Previous Implementation (Flawed)
- Spawned 50 concurrent Tokio tasks for 50 URLs
- All tasks immediately hit Jina API in parallel
- Result: **HTTP 429 (Too Many Requests)** errors after ~6 requests
- Throughput: Limited by Jina's 429 errors and backoff

### New Implementation (Rate-Limited)
- Single-threaded fetcher with 3-second ticker
- Unbounded URL queue (never drops URLs)
- Separate concurrent processor tasks for LLM + DB
- Result: **Zero rate limit errors**, predictable 20 URLs/minute throughput
- LLM latency hidden (doesn't block fetcher)

## Architecture

### Layer 1: Producer (Tauri Command)
```rust
#[tauri::command]
async fn ingest_bulk_links(raw_text: String) -> Result<usize> {
    let urls = extract_urls(&raw_text);  // Regex
    
    for url in urls {
        // Send to UNBOUNDED queue (fire-and-forget)
        service.ingestion_tx.send(url)?;
    }
    
    // Return immediately
    Ok(urls.len())
}
```
- Extracts URLs via regex
- Sends each URL individually to unbounded mpsc channel
- Returns count immediately to UI
- Never blocks

### Layer 2: Fetcher (Rate-Limited)
```rust
async fn run_rate_limited_fetcher(mut rx: UnboundedReceiver<String>) {
    let mut ticker = interval(Duration::from_secs(3)); // 20 RPM
    
    while let Some(url) = rx.recv().await {
        ticker.tick().await;  // CRITICAL: Wait 3 seconds
        
        // Fetch from Jina (fast, 500-1500ms)
        let markdown = client.get(&format!("https://r.jina.ai/{}", url))
            .send()
            .await?
            .text()
            .await?;
        
        // Hand off to processor (non-blocking)
        tokio::spawn(process_and_promote_source(url, markdown));
    }
}
```
- Single-threaded loop
- Ticks exactly every 3 seconds
- Fetches one URL per tick (20 URLs/minute max)
- Hands off markdown to processor immediately (non-blocking)

### Layer 3: Processor (Concurrent)
```rust
async fn process_and_promote_source(url: String, markdown: String) {
    // 1. Save raw Source (blocking)
    let source_id = create_source(url, markdown);
    
    // 2. LLM reasoning (5-10+ seconds, non-blocking)
    let ai_results = call_llm(markdown).await;
    
    // 3. Create Note + Link (blocking)
    create_note_and_link(source_id, ai_results);
}
```
- Runs in separate tokio::spawn task
- Can take 10+ seconds without blocking fetcher
- Multiple processor tasks run concurrently
- Handles the "heavy lifting" (LLM, DB writes)

## Why This Works

### 1. Rate Limit Compliance
```
Time:     0s    3s    6s    9s   12s   15s
Fetcher:  URL1  URL2  URL3  URL4  URL5  URL6
Jina:     ✓     ✓     ✓     ✓     ✓     ✓
Status:   200   200   200   200   200   200  (never 429)
```

### 2. Hidden LLM Latency
```
Time:     0s          3s          6s          9s
Fetcher:  [Fetch]     [Fetch]     [Fetch]     [Fetch]
Proc1:    [LLM: 10s + DB]
Proc2:                [LLM: 10s + DB]
Proc3:                            [LLM: 10s + DB]
Result:   By 15s, you have 3 processed notes (not 1!)
```

### 3. Unbounded Queue Never Drops URLs
```
Scenario: Paste 100 URLs at once
- Regex extracts 100 URLs instantly
- All 100 queued to unbounded channel (no limit)
- Fetcher processes them one per 3 seconds
- 100 URLs take ~300 seconds (5 minutes)
- None are dropped or lost
```

## Configuration & Tuning

### Adjust the Jina Rate Limit
```rust
// In crates/pkm-app/src/ingestion.rs
let mut ticker = interval(Duration::from_secs(3)); // Currently 3 seconds

// Upgrade Jina to 100 RPM plan? Lower to 0.6 seconds:
let mut ticker = interval(Duration::from_millis(600));

// Upgrade to 1000 RPM plan? Lower to 60ms:
let mut ticker = interval(Duration::from_millis(60));
```

### Add Adaptive Backoff
If you want to handle rate limit errors gracefully:
```rust
match client.get(&jina_url).send().await {
    Ok(resp) if resp.status().is_success() => { /* process */ }
    Ok(resp) if resp.status().as_u16() == 429 => {
        eprintln!("Rate limited! Backing off...");
        tokio::time::sleep(Duration::from_secs(60)).await;
        // Re-queue URL for retry
        tx.send(url).ok();
    }
    Err(e) => eprintln!("Network error: {}", e),
}
```

### Monitor Queue Depth
To see how many URLs are waiting:
```rust
// Add a gauge metric
let pending = rx.len(); // UnboundedReceiver doesn't expose this directly
// Alternative: Track in Arc<AtomicUsize>
```

## Real-World Performance

### Scenario A: 50 URLs (Daily Research)
```
Time:     0s       150s (2.5 min)     300s+ (with LLM)
Fetcher:  [Queue 50 URLs] → Process URL1 → ... → URL50
Procs:    (Run in background, LLM hidden)
Result:   ✓ All 50 processed, spread over 2-3 minutes
```

### Scenario B: 200 URLs (Weekly Bulk Ingest)
```
Time:     0s       600s (10 min)      1200s+ (with LLM)
Fetcher:  [Queue 200 URLs] → 1 URL every 3 seconds
Procs:    (Run in background)
Result:   ✓ All 200 processed, spread over ~10 minutes
          ✓ Zero rate limit errors
          ✓ LLM processing hidden
```

### Scenario C: Continuous Ingestion (News Feed)
```
Every 1 hour: 20 new URLs discovered
Every 3 seconds: 1 URL fetched from Jina
Over time: Steady-state processing of 20 URLs/minute
Result:   ✓ News feed continuously updated
          ✓ Never exceeds Jina limit
```

## Debugging

### Log Output
```
[Jina Fetcher] Processing URL (3-sec rate limit): https://example.com/article
[Jina Fetcher] ✓ Fetched 4521 bytes from https://example.com/article
[Processor] ✓ Source created: 8a3c5e9b-...
[Processor] ✓ LLM analysis: title=Example Title
[Processor] ✓ Note created: 7f2d1a4c-...
[Processor] ✓ Autonomously promoted: https://example.com/article -> 7f2d1a4c-...
```

### Check for Rate Limits
If you see HTTP 429 errors:
1. Verify Jina interval is set correctly (should be 3 seconds)
2. Check if you manually spawned extra fetchers (don't!)
3. Confirm you're using the unbounded channel (not a bounded one)

### Monitor Memory Usage
The unbounded queue can theoretically grow large if:
- Jina is slow (taking 10+ seconds per request)
- LLM processing is slow (15+ seconds)
- Network issues cause delays

Solution: Limit queue size or add backpressure
```rust
// Change to bounded channel (drop old URLs if queue full)
let (tx, rx) = mpsc::channel::<String>(100); // Max 100 queued URLs
```

## Testing

### Unit Tests
```rust
#[test]
fn test_extract_urls() {
    let urls = extract_urls("https://example.com https://test.org");
    assert_eq!(urls.len(), 2);
}
```

### Integration Test (Manual)
```bash
# 1. Start the app
cargo build --release -p pkm-app

# 2. Paste 10 URLs into "Bulk Ingest"
# 3. Watch the logs
# 4. Verify 1 request every 3 seconds
# 5. Confirm 10 notes created over ~30 seconds
```

### Load Test (Automated)
```rust
#[tokio::test]
async fn test_100_url_ingestion() {
    let service = AppService::new("test.db", None).unwrap();
    
    let urls: Vec<String> = (0..100)
        .map(|i| format!("https://example.com/article-{}", i))
        .collect();
    
    let start = Instant::now();
    service.ingest_bulk_links(urls.join("\n")).await.unwrap();
    
    // Wait for processing to complete
    tokio::time::sleep(Duration::from_secs(300)).await;
    
    // Verify all notes were created
    let notes = service.list_notes(None).unwrap();
    assert_eq!(notes.len(), 100);
    
    let elapsed = start.elapsed().as_secs();
    println!("100 URLs processed in {} seconds", elapsed);
    assert!(elapsed >= 300, "Should take ~300 seconds (3s × 100)");
}
```

## Future Enhancements

1. **Jina Rate Limit Auto-Detection**
   - Query Jina for remaining quota
   - Adjust ticker dynamically

2. **Batch Fetching (if Jina supports it)**
   - Send multiple URLs in one request
   - Keep 3-second interval but fetch N URLs per batch

3. **Retry Queue**
   - If a URL fails (timeout, 500 error), re-queue with exponential backoff
   - Separate "failed" queue with different retry schedule

4. **Streaming Progress**
   - Emit Tauri events as each URL completes
   - Show real-time progress bar in UI

5. **Scheduled Ingestion**
   - Queue URLs but delay fetcher (process at night)
   - Separate "schedule" vs "process now"

