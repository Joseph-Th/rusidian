//! Ingestion pipeline (AGENTS.md "Ingestion Pipeline").
//!
//! Ingestion is a PIPELINE WITH EXPLICIT STATES, not an inbox folder. Each
//! stage produces derived content that links back to the immutable raw source.
//!
//! INVARIANTS:
//! - Raw capture is immutable unless the user explicitly edits it.
//! - Cleanup/summary/entity/link outputs are derived + reviewable, never
//!   silently promoted.
//! - Failed ingestion preserves diagnostics to allow retry.

use serde::{Deserialize, Serialize};

/// The explicit pipeline states. Product invariant set (AGENTS.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionState {
    Captured,
    Parsed,
    Cleaned,
    Indexed,
    Summarized,
    Classified,
    Linked,
    AwaitingReview,
    Promoted,
    Archived,
    Rejected,
    Failed,
}

impl IngestionState {
    /// Allowed forward transitions. STUB — task D3 must fill in the real
    /// transition table and reject illegal jumps (return `false`). Tests for
    /// this state machine are required (AGENTS.md Testing Expectations).
    pub fn can_transition_to(self, _next: IngestionState) -> bool {
        // TODO(D3): implement the real transition rules + unit tests.
        unimplemented!("ingestion transition table — STATUS.md task D3")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingestion_state_round_trips_as_snake_case() {
        for state in [
            IngestionState::Captured,
            IngestionState::Parsed,
            IngestionState::Cleaned,
            IngestionState::Indexed,
            IngestionState::Summarized,
            IngestionState::Classified,
            IngestionState::Linked,
            IngestionState::AwaitingReview,
            IngestionState::Promoted,
            IngestionState::Archived,
            IngestionState::Rejected,
            IngestionState::Failed,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let back: IngestionState = serde_json::from_str(&json).unwrap();
            assert_eq!(back, state);
        }
    }
}
