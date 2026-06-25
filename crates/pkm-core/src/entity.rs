//! Entity: a normalized thing the system recognizes and links (AGENTS.md
//! "Entity"). Entities must support aliases and merge operations.

use serde::{Deserialize, Serialize};

use crate::id::EntityId;

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

/// STUB. Merge semantics (which id survives, how aliases/links re-point) is a
/// dedicated task — see STATUS.md C4. Do not implement a lossy merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    pub name: String,
    pub aliases: Vec<String>,
    // TODO(C4): canonical/merged-into ref, created_by, created_at.
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
