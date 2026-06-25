//! Ingestion pipeline state machine.
//!
//! The ingestion system moves information through explicit states from capture
//! through review to promotion or rejection (AGENTS.md "Ingestion Pipeline").

use serde::{Deserialize, Serialize};

/// The explicit pipeline states. Product invariant set (AGENTS.md).
///
/// Ingestion progresses: Captured → Parsed → Cleaned → Indexed → Summarized →
/// Classified → Linked → AwaitingReview → (Promoted | Archived | Rejected).
/// Failed indicates a processing error with diagnostics retained for retry.
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
