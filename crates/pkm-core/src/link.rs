//! Typed relationships between objects.
//!
//! AGENTS.md is explicit: do NOT model all relationships as generic backlinks.
//! `LinkType` is a product invariant. Adding a variant is a product decision
//! (write an ADR); removing one breaks stored data and needs a migration.

use serde::{Deserialize, Serialize};

use crate::id::{LinkId, ObjectRef};

/// The minimum supported set of typed relationships (AGENTS.md "Link").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    RelatedTo,
    Cites,
    Supports,
    Contradicts,
    Summarizes,
    DerivedFrom,
    Mentions,
    PartOf,
    DependsOn,
    DecidedIn,
    AssignedTo,
    FollowsUp,
}

/// A directed, typed edge from `from` to `to`.
///
/// Links carry provenance so the system can distinguish inferred suggestions
/// from user-confirmed knowledge. Keep `from`, `to`, `link_type` stable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub id: LinkId,
    pub from: ObjectRef,
    pub to: ObjectRef,
    pub link_type: LinkType,
    pub created_by: crate::Actor,
    pub created_at: crate::Timestamp,
    pub reviewed: crate::review::ReviewState,
    /// Optional confidence score (0.0-1.0) for inferred links.
    pub confidence: Option<f32>,
    /// Current version number (increments on each update).
    pub version: u32,
    /// When this version was created.
    pub updated_at: crate::Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::NoteId;
    use crate::review::ReviewState;
    use crate::{Actor, Timestamp};

    #[test]
    fn link_type_round_trips_as_snake_case() {
        let json = serde_json::to_string(&LinkType::DerivedFrom).unwrap();
        assert_eq!(json, "\"derived_from\"");
        let back: LinkType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, LinkType::DerivedFrom);
    }

    #[test]
    fn link_round_trips() {
        let now = Timestamp::now_utc();
        let link = Link {
            id: LinkId::new(),
            from: ObjectRef::Note(NoteId::new()),
            to: ObjectRef::Note(NoteId::new()),
            link_type: LinkType::RelatedTo,
            created_by: Actor::User,
            created_at: now,
            reviewed: ReviewState::Proposed,
            confidence: Some(0.85),
            version: 1,
            updated_at: now,
        };

        let json = serde_json::to_string(&link).expect("serialize");
        let back: Link = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back.id, link.id);
        assert_eq!(back.from, link.from);
        assert_eq!(back.to, link.to);
        assert_eq!(back.link_type, link.link_type);
        assert_eq!(back.created_by, link.created_by);
        assert_eq!(back.reviewed, link.reviewed);
        assert_eq!(back.confidence, link.confidence);
        assert_eq!(back.version, link.version);
    }
}
