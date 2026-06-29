//! Agent Action: a recorded operation performed or proposed by an agent
//! (AGENTS.md "Agent Action" + "Versioning and Audit Rules").
//!
//! This is the heart of agent safety. EVERY agent-originated mutation that
//! touches user-facing knowledge must produce an `AgentAction` record with a
//! diff and a rollback path. Direct application is only acceptable for low-risk
//! mechanical actions (indexing, derived caches).

use serde::{Deserialize, Serialize};

use crate::block::Block;
use crate::id::{AgentActionId, BlockId, EntityId, LinkId, NoteId, ObjectRef};
use crate::link::LinkType;
use crate::{Actor, Timestamp};
use std::collections::HashMap;

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

/// Strongly-typed before/after data for each operation kind.
/// Replaces generic `serde_json::Value` — the compiler verifies structure at compile time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActionDiff {
    CreateNote {
        note_id: NoteId,
        title: String,
    },
    CreateBlock {
        note_id: NoteId,
        block_id: BlockId,
        content: crate::block::BlockContent,
    },
    UpdateBlock {
        before: Box<Block>,
        after: Box<Block>,
    },
    MoveBlock {
        before: Vec<BlockId>,
        note_id: NoteId,
        block_id: BlockId,
        new_index: usize,
    },
    CreateEntity {
        entity_id: EntityId,
        name: String,
    },
    MergeEntities {
        survivor_id: EntityId,
        loser_ids: Vec<EntityId>,
        repointed_links: HashMap<String, String>,
    },
    CreateTypedLink {
        from: ObjectRef,
        to: ObjectRef,
        link_type: LinkType,
        created_link_id: LinkId,
    },
    CreateSource {
        source_id: crate::id::SourceId,
        title: String,
    },
    CreateView {
        view_id: crate::id::ViewId,
        view_kind: crate::view::ViewKind,
        title: String,
        params: Box<crate::view::ViewParams>,
    },
    UpdateView {
        view_id: crate::id::ViewId,
        params: Box<crate::view::ViewParams>,
    },
    /// Fallback for operations that carry no meaningful before/after state.
    NoChange,
}

impl ActionDiff {
    /// Build a NoChange diff.
    pub fn none() -> Self {
        ActionDiff::NoChange
    }
}

/// A recorded agent action with its audit trail and diff.
///
/// The diff field stores operation-specific before/after snapshots.
/// Full snapshots provide safe, bulletproof rollbacks without fragile diffing logic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentAction {
    pub id: AgentActionId,
    pub actor: Actor,
    pub operation: OperationKind,
    pub target: ObjectRef,
    pub status: AgentActionStatus,
    pub rationale: String,
    pub created_at: Timestamp,
    /// Strongly-typed diff — not a generic JSON blob.
    /// Each variant stores exactly the before/after data needed for rollback.
    pub diff: ActionDiff,
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

    #[test]
    fn action_diff_round_trips() {
        use crate::block::BlockContent;

        let now = crate::Timestamp::now_utc();
        let block = Block {
            id: BlockId::new(),
            note_id: NoteId::new(),
            content: BlockContent::Markdown { text: "hello".into() },
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        };

        let diffs: Vec<ActionDiff> = vec![
            ActionDiff::CreateNote {
                note_id: NoteId::new(),
                title: "test".into(),
            },
            ActionDiff::UpdateBlock {
                before: Box::new(block.clone()),
                after: Box::new(block.clone()),
            },
            ActionDiff::MergeEntities {
                survivor_id: EntityId::new(),
                loser_ids: vec![],
                repointed_links: HashMap::new(),
            },
            ActionDiff::NoChange,
        ];

        for diff in diffs {
            let json = serde_json::to_string(&diff).unwrap();
            let back: ActionDiff = serde_json::from_str(&json).unwrap();
            assert_eq!(back, diff);
        }
    }
}
