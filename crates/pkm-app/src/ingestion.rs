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
use pkm_core::id::{SourceId, NoteId, ObjectRef, BlockId};
use pkm_core::source::{Source, SourceOrigin};
use pkm_core::ingestion::IngestionState;
use pkm_core::ports::SourceRepo;
use pkm_core::{Actor, Timestamp};
use pkm_storage::{SqliteSourceRepo, SqliteAgentActionRepo, SqliteNoteRepo};
use pkm_storage::repositories::SqliteLinkRepo;
use pkm_agent::{Operation, OperationRequest};
use regex::Regex;
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
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Start the rate-limited background ingestion worker.
/// Returns a sender for queuing individual URLs.
///
/// Architecture:
/// - URLs are persisted to the SQLite ingestion_queue table
/// - Fetcher loop runs at exactly 1 request per 3 seconds (20 RPM)
/// - LLM + DB work happens in separate tokio::spawn tasks (non-blocking)
/// - Queue survives app restarts for durability
pub fn start_ingestion_worker(
    pool: DbPool,
    vault_path: std::path::PathBuf,
) -> mpsc::Sender<String> {
    let (tx, rx) = mpsc::channel::<String>(100);

    tokio::spawn(run_ingestion_dispatcher(pool.clone(), rx));
    tokio::spawn(run_rate_limited_fetcher(pool, vault_path));

    tx
}

/// Dispatcher: receives URLs from the channel and persists them to SQLite.
async fn run_ingestion_dispatcher(
    pool: DbPool,
    mut rx: mpsc::Receiver<String>,
) {
    while let Some(url) = rx.recv().await {
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(conn) = pool_clone.get() {
                    let now = chrono::Utc::now().to_rfc3339();
                    let _ = conn.execute(
                        "INSERT INTO ingestion_queue (url, status, created_at) VALUES (?1, 'pending', ?2)",
                        rusqlite::params![&url, &now],
                    );
                }
            }).await;
        });
    }
}

