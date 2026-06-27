//! Autonomous bulk link ingestion system with Jina free tier rate limiting.
//!
//! Architecture:
//! - Producer: URL extraction from pasted text
//! - Queue: Unbounded mpsc channel for URL buffering
//! - Fetcher: Single-threaded 3-second ticker (20 RPM = 1 request/3 seconds)
//! - Processor: Separate tokio::spawn tasks for LLM + DB work
//!
//! This ensures Jina free tier compliance while maximizing throughput.

use crate::db_pool::DbPool;
use pkm_core::id::{SourceId, NoteId, ObjectRef};
use pkm_core::source::{Source, SourceOrigin};
use pkm_core::ingestion::IngestionState;
use pkm_core::ports::SourceRepo;
use pkm_core::{Actor, Timestamp};
use pkm_storage::{SqliteSourceRepo, SqliteAgentActionRepo, SqliteNoteRepo};
use pkm_storage::repositories::SqliteLinkRepo;
use pkm_agent::{Operation, OperationRequest};
use regex::Regex;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

/// Extract all URLs from a text block using regex.
pub fn extract_urls(text: &str) -> Vec<String> {
    let url_regex = Regex::new(r"https?://[^\s]+").expect("Invalid URL regex");
    url_regex
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
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Start the rate-limited background ingestion worker.
/// Returns an unbounded sender for queuing individual URLs.
///
/// Architecture:
/// - The sender accepts URLs immediately (unbounded queue)
/// - Fetcher loop runs at exactly 1 request per 3 seconds (20 RPM)
/// - LLM + DB work happens in separate tokio::spawn tasks (non-blocking)
pub fn start_ingestion_worker(
    pool: DbPool,
    vault_path: std::path::PathBuf,
) -> mpsc::UnboundedSender<String> {
    let (tx, rx) = mpsc::unbounded_channel::<String>();

    tokio::spawn(run_rate_limited_fetcher(pool, vault_path, rx));

    tx
}

/// The rate-limited fetcher loop.
/// Processes exactly one URL every 3 seconds (20 RPM limit).
///
/// This ensures we never violate Jina's free tier rate limit while keeping
/// the fetcher moving at a steady cadence regardless of LLM latency.
async fn run_rate_limited_fetcher(
    pool: DbPool,
    vault_path: std::path::PathBuf,
    mut rx: mpsc::UnboundedReceiver<String>,
) {
    let client = reqwest::Client::new();
    let mut ticker = interval(Duration::from_secs(3)); // 20 RPM = 1 request per 3 seconds

    while let Some(url) = rx.recv().await {
        // CRITICAL: Wait for the tick before processing the next URL
        ticker.tick().await;

        println!("[Jina Fetcher] Processing URL (3-sec rate limit): {}", url);

        // Clone data for the async task
        let pool_clone = pool.clone();
        let vault_clone = vault_path.clone();
        let client_clone = client.clone();

        // Fetch markdown from Jina (blocking I/O, but quick)
        let jina_url = format!("https://r.jina.ai/{}", url.clone());
        let markdown = match client.get(&jina_url).send().await {
            Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
            Ok(resp) => {
                eprintln!("[Jina] HTTP {}: {}", resp.status(), url);
                continue;
            }
            Err(e) => {
                eprintln!("[Jina] Network error: {}", e);
                continue;
            }
        };

        if markdown.is_empty() {
            eprintln!("[Jina] Empty response for {}", url);
            continue;
        }

        println!("[Jina Fetcher] ✓ Fetched {} bytes from {}", markdown.len(), url);

        // CRITICAL: Spawn a separate task for LLM reasoning + database writes.
        // This ensures a slow LLM (10+ seconds) doesn't block the next 3-second Jina fetch.
        tokio::spawn(async move {
            if let Err(e) = process_and_promote_source(
                pool_clone,
                vault_clone,
                client_clone,
                url.clone(),
                markdown,
            )
            .await
            {
                eprintln!("[Processor] Error for {}: {}", url, e);
            }
        });
    }
}

/// Process markdown: LLM reasoning + autonomous Note/Link creation.
/// This runs in a separate tokio::spawn so it doesn't block the Jina fetcher.
async fn process_and_promote_source(
    pool: DbPool,
    vault_path: std::path::PathBuf,
    _http_client: reqwest::Client,
    url: String,
    markdown: String,
) -> Result<(), String> {
    let now = Timestamp::now_utc();
    let source_id = SourceId::new();

    // ==================== STEP 1: SAVE RAW SOURCE (Captured) ====================
    {
        let pool_db = pool.clone();
        let src_url = url.clone();
        let src_md = markdown.clone();

        tokio::task::spawn_blocking(move || {
            let conn = pool_db.get().map_err(|e| format!("DB connection failed: {}", e))?;
            let source_repo = SqliteSourceRepo { conn: &conn };

            let source = Source {
                id: source_id,
                origin: SourceOrigin::WebArticle { url: src_url },
                title: None,
                raw_content: src_md.clone(),
                captured_at: now,
                content_hash: compute_hash(&src_md),
                ingestion_state: IngestionState::Captured,
                created_by: Actor::System,
                created_at: now,
                version: 1,
                updated_at: now,
            };

            source_repo
                .create(&source)
                .map_err(|e| format!("Failed to create source: {}", e))?;

            Ok::<(), String>(())
        })
        .await
        .map_err(|e| format!("Spawn blocking error: {}", e))??;

        println!("[Processor] ✓ Source created: {}", source_id);
    }

    // ==================== STEP 2: LLM REASONING ====================
    // This can take 5-10+ seconds without blocking the Jina fetcher!
    let ai_results = analyze_content_mock(&markdown, &url).await;
    println!("[Processor] ✓ LLM analysis: title={}", ai_results.title);

    // ==================== STEP 3: AUTONOMOUS PROMOTION ====================
    // All agent operations run here, non-blocking
    {
        let pool_db = pool.clone();
        let vault = vault_path.clone();
        let ai_title = ai_results.title.clone();
        let src_url = url.clone();

        tokio::task::spawn_blocking(move || {
            let conn = pool_db.get().map_err(|e| format!("DB connection failed: {}", e))?;

            let action_repo = SqliteAgentActionRepo { conn: &conn };
            let note_repo = SqliteNoteRepo { conn: &conn, vault_path: vault };
            let link_repo = SqliteLinkRepo { conn: &conn };

            let agent_actor = Actor::Agent {
                name: "Autonomous-Ingestor-Haiku".to_string(),
            };

            // ACTION A: Create Note
            let note_id = NoteId::new();
            let create_note_op = OperationRequest {
                actor: agent_actor.clone(),
                rationale: "Autonomous bulk ingestion (rate-limited Jina)".to_string(),
                operation: Operation::CreateNote {
                    note_id,
                    title: ai_title,
                },
            };

            let action = pkm_agent::execute(create_note_op, &action_repo)
                .map_err(|e| format!("Execute CreateNote failed: {}", e))?;
            pkm_agent::apply_action(action.id, &action_repo, &note_repo, Some(&link_repo))
                .map_err(|e| format!("Apply CreateNote failed: {}", e))?;

            println!("[Processor] ✓ Note created: {}", note_id);

            // ACTION B: Create DerivedFrom link
            let link_op = OperationRequest {
                actor: agent_actor.clone(),
                rationale: "Provenance link to source".to_string(),
                operation: Operation::CreateTypedLink {
                    from: ObjectRef::Note(note_id),
                    to: ObjectRef::Source(source_id),
                    link_type: pkm_core::link::LinkType::DerivedFrom,
                },
            };

            let action = pkm_agent::execute(link_op, &action_repo)
                .map_err(|e| format!("Execute CreateTypedLink failed: {}", e))?;
            pkm_agent::apply_action(action.id, &action_repo, &note_repo, Some(&link_repo))
                .map_err(|e| format!("Apply CreateTypedLink failed: {}", e))?;

            // ACTION C: Promote source
            conn.execute(
                "UPDATE source SET ingestion_state = ?1 WHERE id = ?2",
                rusqlite::params!["promoted", source_id.to_string()],
            )
            .map_err(|e| format!("Update source state failed: {}", e))?;

            println!("[Processor] ✓ Autonomously promoted: {} -> {}", src_url, note_id);
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| format!("Spawn blocking error: {}", e))??;
    }

    Ok(())
}

/// Mock AI analysis. Replace with real API calls to Gemini/Claude.
async fn analyze_content_mock(markdown: &str, url: &str) -> AiAnalysisResult {
    // Simulate LLM latency (real LLM would be slower)
    tokio::time::sleep(Duration::from_millis(100)).await;

    let title = extract_title(markdown, url);
    let summary = markdown.chars().take(200).collect::<String>();

    AiAnalysisResult { title, summary }
}

/// Extract title from markdown or URL
fn extract_title(markdown: &str, url: &str) -> String {
    // Try first heading
    if let Some(line) = markdown.lines().find(|l| l.starts_with('#')) {
        return line.trim_start_matches('#').trim().to_string();
    }

    // Fallback to URL domain
    if let Ok(parsed_url) = url::Url::parse(url) {
        if let Some(host) = parsed_url.host_str() {
            return host.to_string();
        }
    }

    url.to_string()
}

/// Result from AI content analysis
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
