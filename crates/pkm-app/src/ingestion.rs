//! Autonomous bulk link ingestion system with Jina free tier rate limiting.
//!
//! Architecture:
//! - Producer: URL extraction from pasted text
//! - Queue: Pure in-memory unbounded mpsc channel (No SQLite!)
//! - Fetcher: Rate-limited ticker (1 request / 3 seconds = 20 RPM)
//! - Processor: Non-blocking tokio spawned tasks that write through repository traits

use pkm_core::block::{Block, BlockContent};
use pkm_core::id::{BlockId, NoteId, ObjectRef, SourceId, LinkId};
use pkm_core::ingestion::IngestionState;
use pkm_core::link::Link;
use pkm_core::note::Note;
use pkm_core::ports::{NoteRepo, SourceRepo, LinkRepo};
use pkm_core::source::{Source, SourceOrigin};
use pkm_core::{Actor, Timestamp};
use pkm_fs::SharedVault;
use pkm_fs::{FsNoteRepo, FsSourceRepo, FsLinkRepo};
use regex::Regex;
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

/// A URL queued for ingestion, with retry tracking.
#[derive(Debug, Clone)]
pub struct IngestPayload {
    pub url: String,
    pub retries: u8,
}

/// Start the in-memory, rate-limited background ingestion worker.
/// Returns a sender for queuing individual URLs.
pub fn start_ingestion_worker(
    vault_state: SharedVault,
    vault_path: PathBuf,
) -> mpsc::UnboundedSender<IngestPayload> {
    let (tx, rx) = mpsc::unbounded_channel::<IngestPayload>();
    let tx_clone = tx.clone();
    tokio::spawn(run_rate_limited_fetcher(rx, tx_clone, vault_state, vault_path));
    tx
}

/// Create a Source in Captured state for a URL before the fetch begins.
/// This ensures the URL is never lost — even if the fetch fails, the user can
/// see the failure in the UI and retry.
fn create_source_record(url: &str, vault_state: &SharedVault, vault_path: &PathBuf) -> Source {
    let now = Timestamp::now_utc();
    let source = Source {
        id: SourceId::new(),
        origin: SourceOrigin::WebArticle { url: url.to_string() },
        title: None,
        raw_content: String::new(),
        captured_at: now,
        content_hash: String::new(),
        ingestion_state: IngestionState::Captured,
        created_by: Actor::Agent { name: "Autonomous-Ingestor".into() },
        created_at: now,
        version: 1,
        updated_at: now,
    };

    // Write source record to disk via repository
    let source_repo = FsSourceRepo { state: vault_state.clone(), vault_path: vault_path.clone() };
    let _ = source_repo.create(&source);

    source
}

/// Mark a source as Failed with an error message.
fn mark_source_failed(source_id: SourceId, error_message: &str, vault_state: &SharedVault, vault_path: &PathBuf) {
    let source_repo = FsSourceRepo { state: vault_state.clone(), vault_path: vault_path.clone() };
    let _ = source_repo.update_ingestion_state(source_id, IngestionState::Failed);
    eprintln!("[Jina] Source {} failed: {}", source_id, error_message);
}

