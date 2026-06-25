//! Entity: a normalized thing the system recognizes and links (AGENTS.md
//! "Entity"). Entities must support aliases and merge operations.

use serde::{Deserialize, Serialize};

use crate::id::EntityId;
use crate::{Actor, Timestamp};

/// Entity classification. Product invariant set — extend via ADR only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Person,
    Organization,
    Project,
    Topic,
    Location,
    Event,
    Book,
    Paper,
    Product,
    Claim,
    Decision,
}

/// A normalized object the system can recognize, link, and retrieve.
///
/// Entities support non-lossy merges: when merging A into B, A is marked
/// merged-into B (preserving history), all links are re-pointed, and all
/// aliases are preserved. Merges are reversible via rollback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    pub name: String,
    pub aliases: Vec<String>,
    /// If Some(entity_id), this entity was merged into that entity. Used to
    /// preserve history and enable rollback. The survivor keeps this as None.
    pub merged_into: Option<EntityId>,
    /// Who created this entity (user or agent).
    pub created_by: Actor,
    /// When this entity was created.
    pub created_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_kind_round_trips_as_snake_case() {
        for kind in [
            EntityKind::Person,
            EntityKind::Organization,
            EntityKind::Project,
            EntityKind::Topic,
            EntityKind::Location,
            EntityKind::Event,
            EntityKind::Book,
            EntityKind::Paper,
            EntityKind::Product,
            EntityKind::Claim,
            EntityKind::Decision,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: EntityKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, kind);
        }
    }
}
