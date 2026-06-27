# Autonomous Bulk Link Ingestion - Usage Examples

This document shows practical examples of how to use the bulk ingestion system from both the frontend and backend.

## Frontend (TypeScript/JavaScript)

### Example 1: Simple Bulk Ingest

```typescript
// In your React component or Svelte store
import { invoke } from '@tauri-apps/api/core';

async function handleBulkIngest() {
  const pastedText = document.getElementById('url-textarea').value;
  
  try {
    const response = await invoke('ingest_bulk_links', {
      rawText: pastedText
    });
    
    console.log(`Processing ${response.count} links in background...`);
    
    // Show toast notification
    showToast(`Processing ${response.count} links. You can continue browsing.`);
    
    // Optional: Start polling for progress (not implemented yet)
    // waitForIngestionComplete();
  } catch (error) {
    showToast(`Error: ${error}`, 'error');
  }
}
```

### Example 2: Bulk Ingest with Progress Monitoring

```typescript
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

async function handleBulkIngestWithProgress() {
  const pastedText = document.getElementById('url-textarea').value;
  
  // Listen for ingestion progress events (future enhancement)
  const unlisten = await listen('ingestion-progress', (event) => {
    const { completed, total, current_url } = event.payload;
    console.log(`Ingesting ${completed}/${total}: ${current_url}`);
    updateProgressBar(completed, total);
  });
  
  try {
    const response = await invoke('ingest_bulk_links', {
      rawText: pastedText
    });
    
    console.log(`Queued ${response.count} links for processing`);
    
  } catch (error) {
    showToast(`Error: ${error}`, 'error');
  }
}
```

### Example 3: Bulk Ingest with Auto-Refresh

```typescript
async function handleBulkIngestAndRefresh() {
  const pastedText = document.getElementById('url-textarea').value;
  
  // Send ingestion command
  const response = await invoke('ingest_bulk_links', {
    rawText: pastedText
  });
  
  // Poll for changes (simple approach; better: use file watcher)
  let attempts = 0;
  const maxAttempts = 120; // 2 minutes (every 1 second)
  
  const checkForNew = async () => {
    const notes = await invoke('list_notes', { limit: 100 });
    const newCount = notes.length;
    
    if (newCount > lastKnownCount) {
      // New notes appeared! Refresh the UI
      refreshNotesList();
      lastKnownCount = newCount;
    }
    
    if (attempts < maxAttempts) {
      attempts++;
      setTimeout(checkForNew, 1000);
    }
  };
  
  const lastKnownCount = await invoke('list_notes', { limit: 1 })
    .then(notes => notes.length);
  
  checkForNew();
}
```

### Example 4: Rollback Recent Ingestion

```typescript
async function handleRollbackRecent() {
  const minutes = parseInt(prompt('Rollback ingestion from last N minutes:', '5'));
  
  if (isNaN(minutes) || minutes <= 0) {
    showToast('Please enter a valid number of minutes', 'error');
    return;
  }
  
  // Confirm with user
  const confirmed = confirm(
    `Are you sure? This will roll back all autonomous ingestion actions from the past ${minutes} minutes. ` +
    `This operation is irreversible (but logged in the audit trail).`
  );
  
  if (!confirmed) return;
  
  try {
    const response = await invoke('rollback_autonomous_ingestion', {
      minutes: minutes
    });
    
    showToast(response.message, 'success');
    
    // Refresh the UI
    await new Promise(resolve => setTimeout(resolve, 1000));
    refreshNotesList();
    
  } catch (error) {
    showToast(`Rollback failed: ${error}`, 'error');
  }
}
```

### Example 5: Bulk Ingest UI Component (React)

```typescript
import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export function BulkIngestPanel() {
  const [pastedText, setPastedText] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [status, setStatus] = useState<string | null>(null);

  const handleIngest = async () => {
    setIsLoading(true);
    try {
      const response = await invoke('ingest_bulk_links', {
        rawText: pastedText
      });
      
      setStatus(`✓ ${response.message}`);
      setPastedText(''); // Clear the textarea
      
      // Auto-clear status message after 5 seconds
      setTimeout(() => setStatus(null), 5000);
      
    } catch (error) {
      setStatus(`✗ Error: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="bulk-ingest-panel">
      <h2>Bulk Ingest URLs</h2>
      <p className="hint">
        Paste raw text containing URLs (e.g., output from an AI research session).
        Links will be extracted and processed in the background.
      </p>
      
      <textarea
        value={pastedText}
        onChange={(e) => setPastedText(e.target.value)}
        placeholder="Paste URLs here..."
        rows={10}
        disabled={isLoading}
      />
      
      <button 
        onClick={handleIngest}
        disabled={isLoading || pastedText.trim() === ''}
      >
        {isLoading ? 'Processing...' : 'Ingest'}
      </button>
      
      {status && (
        <div className={`status ${status.startsWith('✓') ? 'success' : 'error'}`}>
          {status}
        </div>
      )}
    </div>
  );
}
```

## Backend (Rust)

### Example 1: Direct Command Invocation

```rust
// In tests or CLI
use pkm_app::AppService;

