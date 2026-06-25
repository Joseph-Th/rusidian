//! Agent Action: a recorded operation performed or proposed by an agent
//! (AGENTS.md "Agent Action" + "Versioning and Audit Rules").
//!
//! This is the heart of agent safety. EVERY agent-originated mutation that
//! touches user-facing knowledge must produce an `AgentAction` record with a
//! diff and a rollback path. Direct application is only acceptable for low-risk
//! mechanical actions (indexing, derived caches).

use serde::{Deserialize, Serialize};

use crate::id::{AgentActionId, ObjectRef};
use crate::{Actor, Timestamp};

/// Lifecycle of an agent action. Product invariant set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActionStatus {
    Proposed,
    Accepted,
    Rejected,
    Applied,
    Reverted,
    Failed,
}

/// The typed operation an action represents. Mirrors the "Good operations" list
/// in AGENTS.md. The agent crate maps these to executable handlers (task D1).
/// Bad operations (rewrite_vault, mutate_markdown_blob, ...) must never exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    CreateSource,
    ParseSource,
    CreateNote,
    CreateBlock,
    UpdateBlock,
    MoveBlock,
    CreateEntity,
    MergeEntities,
    CreateTypedLink,
    ProposeTypedLink,
    AttachSourceToNote,
    GenerateSummary,
    ProposeSummary,
    MarkReviewed,
    CreateView,
    UpdateView,
    RollbackAction,
}

/// STUB. The before/after representation (full snapshot vs. structured patch)
/// is an explicit architecture decision — see STATUS.md task D2 and write an
/// ADR. Do not pick the easy "store whole blob" path without recording why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentAction {
    pub id: AgentActionId,
    pub actor: Actor,
    pub operation: OperationKind,
    pub target: ObjectRef,
    pub status: AgentActionStatus,
    pub rationale: String,
    pub created_at: Timestamp,
    /// JSON-encoded before/after diff. Schema decided in task D2.
    pub diff: serde_json::Value,
    /// Action this one rolls back, if any.
    pub rollback_of: Option<AgentActionId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_action_status_round_trips_as_snake_case() {
        for status in [
            AgentActionStatus::Proposed,
            AgentActionStatus::Accepted,
            AgentActionStatus::Rejected,
            AgentActionStatus::Applied,
            AgentActionStatus::Reverted,
            AgentActionStatus::Failed,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: AgentActionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }

    #[test]
    fn operation_kind_round_trips_as_snake_case() {
        for kind in [
            OperationKind::CreateSource,
            OperationKind::ParseSource,
            OperationKind::CreateNote,
            OperationKind::CreateBlock,
            OperationKind::UpdateBlock,
            OperationKind::MoveBlock,
            OperationKind::CreateEntity,
            OperationKind::MergeEntities,
            OperationKind::CreateTypedLink,
            OperationKind::ProposeTypedLink,
            OperationKind::AttachSourceToNote,
            OperationKind::GenerateSummary,
            OperationKind::ProposeSummary,
            OperationKind::MarkReviewed,
            OperationKind::CreateView,
            OperationKind::UpdateView,
            OperationKind::RollbackAction,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: OperationKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, kind);
        }
    }
}
