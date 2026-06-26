//! pkm-search: pure query-parsing + ranking logic.
//!
//! The retrieval BOUNDARY ([`pkm_core::ports::Retriever`]) and its result/query
//! types live in `pkm-core`. The concrete SQLite/FTS implementation lives in
//! `pkm-storage`. THIS crate holds the pure, testable pieces: parsing a user
//! query string into a structured [`pkm_core::ports::SearchQuery`], and ranking
//! candidate hits. Keep these as pure functions (AGENTS.md Coding Standards:
//! "pure functions for parsing, transformation, ranking, and rendering").
//!
//! INVARIANT: ranking/filtering must never drop a hit's
//! [`pkm_core::provenance::ContentStatus`]; the UI relies on it to avoid
//! presenting unreviewed/generated content as settled knowledge.

use pkm_core::ports::{SearchFilters, SearchHit, SearchMode, SearchQuery};
use pkm_core::review::ReviewState;

/// Parse a raw user query string into a structured query for `mode`.
/// Handles quoted phrases (exact), bare terms (fuzzy), and field filters
/// (e.g. `type:note`, `reviewed:true`, `status:proposed`, `date:...`, `project:...`).
pub fn parse_query(mode: SearchMode, raw: &str) -> SearchQuery {
    let mut text = String::new();
    let mut filters = SearchFilters::default();
    let trimmed = raw.trim();

    let mut chars = trimmed.chars().peekable();
    while let Some(ch) = chars.next() {
        // Parse field filters (format: key:value)
        if ch.is_alphabetic() {
            let mut key = String::from(ch);
            let mut is_filter = false;

            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    key.push(chars.next().unwrap());
                } else if c == ':' {
                    chars.next(); // consume ':'
                    is_filter = true;
                    let mut value = String::new();
                    while let Some(&c) = chars.peek() {
                        if c.is_whitespace() {
                            break;
                        }
                        if c == '"' || c == '\'' {
                            break;
                        }
                        value.push(chars.next().unwrap());
                    }
                    apply_filter(&mut filters, &key, &value);
                    break;
                } else {
                    break;
                }
            }

            // If no ':' was found, add back to text search
            if !is_filter {
                text.push_str(&key);
                text.push(' ');
            }
        } else if ch == '"' {
            // Quoted phrase (treated as exact match in FTS)
            let mut phrase = String::new();
            for c in chars.by_ref() {
                if c == '"' {
                    break;
                }
                phrase.push(c);
            }
            text.push('"');
            text.push_str(&phrase);
            text.push('"');
            text.push(' ');
        } else if !ch.is_whitespace() {
            // Bare term
            let mut term = String::from(ch);
            while let Some(&c) = chars.peek() {
                if !c.is_alphanumeric() && c != '_' && c != '-' {
                    break;
                }
                term.push(chars.next().unwrap());
            }
            text.push_str(&term);
            text.push(' ');
        }
    }

    SearchQuery {
        mode,
        text: text.trim().to_string(),
        filters,
    }
}

/// Apply a parsed key:value filter to the SearchFilters.
fn apply_filter(filters: &mut SearchFilters, key: &str, value: &str) {
    match key {
        "type" | "object_type" => {
            filters.object_type = Some(value.to_lowercase());
        }
        "status" | "content_status" => {
            filters.object_type = Some(value.to_lowercase());
        }
        "reviewed" | "review_state" | "review" => {
            if let Ok(state) = parse_review_state(value) {
                filters.review_state = Some(state);
            }
        }
        "date" => {
            // Support date:YYYY-MM-DD or date:YYYY-MM-DD..YYYY-MM-DD
            let parts: Vec<&str> = value.split("..").collect();
            if parts.len() == 2 {
                filters.date_range = Some((parts[0].to_string(), parts[1].to_string()));
            } else if parts.len() == 1 {
                // Single date becomes a range of that day
                filters.date_range = Some((parts[0].to_string(), parts[0].to_string()));
            }
        }
        "project" => {
            filters.project = Some(value.to_string());
        }
        _ => {}
    }
}

/// Parse review state from string (e.g., "proposed", "accepted", "rejected").
fn parse_review_state(s: &str) -> Result<ReviewState, ()> {
    match s.to_lowercase().as_str() {
        "proposed" => Ok(ReviewState::Proposed),
        "accepted" => Ok(ReviewState::Accepted),
        "rejected" => Ok(ReviewState::Rejected),
        _ => Err(()),
    }
}

