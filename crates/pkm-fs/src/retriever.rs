use pkm_core::ports::{Retriever, SearchQuery, SearchHit};
use pkm_core::provenance::ContentStatus;
use pkm_core::id::ObjectRef;
use crate::state::SharedVault;
use pkm_search::rank;

/// Format a Timestamp as an RFC3339 string for consistent date comparison
/// with query filters (which use T separators).
fn fmt_rfc3339(ts: &pkm_core::Timestamp) -> String {
    ts.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| ts.to_string())
}

fn infer_content_status(actor: &pkm_core::Actor) -> ContentStatus {
    match actor {
        pkm_core::Actor::User => ContentStatus::UserAuthored,
        pkm_core::Actor::Agent { .. } => ContentStatus::AiSummary,
        pkm_core::Actor::System => ContentStatus::ExtractedMetadata,
    }
}

pub struct FsRetriever {
    pub state: SharedVault,
}

impl Retriever for FsRetriever {
    fn search(&self, query: &SearchQuery) -> pkm_core::Result<Vec<SearchHit>> {
        let search_term = query.text.to_lowercase();

        if search_term.is_empty() {
            return Ok(vec![]);
        }

        let mut results = Vec::new();
        let state = self.state.read().unwrap();

        // 1. Search blocks
        for block in state.blocks.values() {
            let block_status = infer_content_status(&block.created_by);
            let text = match &block.content {
                pkm_core::block::BlockContent::Markdown { text } => text.as_str(),
                pkm_core::block::BlockContent::Table { headers, rows } => {
                    let combined = format!("{} {}",
                        headers.join(" "),
                        rows.iter().map(|r| r.join(" ")).collect::<Vec<_>>().join(" ")
                    );
                    if combined.to_lowercase().contains(&search_term) {
                        results.push(SearchHit {
                            object: ObjectRef::Block(block.id),
                            status: block_status,
                            score: None,
                            snippet: extract_snippet(&combined, &search_term, 150),
                            created_at: Some(fmt_rfc3339(&block.created_at)),
                            project: None,
                        });
                    }
                    continue;
                }
                pkm_core::block::BlockContent::Math { expression, .. } => expression.as_str(),
                pkm_core::block::BlockContent::Media { alt_text, .. } => alt_text.as_str(),
                _ => "",
            };
            if text.to_lowercase().contains(&search_term) {
                results.push(SearchHit {
                    object: ObjectRef::Block(block.id),
                    status: block_status,
                    score: None,
                    snippet: extract_snippet(text, &search_term, 150),
                    created_at: Some(fmt_rfc3339(&block.created_at)),
                    project: None,
                });
            }
        }

        // 2. Search notes
        for note in state.notes.values() {
            let note_status = infer_content_status(&note.created_by);
            if note.title.to_lowercase().contains(&search_term) {
                results.push(SearchHit {
                    object: ObjectRef::Note(note.id),
                    status: note_status,
                    score: None,
                    snippet: Some(format!("Note: {}", note.title)),
                    created_at: Some(fmt_rfc3339(&note.created_at)),
                    project: note.metadata.project.clone(),
                });
            }
        }

        // 3. Search sources
        for source in state.sources.values() {
            let title_match = source.title.as_ref().map(|t| t.to_lowercase().contains(&search_term)).unwrap_or(false);
            let content_match = source.raw_content.to_lowercase().contains(&search_term);
            if title_match || content_match {
                results.push(SearchHit {
                    object: ObjectRef::Source(source.id),
                    status: ContentStatus::RawSource,
                    score: None,
                    snippet: extract_snippet(&source.raw_content, &search_term, 150),
                    created_at: Some(fmt_rfc3339(&source.created_at)),
                    project: None,
                });
            }
        }

        // 4. Search entities
        for entity in state.entities.values() {
            let name_match = entity.name.to_lowercase().contains(&search_term);
            let alias_match = entity.aliases.iter().any(|a| a.to_lowercase().contains(&search_term));
            if name_match || alias_match {
                results.push(SearchHit {
                    object: ObjectRef::Entity(entity.id),
                    status: ContentStatus::ExtractedMetadata,
                    score: None,
                    snippet: Some(format!("Entity: {}", entity.name)),
                    created_at: Some(fmt_rfc3339(&entity.created_at)),
                    project: None,
                });
            }
        }

        // Lock is dropped here when state goes out of scope

        // Apply filters
        apply_filters(&mut results, query);

        // Rank results using search query
        let ranked = rank(query, results);

        Ok(ranked)
    }
}

fn extract_snippet(text: &str, search_term: &str, max_len: usize) -> Option<String> {
    let text_lower = text.to_lowercase();
    let search_lower = search_term.to_lowercase();

    if let Some(lower_pos) = text_lower.find(&search_lower) {
        // Map the byte position in lowercased text back to the original text
        // by counting characters up to the match position
        let char_count = text_lower[..lower_pos].chars().count();
        let original_pos = text
            .char_indices()
            .nth(char_count)
            .map(|(i, _)| i)
            .unwrap_or(text.len());

        let start = text.floor_char_boundary(original_pos.saturating_sub(50));
        let raw_end = std::cmp::min(original_pos + search_term.len() + 50, text.len());
        let end = if raw_end >= text.len() {
            text.len()
        } else if text.is_char_boundary(raw_end) {
            raw_end
        } else {
            let next_char_len = text[raw_end..].chars().next().map(|c| c.len_utf8()).unwrap_or(0);
            raw_end + next_char_len
        };

        let snippet = &text[start..end];
        if snippet.len() <= max_len {
            Some(format!("...{}...", snippet.trim()))
        } else {
            let safe_end = snippet
                .char_indices()
                .map(|(i, c)| i + c.len_utf8())
                .take_while(|&end| end <= max_len)
                .last()
                .unwrap_or(0);
            Some(format!("...{}...", &snippet[..safe_end].trim()))
        }
    } else {
        None
    }
}

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
            #[allow(clippy::match_like_matches_macro)]
            match (hit.status, review_state) {
                (ContentStatus::Reviewed, pkm_core::review::ReviewState::Accepted) => true,
                (ContentStatus::UserAuthored, pkm_core::review::ReviewState::Accepted) => true,
                (ContentStatus::AiSummary, pkm_core::review::ReviewState::Accepted) => true,
                (ContentStatus::InferredLink, pkm_core::review::ReviewState::Accepted) => true,
                (ContentStatus::ExtractedMetadata, pkm_core::review::ReviewState::Accepted) => true,
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
