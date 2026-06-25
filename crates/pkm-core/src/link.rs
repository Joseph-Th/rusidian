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
/// STUB: fields are the agreed minimum. Lesser agents may extend with
/// provenance/confidence per STATUS.md task C3 — but keep `from`, `to`,
/// `link_type` stable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub id: LinkId,
    pub from: ObjectRef,
    pub to: ObjectRef,
    pub link_type: LinkType,
    // TODO(C3): created_by: crate::Actor, created_at: crate::Timestamp,
    //           reviewed: review::ReviewState, confidence: Option<f32>.
}

#[cfg(test)]
mod tests {
    use super::*;

    // Smoke test that the invariant enum survives a serde round-trip with the
    // snake_case wire format the storage layer relies on. Extend per task A1.
    #[test]
    fn link_type_round_trips_as_snake_case() {
        let json = serde_json::to_string(&LinkType::DerivedFrom).unwrap();
        assert_eq!(json, "\"derived_from\"");
        let back: LinkType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, LinkType::DerivedFrom);
    }
}
