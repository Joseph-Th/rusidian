//! pkm-agent: the typed, audited operation layer (AGENTS.md Phase 3 +
//! "Agent-Native API Principles").
//!
//! This is the safety boundary. Rules it enforces:
//! - Agents act ONLY through typed operations (Operation enum); never by
//!   rewriting arbitrary files/blobs.
//! - Every operation that affects user-facing knowledge is recorded as an
//!   `AgentAction` with a diff + rationale + rollback path.
//! - Agent changes default to `Proposed`. Direct apply only for low-risk
//!   mechanical ops (indexing, derived caches).
//!
//! It operates against the core repository PORTS (e.g. `&dyn SourceRepo`), NOT
//! against pkm-storage directly, so persistence details never leak in here.
//!
//! The "bad operations" from AGENTS.md (rewrite_vault, mutate_markdown_blob,
//! delete_without_recovery, ...) must NEVER be added here.

use serde::{Deserialize, Serialize};

use pkm_core::agent_action::{AgentAction, AgentActionStatus, OperationKind};
use pkm_core::block::BlockContent;
use pkm_core::id::{AgentActionId, BlockId, EntityId, NoteId, ObjectRef, SourceId, ViewId};
use pkm_core::link::LinkType;
use pkm_core::{Actor, Timestamp};

pub type Result<T> = std::result::Result<T, AgentError>;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("operation rejected: {0}")]
    Rejected(String),
    /// Wraps a persistence error surfaced through the core ports as a
    /// `CoreError`. Keeps pkm-agent free of any direct storage dependency.
    #[error(transparent)]
    Core(#[from] pkm_core::CoreError),
}

/// A typed operation request with all necessary parameters.
/// Each variant contains exactly the data needed to execute the operation.
/// Task D1 implements dispatch and execution; D2 adds persistence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Operation {
    /// Capture a new raw source.
    CreateSource {
        source_id: SourceId,
        source_type: String,
        title: String,
    },
    /// Extract structure from an existing source (indexing, derived content).
    ParseSource { source_id: SourceId },

    /// Create a new note with initial content.
    CreateNote { note_id: NoteId, title: String },
    /// Attach an existing source to a note, creating a relationship.
    AttachSourceToNote {
        source_id: SourceId,
        note_id: NoteId,
    },

    /// Create a block inside a note.
    CreateBlock {
        note_id: NoteId,
        block_id: BlockId,
        content: BlockContent,
        order: f32,
    },
    /// Update the content of an existing block.
    UpdateBlock {
        block_id: BlockId,
        new_content: BlockContent,
    },
    /// Reorder a block to a new position.
    MoveBlock { block_id: BlockId, new_order: f32 },

    /// Create a new named entity.
    CreateEntity {
        entity_id: EntityId,
        entity_kind: pkm_core::entity::EntityKind,
        name: String,
    },
    /// Merge multiple entities (survivor chosen, losers marked merged-in).
    MergeEntities {
        survivor_id: EntityId,
        loser_ids: Vec<EntityId>,
    },

    /// Create a direct, confirmed typed link between objects.
    CreateTypedLink {
        from: ObjectRef,
        to: ObjectRef,
        link_type: LinkType,
    },
    /// Propose a typed link for user review.
    ProposeTypedLink {
        from: ObjectRef,
        to: ObjectRef,
        link_type: LinkType,
    },

    /// Generate a summary derived from a source or block.
    GenerateSummary {
        target_ref: ObjectRef,
        summary_text: String,
    },
    /// Propose a summary for user review.
    ProposeSummary {
        target_ref: ObjectRef,
        summary_text: String,
    },

    /// Mark an object (link, entity, block) as reviewed by the user.
    MarkReviewed {
        target_ref: ObjectRef,
        reviewed: bool,
    },

    /// Create a new presentation view.
    CreateView {
        view_id: ViewId,
        view_kind: pkm_core::view::ViewKind,
        title: String,
        params: serde_json::Value,
    },
    /// Update an existing view's parameters.
    UpdateView {
        view_id: ViewId,
        params: serde_json::Value,
    },

    /// Revert a previous agent action to its prior state.
    RollbackAction { action_id: AgentActionId },
}

