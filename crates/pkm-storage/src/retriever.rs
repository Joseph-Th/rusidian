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
        if query.text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let conn = self
            .conn
            .lock()
            .map_err(|_| pkm_core::CoreError::Invariant("failed to lock connection".to_string()))?;

        use pkm_core::ports::SearchMode;

        match query.mode {
            SearchMode::ExactText => {
                results.extend(search_notes_fts(&conn, &query.text, false)?);
                results.extend(search_blocks_fts(&conn, &query.text, false)?);
                results.extend(search_sources_fts(&conn, &query.text, false)?);
                results.extend(search_entities_fts(&conn, &query.text, false)?);
            }
            SearchMode::FuzzyText => {
                results.extend(search_notes_fts(&conn, &query.text, true)?);
                results.extend(search_blocks_fts(&conn, &query.text, true)?);
                results.extend(search_sources_fts(&conn, &query.text, true)?);
                results.extend(search_entities_fts(&conn, &query.text, true)?);
            }
            SearchMode::Semantic => {
                return Err(pkm_core::CoreError::Invariant(
                    "Semantic search (Phase 7) will use vector embeddings to find conceptually \
                     related content. Coming after Phase 6 retrieval basics are stable. For now, \
                     use ExactText or FuzzyText search."
                        .to_string(),
                ));
            }
            SearchMode::Entity => {
                results.extend(search_entities_fts(&conn, &query.text, true)?);
            }
            SearchMode::Source => {
                results.extend(search_sources_fts(&conn, &query.text, true)?);
            }
            SearchMode::LinkTraversal => {
                results.extend(search_link_traversal(&conn, query)?);
            }
        }

        // Apply filters
        apply_filters(&mut results, query);

        // Rank the results
        let ranked = rank(query, results);

        Ok(ranked)
    }
}

/// Search for objects reachable via typed links (graph traversal).
/// Parses the query text as "type:id" (e.g., "note:abc123") or just an id.
/// Traverses links up to depth 2 to find related objects.
fn search_link_traversal(
    conn: &Connection,
    query: &SearchQuery,
) -> pkm_core::Result<Vec<SearchHit>> {
    // Parse starting object from query text
    let starting_obj = parse_starting_object(&query.text)?;
    let mut hits = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    let mut depth = std::collections::HashMap::new();

    queue.push_back(starting_obj);
    visited.insert(object_ref_to_string(&starting_obj));
    depth.insert(object_ref_to_string(&starting_obj), 0);

    // BFS traversal with depth limit of 2
    const MAX_DEPTH: usize = 2;
    let mut results_limit = 100;

    while let Some(current) = queue.pop_front() {
        let current_depth = *depth.get(&object_ref_to_string(&current)).unwrap_or(&0);

        if current_depth < MAX_DEPTH && results_limit > 0 {
            // Get links from current object
            let linked_objects = get_linked_objects(conn, &current)?;

            for obj in linked_objects {
                let obj_str = object_ref_to_string(&obj);
                if !visited.contains(&obj_str) {
                    visited.insert(obj_str.clone());
                    depth.insert(obj_str, current_depth + 1);
                    queue.push_back(obj);
                    results_limit -= 1;
                }
            }
        }

        // Add current object to results (except the starting object itself)
        if object_ref_to_string(&current) != object_ref_to_string(&starting_obj) {
            if let Some(hit) = create_hit_for_object(conn, &current)? {
                hits.push(hit);
            }
        }
    }

    Ok(hits)
}