/// Rank candidate hits for a query (pure; no IO). Performs scoring and stable ordering.
pub fn rank(_query: &SearchQuery, mut candidates: Vec<SearchHit>) -> Vec<SearchHit> {
    // Score each hit based on the query and its status. Preserve ContentStatus.
    for hit in &mut candidates {
        // Prefer reviewed/authored content over generated/unreviewed.
        let base_score = match hit.status {
            pkm_core::provenance::ContentStatus::UserAuthored => 1.0,
            pkm_core::provenance::ContentStatus::Reviewed => 0.95,
            pkm_core::provenance::ContentStatus::RawSource => 0.8,
            pkm_core::provenance::ContentStatus::ExtractedMetadata => 0.7,
            pkm_core::provenance::ContentStatus::AiSummary => 0.6,
            pkm_core::provenance::ContentStatus::InferredLink => 0.5,
            pkm_core::provenance::ContentStatus::UnreviewedSuggestion => 0.3,
        };

        hit.score = Some(base_score);
    }

    // Sort by score (descending), preserving ContentStatus for display.
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_core::id::{NoteId, SourceId};
    use pkm_core::provenance::ContentStatus;

    #[test]
    fn parse_bare_terms() {
        let query = parse_query(SearchMode::FuzzyText, "hello world");
        assert_eq!(query.mode, SearchMode::FuzzyText);
        assert!(query.text.contains("hello"));
        assert!(query.text.contains("world"));
        assert_eq!(query.filters.object_type, None);
    }

    #[test]
    fn parse_quoted_phrase() {
        let query = parse_query(SearchMode::ExactText, r#""exact phrase""#);
        assert!(query.text.contains("exact phrase"));
    }

    #[test]
    fn parse_object_type_filter() {
        let query = parse_query(SearchMode::FuzzyText, "search type:note");
        assert_eq!(query.filters.object_type, Some("note".to_string()));
        assert!(query.text.contains("search"));
    }

    #[test]
    fn parse_review_state_filter() {
        let query = parse_query(SearchMode::FuzzyText, "search reviewed:accepted");
        assert_eq!(query.filters.review_state, Some(ReviewState::Accepted));
    }

    #[test]
    fn parse_date_filter() {
        let query = parse_query(SearchMode::FuzzyText, "search date:2025-01-01");
        assert!(query.filters.date_range.is_some());
    }

    #[test]
    fn parse_project_filter() {
        let query = parse_query(SearchMode::FuzzyText, "search project:myproject");
        assert_eq!(query.filters.project, Some("myproject".to_string()));
    }

    #[test]
    fn rank_preserves_content_status() {
        let hits = vec![
            SearchHit {
                object: pkm_core::id::ObjectRef::Note(NoteId::new()),
                status: ContentStatus::UserAuthored,
                score: None,
                snippet: None,
                created_at: None,
                project: None,
            },
            SearchHit {
                object: pkm_core::id::ObjectRef::Note(NoteId::new()),
                status: ContentStatus::UnreviewedSuggestion,
                score: None,
                snippet: None,
                created_at: None,
                project: None,
            },
        ];

        let query = SearchQuery {
            mode: SearchMode::FuzzyText,
            text: "test".to_string(),
            filters: SearchFilters::default(),
        };

        let ranked = rank(&query, hits);
        assert_eq!(ranked.len(), 2);
        // UserAuthored should be first
        assert_eq!(ranked[0].status, ContentStatus::UserAuthored);
        // UnreviewedSuggestion should be last, and NEVER dropped
        assert_eq!(ranked[1].status, ContentStatus::UnreviewedSuggestion);
    }

    #[test]
    fn rank_produces_scores() {
        let hits = vec![SearchHit {
            object: pkm_core::id::ObjectRef::Source(SourceId::new()),
            status: ContentStatus::Reviewed,
            score: None,
            snippet: None,
            created_at: None,
            project: None,
        }];

        let query = SearchQuery {
            mode: SearchMode::FuzzyText,
            text: "test".to_string(),
            filters: SearchFilters::default(),
        };

        let ranked = rank(&query, hits);
        assert!(ranked[0].score.is_some());
        assert!(ranked[0].score.unwrap() > 0.0 && ranked[0].score.unwrap() <= 1.0);
    }
}
