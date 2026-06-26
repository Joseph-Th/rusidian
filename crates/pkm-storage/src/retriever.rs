//! FTS5-based retrieval implementation.
//!
//! Provides full-text search over notes, blocks, sources, and entities using
//! SQLite's FTS5 virtual tables. Preserves ContentStatus for every hit so the
//! UI can distinguish raw/reviewed/generated/unreviewed material.

use pkm_core::id::ObjectRef;
use pkm_core::ports::{SearchHit, SearchQuery};
use pkm_core::provenance::ContentStatus;
use pkm_search::rank;
use rusqlite::Connection;

/// SQLite FTS5-backed retriever. Searches across notes, blocks, sources, and
/// entities with keyword and phrase matching. Preserves ContentStatus for display.
pub struct SqliteRetriever {
    conn: std::sync::Arc<std::sync::Mutex<Connection>>,
}

impl SqliteRetriever {
    /// Create a new retriever backed by an existing SQLite connection.
    pub fn new(conn: std::sync::Arc<std::sync::Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

impl pkm_core::ports::Retriever for SqliteRetriever {
    fn search(&self, query: &SearchQuery) -> pkm_core::Result<Vec<SearchHit>> {
        let mut results = Vec::new();
        let conn = self
            .conn
            .lock()
            .map_err(|_| pkm_core::CoreError::Invariant("failed to lock connection".to_string()))?;

        use pkm_core::ports::SearchMode;

        match query.mode {
            SearchMode::ExactText => {
                results.extend(search_notes_exact(&conn, query)?);
                results.extend(search_blocks_exact(&conn, query)?);
                results.extend(search_sources_exact(&conn, query)?);
                results.extend(search_entities_exact(&conn, query)?);
            }
            SearchMode::FuzzyText => {
                results.extend(search_notes_fuzzy(&conn, query)?);
                results.extend(search_blocks_fuzzy(&conn, query)?);
                results.extend(search_sources_fuzzy(&conn, query)?);
                results.extend(search_entities_fuzzy(&conn, query)?);
            }
            SearchMode::Semantic => {
                return Err(pkm_core::CoreError::Invariant(
                    "semantic search not yet implemented".to_string(),
                ));
            }
            SearchMode::Entity => {
                results.extend(search_entities_fuzzy(&conn, query)?);
            }
            SearchMode::Source => {
                results.extend(search_sources_fuzzy(&conn, query)?);
            }
            SearchMode::LinkTraversal => {
                return Err(pkm_core::CoreError::Invariant(
                    "link traversal search not yet implemented".to_string(),
                ));
            }
        }

        // Apply filters
        apply_filters(&mut results, query);

        // Rank the results
        let ranked = rank(query, results);

        Ok(ranked)
    }
}

/// Extract snippet around first occurrence of search term.
/// Returns ~50 chars before + match + ~50 chars after, centered on the match.
fn extract_snippet(text: &str, search_term: &str, max_len: usize) -> Option<String> {
    let text_lower = text.to_lowercase();
    let search_lower = search_term.to_lowercase();

    if let Some(pos) = text_lower.find(&search_lower) {
        let context_before = pos.saturating_sub(50);
        let context_after = std::cmp::min(pos + search_term.len() + 50, text.len());

        let snippet = &text[context_before..context_after];
        if snippet.len() <= max_len {
            Some(format!("...{}...", snippet.trim()))
        } else {
            Some(format!("...{}...", &snippet[..max_len].trim()))
        }
    } else {
        None
    }
}

/// Search notes with exact phrase matching.
fn search_notes_exact(conn: &Connection, query: &SearchQuery) -> pkm_core::Result<Vec<SearchHit>> {
    let mut stmt = conn
        .prepare("SELECT rowid FROM note_fts WHERE note_fts MATCH ? LIMIT 100")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let note_ids: Vec<String> = stmt
        .query_map([&query.text], |row| row.get(0))
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let mut hits = Vec::new();

    // For each note, fetch title and create snippet
    for note_id in note_ids {
        let mut title_stmt = conn
            .prepare("SELECT title FROM note WHERE id = ?")
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

        if let Ok(title) = title_stmt.query_row([&note_id], |row| row.get::<_, String>(0)) {
            let snippet = extract_snippet(&title, &query.text, 150);

            hits.push(SearchHit {
                object: ObjectRef::Note(pkm_core::id::NoteId(
                    uuid::Uuid::parse_str(&note_id).unwrap(),
                )),
                status: ContentStatus::UserAuthored,
                score: None,
                snippet,
            });
        }
    }

    Ok(hits)
}

/// Search blocks with exact phrase matching.
fn search_blocks_exact(conn: &Connection, query: &SearchQuery) -> pkm_core::Result<Vec<SearchHit>> {
    let mut stmt = conn
        .prepare("SELECT rowid FROM block_fts WHERE block_fts MATCH ? LIMIT 100")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let block_ids: Vec<String> = stmt
        .query_map([&query.text], |row| row.get(0))
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let mut hits = Vec::new();

    for block_id in block_ids {
        let mut content_stmt = conn
            .prepare("SELECT content FROM block WHERE id = ?")
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

        if let Ok(content) = content_stmt.query_row([&block_id], |row| row.get::<_, String>(0)) {
            let snippet = extract_snippet(&content, &query.text, 150);

            hits.push(SearchHit {
                object: ObjectRef::Block(pkm_core::id::BlockId(
                    uuid::Uuid::parse_str(&block_id).unwrap(),
                )),
                status: ContentStatus::UserAuthored,
                score: None,
                snippet,
            });
        }
    }

    Ok(hits)
}

/// Search sources with exact phrase matching.
fn search_sources_exact(
    conn: &Connection,
    query: &SearchQuery,
) -> pkm_core::Result<Vec<SearchHit>> {
    let mut stmt = conn
        .prepare("SELECT rowid FROM source_fts WHERE source_fts MATCH ? LIMIT 100")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let source_ids: Vec<String> = stmt
        .query_map([&query.text], |row| row.get(0))
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let mut hits = Vec::new();

    for source_id in source_ids {
        let mut title_stmt = conn
            .prepare("SELECT title FROM source WHERE id = ?")
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

        if let Ok(title) = title_stmt.query_row([&source_id], |row| row.get::<_, String>(0)) {
            let snippet = extract_snippet(&title, &query.text, 150);

            hits.push(SearchHit {
                object: ObjectRef::Source(pkm_core::id::SourceId(
                    uuid::Uuid::parse_str(&source_id).unwrap(),
                )),
                status: ContentStatus::RawSource,
                score: None,
                snippet,
            });
        }
    }

    Ok(hits)
}

/// Search entities with exact phrase matching.
fn search_entities_exact(
    conn: &Connection,
    query: &SearchQuery,
) -> pkm_core::Result<Vec<SearchHit>> {
    let mut stmt = conn
        .prepare("SELECT rowid FROM entity_fts WHERE entity_fts MATCH ? LIMIT 100")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let entity_ids: Vec<String> = stmt
        .query_map([&query.text], |row| row.get(0))
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let mut hits = Vec::new();

    for entity_id in entity_ids {
        let mut name_stmt = conn
            .prepare("SELECT name FROM entity WHERE id = ?")
            .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

        if let Ok(name) = name_stmt.query_row([&entity_id], |row| row.get::<_, String>(0)) {
            let snippet = extract_snippet(&name, &query.text, 150);

            hits.push(SearchHit {
                object: ObjectRef::Entity(pkm_core::id::EntityId(
                    uuid::Uuid::parse_str(&entity_id).unwrap(),
                )),
                status: ContentStatus::ExtractedMetadata,
                score: None,
                snippet,
            });
        }
    }

    Ok(hits)
}

/// Search notes with fuzzy matching. Tries partial token matching with prefix wildcards.
fn search_notes_fuzzy(conn: &Connection, query: &SearchQuery) -> pkm_core::Result<Vec<SearchHit>> {
    // FTS5 fuzzy: search for any token starting with the query text using * wildcard
    let fuzzy_query = format!("{}*", query.text);
    let mut stmt = conn
        .prepare("SELECT rowid FROM note_fts WHERE note_fts MATCH ? LIMIT 100")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let hits: Vec<SearchHit> = stmt
        .query_map([&fuzzy_query], |row| {
            let note_id: String = row.get(0)?;
            Ok(SearchHit {
                object: ObjectRef::Note(pkm_core::id::NoteId(
                    uuid::Uuid::parse_str(&note_id).unwrap(),
                )),
                status: ContentStatus::UserAuthored,
                score: None,
                snippet: None,
            })
        })
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    Ok(hits)
}

/// Search blocks with fuzzy matching.
fn search_blocks_fuzzy(conn: &Connection, query: &SearchQuery) -> pkm_core::Result<Vec<SearchHit>> {
    let fuzzy_query = format!("{}*", query.text);
    let mut stmt = conn
        .prepare("SELECT rowid FROM block_fts WHERE block_fts MATCH ? LIMIT 100")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let hits: Vec<SearchHit> = stmt
        .query_map([&fuzzy_query], |row| {
            let block_id: String = row.get(0)?;
            Ok(SearchHit {
                object: ObjectRef::Block(pkm_core::id::BlockId(
                    uuid::Uuid::parse_str(&block_id).unwrap(),
                )),
                status: ContentStatus::UserAuthored,
                score: None,
                snippet: None,
            })
        })
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    Ok(hits)
}

/// Search sources with fuzzy matching.
fn search_sources_fuzzy(
    conn: &Connection,
    query: &SearchQuery,
) -> pkm_core::Result<Vec<SearchHit>> {
    let fuzzy_query = format!("{}*", query.text);
    let mut stmt = conn
        .prepare("SELECT rowid FROM source_fts WHERE source_fts MATCH ? LIMIT 100")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let hits: Vec<SearchHit> = stmt
        .query_map([&fuzzy_query], |row| {
            let source_id: String = row.get(0)?;
            Ok(SearchHit {
                object: ObjectRef::Source(pkm_core::id::SourceId(
                    uuid::Uuid::parse_str(&source_id).unwrap(),
                )),
                status: ContentStatus::RawSource,
                score: None,
                snippet: None,
            })
        })
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    Ok(hits)
}

/// Search entities with fuzzy matching.
fn search_entities_fuzzy(
    conn: &Connection,
    query: &SearchQuery,
) -> pkm_core::Result<Vec<SearchHit>> {
    let fuzzy_query = format!("{}*", query.text);
    let mut stmt = conn
        .prepare("SELECT rowid FROM entity_fts WHERE entity_fts MATCH ? LIMIT 100")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let hits: Vec<SearchHit> = stmt
        .query_map([&fuzzy_query], |row| {
            let entity_id: String = row.get(0)?;
            Ok(SearchHit {
                object: ObjectRef::Entity(pkm_core::id::EntityId(
                    uuid::Uuid::parse_str(&entity_id).unwrap(),
                )),
                status: ContentStatus::ExtractedMetadata,
                score: None,
                snippet: None,
            })
        })
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    Ok(hits)
}

/// Apply filters to search results. Removes hits that don't match the filters.
fn apply_filters(results: &mut Vec<SearchHit>, query: &SearchQuery) {
    let filters = &query.filters;

    if let Some(ref obj_type) = filters.object_type {
        results.retain(|hit| {
            let hit_type = match hit.object {
                ObjectRef::Source(_) => "source",
                ObjectRef::Note(_) => "note",
                ObjectRef::Block(_) => "block",
                ObjectRef::Entity(_) => "entity",
                ObjectRef::Link(_) => "link",
                ObjectRef::View(_) => "view",
            };
            hit_type.to_lowercase() == obj_type.to_lowercase()
        });
    }

    if let Some(ref review_state) = filters.review_state {
        results.retain(|hit| {
            // For now, only reviewed content matches "accepted" filter
            #[allow(clippy::match_like_matches_macro)]
            match (hit.status, review_state) {
                (ContentStatus::Reviewed, pkm_core::review::ReviewState::Accepted) => true,
                (ContentStatus::UserAuthored, pkm_core::review::ReviewState::Accepted) => true,
                (ContentStatus::UnreviewedSuggestion, pkm_core::review::ReviewState::Proposed) => {
                    true
                }
                _ => false,
            }
        });
    }

    // TODO(E2): implement date_range and project filters when those fields are persisted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_retriever_initialization() {
        // This is a placeholder test that ensures the retriever can be instantiated.
        // Full integration tests should use the migration test infrastructure.
        let _ = SqliteRetriever::new(std::sync::Arc::new(std::sync::Mutex::new(
            rusqlite::Connection::open_in_memory().unwrap(),
        )));
    }
}
