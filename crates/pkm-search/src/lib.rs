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

use pkm_core::ports::{SearchHit, SearchMode, SearchQuery};

/// Parse a raw user query string into a structured query for `mode`.
/// STUB — task E2: handle quoted phrases (exact), bare terms (fuzzy), and
/// field filters (e.g. `type:note`, `reviewed:true`). Pure + unit-tested.
pub fn parse_query(_mode: SearchMode, _raw: &str) -> SearchQuery {
    // TODO(E2): real parser + tests.
    unimplemented!("query parser — STATUS.md task E2")
}

/// Rank candidate hits for a query (pure; no IO). STUB — task E2.
pub fn rank(_query: &SearchQuery, _candidates: Vec<SearchHit>) -> Vec<SearchHit> {
    // TODO(E2): scoring + stable ordering + tests on ranking quality.
    unimplemented!("ranking — STATUS.md task E2")
}
