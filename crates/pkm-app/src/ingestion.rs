//! Autonomous bulk link ingestion system.
//!
//! This module handles the background processing of bulk URL ingestion:
//! 1. Extracts URLs from pasted text using regex
//! 2. Fetches content via Jina AI markdown API
//! 3. Creates Source objects in the database
//! 4. Calls an AI model to generate summaries
//! 5. Autonomously creates Notes and links them back to Sources
//! 6. Handles concurrent processing with Tokio

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

/// Extract all URLs from a text block using regex.
pub fn extract_urls(text: &str) -> Vec<String> {
    let url_regex = Regex::new(r"https?://[^\s]+").expect("Invalid URL regex");
    url_regex
        .find_iter(text)
        .map(|mat| mat.as_str().trim_end_matches(&[')', ']', '"', '\''][..]).to_string())
        .collect()
}

/// Compute a hash of content for deduplication.
pub fn compute_hash(content: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Start the background ingestion worker.
/// Returns an mpsc sender that can be used to queue URL batches for processing.
pub fn start_ingestion_worker(pool: DbPool, vault_path: std::path::PathBuf) -> mpsc::Sender<Vec<String>> {
    let (tx, mut rx) = mpsc::channel::<Vec<String>>(50);
    let pool_clone = pool.clone();

    tokio::spawn(async move {
        let client = reqwest::Client::new();

        while let Some(urls) = rx.recv().await {
            for url in urls {
                let pool = pool_clone.clone();
                let http_client = client.clone();
                let vault = vault_path.clone();

                // Spawn a concurrent task for each URL
                tokio::spawn(async move {
                    if let Err(e) = process_single_url(
                        url,
                        pool,
                        http_client,
                        vault,
                    ).await {
                        eprintln!("Error processing URL: {}", e);
                    }
                });
            }
        }
    });

    tx
}

/// Process a single URL: fetch, scrape, create source, analyze, and autonomously promote.
async fn process_single_url(
    url: String,
    pool: DbPool,
    http_client: reqwest::Client,
    vault_path: std::path::PathBuf,
) -> Result<(), String> {
    // ==================== STEP 1: SCRAPE ====================
    // Fetch markdown via Jina AI
    let jina_url = format!("https://r.jina.ai/{}", url);
    let markdown = match http_client.get(&jina_url).send().await {
        Ok(resp) => resp.text().await.unwrap_or_default(),
        Err(e) => {
            eprintln!("Failed to fetch from Jina: {}", e);
            return Err(format!("Failed to fetch: {}", e));
        }
    };

    if markdown.is_empty() {
        return Err("Empty markdown content from Jina".to_string());
    }

    // ==================== STEP 2: SAVE RAW SOURCE ====================
    let source_id = SourceId::new();
    let content_hash = compute_hash(&markdown);
    let source = Source {
        id: source_id,
        origin: SourceOrigin::WebArticle { url: url.clone() },
        title: None, // Will be updated by LLM
        raw_content: markdown.clone(),
        captured_at: Timestamp::now_utc(),
        content_hash,
        ingestion_state: IngestionState::Captured,
        created_by: Actor::System,
        created_at: Timestamp::now_utc(),
        version: 1,
        updated_at: Timestamp::now_utc(),
    };

    // Insert raw source (blocking DB call)
    {
        let pool_db = pool.clone();
        let src = source.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool_db.get().map_err(|e| format!("DB connection failed: {}", e))?;
            SqliteSourceRepo { conn: &conn }
                .create(&src)
                .map_err(|e| format!("Failed to create source: {}", e))?;
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| format!("Spawn blocking error: {}", e))??;
    }

    // ==================== STEP 3: REASONING PHASE ====================
    // For MVP, we'll create a simple AI analysis (mock or real)
    // This is where you'd call Gemini 3.5 Flash or Claude Haiku 4.5
    let ai_results = analyze_content_mock(&markdown, &url).await;

    // ==================== STEP 4: AUTONOMOUS PROMOTION ====================
    // Create Note and link it back to Source, without human review
    {
        let pool_db = pool.clone();
        let vault = vault_path.clone();
        let ai_title = ai_results.title.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool_db.get().map_err(|e| format!("DB connection failed: {}", e))?;

            let action_repo = SqliteAgentActionRepo { conn: &conn };
            let note_repo = SqliteNoteRepo { conn: &conn, vault_path: vault };
            let link_repo = SqliteLinkRepo { conn: &conn };

            let agent_actor = Actor::Agent {
                name: "Autonomous-Ingestor-Haiku".to_string(),
            };

            // ACTION A: Create a durable Note from the summary
            let note_id = NoteId::new();
            let create_note_op = OperationRequest {
                actor: agent_actor.clone(),
                rationale: "Autonomous bulk ingestion".to_string(),
                operation: Operation::CreateNote {
                    note_id,
                    title: ai_title,
                },
            };

            // Execute & immediately apply
            let action = pkm_agent::execute(create_note_op, &action_repo)
                .map_err(|e| format!("Execute failed: {}", e))?;
            pkm_agent::apply_action(
                action.id,
                &action_repo,
                &note_repo,
                Some(&link_repo)
            ).map_err(|e| format!("Apply failed: {}", e))?;

            // ACTION B: Create a DerivedFrom link from Note back to Source (Provenance)
            let link_op = OperationRequest {
                actor: agent_actor.clone(),
                rationale: "Linking autonomous note to origin".to_string(),
                operation: Operation::CreateTypedLink {
                    from: ObjectRef::Note(note_id),
                    to: ObjectRef::Source(source_id),
                    link_type: pkm_core::link::LinkType::DerivedFrom,
                },
            };
            let action = pkm_agent::execute(link_op, &action_repo)
                .map_err(|e| format!("Execute link failed: {}", e))?;
            pkm_agent::apply_action(
                action.id,
                &action_repo,
                &note_repo,
                Some(&link_repo)
            ).map_err(|e| format!("Apply link failed: {}", e))?;

            // Update the Source state directly to PROMOTED (Skipping AwaitingReview)
            conn.execute(
                "UPDATE source SET ingestion_state = ?1 WHERE id = ?2",
                rusqlite::params!["promoted", source_id.to_string()],
            ).map_err(|e| format!("Update source failed: {}", e))?;

            println!("✓ Autonomously promoted: {} -> Note {}", url, note_id);
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| format!("Spawn blocking error: {}", e))??;
    }

    Ok(())
}

/// Mock AI analysis (replace with real API calls to Gemini/Claude)
async fn analyze_content_mock(markdown: &str, url: &str) -> AiAnalysisResult {
    // Extract title from URL or first heading
    let title = extract_title(markdown, url);

    // For MVP, create a simple summary from the first 200 chars
    let summary = markdown.chars().take(200).collect::<String>();

    AiAnalysisResult {
        title,
        summary,
    }
}

/// Extract a title from markdown content or URL
fn extract_title(markdown: &str, url: &str) -> String {
    // Try to find first heading
    if let Some(line) = markdown.lines().find(|l| l.starts_with('#')) {
        return line.trim_start_matches('#').trim().to_string();
    }

    // Fallback to URL domain or host
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
