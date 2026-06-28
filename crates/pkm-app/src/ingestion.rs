//! Autonomous bulk link ingestion system with Jina free tier rate limiting.
//!
//! Architecture:
//! - Producer: URL extraction from pasted text
//! - Queue: Pure in-memory unbounded mpsc channel (No SQLite!)
//! - Fetcher: Rate-limited ticker (1 request / 3 seconds = 20 RPM)
//! - Processor: Non-blocking tokio spawned tasks that write directly to Memory & Disk

use pkm_core::block::{Block, BlockContent};
use pkm_core::id::{BlockId, NoteId, ObjectRef, SourceId, LinkId};
use pkm_core::ingestion::IngestionState;
use pkm_core::link::Link;
use pkm_core::note::Note;
use pkm_core::source::{Source, SourceOrigin};
use pkm_core::{Actor, Timestamp};
use pkm_fs::SharedVault;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::LazyLock;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://[^\s]+").expect("Invalid URL regex")
});

/// Extract all URLs from a text block using regex.
pub fn extract_urls(text: &str) -> Vec<String> {
    URL_REGEX
        .find_iter(text)
        .map(|mat| {
            mat.as_str()
                .trim_end_matches(&[')', ']', '"', '\'', '.', ','][..])
                .to_string()
        })
        .collect()
}

/// Compute a hash of content for deduplication.
pub fn compute_hash(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Start the in-memory, rate-limited background ingestion worker.
/// Returns a sender for queuing individual URLs.
pub fn start_ingestion_worker(
    vault_state: SharedVault,
    vault_path: PathBuf,
) -> mpsc::UnboundedSender<String> {
    // Unbounded channel: users can paste 500 URLs, they just sit in RAM.
    let (tx, rx) = mpsc::unbounded_channel::<String>();

    tokio::spawn(run_rate_limited_fetcher(rx, vault_state, vault_path));

    tx
}

/// The main worker loop. Pulls from the channel, waits for the rate limit tick,
/// hits Jina, and then passes the markdown off to a spawned task.
async fn run_rate_limited_fetcher(
    mut rx: mpsc::UnboundedReceiver<String>,
    vault_state: SharedVault,
    vault_path: PathBuf,
) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
        
    // Enforce exactly 1 iteration per 3 seconds (20 RPM limit for Jina Free Tier)
    let mut ticker = interval(Duration::from_secs(3));

    while let Some(url) = rx.recv().await {
        // Block until the 3-second window has passed
        ticker.tick().await;

        println!("[Jina Fetcher] Processing URL: {}", url);
        let jina_url = format!("https://r.jina.ai/{}", url);

        let markdown = match client.get(&jina_url).send().await {
            Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
            Ok(resp) => {
                eprintln!("[Jina] HTTP {}: {} (Skipping)", resp.status(), url);
                continue; // No database state to update, just drop it!
            }
            Err(e) => {
                eprintln!("[Jina] Network error: {} (Skipping)", e);
                continue; // Drop it!
            }
        };

        if markdown.is_empty() {
            eprintln!("[Jina] Empty response for {} (Skipping)", url);
            continue;
        }

        println!("[Jina Fetcher] ✓ Fetched {} bytes", markdown.len());

        // Spawn LLM and Disk I/O into a separate task so the ticker can proceed immediately
        let vault_state_clone = vault_state.clone();
        let vault_path_clone = vault_path.clone();
        let url_clone = url.clone();

        tokio::spawn(async move {
            if let Err(e) = process_and_promote(
                url_clone.clone(),
                markdown,
                vault_state_clone,
                vault_path_clone,
            )
            .await
            {
                eprintln!("[Processor] Error processing {}: {}", url_clone, e);
            }
        });
    }
}

