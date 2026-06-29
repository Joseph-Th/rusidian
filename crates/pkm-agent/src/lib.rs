use serde::{Deserialize, Serialize};

use pkm_core::agent_action::{ActionDiff, AgentAction, AgentActionStatus, OperationKind};
use pkm_core::block::{Block, BlockContent};
use pkm_core::id::{AgentActionId, BlockId, EntityId, NoteId, ObjectRef, SourceId, ViewId};
use pkm_core::link::LinkType;
use pkm_core::note::Note;
use pkm_core::ports::{AgentActionRepo, EntityRepo, LinkRepo, NoteRepo};
use pkm_core::view::ViewParams;
use pkm_core::{Actor, Timestamp};

pub type Result<T> = std::result::Result<T, AgentError>;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("operation rejected: {0}")]
    Rejected(String),
    #[error(transparent)]
    Core(#[from] pkm_core::CoreError),
}

/// A typed operation request with all necessary parameters.
/// Each variant contains exactly the data needed to execute the operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Operation {
    /// Capture a new raw source.
    CreateSource {
        source_id: SourceId,
        source_type: String,
        title: String,
    },
    /// Extract structure from an existing source.
    ParseSource { source_id: SourceId },

    /// Create a new note with initial content.
    CreateNote { note_id: NoteId, title: String },
    /// Attach an existing source to a note.
    AttachSourceToNote {
        source_id: SourceId,
        note_id: NoteId,
    },

    /// Create a block inside a note.
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
    /// Merge multiple entities.
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

    /// Mark an object as reviewed by the user.
    MarkReviewed {
        target_ref: ObjectRef,
        reviewed: bool,
    },

    /// Create a new presentation view with strongly-typed params.
    CreateView {
        view_id: ViewId,
        view_kind: pkm_core::view::ViewKind,
        title: String,
        params: ViewParams,
    },
    /// Update an existing view's parameters.
    UpdateView {
        view_id: ViewId,
        params: ViewParams,
    },

    /// Revert a previous agent action.
    RollbackAction { action_id: AgentActionId },
}