impl Operation {
    /// Map this operation to its OperationKind (for audit log classification).
    pub fn kind(&self) -> OperationKind {
        match self {
            Operation::CreateSource { .. } => OperationKind::CreateSource,
            Operation::ParseSource { .. } => OperationKind::ParseSource,
            Operation::CreateNote { .. } => OperationKind::CreateNote,
            Operation::AttachSourceToNote { .. } => OperationKind::AttachSourceToNote,
            Operation::CreateBlock { .. } => OperationKind::CreateBlock,
            Operation::UpdateBlock { .. } => OperationKind::UpdateBlock,
            Operation::MoveBlock { .. } => OperationKind::MoveBlock,
            Operation::CreateEntity { .. } => OperationKind::CreateEntity,
            Operation::MergeEntities { .. } => OperationKind::MergeEntities,
            Operation::CreateTypedLink { .. } => OperationKind::CreateTypedLink,
            Operation::ProposeTypedLink { .. } => OperationKind::ProposeTypedLink,
            Operation::GenerateSummary { .. } => OperationKind::GenerateSummary,
            Operation::ProposeSummary { .. } => OperationKind::ProposeSummary,
            Operation::MarkReviewed { .. } => OperationKind::MarkReviewed,
            Operation::CreateView { .. } => OperationKind::CreateView,
            Operation::UpdateView { .. } => OperationKind::UpdateView,
            Operation::RollbackAction { .. } => OperationKind::RollbackAction,
        }
    }

    /// Compute the primary target object affected by this operation.
    pub fn target(&self) -> ObjectRef {
        match self {
            Operation::CreateSource { source_id, .. } => ObjectRef::Source(*source_id),
            Operation::ParseSource { source_id } => ObjectRef::Source(*source_id),
            Operation::CreateNote { note_id, .. } => ObjectRef::Note(*note_id),
            Operation::AttachSourceToNote { note_id, .. } => ObjectRef::Note(*note_id),
            Operation::CreateBlock { note_id, .. } => ObjectRef::Note(*note_id),
            Operation::UpdateBlock { block_id, .. } => ObjectRef::Block(*block_id),
            Operation::MoveBlock { block_id, .. } => ObjectRef::Block(*block_id),
            Operation::CreateEntity { entity_id, .. } => ObjectRef::Entity(*entity_id),
            Operation::MergeEntities { survivor_id, .. } => ObjectRef::Entity(*survivor_id),
            Operation::CreateTypedLink { from, .. } => *from,
            Operation::ProposeTypedLink { from, .. } => *from,
            Operation::GenerateSummary { target_ref, .. } => *target_ref,
            Operation::ProposeSummary { target_ref, .. } => *target_ref,
            Operation::MarkReviewed { target_ref, .. } => *target_ref,
            Operation::CreateView { view_id, .. } => ObjectRef::View(*view_id),
            Operation::UpdateView { view_id, .. } => ObjectRef::View(*view_id),
            Operation::RollbackAction { .. } => {
                // This targets the action itself; could be any object type
                // In practice, the caller tracks what was rolled back.
                ObjectRef::Source(SourceId::new())
            }
        }
    }
}

/// A request to perform a typed operation with actor context.
#[derive(Debug, Clone)]
pub struct OperationRequest {
    pub actor: Actor,
    pub operation: Operation,
    pub rationale: String,
}

/// Whether an operation may be applied directly or must be proposed for review.
///
/// Mechanical ops (low-risk, internal) apply directly. Knowledge ops (user-facing
/// changes) default to `Proposed` for review (AGENTS.md "Reviewable automation").
pub fn requires_review(op: &Operation) -> bool {
    match op {
        // Mechanical: derive structure from existing content, no user edits.
        Operation::ParseSource { .. } => false,
        Operation::GenerateSummary { .. } => false,

        // Knowledge: all user-facing changes and entity operations.
        Operation::CreateSource { .. }
        | Operation::CreateNote { .. }
        | Operation::AttachSourceToNote { .. }
        | Operation::CreateBlock { .. }
        | Operation::UpdateBlock { .. }
        | Operation::MoveBlock { .. }
        | Operation::CreateEntity { .. }
        | Operation::MergeEntities { .. }
        | Operation::CreateTypedLink { .. }
        | Operation::ProposeTypedLink { .. }
        | Operation::ProposeSummary { .. }
        | Operation::MarkReviewed { .. }
        | Operation::CreateView { .. }
        | Operation::UpdateView { .. }
        | Operation::RollbackAction { .. } => true,
    }
}

