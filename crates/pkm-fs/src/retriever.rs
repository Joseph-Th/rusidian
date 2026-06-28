use pkm_core::ports::{Retriever, SearchQuery, SearchHit, SearchMode};
use pkm_core::provenance::ContentStatus;
use pkm_core::id::ObjectRef;
use crate::state::SharedVault;
use rayon::prelude::*;
use pkm_search::rank;

pub struct FsRetriever {
    pub state: SharedVault,
}

impl Retriever for FsRetriever {
    fn search(&self, query: &SearchQuery) -> pkm_core::Result<Vec<SearchHit>> {
        let state = self.state.read().unwrap();
        let search_term = query.text.to_lowercase();
        
        if search_term.is_empty() {
            return Ok(vec![]);
        }

        let mut results = Vec::new();

        // 1. Search blocks
        let block_hits: Vec<SearchHit> = state.blocks.par_iter()
            .filter_map(|(id, block)| {
                let text = match &block.content {
                    pkm_core::block::BlockContent::Markdown { text } => text.clone(),
                    pkm_core::block::BlockContent::Table { headers, rows } => {
                        format!("{} {}", headers.join(" "), rows.iter().map(|r| r.join(" ")).collect::<Vec<_>>().join(" "))
                    }
                    pkm_core::block::BlockContent::Math { expression, .. } => expression.clone(),
                    pkm_core::block::BlockContent::Media { alt_text, .. } => alt_text.clone(),
                    _ => return None,
                };
                let text_lower = text.to_lowercase();
                if text_lower.contains(&search_term) {
                    Some(SearchHit {
                        object: ObjectRef::Block(*id),
                        status: ContentStatus::UserAuthored,
                        score: None,
                        snippet: extract_snippet(&text, &search_term, 150),
                        created_at: Some(block.created_at.to_string()),
                        project: None,
                    })
                } else {
                    None
                }
            })
            .collect();
        results.extend(block_hits);

        // 2. Search notes
        let note_hits: Vec<SearchHit> = state.notes.par_iter()
            .filter_map(|(id, note)| {
                let text_lower = note.title.to_lowercase();
                if text_lower.contains(&search_term) {
                    let project = note.metadata.get("project")
                        .and_then(|v| v.as_str().map(|s| s.to_string()));
                    Some(SearchHit {
                        object: ObjectRef::Note(*id),
                        status: ContentStatus::UserAuthored,
                        score: None,
                        snippet: Some(format!("Note: {}", note.title)),
                        created_at: Some(note.created_at.to_string()),
                        project,
                    })
                } else {
                    None
                }
            })
            .collect();
        results.extend(note_hits);

        // 3. Search sources
        let source_hits: Vec<SearchHit> = state.sources.par_iter()
            .filter_map(|(id, source)| {
                let title_match = source.title.as_ref().map(|t| t.to_lowercase().contains(&search_term)).unwrap_or(false);
                let content_match = source.raw_content.to_lowercase().contains(&search_term);
                if title_match || content_match {
                    Some(SearchHit {
                        object: ObjectRef::Source(*id),
                        status: ContentStatus::RawSource,
                        score: None,
                        snippet: extract_snippet(&source.raw_content, &search_term, 150),
                        created_at: Some(source.created_at.to_string()),
                        project: None,
                    })
                } else {
                    None
                }
            })
            .collect();
        results.extend(source_hits);

        // 4. Search entities
        let entity_hits: Vec<SearchHit> = state.entities.par_iter()
            .filter_map(|(id, entity)| {
                let name_match = entity.name.to_lowercase().contains(&search_term);
                let alias_match = entity.aliases.iter().any(|a| a.to_lowercase().contains(&search_term));
                if name_match || alias_match {
                    Some(SearchHit {
                        object: ObjectRef::Entity(*id),
                        status: ContentStatus::ExtractedMetadata,
                        score: None,
                        snippet: Some(format!("Entity: {}", entity.name)),
                        created_at: Some(entity.created_at.to_string()),
                        project: None,
                    })
                } else {
                    None
                }
            })
            .collect();
        results.extend(entity_hits);

        // Apply filters (matching the behavior of SqliteRetriever)
        apply_filters(&mut results, query);

        // Rank results using search query
        let ranked = rank(query, results);

        Ok(ranked)
    }
}

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
                ObjectRef::AgentAction(_) => "agent_action",
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
