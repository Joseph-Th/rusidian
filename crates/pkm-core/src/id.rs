//! Stable, typed identifiers.
//!
//! Every durable object has its own id type so you cannot accidentally pass a
//! `NoteId` where a `SourceId` is expected. Ids are UUIDv7 (time-sortable) and
//! serialize as plain strings for export/JSON compatibility.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! typed_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        // `new()` mints a random id; a `Default` impl would wrongly imply a
        // deterministic zero value, so we opt out deliberately.
        #[allow(clippy::new_without_default)]
        impl $name {
            /// Generate a fresh time-sortable id.
            ///
            /// NOTE: uses `Uuid::now_v7()`, which reads the wall clock. Keep id
            /// generation at the edges (storage/agent layers), not inside pure
            /// transform functions, so those stay deterministic and testable.
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

typed_id!(
    /// Identifies a [`crate::source::Source`].
    SourceId
);
typed_id!(
    /// Identifies a [`crate::note::Note`].
    NoteId
);
typed_id!(
    /// Identifies a [`crate::block::Block`].
    BlockId
);
typed_id!(
    /// Identifies an [`crate::entity::Entity`].
    EntityId
);
typed_id!(
    /// Identifies a [`crate::link::Link`].
    LinkId
);
typed_id!(
    /// Identifies a [`crate::view::View`].
    ViewId
);
typed_id!(
    /// Identifies an [`crate::agent_action::AgentAction`].
    AgentActionId
);

/// A tagged reference to any durable object. Used by links and agent actions,
/// which must be able to point at a target of any type without losing which
/// type it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum ObjectRef {
    Source(SourceId),
    Note(NoteId),
    Block(BlockId),
    Entity(EntityId),
    Link(LinkId),
    View(ViewId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_ref_round_trips() {
        for obj_ref in [
            ObjectRef::Source(SourceId::new()),
            ObjectRef::Note(NoteId::new()),
            ObjectRef::Block(BlockId::new()),
            ObjectRef::Entity(EntityId::new()),
            ObjectRef::Link(LinkId::new()),
            ObjectRef::View(ViewId::new()),
        ] {
            let json = serde_json::to_string(&obj_ref).unwrap();
            let back: ObjectRef = serde_json::from_str(&json).unwrap();
            assert_eq!(back, obj_ref);
        }
    }
}
