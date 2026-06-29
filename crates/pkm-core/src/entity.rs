//! Entity: a normalized thing the system recognizes and links (AGENTS.md
//! "Entity"). Entities must support aliases and merge operations.

use serde::{Deserialize, Serialize};

use crate::id::{EntityId, LinkId, ObjectRef};
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

/// Snapshot of a link's original from/to targets before an entity merge
/// mutation, enabling rollback via clear_merged_into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkBackup {
    pub link_id: LinkId,
    pub original_from: ObjectRef,
    pub original_to: ObjectRef,
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
    /// Backup of original link targets before merge mutation, enabling rollback
    /// via clear_merged_into without data loss.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub merged_link_backups: Vec<LinkBackup>,
    /// Who created this entity (user or agent).
    pub created_by: Actor,
    /// When this entity was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: Timestamp,
    /// Semantic date (when the entity/event actually occurred), RFC3339 format.
    /// Used for timeline visualizations.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub semantic_date: Option<Timestamp>,
    /// Current version number (increments on each update).
    pub version: u32,
    /// When this version was created.
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: Timestamp,
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

    #[test]
    fn entity_round_trips() {
        let now = crate::Timestamp::now_utc();
        let entity = Entity {
            id: EntityId::new(),
            kind: EntityKind::Person,
            name: "John Doe".to_string(),
            aliases: vec!["J. Doe".to_string()],
            merged_into: None,
            merged_link_backups: vec![],
            created_by: Actor::User,
            created_at: now,
            semantic_date: None,
            version: 1,
            updated_at: now,
        };

        let json = serde_json::to_string(&entity).unwrap();
        let back: Entity = serde_json::from_str(&json).unwrap();

        assert_eq!(back.id, entity.id);
        assert_eq!(back.name, entity.name);
        assert_eq!(back.version, entity.version);
    }
}