async fn run_rate_limited_fetcher(
    pool: DbPool,
    vault_path: std::path::PathBuf,
) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let mut ticker = interval(Duration::from_secs(3)); // 20 RPM = 1 request per 3 seconds

    loop {
        // CRITICAL: Wait for the tick before processing the next URL
        ticker.tick().await;

        // Fetch one pending URL from the queue, atomically marking it as processing
        let pool_clone_fetch = pool.clone();
        let db_res = tokio::task::spawn_blocking(move || {
            let conn = pool_clone_fetch.get().map_err(|e| e.to_string())?;
            let mut stmt = conn.prepare(
                "UPDATE ingestion_queue SET status = 'processing' \
                 WHERE id = (SELECT id FROM ingestion_queue WHERE status = 'pending' ORDER BY created_at ASC LIMIT 1) \
                 RETURNING id, url"
            ).map_err(|e| e.to_string())?;

            match stmt.query_row([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            }) {
                Ok(result) => Ok(Some(result)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.to_string()),
            }
        }).await;

        let db_res = match db_res {
            Ok(Ok(res)) => res,
            Ok(Err(e)) => {
                eprintln!("[Queue] DB error: {}", e);
                continue;
            }
            Err(e) => {
                eprintln!("[Queue] Spawn blocking join error: {}", e);
                continue;
            }
        };

        let (queue_id, url) = match db_res {
            Some(res) => res,
            None => continue, // No more URLs to process
        };

        println!("[Jina Fetcher] Processing URL (3-sec rate limit): {}", url);

        // Clone data for the async task
        let pool_clone = pool.clone();
        let vault_clone = vault_path.clone();
        let client_clone = client.clone();

        // Spawn the fetch + process in a separate task so the loop immediately loops again
        // This decouples network latency from the rate limit ticker
        tokio::spawn(async move {
            let jina_url = format!("https://r.jina.ai/{}", url.clone());
            let markdown = match client_clone.get(&jina_url).send().await {
                Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
                Ok(resp) => {
                    eprintln!("[Jina] HTTP {}: {}", resp.status(), url);
                    // Mark as failed
                    let pool_db = pool_clone.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        if let Ok(conn) = pool_db.get() {
                            let _ = conn.execute(
                                "UPDATE ingestion_queue SET status = 'failed', error_message = ?1, processed_at = ?2 WHERE id = ?3",
                                rusqlite::params![format!("HTTP {}", resp.status()), chrono::Utc::now().to_rfc3339(), queue_id],
                            );
                        }
                    }).await;
                    return;
                }
                Err(e) => {
                    eprintln!("[Jina] Network error (timeout or connection): {}", e);
                    // Mark as failed
                    let pool_db = pool_clone.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        if let Ok(conn) = pool_db.get() {
                            let _ = conn.execute(
                                "UPDATE ingestion_queue SET status = 'failed', error_message = ?1, processed_at = ?2 WHERE id = ?3",
                                rusqlite::params![e.to_string(), chrono::Utc::now().to_rfc3339(), queue_id],
                            );
                        }
                    }).await;
                    return;
                }
            };

            if markdown.is_empty() {
                eprintln!("[Jina] Empty response for {}", url);
                let pool_db = pool_clone.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    if let Ok(conn) = pool_db.get() {
                        let _ = conn.execute(
                            "UPDATE ingestion_queue SET status = 'failed', error_message = ?1, processed_at = ?2 WHERE id = ?3",
                            rusqlite::params!["Empty response", chrono::Utc::now().to_rfc3339(), queue_id],
                        );
                    }
                }).await;
                return;
            }

            println!("[Jina Fetcher] ✓ Fetched {} bytes from {}", markdown.len(), url);

            // Process markdown (LLM reasoning + DB writes) in the spawned task
            match process_and_promote_source(
                pool_clone.clone(),
                vault_clone,
                client_clone,
                url.clone(),
                markdown,
            )
            .await
            {
                Ok(()) => {
                    // Mark as completed
                    let pool_db = pool_clone.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        if let Ok(conn) = pool_db.get() {
                            let _ = conn.execute(
                                "UPDATE ingestion_queue SET status = 'completed', processed_at = ?1 WHERE id = ?2",
                                rusqlite::params![chrono::Utc::now().to_rfc3339(), queue_id],
                            );
                        }
                    }).await;
                }
                Err(e) => {
                    eprintln!("[Processor] Error for {}: {}", url, e);
                    // Mark as failed
                    let pool_db = pool_clone.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        if let Ok(conn) = pool_db.get() {
                            let _ = conn.execute(
                                "UPDATE ingestion_queue SET status = 'failed', error_message = ?1, processed_at = ?2 WHERE id = ?3",
                                rusqlite::params![e, chrono::Utc::now().to_rfc3339(), queue_id],
                            );
                        }
                    }).await;
                }
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

            let action = pkm_agent::execute(create_note_op, &action_repo, &note_repo)
                .map_err(|e| format!("Execute CreateNote failed: {}", e))?;
            pkm_agent::apply_action(action.id, &action_repo, &note_repo, Some(&link_repo))
                .map_err(|e| format!("Apply CreateNote failed: {}", e))?;

            println!("[Processor] ✓ Note created: {}", note_id);

            // ACTION B: Create Block (inject ai_results.summary)
            let block_id = BlockId::new();
            let create_block_op = OperationRequest {
                actor: agent_actor.clone(),
                rationale: "Autonomous note summary block".to_string(),
                operation: Operation::CreateBlock {
                    note_id,
                    block_id,
                    content: pkm_core::block::BlockContent::Markdown {
                        text: ai_results.summary.clone(),
                    },
                    order: "000000".to_string(),
                },
            };

            let action = pkm_agent::execute(create_block_op, &action_repo, &note_repo)
                .map_err(|e| format!("Execute CreateBlock failed: {}", e))?;
            pkm_agent::apply_action(action.id, &action_repo, &note_repo, Some(&link_repo))
                .map_err(|e| format!("Apply CreateBlock failed: {}", e))?;

            println!("[Processor] ✓ Block created: {}", block_id);

            // ACTION C: Create DerivedFrom link
            let link_op = OperationRequest {
                actor: agent_actor.clone(),
                rationale: "Provenance link to source".to_string(),
                operation: Operation::CreateTypedLink {
                    from: ObjectRef::Note(note_id),
                    to: ObjectRef::Source(source_id),
                    link_type: pkm_core::link::LinkType::DerivedFrom,
                },
            };

            let action = pkm_agent::execute(link_op, &action_repo, &note_repo)
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
