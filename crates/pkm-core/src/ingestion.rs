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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionState {
    #[default]
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
    /// Check if a transition from `self` to `next` is allowed.
    ///
    /// Ingestion progresses through a linear pipeline:
    ///   Captured → Parsed → Cleaned → Indexed → Summarized → Classified
    ///   → Linked → AwaitingReview
    ///
    /// From AwaitingReview, the user decides: Promoted, Archived, or Rejected.
    /// From any non-terminal state, a transition to Failed is allowed (error).
    /// From Failed, transition back to any earlier processing stage (retry).
    /// Terminal states (Promoted, Archived, Rejected) do not transition.
    pub fn can_transition_to(self, next: IngestionState) -> bool {
        use IngestionState::*;

        match (self, next) {
            // Forward progression through the pipeline.
            (Captured, Parsed)
            | (Parsed, Cleaned)
            | (Cleaned, Indexed)
            | (Indexed, Summarized)
            | (Summarized, Classified)
            | (Classified, Linked)
            | (Linked, AwaitingReview) => true,

            // From AwaitingReview: user decides the outcome.
            (AwaitingReview, Promoted)
            | (AwaitingReview, Archived)
            | (AwaitingReview, Rejected) => true,

            // From any non-terminal state, can fail (error handling).
            (state, Failed) if !state.is_terminal() => true,

            // From Failed, can retry by going back to earlier processing stages.
            // This allows re-parsing, re-indexing, etc. after fixing issues.
            (Failed, target) if !target.is_terminal() => true,

            // No other transitions are allowed.
            _ => false,
        }
    }

    /// Check if this state is a terminal state (does not transition further).
    fn is_terminal(self) -> bool {
        matches!(
            self,
            IngestionState::Promoted | IngestionState::Archived | IngestionState::Rejected
        )
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

    #[test]
    fn forward_pipeline_transitions_allowed() {
        // Test the happy path through the pipeline.
        assert!(IngestionState::Captured.can_transition_to(IngestionState::Parsed));
        assert!(IngestionState::Parsed.can_transition_to(IngestionState::Cleaned));
        assert!(IngestionState::Cleaned.can_transition_to(IngestionState::Indexed));
        assert!(IngestionState::Indexed.can_transition_to(IngestionState::Summarized));
        assert!(IngestionState::Summarized.can_transition_to(IngestionState::Classified));
        assert!(IngestionState::Classified.can_transition_to(IngestionState::Linked));
        assert!(IngestionState::Linked.can_transition_to(IngestionState::AwaitingReview));
    }

    #[test]
    fn awaiting_review_transitions_allowed() {
        // User can decide the outcome from AwaitingReview.
        assert!(IngestionState::AwaitingReview.can_transition_to(IngestionState::Promoted));
        assert!(IngestionState::AwaitingReview.can_transition_to(IngestionState::Archived));
        assert!(IngestionState::AwaitingReview.can_transition_to(IngestionState::Rejected));
    }

    #[test]
    fn error_transitions_allowed() {
        // Any non-terminal state can fail.
        let non_terminal_states = [
            IngestionState::Captured,
            IngestionState::Parsed,
            IngestionState::Cleaned,
            IngestionState::Indexed,
            IngestionState::Summarized,
            IngestionState::Classified,
            IngestionState::Linked,
            IngestionState::AwaitingReview,
        ];

        for state in &non_terminal_states {
            assert!(
                state.can_transition_to(IngestionState::Failed),
                "{:?} should be able to fail",
                state
            );
        }
    }

    #[test]
    fn retry_transitions_from_failed_allowed() {
        // From Failed, can retry by going to earlier stages.
        let retry_targets = [
            IngestionState::Captured,
            IngestionState::Parsed,
            IngestionState::Cleaned,
            IngestionState::Indexed,
            IngestionState::Summarized,
            IngestionState::Classified,
            IngestionState::Linked,
            IngestionState::AwaitingReview,
        ];

        for target in &retry_targets {
            assert!(
                IngestionState::Failed.can_transition_to(*target),
                "Failed should be able to retry to {:?}",
                target
            );
        }
    }

    #[test]
    fn illegal_backward_transitions_rejected() {
        // Cannot go backwards in the pipeline (except from Failed).
        assert!(!IngestionState::Parsed.can_transition_to(IngestionState::Captured));
        assert!(!IngestionState::Cleaned.can_transition_to(IngestionState::Parsed));
        assert!(!IngestionState::Indexed.can_transition_to(IngestionState::Cleaned));
        assert!(!IngestionState::AwaitingReview.can_transition_to(IngestionState::Linked));
    }

    #[test]
    fn illegal_transitions_from_promoted_rejected() {
        // Terminal states do not transition.
        assert!(!IngestionState::Promoted.can_transition_to(IngestionState::Archived));
        assert!(!IngestionState::Promoted.can_transition_to(IngestionState::Parsed));
        assert!(!IngestionState::Promoted.can_transition_to(IngestionState::Failed));
    }

    #[test]
    fn illegal_transitions_from_archived_rejected() {
        // Terminal states do not transition.
        assert!(!IngestionState::Archived.can_transition_to(IngestionState::Promoted));
        assert!(!IngestionState::Archived.can_transition_to(IngestionState::Parsed));
        assert!(!IngestionState::Archived.can_transition_to(IngestionState::Failed));
    }

    #[test]
    fn illegal_transitions_from_rejected_rejected() {
        // Terminal states do not transition.
        assert!(!IngestionState::Rejected.can_transition_to(IngestionState::Promoted));
        assert!(!IngestionState::Rejected.can_transition_to(IngestionState::Parsed));
        assert!(!IngestionState::Rejected.can_transition_to(IngestionState::Failed));
    }

    #[test]
    fn illegal_skipping_stages_rejected() {
        // Cannot skip stages in the pipeline.
        assert!(!IngestionState::Captured.can_transition_to(IngestionState::Cleaned));
        assert!(!IngestionState::Captured.can_transition_to(IngestionState::AwaitingReview));
        assert!(!IngestionState::Parsed.can_transition_to(IngestionState::Indexed));
        assert!(!IngestionState::Indexed.can_transition_to(IngestionState::AwaitingReview));
    }

    #[test]
    fn illegal_transitions_to_failed_from_terminal_rejected() {
        // Terminal states cannot fail (they're already done).
        assert!(!IngestionState::Promoted.can_transition_to(IngestionState::Failed));
        assert!(!IngestionState::Archived.can_transition_to(IngestionState::Failed));
        assert!(!IngestionState::Rejected.can_transition_to(IngestionState::Failed));
    }
}
