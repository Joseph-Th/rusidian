use pkm_core::ports::{Retriever, SearchQuery, SearchHit};
use pkm_core::provenance::ContentStatus;
use pkm_core::id::ObjectRef;
use crate::state::SharedVault;
use pkm_search::rank;

pub struct FsRetriever {
    pub state: SharedVault,
}

impl Retriever for FsRetriever {
    fn search(&self, query: &SearchQuery) -> pkm_core::Result<Vec<SearchHit>> {
        let search_term = query.text.to_lowercase();

        if search_term.is_empty() {
            return Ok(vec![]);
        }

        // Extract search data under lock, then drop to avoid blocking writes
        let (block_data, note_data, source_data, entity_data) = {
            let state = self.state.read().unwrap();

            let blocks: Vec<_> = state.blocks.values().map(|b| {
                let text = match &b.content {
                    pkm_core::block::BlockContent::Markdown { text } => text.clone(),
                    pkm_core::block::BlockContent::Table { headers, rows } => {
                        format!("{} {}", headers.join(" "), rows.iter().map(|r| r.join(" ")).collect::<Vec<_>>().join(" "))
                    }
                    pkm_core::block::BlockContent::Math { expression, .. } => expression.clone(),
                    pkm_core::block::BlockContent::Media { alt_text, .. } => alt_text.clone(),
                    _ => String::new(),
                };
                (b.id, text, b.created_at.to_string())
            }).collect();

            let notes: Vec<_> = state.notes.values().map(|n| {
                let project = n.metadata.project.clone();
                (n.id, n.title.clone(), n.created_at.to_string(), project)
            }).collect();

            let sources: Vec<_> = state.sources.values().map(|s| {
                (s.id, s.title.clone(), s.raw_content.clone(), s.created_at.to_string())
            }).collect();

            let entities: Vec<_> = state.entities.values().map(|e| {
                (e.id, e.name.clone(), e.aliases.clone(), e.created_at.to_string())
            }).collect();

            (blocks, notes, sources, entities)
        };

        let mut results = Vec::new();

        // 1. Search blocks
        for (id, text, created_at) in &block_data {
            let text_lower = text.to_lowercase();
            if text_lower.contains(&search_term) {
                results.push(SearchHit {
                    object: ObjectRef::Block(*id),
                    status: ContentStatus::UserAuthored,
                    score: None,
                    snippet: extract_snippet(text, &search_term, 150),
                    created_at: Some(created_at.clone()),
                    project: None,
                });
            }
        }

        // 2. Search notes
        for (id, title, created_at, project) in &note_data {
            if title.to_lowercase().contains(&search_term) {
                results.push(SearchHit {
                    object: ObjectRef::Note(*id),
                    status: ContentStatus::UserAuthored,
                    score: None,
                    snippet: Some(format!("Note: {}", title)),
                    created_at: Some(created_at.clone()),
                    project: project.clone(),
                });
            }
        }

        // 3. Search sources
        for (id, title, raw_content, created_at) in &source_data {
            let title_match = title.as_ref().map(|t| t.to_lowercase().contains(&search_term)).unwrap_or(false);
            let content_match = raw_content.to_lowercase().contains(&search_term);
            if title_match || content_match {
                results.push(SearchHit {
                    object: ObjectRef::Source(*id),
                    status: ContentStatus::RawSource,
                    score: None,
                    snippet: extract_snippet(raw_content, &search_term, 150),
                    created_at: Some(created_at.clone()),
                    project: None,
                });
            }
        }

        // 4. Search entities
        for (id, name, aliases, created_at) in &entity_data {
            let name_match = name.to_lowercase().contains(&search_term);
            let alias_match = aliases.iter().any(|a| a.to_lowercase().contains(&search_term));
            if name_match || alias_match {
                results.push(SearchHit {
                    object: ObjectRef::Entity(*id),
                    status: ContentStatus::ExtractedMetadata,
                    score: None,
                    snippet: Some(format!("Entity: {}", name)),
                    created_at: Some(created_at.clone()),
                    project: None,
                });
            }
        }

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