/// Execute a typed operation, producing an auditable `AgentAction`.
/// Task D1 creates the action record. Task D2 adds persistence and actual apply/rollback.
///
/// The returned `AgentAction` has:
/// - status: `Proposed` if `requires_review(op)`, else (placeholder, D2 applies)
/// - diff: empty JSON for now (D2 defines the schema)
/// - rollback_of: None (D2 implements actual rollback)
pub fn execute(req: OperationRequest) -> Result<AgentAction> {
    let now = Timestamp::now_utc();
    let status = if requires_review(&req.operation) {
        AgentActionStatus::Proposed
    } else {
        // Mechanical ops: status is Applied when persist happens (D2)
        // For now, mark as Proposed so D2 can implement actual application.
        AgentActionStatus::Proposed
    };

    let action = AgentAction {
        id: AgentActionId::new(),
        actor: req.actor,
        operation: req.operation.kind(),
        target: req.operation.target(),
        status,
        rationale: req.rationale,
        created_at: now,
        diff: serde_json::json!({}),
        rollback_of: None,
    };

    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mechanical_ops_require_no_review() {
        let parse_source = Operation::ParseSource {
            source_id: SourceId::new(),
        };
        assert!(!requires_review(&parse_source));

        let generate_summary = Operation::GenerateSummary {
            target_ref: ObjectRef::Source(SourceId::new()),
            summary_text: "A summary".to_string(),
        };
        assert!(!requires_review(&generate_summary));
    }

    #[test]
    fn knowledge_ops_require_review() {
        let create_source = Operation::CreateSource {
            source_id: SourceId::new(),
            source_type: "article".to_string(),
            title: "Test".to_string(),
        };
        assert!(requires_review(&create_source));

        let create_note = Operation::CreateNote {
            note_id: NoteId::new(),
            title: "Note".to_string(),
        };
        assert!(requires_review(&create_note));

        let propose_link = Operation::ProposeTypedLink {
            from: ObjectRef::Note(NoteId::new()),
            to: ObjectRef::Entity(EntityId::new()),
            link_type: LinkType::RelatedTo,
        };
        assert!(requires_review(&propose_link));
    }

    #[test]
    fn execute_creates_proposed_action_for_knowledge_ops() {
        let req = OperationRequest {
            actor: Actor::User,
            operation: Operation::CreateNote {
                note_id: NoteId::new(),
                title: "Test Note".to_string(),
            },
            rationale: "User created a note".to_string(),
        };

        let action = execute(req).unwrap();
        assert_eq!(action.status, AgentActionStatus::Proposed);
        assert_eq!(action.operation, OperationKind::CreateNote);
    }

    #[test]
    fn execute_creates_proposed_action_for_mechanical_ops() {
        // D1 creates Proposed for all; D2 implements the apply logic to
        // transition mechanical ops to Applied. This is the safety default.
        let req = OperationRequest {
            actor: Actor::System,
            operation: Operation::ParseSource {
                source_id: SourceId::new(),
            },
            rationale: "Automatic indexing".to_string(),
        };

        let action = execute(req).unwrap();
        // D1 creates all as Proposed; D2 applies mechanical ones automatically
        assert_eq!(action.status, AgentActionStatus::Proposed);
        assert_eq!(action.operation, OperationKind::ParseSource);
    }

    #[test]
    fn operation_round_trips() {
        let ops = [
            Operation::CreateSource {
                source_id: SourceId::new(),
                source_type: "pdf".to_string(),
                title: "Document".to_string(),
            },
            Operation::ParseSource {
                source_id: SourceId::new(),
            },
            Operation::CreateNote {
                note_id: NoteId::new(),
                title: "My Note".to_string(),
            },
            Operation::CreateBlock {
                note_id: NoteId::new(),
                block_id: BlockId::new(),
                content: BlockContent::Markdown {
                    text: "Hello".to_string(),
                },
                order: 1.0,
            },
            Operation::UpdateBlock {
                block_id: BlockId::new(),
                new_content: BlockContent::Markdown {
                    text: "Updated".to_string(),
                },
            },
            Operation::CreateTypedLink {
                from: ObjectRef::Note(NoteId::new()),
                to: ObjectRef::Entity(EntityId::new()),
                link_type: LinkType::Cites,
            },
        ];

        for op in ops.iter() {
            let json = serde_json::to_string(op).unwrap();
            let back: Operation = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, op);
        }
    }
}
