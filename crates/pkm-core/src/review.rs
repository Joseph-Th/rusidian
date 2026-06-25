//! Review state — cross-cutting, so it lives in core.
//!
//! Agent-generated summaries, links, metadata, and entity merges normally enter
//! a `Proposed` state before becoming `Accepted` knowledge (AGENTS.md
//! "Reviewable automation"). Links, blocks, summaries, and ingestion items all
//! reference this one enum.

use serde::{Deserialize, Serialize};

/// Whether a piece of proposed structure has been reviewed by the user.
///
/// Invariant enum — change via ADR (+ migration, since it is persisted).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    /// Proposed by automation, not yet acted on by the user.
    Proposed,
    /// User accepted it as durable knowledge.
    Accepted,
    /// User rejected it; kept for audit, not surfaced as knowledge.
    Rejected,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_state_round_trips_as_snake_case() {
        for state in [
            ReviewState::Proposed,
            ReviewState::Accepted,
            ReviewState::Rejected,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let back: ReviewState = serde_json::from_str(&json).unwrap();
            assert_eq!(back, state);
        }
    }
}
