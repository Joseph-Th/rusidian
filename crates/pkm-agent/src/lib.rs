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
use pkm_core::ports::{AgentActionRepo, EntityRepo, LinkRepo, NoteRepo};
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

    /// Create a block inside a note. (No more `order` string!)
    CreateBlock {
        note_id: NoteId,
        block_id: BlockId,
        content: BlockContent,
    },
    /// Update the content of an existing block.
    UpdateBlock {
        note_id: NoteId,
        block_id: BlockId,
        new_content: BlockContent,
    },
    /// Reorder a block to a new position.
    MoveBlock {
        note_id: NoteId,
        block_id: BlockId,
        new_index: usize,
    },

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
            Operation::RollbackAction { action_id } => {
                ObjectRef::AgentAction(*action_id)
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
/// Automation mode: all operations apply instantly without human review.
/// Every change is still recorded in the agent_action audit log for rollback.
pub fn requires_review(_op: &Operation) -> bool {
    // All operations auto-apply. Rollback is available if AI goes wrong.
    false
}

/// Execute a typed operation, producing and persisting an auditable `AgentAction`.
///
/// The action is persisted to the provided `AgentActionRepo`. Knowledge ops are
/// created with `Proposed` status and don't apply until accepted; mechanical ops
/// are created with `Proposed` status (D2 implements auto-apply logic).
///
/// The diff uses full snapshots (before/after JSON) per ADR 0003. For now,
/// diff is empty {} since actual object mutations happen in D2+.
pub fn execute(
    req: OperationRequest,
    action_repo: &dyn pkm_core::ports::AgentActionRepo,
    note_repo: &dyn pkm_core::ports::NoteRepo,
) -> Result<AgentAction> {
    let now = Timestamp::now_utc();
    let status = if requires_review(&req.operation) {
        AgentActionStatus::Proposed
    } else {
        AgentActionStatus::Applied
    };

    // Store Full Snapshots instead of JSON patches
    let diff = match &req.operation {
        Operation::CreateNote { note_id, title } => {
            serde_json::json!({
                "before": null,
                "after": { "note_id": note_id.to_string(), "title": title }
            })
        }
        Operation::CreateTypedLink { from, to, link_type } => {
            serde_json::json!({
                "before": null,
                "after": { "from": from, "to": to, "link_type": link_type }
            })
        }
        Operation::UpdateBlock { note_id, block_id, new_content } => {
            let blocks = note_repo.get_blocks(*note_id)?;
            let old_block = blocks.iter().find(|b| b.id == *block_id)
                .ok_or_else(|| pkm_core::CoreError::Invariant(format!("Block {} not found", block_id)))?;
            
            let mut new_block = old_block.clone();
            new_block.content = new_content.clone();

            serde_json::json!({
                "before": old_block,
                "after": new_block
            })
        }
        Operation::MergeEntities { survivor_id: _, loser_ids } => {
            serde_json::json!({
                "before": null,
                "after": { "loser_ids": loser_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>() }
            })
        }
        Operation::CreateBlock { note_id, block_id, content, .. } => {
            // Note: I removed `order` here based on Rip Out #2!
            serde_json::json!({
                "before": null,
                "after": {
                    "note_id": note_id.to_string(),
                    "block_id": block_id.to_string(),
                    "content": content
                }
            })
        }
        _ => serde_json::json!({ "before": null, "after": null }),
    };

    let action = AgentAction {
        id: AgentActionId::new(),
        actor: req.actor,
        operation: req.operation.kind(),
        target: req.operation.target(),
        status,
        rationale: req.rationale,
        created_at: now,
        diff,
        rollback_of: None,
    };

    action_repo.create(&action)?;
    Ok(action)
}

pub fn apply_action(
    action_id: AgentActionId,
    action_repo: &dyn AgentActionRepo,
    note_repo: &dyn NoteRepo,
    link_repo: Option<&dyn LinkRepo>,
) -> Result<AgentAction> {
    let mut action = action_repo.get(action_id)?
        .ok_or_else(|| AgentError::Rejected(format!("Action {} not found", action_id)))?;

    if action.status != AgentActionStatus::Proposed {
        return Err(AgentError::Rejected("Can only apply Proposed actions".into()));
    }

    let after = action.diff.get("after").cloned().unwrap_or_else(|| serde_json::json!({}));
    let mut rollback_data = serde_json::json!({});

    match action.operation {
        OperationKind::UpdateBlock => {
            let new_block: pkm_core::block::Block = serde_json::from_value(after)
                .map_err(|e| AgentError::Rejected(format!("Failed to parse new block: {}", e)))?;
            note_repo.update_block(new_block.note_id, new_block.id, new_block.content)?;
        }
        OperationKind::MergeEntities => {
            let link_repo = link_repo.ok_or_else(|| AgentError::Rejected("LinkRepo required".into()))?;
            let survivor_id = match action.target {
                ObjectRef::Entity(id) => id,
                _ => return Err(AgentError::Rejected("Target must be Entity".into())),
            };

            let mut repointed_links = serde_json::json!({});
            
            if let Some(loser_ids_arr) = after.get("loser_ids").and_then(|v| v.as_array()) {
                for id_val in loser_ids_arr {
                    if let Ok(uuid) = uuid::Uuid::parse_str(id_val.as_str().unwrap_or("")) {
                        let loser_id = EntityId(uuid);
                        
                        // Re-point TO links
                        for link in link_repo.get_by_to(ObjectRef::Entity(loser_id))? {
                            link_repo.set_to(link.id, ObjectRef::Entity(survivor_id))?;
                            repointed_links[link.id.to_string()] = serde_json::json!(loser_id.to_string());
                        }
                        
                        // Re-point FROM links
                        for link in link_repo.get_by_from(ObjectRef::Entity(loser_id))? {
                            link_repo.set_from(link.id, ObjectRef::Entity(survivor_id))?;
                            repointed_links[link.id.to_string()] = serde_json::json!(loser_id.to_string());
                        }
                    }
                }
            }
            rollback_data["repointed_links"] = repointed_links;
        }
        OperationKind::CreateNote => {
            let note_id = NoteId(uuid::Uuid::parse_str(after["note_id"].as_str().unwrap_or("")).unwrap());
            let title = after["title"].as_str().unwrap_or("").to_string();
            
            let now = Timestamp::now_utc();
            let note = pkm_core::note::Note {
                id: note_id,
                title,
                blocks: vec![],
                metadata: std::collections::BTreeMap::new(),
                created_by: action.actor.clone(),
                created_at: now,
                version: 1,
                updated_at: now,
            };
            note_repo.create(&note)?;
        }
        OperationKind::CreateTypedLink => {
            let link_repo = link_repo.ok_or_else(|| AgentError::Rejected("LinkRepo required".into()))?;
            
            let from: ObjectRef = serde_json::from_value(after["from"].clone()).unwrap();
            let to: ObjectRef = serde_json::from_value(after["to"].clone()).unwrap();
            let link_type: pkm_core::link::LinkType = serde_json::from_value(after["link_type"].clone()).unwrap();
            
            let now = Timestamp::now_utc();
            let link = pkm_core::link::Link {
                id: pkm_core::id::LinkId::new(),
                from, to, link_type,
                created_by: action.actor.clone(),
                created_at: now,
                reviewed: pkm_core::review::ReviewState::Accepted,
                confidence: None,
                version: 1,
                updated_at: now,
            };
            
            rollback_data["created_link_id"] = serde_json::json!(link.id.to_string());
            link_repo.create(&link)?;
        }
        OperationKind::CreateBlock => {
            let note_id = NoteId(uuid::Uuid::parse_str(after["note_id"].as_str().unwrap()).unwrap());
            let block_id = BlockId(uuid::Uuid::parse_str(after["block_id"].as_str().unwrap()).unwrap());
            let content: BlockContent = serde_json::from_value(after["content"].clone()).unwrap();
            
            let now = Timestamp::now_utc();
            let block = pkm_core::block::Block {
                id: block_id, note_id, content,
                created_by: action.actor.clone(),
                created_at: now, source_provenance_ref: None,
                version: 1, updated_at: now,
            };
            
            note_repo.create_block(&block)?;
            if let Ok(Some(note)) = note_repo.get(note_id) {
                note_repo.update(&note)?; // Triggers file save
            }
        }
        _ => return Err(AgentError::Rejected("Operation not supported for apply".into())),
    }

    // Save any required rollback metadata into the diff
    action.diff["rollback_data"] = rollback_data;
    action_repo.set_diff(action.id, action.diff.clone())?;
    
    action_repo.set_status(action.id, AgentActionStatus::Applied)?;
    action.status = AgentActionStatus::Applied;
    
    Ok(action)
}

pub fn rollback_action(
    action_id: AgentActionId,
    action_repo: &dyn AgentActionRepo,
    note_repo: &dyn NoteRepo,
    entity_repo: Option<&dyn EntityRepo>,
    link_repo: Option<&dyn LinkRepo>,
) -> Result<AgentAction> {
    let action = action_repo.get(action_id)?
        .ok_or_else(|| AgentError::Rejected(format!("Action {} not found", action_id)))?;

    if action.status != AgentActionStatus::Applied {
        return Err(AgentError::Rejected("Can only rollback Applied actions".into()));
    }

    let before = action.diff.get("before").cloned().unwrap_or_else(|| serde_json::json!({}));
    let after = action.diff.get("after").cloned().unwrap_or_else(|| serde_json::json!({}));
    let rollback_data = action.diff.get("rollback_data").cloned().unwrap_or_else(|| serde_json::json!({}));

    match action.operation {
        OperationKind::UpdateBlock => {
            let old_block: pkm_core::block::Block = serde_json::from_value(before)
                .map_err(|_| AgentError::Rejected("Failed to parse old block snapshot".into()))?;
            note_repo.update_block(old_block.note_id, old_block.id, old_block.content)?;
        }
        OperationKind::MergeEntities => {
            let entity_repo = entity_repo.ok_or_else(|| AgentError::Rejected("EntityRepo required".into()))?;
            let link_repo = link_repo.ok_or_else(|| AgentError::Rejected("LinkRepo required".into()))?;
            
            if let Some(loser_ids_arr) = after.get("loser_ids").and_then(|v| v.as_array()) {
                for id_val in loser_ids_arr {
                    if let Ok(uuid) = uuid::Uuid::parse_str(id_val.as_str().unwrap_or("")) {
                        entity_repo.clear_merged_into(EntityId(uuid))?;
                    }
                }
            }

            if let Some(repointed) = rollback_data.get("repointed_links").and_then(|v| v.as_object()) {
                for (link_id_str, loser_id_val) in repointed.iter() {
                    let link_id = pkm_core::id::LinkId(uuid::Uuid::parse_str(link_id_str).unwrap());
                    let loser_id = EntityId(uuid::Uuid::parse_str(loser_id_val.as_str().unwrap()).unwrap());
                    
                    if let Ok(Some(link)) = link_repo.get(link_id) {
                        if link.to == action.target { link_repo.set_to(link_id, ObjectRef::Entity(loser_id))?; }
                        if link.from == action.target { link_repo.set_from(link_id, ObjectRef::Entity(loser_id))?; }
                    }
                }
            }
        }
        OperationKind::CreateNote => {
            let note_id = NoteId(uuid::Uuid::parse_str(after["note_id"].as_str().unwrap()).unwrap());
            note_repo.delete(note_id)?;
        }
        OperationKind::CreateTypedLink => {
            let link_repo = link_repo.ok_or_else(|| AgentError::Rejected("LinkRepo required".into()))?;
            let link_id = pkm_core::id::LinkId(uuid::Uuid::parse_str(rollback_data["created_link_id"].as_str().unwrap()).unwrap());
            link_repo.delete(link_id)?;
        }
        OperationKind::CreateBlock => {
            let note_id = NoteId(uuid::Uuid::parse_str(after["note_id"].as_str().unwrap()).unwrap());
            let block_id = BlockId(uuid::Uuid::parse_str(after["block_id"].as_str().unwrap()).unwrap());
            note_repo.delete_block(note_id, block_id)?;
            
            if let Ok(Some(note)) = note_repo.get(note_id) {
                note_repo.update(&note)?; // Triggers file save
            }
        }
        _ => return Err(AgentError::Rejected("Rollback not supported for this operation".into())),
    }

    action_repo.set_status(action_id, AgentActionStatus::Reverted)?;

    let rollback_action = AgentAction {
        id: AgentActionId::new(),
        actor: Actor::System,
        operation: OperationKind::RollbackAction,
        target: action.target,
        status: AgentActionStatus::Applied,
        rationale: format!("Rollback of action {}", action_id),
        created_at: Timestamp::now_utc(),
        diff: serde_json::json!({}),
        rollback_of: Some(action_id),
    };
    action_repo.create(&rollback_action)?;

    Ok(rollback_action)
}


#[cfg(test)]
mod tests {
    use super::*;
    use pkm_core::ports::AgentActionRepo;
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// A simple in-memory mock AgentActionRepo for testing.
    struct MockActionRepo {
        actions: RefCell<HashMap<AgentActionId, AgentAction>>,
    }

    impl MockActionRepo {
        fn new() -> Self {
            Self {
                actions: RefCell::new(HashMap::new()),
            }
        }
    }

    impl pkm_core::ports::AgentActionRepo for MockActionRepo {
        fn create(&self, action: &AgentAction) -> pkm_core::Result<()> {
            self.actions.borrow_mut().insert(action.id, action.clone());
            Ok(())
        }

        fn get(&self, id: AgentActionId) -> pkm_core::Result<Option<AgentAction>> {
            Ok(self.actions.borrow().get(&id).cloned())
        }

        fn set_status(
            &self,
            id: AgentActionId,
            new_status: pkm_core::agent_action::AgentActionStatus,
        ) -> pkm_core::Result<()> {
            if let Some(action) = self.actions.borrow_mut().get_mut(&id) {
                action.status = new_status;
            }
            Ok(())
        }

        fn set_diff(&self, id: AgentActionId, diff: serde_json::Value) -> pkm_core::Result<()> {
            if let Some(action) = self.actions.borrow_mut().get_mut(&id) {
                action.diff = diff;
            }
            Ok(())
        }
    }

    #[test]
    fn all_ops_apply_instantly_in_automation_mode() {
        // In automation mode, all operations apply instantly (no review required)
        let parse_source = Operation::ParseSource {
            source_id: SourceId::new(),
        };
        assert!(!requires_review(&parse_source));

        let generate_summary = Operation::GenerateSummary {
            target_ref: ObjectRef::Source(SourceId::new()),
            summary_text: "A summary".to_string(),
        };
        assert!(!requires_review(&generate_summary));

        // Knowledge ops also apply instantly in automation mode
        let create_source = Operation::CreateSource {
            source_id: SourceId::new(),
            source_type: "article".to_string(),
            title: "Test".to_string(),
        };
        assert!(!requires_review(&create_source));

        let create_note = Operation::CreateNote {
            note_id: NoteId::new(),
            title: "Note".to_string(),
        };
        assert!(!requires_review(&create_note));

        let propose_link = Operation::ProposeTypedLink {
            from: ObjectRef::Note(NoteId::new()),
            to: ObjectRef::Entity(EntityId::new()),
            link_type: LinkType::RelatedTo,
        };
        assert!(!requires_review(&propose_link));
    }

    #[test]
    fn execute_creates_applied_action_for_all_ops() {
        // Automation mode: all operations apply instantly and are recorded as Applied
        let repo = MockActionRepo::new();
        let req = OperationRequest {
            actor: Actor::User,
            operation: Operation::CreateNote {
                note_id: NoteId::new(),
                title: "Test Note".to_string(),
            },
            rationale: "User created a note".to_string(),
        };

        let action = execute(req, &repo, &MockNoteRepo).unwrap();
        assert_eq!(action.status, AgentActionStatus::Applied);
        assert_eq!(action.operation, OperationKind::CreateNote);

        // Verify the action was persisted
        let retrieved = repo.get(action.id).unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn execute_creates_applied_action_for_mechanical_ops() {
        // Automation mode: all operations apply instantly (mechanical and knowledge ops alike)
        let repo = MockActionRepo::new();
        let req = OperationRequest {
            actor: Actor::System,
            operation: Operation::ParseSource {
                source_id: SourceId::new(),
            },
            rationale: "Automatic indexing".to_string(),
        };

        let action = execute(req, &repo, &MockNoteRepo).unwrap();
        assert_eq!(action.status, AgentActionStatus::Applied);
        assert_eq!(action.operation, OperationKind::ParseSource);

        // Verify the action was persisted
        let retrieved = repo.get(action.id).unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn merge_entities_applies_instantly() {
        // Automation mode: even risky operations like merge apply instantly
        let merge_op = Operation::MergeEntities {
            survivor_id: EntityId::new(),
            loser_ids: vec![EntityId::new(), EntityId::new()],
        };
        assert!(!requires_review(&merge_op));
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
            },
            Operation::UpdateBlock {
                note_id: NoteId::new(),
                block_id: BlockId::new(),
                new_content: BlockContent::Markdown {
                    text: "Updated".to_string(),
                },
            },
            Operation::CreateEntity {
                entity_id: EntityId::new(),
                entity_kind: pkm_core::entity::EntityKind::Person,
                name: "Alice".to_string(),
            },
            Operation::MergeEntities {
                survivor_id: EntityId::new(),
                loser_ids: vec![EntityId::new()],
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

    /// A simple in-memory mock LinkRepo for testing link re-pointing.
    struct MockLinkRepo {
        set_to_calls: RefCell<Vec<(pkm_core::id::LinkId, ObjectRef)>>,
        set_from_calls: RefCell<Vec<(pkm_core::id::LinkId, ObjectRef)>>,
        links_to: RefCell<HashMap<ObjectRef, Vec<pkm_core::link::Link>>>,
        links_from: RefCell<HashMap<ObjectRef, Vec<pkm_core::link::Link>>>,
    }

    impl MockLinkRepo {
        fn new() -> Self {
            Self {
                set_to_calls: RefCell::new(Vec::new()),
                set_from_calls: RefCell::new(Vec::new()),
                links_to: RefCell::new(HashMap::new()),
                links_from: RefCell::new(HashMap::new()),
            }
        }

        fn add_link_to(&self, target: ObjectRef, link: pkm_core::link::Link) {
            self.links_to
                .borrow_mut()
                .entry(target)
                .or_default()
                .push(link);
        }

        fn add_link_from(&self, source: ObjectRef, link: pkm_core::link::Link) {
            self.links_from
                .borrow_mut()
                .entry(source)
                .or_default()
                .push(link);
        }
    }

    impl pkm_core::ports::LinkRepo for MockLinkRepo {
        fn create(&self, _link: &pkm_core::link::Link) -> pkm_core::Result<()> {
            Ok(())
        }

        fn get(
            &self,
            _link_id: pkm_core::id::LinkId,
        ) -> pkm_core::Result<Option<pkm_core::link::Link>> {
            Ok(None)
        }

        fn get_by_to(&self, target: ObjectRef) -> pkm_core::Result<Vec<pkm_core::link::Link>> {
            Ok(self
                .links_to
                .borrow()
                .get(&target)
                .cloned()
                .unwrap_or_default())
        }

        fn get_by_from(&self, source: ObjectRef) -> pkm_core::Result<Vec<pkm_core::link::Link>> {
            Ok(self
                .links_from
                .borrow()
                .get(&source)
                .cloned()
                .unwrap_or_default())
        }

        fn set_to(&self, link_id: pkm_core::id::LinkId, new_to: ObjectRef) -> pkm_core::Result<()> {
            self.set_to_calls.borrow_mut().push((link_id, new_to));
            Ok(())
        }

        fn set_from(
            &self,
            link_id: pkm_core::id::LinkId,
            new_from: ObjectRef,
        ) -> pkm_core::Result<()> {
            self.set_from_calls.borrow_mut().push((link_id, new_from));
            Ok(())
        }

        fn delete(&self, _link_id: pkm_core::id::LinkId) -> pkm_core::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn merge_entities_apply_and_rollback() {
        use pkm_core::link::{Link, LinkType};
        use pkm_core::review::ReviewState;

        let action_repo = MockActionRepo::new();
        let link_repo = MockLinkRepo::new();
        let entity_repo = MockEntityRepo::new();

        // Create test entity IDs
        let survivor_id = EntityId::new();
        let loser_id = EntityId::new();

        // Create loser entity
        let now = Timestamp::now_utc();
        let loser_entity = pkm_core::entity::Entity {
            id: loser_id,
            kind: pkm_core::entity::EntityKind::Person,
            name: "Loser Entity".to_string(),
            aliases: vec![],
            semantic_date: None,
            created_by: Actor::User,
            created_at: now,
            merged_into: None,
            version: 1,
            updated_at: now,
        };

        entity_repo.create_entity(loser_entity);

        // Create test links pointing to the loser entity
        let link1 = Link {
            id: pkm_core::id::LinkId::new(),
            from: ObjectRef::Note(NoteId::new()),
            to: ObjectRef::Entity(loser_id),
            link_type: LinkType::RelatedTo,
            created_by: Actor::User,
            created_at: now,
            reviewed: ReviewState::Accepted,
            confidence: None,
            version: 1,
            updated_at: now,
        };

        link_repo.add_link_to(ObjectRef::Entity(loser_id), link1.clone());

        // Create merge action
        let diff = serde_json::json!({
            "before": null,
            "after": {
                "loser_ids": [loser_id.to_string()]
            }
        });

        let merge_action = AgentAction {
            id: AgentActionId::new(),
            actor: Actor::User,
            operation: OperationKind::MergeEntities,
            target: ObjectRef::Entity(survivor_id),
            status: AgentActionStatus::Proposed,
            rationale: "Merging duplicate entities".to_string(),
            created_at: Timestamp::now_utc(),
            diff,
            rollback_of: None,
        };

        action_repo.create(&merge_action).unwrap();

        // Apply the merge
        let applied = apply_action(
            merge_action.id,
            &action_repo,
            &MockNoteRepo,
            Some(&link_repo),
        )
        .unwrap();

        assert_eq!(applied.status, AgentActionStatus::Applied);

        // Update mock entity to reflect the merge
        let _ = entity_repo.set_merged_into(loser_id, survivor_id);

        // Verify link was re-pointed during apply
        let set_to_calls = link_repo.set_to_calls.borrow();
        assert_eq!(set_to_calls.len(), 1);
        assert_eq!(set_to_calls[0].0, link1.id);
        assert_eq!(set_to_calls[0].1, ObjectRef::Entity(survivor_id));

        drop(set_to_calls);

        // Now rollback
        let rollback = rollback_action(
            merge_action.id,
            &action_repo,
            &MockNoteRepo,
            Some(&entity_repo),
            Some(&link_repo),
        )
        .unwrap();

        assert_eq!(rollback.status, AgentActionStatus::Applied);
        assert_eq!(rollback.operation, OperationKind::RollbackAction);

        // Verify loser entity was restored
        assert!(entity_repo.cleared_merged.borrow().contains(&loser_id));
    }

    #[test]
    fn merge_entities_apply_repoints_links() {
        use pkm_core::link::{Link, LinkType};
        use pkm_core::review::ReviewState;

        let action_repo = MockActionRepo::new();
        let link_repo = MockLinkRepo::new();

        // Create test entity IDs
        let survivor_id = EntityId::new();
        let loser_id = EntityId::new();

        // Create some test links pointing to the loser entity
        let now = Timestamp::now_utc();
        let link1 = Link {
            id: pkm_core::id::LinkId::new(),
            from: ObjectRef::Note(NoteId::new()),
            to: ObjectRef::Entity(loser_id),
            link_type: LinkType::RelatedTo,
            created_by: Actor::User,
            created_at: now,
            reviewed: ReviewState::Accepted,
            confidence: None,
            version: 1,
            updated_at: now,
        };

        let link2 = Link {
            id: pkm_core::id::LinkId::new(),
            from: ObjectRef::Entity(loser_id),
            to: ObjectRef::Note(NoteId::new()),
            link_type: LinkType::Mentions,
            created_by: Actor::User,
            created_at: now,
            reviewed: ReviewState::Proposed,
            confidence: Some(0.95),
            version: 1,
            updated_at: now,
        };

        // Add links to the mock repo
        link_repo.add_link_to(ObjectRef::Entity(loser_id), link1.clone());
        link_repo.add_link_from(ObjectRef::Entity(loser_id), link2.clone());

        // Create a merge operation in an action with loser IDs in the diff
        let diff = serde_json::json!({
            "before": null,
            "after": {
                "loser_ids": [loser_id.to_string()]
            }
        });

        let action = AgentAction {
            id: AgentActionId::new(),
            actor: Actor::User,
            operation: OperationKind::MergeEntities,
            target: ObjectRef::Entity(survivor_id),
            status: AgentActionStatus::Proposed,
            rationale: "Merging duplicate entities".to_string(),
            created_at: Timestamp::now_utc(),
            diff,
            rollback_of: None,
        };

        action_repo.create(&action).unwrap();

        // Apply the action, which should re-point the links
        let result = apply_action(action.id, &action_repo, &MockNoteRepo, Some(&link_repo));

        assert!(result.is_ok());
        let applied_action = result.unwrap();
        assert_eq!(applied_action.status, AgentActionStatus::Applied);

        // Verify that set_to was called for link1 (pointing to loser)
        let set_to_calls = link_repo.set_to_calls.borrow();
        assert_eq!(set_to_calls.len(), 1);
        assert_eq!(set_to_calls[0].0, link1.id);
        assert_eq!(set_to_calls[0].1, ObjectRef::Entity(survivor_id));

        // Verify that set_from was called for link2 (from loser)
        let set_from_calls = link_repo.set_from_calls.borrow();
        assert_eq!(set_from_calls.len(), 1);
        assert_eq!(set_from_calls[0].0, link2.id);
        assert_eq!(set_from_calls[0].1, ObjectRef::Entity(survivor_id));
    }

    /// A minimal mock NoteRepo for testing.
    struct MockNoteRepo;

    impl pkm_core::ports::NoteRepo for MockNoteRepo {
        fn create(&self, _note: &pkm_core::note::Note) -> pkm_core::Result<()> {
            Ok(())
        }

        fn get(&self, _id: NoteId) -> pkm_core::Result<Option<pkm_core::note::Note>> {
            Ok(None)
        }

        fn list(&self, _limit: Option<usize>) -> pkm_core::Result<Vec<pkm_core::note::Note>> {
            Ok(vec![])
        }

        fn update(&self, _note: &pkm_core::note::Note) -> pkm_core::Result<()> {
            Ok(())
        }

        fn delete(&self, _id: NoteId) -> pkm_core::Result<()> {
            Ok(())
        }

        fn update_block(
            &self,
            _note_id: NoteId,
            _block_id: BlockId,
            _new_content: pkm_core::block::BlockContent,
        ) -> pkm_core::Result<pkm_core::block::Block> {
            unimplemented!()
        }

        fn get_blocks(&self, _note_id: NoteId) -> pkm_core::Result<Vec<pkm_core::block::Block>> {
            Ok(vec![])
        }

        fn get_note_id_for_block(&self, _block_id: BlockId) -> pkm_core::Result<Option<NoteId>> {
            Ok(None)
        }

        fn create_block(&self, _block: &pkm_core::block::Block) -> pkm_core::Result<()> {
            Ok(())
        }

        fn delete_block(&self, _note_id: NoteId, _block_id: BlockId) -> pkm_core::Result<()> {
            Ok(())
        }
    }

    /// A simple in-memory mock EntityRepo for testing.
    struct MockEntityRepo {
        entities: RefCell<HashMap<EntityId, pkm_core::entity::Entity>>,
        merged_into: RefCell<HashMap<EntityId, EntityId>>,
        cleared_merged: RefCell<Vec<EntityId>>,
    }

    impl MockEntityRepo {
        fn new() -> Self {
            Self {
                entities: RefCell::new(HashMap::new()),
                merged_into: RefCell::new(HashMap::new()),
                cleared_merged: RefCell::new(Vec::new()),
            }
        }

        fn create_entity(&self, entity: pkm_core::entity::Entity) {
            self.entities.borrow_mut().insert(entity.id, entity);
        }
    }

    impl pkm_core::ports::EntityRepo for MockEntityRepo {
        fn create(&self, entity: &pkm_core::entity::Entity) -> pkm_core::Result<()> {
            self.entities.borrow_mut().insert(entity.id, entity.clone());
            Ok(())
        }

        fn get(&self, id: EntityId) -> pkm_core::Result<Option<pkm_core::entity::Entity>> {
            Ok(self.entities.borrow().get(&id).cloned())
        }

        fn set_merged_into(
            &self,
            loser_id: EntityId,
            survivor_id: EntityId,
        ) -> pkm_core::Result<()> {
            self.merged_into.borrow_mut().insert(loser_id, survivor_id);
            Ok(())
        }

        fn clear_merged_into(&self, entity_id: EntityId) -> pkm_core::Result<()> {
            self.cleared_merged.borrow_mut().push(entity_id);
            self.merged_into.borrow_mut().remove(&entity_id);
            Ok(())
        }
    }
}