#[tokio::main]
async fn main() {
    let service = AppService::new("~/.pkm/pkm.db", None)
        .expect("Failed to create service");
    
    let raw_text = r#"
        Here are some research URLs:
        https://example.com/article-1
        https://research.org/paper-2
        https://news.site/story-3
    "#;
    
    let result = service.ingest_bulk_links(raw_text.to_string()).await
        .expect("Ingestion failed");
    
    println!("Queued {} links for processing", result);
    
    // Wait for processing to complete (not implemented yet)
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
}
```

### Example 2: Processing with Custom Configuration

```rust
use pkm_app::ingestion;
use std::time::Duration;

async fn ingest_with_config() {
    // Extract URLs from text
    let raw_text = "...pasted text...";
    let urls = ingestion::extract_urls(raw_text);
    
    println!("Found {} URLs", urls.len());
    
    // Manually queue for processing
    let service = AppService::new("~/.pkm/pkm.db", None)?;
    service.ingest_bulk_links(raw_text.to_string()).await?;
    
    // Wait for Tokio tasks to complete
    tokio::time::sleep(Duration::from_secs(60)).await;
}
```

### Example 3: Testing URL Extraction

```rust
#[cfg(test)]
mod tests {
    use pkm_app::ingestion;

    #[test]
    fn test_extract_urls() {
        let text = r#"
            Check out these links:
            https://example.com/article
            https://research.org/paper-2?id=123
            http://old-site.com
            
            And this one: https://news.site/story (in parentheses)
        "#;
        
        let urls = ingestion::extract_urls(text);
        
        assert_eq!(urls.len(), 4);
        assert!(urls[0].contains("example.com"));
        assert!(urls[1].contains("research.org"));
        assert!(urls[2].contains("old-site.com"));
        assert!(urls[3].contains("news.site"));
        
        // Ensure no trailing punctuation
        for url in &urls {
            assert!(!url.ends_with(')'));
            assert!(!url.ends_with('('));
        }
    }

    #[test]
    fn test_extract_urls_mixed_content() {
        let text = "Visit https://example.com or https://other.org for details.";
        let urls = ingestion::extract_urls(text);
        
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "https://example.com");
        assert_eq!(urls[1], "https://other.org");
    }
}
```

### Example 4: Custom AI Analysis Integration

```rust
// Replace the mock implementation with real API calls
use reqwest::Client;

pub async fn analyze_content_gemini(
    markdown: &str,
    url: &str
) -> Result<AiAnalysisResult, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_key = std::env::var("GEMINI_API_KEY")?;
    
    let response = client
        .post("https://generativelanguage.googleapis.com/v1/models/gemini-3.5-flash:generateContent")
        .header("Content-Type", "application/json")
        .query(&[("key", api_key)])
        .json(&serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": format!(
                        "Analyze this web article and provide:\n1. A concise title (max 50 chars)\n2. A 1-sentence summary\n\nContent:\n{}",
                        markdown
                    )
                }]
            }]
        }))
        .send()
        .await?;
    
    let result = response.json::<GeminiResponse>().await?;
    
    // Parse Gemini's response and return title + summary
    let content = result.candidates[0].content.parts[0].text.clone();
    
    let (title, summary) = parse_gemini_response(&content);
    
    Ok(AiAnalysisResult { title, summary })
}

pub async fn analyze_content_claude(
    markdown: &str,
    url: &str
) -> Result<AiAnalysisResult, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_key = std::env::var("ANTHROPIC_API_KEY")?;
    
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": "claude-haiku-4.5-20251001",
            "max_tokens": 200,
            "messages": [{
                "role": "user",
                "content": format!(
                    "Analyze this web article and provide:\n1. A concise title (max 50 chars)\n2. A 1-sentence summary\n\nContent:\n{}",
                    markdown
                )
            }]
        }))
        .send()
        .await?;
    
    let result = response.json::<ClaudeResponse>().await?;
    
    // Parse Claude's response
    let content = &result.content[0].text;
    let (title, summary) = parse_claude_response(content);
    
    Ok(AiAnalysisResult { title, summary })
}
```

## Complete End-to-End Workflow

### Step 1: User Preparation
```
AI Research Session (90 minutes)
  ↓
Collect 50 URLs from various sources
  ↓
Copy text block with all URLs (mixed with notes)
```

### Step 2: Paste into App
```
User clicks "Bulk Ingest" tab
  ↓
Pastes 50 URLs into textarea
  ↓
Clicks "Process Links" button
```

### Step 3: Backend Processing
```
Command: ingest_bulk_links(raw_text)
  ↓
Regex extracts 50 URLs
  ↓