/// Process markdown: LLM reasoning + writing to Memory/Disk.
async fn process_and_promote(
    url: String,
    markdown: String,
    vault_state: SharedVault,
    vault_path: PathBuf,
) -> Result<(), String> {
    let now = Timestamp::now_utc();
    let source_id = SourceId::new();
    let agent_actor = Actor::Agent { name: "Autonomous-Ingestor".into() };

    // 1. Create the Source representation
    let source = Source {
        id: source_id,
        origin: SourceOrigin::WebArticle { url: url.clone() },
        title: None,
        raw_content: markdown.clone(),
        captured_at: now,
        content_hash: compute_hash(&markdown),
        ingestion_state: IngestionState::Promoted, // Straight to promoted!
        created_by: agent_actor.clone(),
        created_at: now,
        version: 1,
        updated_at: now,
    };

    // 2. Simulate LLM latency & reasoning (Doesn't block Jina fetcher!)
    let ai_results = analyze_content_mock(&markdown, &url).await;

    // 3. Create Note and Block
    let note_id = NoteId::new();
    let block_id = BlockId::new();

    let block = Block {
        id: block_id,
        note_id,
        content: BlockContent::Markdown { text: ai_results.summary },
        created_by: agent_actor.clone(),
        created_at: now,
        source_provenance_ref: None,
        version: 1,
        updated_at: now,
    };

    let note = Note {
        id: note_id,
        title: ai_results.title,
        blocks: vec![block_id],
        metadata: BTreeMap::new(),
        created_by: agent_actor.clone(),
        created_at: now,
        version: 1,
        updated_at: now,
    };

    // 4. Create Provenance Link
    let link = Link {
        id: LinkId::new(),
        from: ObjectRef::Note(note_id),
        to: ObjectRef::Source(source_id),
        link_type: pkm_core::link::LinkType::DerivedFrom,
        created_by: Actor::System,
        created_at: now,
        reviewed: pkm_core::review::ReviewState::Accepted,
        confidence: None,
        version: 1,
        updated_at: now,
    };

    // 5. Update the In-Memory Database (Locks only for a few microseconds)
    {
        let mut state = vault_state.write().unwrap();
        state.sources.insert(source_id, source.clone());
        state.notes.insert(note_id, note.clone());
        state.blocks.insert(block_id, block.clone());
        state.links.insert(link.id, link.clone());
        state.rebuild_indexes();

        // Save JSON metadata so they match memory state
        let sources_path = vault_path.join(".pkm").join("sources.json");
        if let Ok(sources_json) = serde_json::to_string_pretty(&state.sources) {
            let _ = std::fs::write(sources_path, sources_json);
        }
        let links_path = vault_path.join(".pkm").join("links.json");
        if let Ok(links_json) = serde_json::to_string_pretty(&state.links) {
            let _ = std::fs::write(links_path, links_json);
        }
    }

    // 6. Persist to Disk
    let sources_dir = vault_path.join("sources");
    let notes_dir = vault_path.join("notes");
    
    std::fs::create_dir_all(&sources_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&notes_dir).map_err(|e| e.to_string())?;

    // Save Source as raw markdown
    std::fs::write(
        sources_dir.join(format!("{}.md", source_id)),
        &source.raw_content,
    ).map_err(|e| e.to_string())?;

    // Save Note as styled markdown
    let note_md = pkm_core::markdown::note_to_markdown(&note, &[block]);
    std::fs::write(
        notes_dir.join(note.file_name()),
        note_md,
    ).map_err(|e| e.to_string())?;

    println!("[Processor] ✓ Successfully ingested & promoted: {}", url);
    Ok(())
}

/// Mock AI analysis. Replace with real API calls to Gemini/Claude.
async fn analyze_content_mock(markdown: &str, url: &str) -> AiAnalysisResult {
    // Simulate LLM latency
    tokio::time::sleep(Duration::from_millis(500)).await;

    let title = if let Some(line) = markdown.lines().find(|l| l.starts_with('#')) {
        line.trim_start_matches('#').trim().to_string()
    } else if let Ok(parsed) = url::Url::parse(url) {
        parsed.host_str().unwrap_or(url).to_string()
    } else {
        url.to_string()
    };

    let summary = markdown.chars().take(200).collect::<String>();
    AiAnalysisResult { title, summary }
}

#[derive(Clone, Debug)]
pub struct AiAnalysisResult {
    pub title: String,
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_urls() {
        let text = "Here are some links: https://example.com https://test.org/path";
        let urls = extract_urls(text);
        assert_eq!(urls.len(), 2);
        assert!(urls[0].starts_with("https://"));
    }

    #[test]
    fn test_extract_urls_with_punctuation() {
        let text = "Check out https://example.com. And also https://test.org)";
        let urls = extract_urls(text);
        assert_eq!(urls.len(), 2);
        assert!(!urls[0].ends_with('.'));
        assert!(!urls[1].ends_with(')'));
    }

    #[test]
    fn test_compute_hash() {
        let hash1 = compute_hash("content");
        let hash2 = compute_hash("content");
        let hash3 = compute_hash("different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