/// The main worker loop. Pulls from the channel, waits for the rate limit tick,
/// hits Jina, and then passes the markdown off to a spawned task.
async fn run_rate_limited_fetcher(
    mut rx: mpsc::UnboundedReceiver<IngestPayload>,
    tx: mpsc::UnboundedSender<IngestPayload>,
    vault_state: SharedVault,
    vault_path: PathBuf,
) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Enforce exactly 1 iteration per 3 seconds (20 RPM limit for Jina Free Tier)
    let mut ticker = interval(Duration::from_secs(3));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    while let Some(payload) = rx.recv().await {
        ticker.tick().await;

        let client = client.clone();
        let tx = tx.clone();
        let vault_state = vault_state.clone();
        let vault_path = vault_path.clone();

        // Spawn the entire HTTP fetch + processing into a background task so
        // the ticker can proceed with the next URL immediately. Without this,
        // a 15-second Jina response would limit throughput to ~4 RPM.
        tokio::spawn(async move {
            println!("[Jina Fetcher] Processing URL: {}", payload.url);

            // Clone before moving into create_source_record
            let vs = vault_state.clone();
            let vp = vault_path.clone();
            // Create source record BEFORE fetch so the URL is never lost
            let source = create_source_record(&payload.url, &vs, &vp);
            let source_id = source.id;

            let jina_url = format!("https://r.jina.ai/{}", payload.url);

            let markdown = match client.get(&jina_url).send().await {
                Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
                Ok(resp) if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    if payload.retries < 3 {
                        eprintln!("[Jina] 429 Rate limited. Re-queuing {} (retry {}/3).", payload.url, payload.retries + 1);
                        mark_source_failed(source_id, &format!("Rate limited (429), retry {}/3", payload.retries + 1), &vault_state, &vault_path);
                        tokio::spawn(async move {
                            tokio::time::sleep(Duration::from_secs(10)).await;
                            let _ = tx.send(IngestPayload { url: payload.url, retries: payload.retries + 1 });
                        });
                    } else {
                        mark_source_failed(source_id, "Max retries (3) reached — rate limited", &vault_state, &vault_path);
                    }
                    return;
                }
                Ok(resp) => {
                    mark_source_failed(source_id, &format!("HTTP {} error", resp.status()), &vault_state, &vault_path);
                    return;
                }
                Err(e) => {
                    mark_source_failed(source_id, &format!("Network error: {}", e), &vault_state, &vault_path);
                    return;
                }
            };

            if markdown.is_empty() {
                mark_source_failed(source_id, "Empty response from Jina", &vault_state, &vault_path);
                return;
            }

            println!("[Jina Fetcher] ✓ Fetched {} bytes", markdown.len());

            if let Err(e) = process_and_promote(
                source_id,
                payload.url.clone(),
                markdown,
                vault_state.clone(),
                vault_path.clone(),
            )
            .await
            {
                mark_source_failed(source_id, &format!("Processing error: {}", e), &vault_state, &vault_path);
            }
        });
    }
}

/// Process markdown: LLM reasoning + writing through repository traits.
/// Uses the existing source_id from the placeholder record to avoid ghost records.
async fn process_and_promote(
    source_id: SourceId,
    url: String,
    markdown: String,
    vault_state: SharedVault,
    vault_path: PathBuf,
) -> Result<(), String> {
    let now = Timestamp::now_utc();
    let agent_actor = Actor::Agent { name: "Autonomous-Ingestor".into() };

    // 1. Create the Source representation
    let source = Source {
        id: source_id,
        origin: SourceOrigin::WebArticle { url: url.clone() },
        title: None,
        raw_content: markdown.clone(),
        captured_at: now,
        content_hash: compute_hash(&markdown),
        ingestion_state: IngestionState::AwaitingReview,
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
        source_provenance_ref: Some(ObjectRef::Source(source_id)),
        version: 1,
        updated_at: now,
    };

    let note = Note {
        id: note_id,
        title: ai_results.title,
        blocks: vec![block_id],
        metadata: pkm_core::note::NoteMetadata::default(),
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

    // 5. Persist through repository traits (wrap blocking I/O in spawn_blocking)
    let source_repo = FsSourceRepo { state: vault_state.clone(), vault_path: vault_path.clone() };
    let note_repo = FsNoteRepo { state: vault_state.clone(), vault_path: vault_path.clone() };
    let link_repo = FsLinkRepo { state: vault_state.clone(), vault_path: vault_path.clone() };

    let source_clone = source.clone();
    let note_clone = note.clone();
    let block_clone = block.clone();
    let link_clone = link.clone();

    let vault_path_captured = vault_path.clone();
    tokio::task::spawn_blocking(move || {
        let note_file_path = vault_path_captured
            .join("notes")
            .join(note_clone.file_name());

        source_repo
            .create(&source_clone)
            .map_err(|e| format!("Failed to save source: {}", e))?;
        note_repo
            .upsert_from_external(&note_clone, &[block_clone], &note_file_path)
            .map_err(|e| format!("Failed to save note: {}", e))?;
        link_repo
            .create(&link_clone)
            .map_err(|e| format!("Failed to save link: {}", e))?;

        // Skip watcher notification AFTER the writes complete to prevent race
        if let Some(handle) = crate::service::get_watcher_ignore_handle() {
            handle.skip_next(note_file_path);
        }
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {}", e))?
    .map_err(|e: String| e)?;

    println!("[Processor] ✓ Successfully ingested & promoted: {}", url);
    Ok(())
}

/// Mock AI analysis. Replace with real API calls to Gemini/Claude.
async fn analyze_content_mock(markdown: &str, url: &str) -> AiAnalysisResult {
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