Tokio spawns 50 concurrent tasks
  ├─ URL 1: Fetch → Scrape → Analyze → Create Note → Link
  ├─ URL 2: Fetch → Scrape → Analyze → Create Note → Link
  ├─ ...
  └─ URL 50: Fetch → Scrape → Analyze → Create Note → Link
  ↓
All operations logged to agent_action table (50 entries)
```

### Step 4: UI Reflects Changes
```
File watcher detects 50 new markdown files
  ↓
AppService syncs notes to database
  ↓
Frontend refreshes view
  ↓
User sees 50 new notes with auto-generated titles + summaries
```

### Step 5: Manual Review (Optional)
```
User creates a ReviewQueue view filtered by:
  created_at > now - 1 hour
  created_by = "System"
  ↓
Reviews each note's title + summary
  ↓
Deletes garbage ones (5-10 notes)
  ↓
Edits promising ones to add context
  ↓
Links notes to entities manually
```

### Step 6: Disaster Recovery (If Needed)
```
User realizes ingestion was bad (AI hallucinated summaries)
  ↓
Clicks "Revert Last Batch" button
  ↓
Choose "Last 60 minutes"
  ↓
System calls: rollback_autonomous_ingestion(60)
  ↓
All 50 created notes deleted
All links removed
All sources reverted to "Captured" state
  ↓
Re-analyze later with different AI model
```

## Real-World Scenarios

### Scenario A: Academic Research (200 URLs over 1 week)

```typescript
// Daily workflow
for (let day = 0; day < 7; day++) {
  // Collect ~30 URLs from research feeds
  const dailyUrls = await collectResearchUrls();
  
  // Bulk ingest at end of day
  await invoke('ingest_bulk_links', { rawText: dailyUrls });
  
  // Notes appear automatically (30s processing)
  // Review tomorrow morning if needed
  
  // By day 7: 200 notes, all linked to sources, tagged
}
```

### Scenario B: News Monitoring (100 URLs per day)

```typescript
// Automated ingestion every morning
function scheduleIngestNews() {
  // 6 AM: Fetch news summaries
  // 6:05 AM: Ingest all URLs
  
  setInterval(async () => {
    const newsText = await fetchNewsFeeds();
    await invoke('ingest_bulk_links', { rawText: newsText });
    
    // Notification: "50 news articles ingested"
    showNotification('50 news articles ingested', 'info');
  }, 24 * 60 * 60 * 1000); // Daily
}
```

### Scenario C: Competitive Analysis (500 URLs one-time)

```typescript
// One-time massive ingest
async function ingestCompetitors() {
  // Collect 500 competitor links across 20 sites
  const competitorLinks = await buildCompetitorList();
  
  // Ingest in batches (avoid Jina rate limits)
  const batches = chunk(competitorLinks, 50);
  
  for (const batch of batches) {
    await invoke('ingest_bulk_links', { rawText: batch.join('\n') });
    
    // Wait 30 seconds between batches
    await delay(30000);
  }
  
  // Result: 500 notes, all with auto-summaries
  // Create a graph view to explore relationships
}
```

## Monitoring & Debugging

### Check Ingestion Queue Status

```rust
// Not yet implemented, but you could add:
#[tauri::command]
async fn get_ingestion_status(
    state: tauri::State<'_, Arc<Mutex<AppService>>>,
) -> Result<IngestionStatus, String> {
    // Return: { pending_urls: 42, processing: 15, completed_today: 300 }
}
```

### View Recent Actions

```sql
-- Query audit trail for recent ingestions
SELECT id, created_at, target, status
FROM agent_action
WHERE actor LIKE '%Autonomous-Ingestor%'
  AND created_at > datetime('now', '-1 hour')
ORDER BY created_at DESC
LIMIT 20;
```

### Debug Individual Ingestion

```rust
// If a URL fails, trace it in logs:
2024-11-15 14:23:45 [INFO] Processing: https://example.com/article
2024-11-15 14:23:45 [INFO] Jina fetch: OK (2340 bytes)
2024-11-15 14:23:46 [INFO] AI analysis: OK (title: "...", summary: "...")
2024-11-15 14:23:46 [INFO] Source created: 8a3c5e9b-...
2024-11-15 14:23:46 [INFO] Note created: 7f2d1a4c-...
2024-11-15 14:23:46 [INFO] Link created: c9e1b3f2-... (DerivedFrom)
2024-11-15 14:23:46 [INFO] ✓ Autonomously promoted
```

## Performance Tips

1. **Batch ingestion:** Queue 30-50 URLs at a time to balance throughput vs. responsiveness
2. **Rate limiting:** Respect Jina's rate limits; add delays between batches if needed
3. **Off-peak ingestion:** Run large ingestions during off-peak hours
4. **Monitor memory:** Tokio tasks are lightweight, but 500+ concurrent tasks might need tuning
5. **AI model selection:** Use Gemini 3.5 Flash (faster, cheaper) for volume; Claude for quality