/// Get all objects that are linked to or from the given object.
fn get_linked_objects(conn: &Connection, obj: &ObjectRef) -> pkm_core::Result<Vec<ObjectRef>> {
    let (obj_type, obj_id) = object_ref_to_string_parts(obj);
    let mut results = Vec::new();

    // Get objects this object links to (outgoing edges)
    let mut stmt = conn
        .prepare("SELECT to_type, to_id FROM link WHERE from_type = ? AND from_id = ? LIMIT 50")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let to_objects: Vec<(String, String)> = stmt
        .query_map([obj_type, &obj_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    for (type_str, id_str) in to_objects {
        if let Ok(obj_ref) = string_to_object_ref(&type_str, &id_str) {
            results.push(obj_ref);
        }
    }

    // Get objects that link to this object (incoming edges)
    let mut stmt = conn
        .prepare("SELECT from_type, from_id FROM link WHERE to_type = ? AND to_id = ? LIMIT 50")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let from_objects: Vec<(String, String)> = stmt
        .query_map([obj_type, &obj_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    for (type_str, id_str) in from_objects {
        if let Ok(obj_ref) = string_to_object_ref(&type_str, &id_str) {
            results.push(obj_ref);
        }
    }

    Ok(results)
}

/// Convert an ObjectRef to a string for use as a HashMap key (type:id).
fn object_ref_to_string(obj: &ObjectRef) -> String {
    let (type_str, id_str) = object_ref_to_string_parts(obj);
    format!("{}:{}", type_str, id_str)
}

/// Convert an ObjectRef to (type, id) parts.
fn object_ref_to_string_parts(obj: &ObjectRef) -> (&'static str, String) {
    match obj {
        ObjectRef::Source(id) => ("source", id.to_string()),
        ObjectRef::Note(id) => ("note", id.to_string()),
        ObjectRef::Block(id) => ("block", id.to_string()),
        ObjectRef::Entity(id) => ("entity", id.to_string()),
        ObjectRef::Link(id) => ("link", id.to_string()),
        ObjectRef::View(id) => ("view", id.to_string()),
    }
}

/// Parse a string ("type:id" or just "id") into an ObjectRef.
fn string_to_object_ref(type_str: &str, id_str: &str) -> pkm_core::Result<ObjectRef> {
    let uuid = uuid::Uuid::parse_str(id_str)
        .map_err(|_| pkm_core::CoreError::Invariant("invalid uuid".to_string()))?;

    Ok(match type_str {
        "source" => ObjectRef::Source(pkm_core::id::SourceId(uuid)),
        "note" => ObjectRef::Note(pkm_core::id::NoteId(uuid)),
        "block" => ObjectRef::Block(pkm_core::id::BlockId(uuid)),
        "entity" => ObjectRef::Entity(pkm_core::id::EntityId(uuid)),
        "link" => ObjectRef::Link(pkm_core::id::LinkId(uuid)),
        "view" => ObjectRef::View(pkm_core::id::ViewId(uuid)),
        _ => {
            return Err(pkm_core::CoreError::Invariant(
                "unknown object type".to_string(),
            ))
        }
    })
}

/// Parse the starting object from query text.
/// Formats: "note:uuid", "source:uuid", or just "uuid" (defaults to note).
fn parse_starting_object(text: &str) -> pkm_core::Result<ObjectRef> {
    let text = text.trim();

    if let Some(idx) = text.find(':') {
        let (type_part, id_part) = text.split_at(idx);
        let id_part = &id_part[1..]; // skip ':'
        string_to_object_ref(type_part.trim(), id_part.trim())
    } else {
        // Try parsing as a raw UUID, default to Note
        string_to_object_ref("note", text)
    }
}

/// Create a SearchHit for an object by fetching its metadata from the database.
fn create_hit_for_object(
    conn: &Connection,
    obj: &ObjectRef,
) -> pkm_core::Result<Option<SearchHit>> {
    match obj {
        ObjectRef::Note(id) => {
            let id_str = id.to_string();
            let mut stmt = conn
                .prepare("SELECT title, created_at, project FROM note WHERE id = ?")
                .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

            if let Ok((title, created_at, project)) = stmt.query_row([&id_str], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            }) {
                Ok(Some(SearchHit {
                    object: *obj,
                    status: ContentStatus::UserAuthored,
                    score: None,
                    snippet: Some(format!("Note: {}", title)),
                    created_at: Some(created_at),
                    project,
                }))
            } else {
                Ok(None)
            }
        }
        ObjectRef::Source(id) => {
            let id_str = id.to_string();
            let mut stmt = conn
                .prepare("SELECT title, created_at FROM source WHERE id = ?")
                .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

            if let Ok((title, created_at)) = stmt.query_row([&id_str], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }) {
                Ok(Some(SearchHit {
                    object: *obj,
                    status: ContentStatus::RawSource,
                    score: None,
                    snippet: Some(format!("Source: {}", title)),
                    created_at: Some(created_at),
                    project: None,
                }))
            } else {
                Ok(None)
            }
        }
        ObjectRef::Entity(id) => {
            let id_str = id.to_string();
            let mut stmt = conn
                .prepare("SELECT name, created_at FROM entity WHERE id = ?")
                .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

            if let Ok((name, created_at)) = stmt.query_row([&id_str], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }) {
                Ok(Some(SearchHit {
                    object: *obj,
                    status: ContentStatus::ExtractedMetadata,
                    score: None,
                    snippet: Some(format!("Entity: {}", name)),
                    created_at: Some(created_at),
                    project: None,
                }))
            } else {
                Ok(None)
            }
        }
        ObjectRef::Block(id) => {
            let id_str = id.to_string();
            let mut stmt = conn
                .prepare("SELECT content, created_at FROM block WHERE id = ?")
                .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

            if let Ok((content, created_at)) = stmt.query_row([&id_str], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }) {
                Ok(Some(SearchHit {
                    object: *obj,
                    status: ContentStatus::UserAuthored,
                    score: None,
                    snippet: Some(format!(
                        "Block: {}",
                        &content[..std::cmp::min(50, content.len())]
                    )),
                    created_at: Some(created_at),
                    project: None,
                }))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

/// Build a safe FTS5 MATCH expression. Double-quotes the token to prevent
/// special characters (-, ", (, ), :, ^) from being interpreted as operators.
/// Fuzzy mode appends '*' outside the quotes for prefix matching.
fn fts_expr(text: &str, fuzzy: bool) -> String {
    // Escape inner double-quotes per FTS5 spec (double them).
    let escaped = text.replace('"', "\"\"");
    if fuzzy {
        format!("\"{}\"*", escaped)
    } else {
        format!("\"{}\"", escaped)
    }
}

/// Search notes via FTS5. Returns hits with note-level metadata.
fn search_notes_fts(
    conn: &Connection,
    text: &str,
    fuzzy: bool,
) -> pkm_core::Result<Vec<SearchHit>> {
    let expr = fts_expr(text, fuzzy);
    let mut stmt = conn
        .prepare("SELECT id FROM note_fts WHERE note_fts MATCH ? LIMIT 100")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let note_ids: Vec<String> = stmt
        .query_map([&expr], |row| row.get(0))
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let mut hits = Vec::new();
    let mut fetch_stmt = conn
        .prepare("SELECT title, created_at, project FROM note WHERE id = ?")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
    for note_id in note_ids {
        if let Ok((title, created_at, project)) = fetch_stmt.query_row([&note_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        }) {
            let snippet = extract_snippet(&title, text, 150);
            hits.push(SearchHit {
                object: pkm_core::id::ObjectRef::Note(pkm_core::id::NoteId(
                    uuid::Uuid::parse_str(&note_id)
                        .map_err(|_| pkm_core::CoreError::Invariant("invalid note uuid in fts".into()))?,
                )),
                status: pkm_core::provenance::ContentStatus::UserAuthored,
                score: None,
                snippet,
                created_at: Some(created_at),
                project,
            });
        }
    }
    Ok(hits)
}

/// Search blocks via FTS5.
fn search_blocks_fts(
    conn: &Connection,
    text: &str,
    fuzzy: bool,
) -> pkm_core::Result<Vec<SearchHit>> {
    let expr = fts_expr(text, fuzzy);
    let mut stmt = conn
        .prepare("SELECT id FROM block_fts WHERE block_fts MATCH ? LIMIT 100")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let block_ids: Vec<String> = stmt
        .query_map([&expr], |row| row.get(0))
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let mut hits = Vec::new();
    let mut fetch_stmt = conn
        .prepare("SELECT content, created_at FROM block WHERE id = ?")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
    for block_id in block_ids {
        if let Ok((content, created_at)) =
            fetch_stmt.query_row([&block_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        {
            let snippet = extract_snippet(&content, text, 150);
            hits.push(SearchHit {
                object: pkm_core::id::ObjectRef::Block(pkm_core::id::BlockId(
                    uuid::Uuid::parse_str(&block_id)
                        .map_err(|_| pkm_core::CoreError::Invariant("invalid block uuid in fts".into()))?,
                )),
                status: pkm_core::provenance::ContentStatus::UserAuthored,
                score: None,
                snippet,
                created_at: Some(created_at),
                project: None,
            });
        }
    }
    Ok(hits)
}

/// Search sources via FTS5.
fn search_sources_fts(
    conn: &Connection,
    text: &str,
    fuzzy: bool,
) -> pkm_core::Result<Vec<SearchHit>> {
    let expr = fts_expr(text, fuzzy);
    let mut stmt = conn
        .prepare("SELECT id FROM source_fts WHERE source_fts MATCH ? LIMIT 100")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let source_ids: Vec<String> = stmt
        .query_map([&expr], |row| row.get(0))
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let mut hits = Vec::new();
    let mut fetch_stmt = conn
        .prepare("SELECT title, created_at FROM source WHERE id = ?")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
    for source_id in source_ids {
        if let Ok((title, created_at)) =
            fetch_stmt.query_row([&source_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        {
            let snippet = extract_snippet(&title, text, 150);
            hits.push(SearchHit {
                object: pkm_core::id::ObjectRef::Source(pkm_core::id::SourceId(
                    uuid::Uuid::parse_str(&source_id)
                        .map_err(|_| pkm_core::CoreError::Invariant("invalid source uuid in fts".into()))?,
                )),
                status: pkm_core::provenance::ContentStatus::RawSource,
                score: None,
                snippet,
                created_at: Some(created_at),
                project: None,
            });
        }
    }
    Ok(hits)
}

/// Search entities via FTS5.
fn search_entities_fts(
    conn: &Connection,
    text: &str,
    fuzzy: bool,
) -> pkm_core::Result<Vec<SearchHit>> {
    let expr = fts_expr(text, fuzzy);
    let mut stmt = conn
        .prepare("SELECT id FROM entity_fts WHERE entity_fts MATCH ? LIMIT 100")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let entity_ids: Vec<String> = stmt
        .query_map([&expr], |row| row.get(0))
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;

    let mut hits = Vec::new();
    let mut fetch_stmt = conn
        .prepare("SELECT name, created_at FROM entity WHERE id = ?")
        .map_err(|e| pkm_core::CoreError::Invariant(e.to_string()))?;
    for entity_id in entity_ids {
        if let Ok((name, created_at)) =
            fetch_stmt.query_row([&entity_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        {
            let snippet = extract_snippet(&name, text, 150);
            hits.push(SearchHit {
                object: pkm_core::id::ObjectRef::Entity(pkm_core::id::EntityId(
                    uuid::Uuid::parse_str(&entity_id)
                        .map_err(|_| pkm_core::CoreError::Invariant("invalid entity uuid in fts".into()))?,
                )),
                status: pkm_core::provenance::ContentStatus::ExtractedMetadata,
                score: None,
                snippet,
                created_at: Some(created_at),
                project: None,
            });
        }
    }
    Ok(hits)
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
            // Find the largest byte index <= max_len that falls on a char boundary.
            let safe_end = snippet
                .char_indices()
                .map(|(i, c)| i + c.len_utf8())
                .take_while(|&end| end <= max_len)
                .last()
                .unwrap_or(0);
            Some(format!("...{}...", snippet[..safe_end].trim()))
        }
    } else {
        None
    }
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

    if let Some((start_date, end_date)) = &filters.date_range {
        results.retain(|hit| {
            if let Some(ref created_at) = hit.created_at {
                // Compare RFC3339 strings lexicographically (works for ISO format dates)
                created_at >= start_date && created_at <= end_date
            } else {
                false
            }
        });
    }

    if let Some(ref project_filter) = filters.project {
        results.retain(|hit| {
            if let Some(ref proj) = hit.project {
                proj == project_filter
            } else {
                false
            }
        });
    }

    if let Some(ref status_filter) = filters.content_status {
        results.retain(|hit| {
            let status_str = match hit.status {
                ContentStatus::UserAuthored => "user_authored",
                ContentStatus::RawSource => "raw_source",
                ContentStatus::UnreviewedSuggestion => "unreviewed_suggestion",
                ContentStatus::Reviewed => "reviewed",
                ContentStatus::ExtractedMetadata => "extracted_metadata",
                ContentStatus::AiSummary => "ai_summary",
                ContentStatus::InferredLink => "inferred_link",
            };
            status_str == status_filter.as_str()
        });
    }
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