impl Operation {
    /// Map this operation to its OperationKind.
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
            Operation::RollbackAction { action_id } => ObjectRef::AgentAction(*action_id),
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
pub fn requires_review(_op: &Operation) -> bool {
    false
}

/// Build the typed diff for an operation.
fn build_diff(op: &Operation, note_repo: &dyn NoteRepo) -> Result<ActionDiff> {
    match op {
        Operation::CreateNote { note_id, title } => Ok(ActionDiff::CreateNote {
            note_id: *note_id,
            title: title.clone(),
        }),
        Operation::CreateBlock { note_id, block_id, content } => {
            Ok(ActionDiff::CreateBlock {
                note_id: *note_id,
                block_id: *block_id,
                content: content.clone(),
            })
        }
        Operation::UpdateBlock { note_id, block_id, new_content } => {
            let blocks = note_repo.get_blocks(*note_id)?;
            let old_block = blocks.iter().find(|b| b.id == *block_id)
                .ok_or_else(|| pkm_core::CoreError::Invariant(format!("Block {} not found", block_id)))?;
            let mut new_block = old_block.clone();
            new_block.content = new_content.clone();
            Ok(ActionDiff::UpdateBlock {
                before: Box::new(old_block.clone()),
                after: Box::new(new_block),
            })
        }
        Operation::MoveBlock { note_id, block_id, new_index } => {
            let note = note_repo.get(*note_id)?
                .ok_or_else(|| pkm_core::CoreError::Invariant(format!("Note {} not found", note_id)))?;
            Ok(ActionDiff::MoveBlock {
                before: note.blocks.clone(),
                note_id: *note_id,
                block_id: *block_id,
                new_index: *new_index,
            })
        }
        Operation::CreateEntity { entity_id, name, .. } => {
            Ok(ActionDiff::CreateEntity {
                entity_id: *entity_id,
                name: name.clone(),
            })
        }
        Operation::MergeEntities { survivor_id, loser_ids } => {
            Ok(ActionDiff::MergeEntities {
                survivor_id: *survivor_id,
                loser_ids: loser_ids.clone(),
                repointed_links: std::collections::HashMap::new(),
            })
        }
        Operation::CreateSource { source_id, title, .. } => Ok(ActionDiff::CreateSource {
            source_id: *source_id,
            title: title.clone(),
        }),
        Operation::CreateTypedLink { from, to, link_type } => {
            Ok(ActionDiff::CreateTypedLink {
                from: *from,
                to: *to,
                link_type: *link_type,
                created_link_id: pkm_core::id::LinkId::new(),
            })
        }
        Operation::CreateView { view_id, view_kind, title, params } => {
            Ok(ActionDiff::CreateView {
                view_id: *view_id,
                view_kind: *view_kind,
                title: title.clone(),
                params: Box::new(params.clone()),
            })
        }
        Operation::UpdateView { view_id, params } => {
            Ok(ActionDiff::UpdateView {
                view_id: *view_id,
                params: Box::new(params.clone()),
            })
        }
        _ => Ok(ActionDiff::NoChange),
    }
}

/// Execute a typed operation, producing and persisting an auditable `AgentAction`.
pub fn execute(
    req: OperationRequest,
    action_repo: &dyn AgentActionRepo,
    note_repo: &dyn NoteRepo,
) -> Result<AgentAction> {
    let now = Timestamp::now_utc();
    let status = if requires_review(&req.operation) {
        AgentActionStatus::Proposed
    } else {
        AgentActionStatus::Applied
    };

    let diff = build_diff(&req.operation, note_repo)?;

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

    let diff = action.diff.clone();
    let mut updated_diff = diff.clone();

    match &action.operation {
        OperationKind::UpdateBlock => {
            if let ActionDiff::UpdateBlock { after, .. } = &diff {
                note_repo.update_block(after.note_id, after.id, after.content.clone())?;
            }
        }
        OperationKind::MergeEntities => {
            let link_repo = link_repo.ok_or_else(|| AgentError::Rejected("LinkRepo required".into()))?;
            let mut repointed_links = std::collections::HashMap::new();

            if let ActionDiff::MergeEntities { survivor_id, loser_ids, repointed_links: _ } = &diff {
                for loser_id in loser_ids {
                    for link in link_repo.get_by_to(ObjectRef::Entity(*loser_id))? {
                        link_repo.set_to(link.id, ObjectRef::Entity(*survivor_id))?;
                        repointed_links.insert(link.id.to_string(), loser_id.to_string());
                    }
                    for link in link_repo.get_by_from(ObjectRef::Entity(*loser_id))? {
                        link_repo.set_from(link.id, ObjectRef::Entity(*survivor_id))?;
                        repointed_links.insert(link.id.to_string(), loser_id.to_string());
                    }
                }
                updated_diff = ActionDiff::MergeEntities {
                    survivor_id: *survivor_id,
                    loser_ids: loser_ids.clone(),
                    repointed_links,
                };
            }
        }
        OperationKind::CreateNote => {
            if let ActionDiff::CreateNote { note_id, title } = &diff {
                let now = Timestamp::now_utc();
                let note = Note {
                    id: *note_id,
                    title: title.clone(),
                    blocks: vec![],
                    metadata: pkm_core::note::NoteMetadata::default(),
                    created_by: action.actor.clone(),
                    created_at: now,
                    version: 1,
                    updated_at: now,
                };
                note_repo.create(&note)?;
            }
        }
        OperationKind::CreateTypedLink => {
            let link_repo = link_repo.ok_or_else(|| AgentError::Rejected("LinkRepo required".into()))?;

            if let ActionDiff::CreateTypedLink { from, to, link_type, created_link_id } = &diff {
                let now = Timestamp::now_utc();
                let link = pkm_core::link::Link {
                    id: *created_link_id,
                    from: *from,
                    to: *to,
                    link_type: *link_type,
                    created_by: action.actor.clone(),
                    created_at: now,
                    reviewed: pkm_core::review::ReviewState::Accepted,
                    confidence: None,
                    version: 1,
                    updated_at: now,
                };
                updated_diff = ActionDiff::CreateTypedLink {
                    from: *from,
                    to: *to,
                    link_type: *link_type,
                    created_link_id: link.id,
                };
                link_repo.create(&link)?;
            }
        }
        OperationKind::CreateBlock => {
            if let ActionDiff::CreateBlock { note_id, block_id, content } = &diff {
                let now = Timestamp::now_utc();
                let block = Block {
                    id: *block_id,
                    note_id: *note_id,
                    content: content.clone(),
                    created_by: action.actor.clone(),
                    created_at: now,
                    source_provenance_ref: None,
                    version: 1,
                    updated_at: now,
                };
                note_repo.create_block(&block)?;
            }
        }
        OperationKind::MoveBlock => {
            if let ActionDiff::MoveBlock { note_id, block_id, new_index, .. } = &diff {
                let mut note = note_repo.get(*note_id)?
                    .ok_or_else(|| AgentError::Rejected("Note not found".into()))?;
                let old_pos = note.blocks.iter().position(|b| *b == *block_id)
                    .ok_or_else(|| AgentError::Rejected("Block not found in note".into()))?;
                note.blocks.remove(old_pos);
                let insert_index = if *new_index > note.blocks.len() { note.blocks.len() } else { *new_index };
                note.blocks.insert(insert_index, *block_id);
                note_repo.update(&note)?;
            }
        }
        _ => return Err(AgentError::Rejected("Operation not supported for apply".into())),
    }

    action_repo.set_diff(action.id, updated_diff)?;
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

    let diff = action.diff.clone();

    match &diff {
        ActionDiff::UpdateBlock { before, .. } => {
            note_repo.update_block(before.note_id, before.id, before.content.clone())?;
        }
        ActionDiff::MergeEntities { survivor_id, loser_ids, repointed_links } => {
            let entity_repo = entity_repo.ok_or_else(|| AgentError::Rejected("EntityRepo required".into()))?;
            let link_repo = link_repo.ok_or_else(|| AgentError::Rejected("LinkRepo required".into()))?;

            for loser_id in loser_ids {
                entity_repo.clear_merged_into(*loser_id)?;
            }

            for (link_id_str, loser_id_str) in repointed_links {
                let link_id = pkm_core::id::LinkId(uuid::Uuid::parse_str(link_id_str).unwrap_or_default());
                let loser_id = EntityId(uuid::Uuid::parse_str(loser_id_str).unwrap_or_default());
                if let Ok(Some(link)) = link_repo.get(link_id) {
                    if link.to == ObjectRef::Entity(*survivor_id) {
                        let _ = link_repo.set_to(link_id, ObjectRef::Entity(loser_id));
                    }
                    if link.from == ObjectRef::Entity(*survivor_id) {
                        let _ = link_repo.set_from(link_id, ObjectRef::Entity(loser_id));
                    }
                }
            }
        }
        ActionDiff::CreateNote { note_id, .. } => {
            note_repo.delete(*note_id)?;
        }
        ActionDiff::CreateTypedLink { created_link_id, .. } => {
            let link_repo = link_repo.ok_or_else(|| AgentError::Rejected("LinkRepo required".into()))?;
            link_repo.delete(*created_link_id)?;
        }
        ActionDiff::CreateBlock { note_id, block_id, .. } => {
            note_repo.delete_block(*note_id, *block_id)?;
        }
        ActionDiff::MoveBlock { before, note_id, .. } => {
            let mut note = note_repo.get(*note_id)?
                .ok_or_else(|| AgentError::Rejected("Note not found".into()))?;
            note.blocks = before.clone();
            note_repo.update(&note)?;
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
        diff: ActionDiff::NoChange,
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

    impl AgentActionRepo for MockActionRepo {
        fn create(&self, action: &AgentAction) -> pkm_core::Result<()> {
            self.actions.borrow_mut().insert(action.id, action.clone());
            Ok(())
        }

        fn get(&self, id: AgentActionId) -> pkm_core::Result<Option<AgentAction>> {
            Ok(self.actions.borrow().get(&id).cloned())
        }

        fn set_status(&self, id: AgentActionId, new_status: pkm_core::agent_action::AgentActionStatus) -> pkm_core::Result<()> {
            if let Some(action) = self.actions.borrow_mut().get_mut(&id) {
                action.status = new_status;
            }
            Ok(())
        }

        fn set_diff(&self, id: AgentActionId, diff: ActionDiff) -> pkm_core::Result<()> {
            if let Some(action) = self.actions.borrow_mut().get_mut(&id) {
                action.diff = diff;
            }
            Ok(())
        }
    }

    #[test]
    fn all_ops_apply_instantly_in_automation_mode() {
        let parse_source = Operation::ParseSource { source_id: SourceId::new() };
        assert!(!requires_review(&parse_source));

        let generate_summary = Operation::GenerateSummary {
            target_ref: ObjectRef::Source(SourceId::new()),
            summary_text: "A summary".to_string(),
        };
        assert!(!requires_review(&generate_summary));

        let create_source = Operation::CreateSource {
            source_id: SourceId::new(),
            source_type: "article".to_string(),
            title: "Test".to_string(),
        };
        assert!(!requires_review(&create_source));
    }

    #[test]
    fn execute_creates_applied_action_for_all_ops() {
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

        let retrieved = repo.get(action.id).unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn operation_round_trips() {
        use pkm_core::view::ViewParams;

        let ops = vec![
            Operation::CreateSource {
                source_id: SourceId::new(),
                source_type: "pdf".to_string(),
                title: "Document".to_string(),
            },
            Operation::ParseSource { source_id: SourceId::new() },
            Operation::CreateNote {
                note_id: NoteId::new(),
                title: "My Note".to_string(),
            },
            Operation::CreateBlock {
                note_id: NoteId::new(),
                block_id: BlockId::new(),
                content: BlockContent::Markdown { text: "Hello".to_string() },
            },
            Operation::UpdateBlock {
                note_id: NoteId::new(),
                block_id: BlockId::new(),
                new_content: BlockContent::Markdown { text: "Updated".to_string() },
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
            Operation::CreateView {
                view_id: ViewId::new(),
                view_kind: pkm_core::view::ViewKind::ReadingQueue,
                title: "My View".to_string(),
                params: ViewParams::reading_queue(),
            },
        ];

        for op in ops.iter() {
            let json = serde_json::to_string(op).unwrap();
            let back: Operation = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, op);
        }
    }

    struct MockNoteRepo;

    impl NoteRepo for MockNoteRepo {
        fn create(&self, _note: &Note) -> pkm_core::Result<()> { Ok(()) }
        fn get(&self, _id: NoteId) -> pkm_core::Result<Option<Note>> { Ok(None) }
        fn list(&self, _limit: Option<usize>) -> pkm_core::Result<Vec<Note>> { Ok(vec![]) }
        fn update(&self, _note: &Note) -> pkm_core::Result<()> { Ok(()) }
        fn delete(&self, _id: NoteId) -> pkm_core::Result<()> { Ok(()) }
        fn update_block(&self, _note_id: NoteId, _block_id: BlockId, _new_content: BlockContent) -> pkm_core::Result<Block> { unimplemented!() }
        fn get_blocks(&self, _note_id: NoteId) -> pkm_core::Result<Vec<Block>> { Ok(vec![]) }
        fn get_note_id_for_block(&self, _block_id: BlockId) -> pkm_core::Result<Option<NoteId>> { Ok(None) }
        fn create_block(&self, _block: &Block) -> pkm_core::Result<()> { Ok(()) }
        fn delete_block(&self, _note_id: NoteId, _block_id: BlockId) -> pkm_core::Result<()> { Ok(()) }
    }

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
            self.links_to.borrow_mut().entry(target).or_default().push(link);
        }

        fn add_link_from(&self, source: ObjectRef, link: pkm_core::link::Link) {
            self.links_from.borrow_mut().entry(source).or_default().push(link);
        }
    }

    impl LinkRepo for MockLinkRepo {
        fn create(&self, _link: &pkm_core::link::Link) -> pkm_core::Result<()> { Ok(()) }
        fn get(&self, _link_id: pkm_core::id::LinkId) -> pkm_core::Result<Option<pkm_core::link::Link>> { Ok(None) }
        fn get_by_to(&self, target: ObjectRef) -> pkm_core::Result<Vec<pkm_core::link::Link>> {
            Ok(self.links_to.borrow().get(&target).cloned().unwrap_or_default())
        }
        fn get_by_from(&self, source: ObjectRef) -> pkm_core::Result<Vec<pkm_core::link::Link>> {
            Ok(self.links_from.borrow().get(&source).cloned().unwrap_or_default())
        }
        fn set_to(&self, link_id: pkm_core::id::LinkId, new_to: ObjectRef) -> pkm_core::Result<()> {
            self.set_to_calls.borrow_mut().push((link_id, new_to));
            Ok(())
        }
        fn set_from(&self, link_id: pkm_core::id::LinkId, new_from: ObjectRef) -> pkm_core::Result<()> {
            self.set_from_calls.borrow_mut().push((link_id, new_from));
            Ok(())
        }
        fn delete(&self, _link_id: pkm_core::id::LinkId) -> pkm_core::Result<()> { Ok(()) }
    }

    #[test]
    fn merge_entities_apply_repoints_links() {
        use pkm_core::link::{Link, LinkType};
        use pkm_core::review::ReviewState;

        let action_repo = MockActionRepo::new();
        let link_repo = MockLinkRepo::new();
        let entity_repo = MockEntityRepo::new();

        let survivor_id = EntityId::new();
        let loser_id = EntityId::new();
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

        let diff = ActionDiff::MergeEntities {
            survivor_id,
            loser_ids: vec![loser_id],
            repointed_links: HashMap::new(),
        };

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

        let applied = apply_action(merge_action.id, &action_repo, &MockNoteRepo, Some(&link_repo)).unwrap();
        assert_eq!(applied.status, AgentActionStatus::Applied);

        let _ = entity_repo.set_merged_into(loser_id, survivor_id);

        let set_to_calls = link_repo.set_to_calls.borrow();
        assert_eq!(set_to_calls.len(), 1);
        assert_eq!(set_to_calls[0].0, link1.id);
        assert_eq!(set_to_calls[0].1, ObjectRef::Entity(survivor_id));
        drop(set_to_calls);

        // Rollback
        let rollback = rollback_action(
            merge_action.id,
            &action_repo,
            &MockNoteRepo,
            Some(&entity_repo),
            Some(&link_repo),
        ).unwrap();

        assert_eq!(rollback.status, AgentActionStatus::Applied);
        assert_eq!(rollback.operation, OperationKind::RollbackAction);
        assert!(entity_repo.cleared_merged.borrow().contains(&loser_id));
    }

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

    impl EntityRepo for MockEntityRepo {
        fn create(&self, entity: &pkm_core::entity::Entity) -> pkm_core::Result<()> {
            self.entities.borrow_mut().insert(entity.id, entity.clone());
            Ok(())
        }
        fn get(&self, id: EntityId) -> pkm_core::Result<Option<pkm_core::entity::Entity>> {
            Ok(self.entities.borrow().get(&id).cloned())
        }
        fn set_merged_into(&self, loser_id: EntityId, survivor_id: EntityId) -> pkm_core::Result<()> {
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
