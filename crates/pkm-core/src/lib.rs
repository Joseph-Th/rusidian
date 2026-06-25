//! pkm-core: the domain model (crate `pkm_core`).
//!
//! Named `pkm-core`, NOT `core`, to avoid confusion with Rust's std `core`.
//!
//! This crate is the single source of truth for the product's primitives.
//! Read AGENTS.md before changing anything here. The enums in this crate
//! (link types, ingestion states, agent-action statuses, content status,
//! review state, actor kinds) are product invariants.
//!
//! INVARIANT-ENUM POLICY: these enums are *authoritative*, not frozen. They are
//! closed to casual edits. A variant may be added/changed/removed only via an
//! ADR (`docs/adr/`). If the enum is persisted (stored in the db or in exported
//! JSON), the change ALSO requires a migration. Never add a stringly-typed
//! escape-hatch field to dodge editing an enum.
//!
//! Rules for this crate (enforced by review, see STATUS.md):
//! - No IO, no database, no UI. Pure types + pure logic only.
//! - No internal crate dependencies. Everything else depends on this.
//! - Every durable object carries a stable typed id (see [`id`]).
//! - Cross-cutting concepts (provenance, review, content status, actor) live
//!   HERE, so storage/search/agent/ingestion all share one definition.

pub mod id;
pub mod error;

pub mod source;
pub mod note;
pub mod block;
pub mod entity;
pub mod link;
pub mod view;
pub mod agent_action;

pub mod provenance;
pub mod review;
pub mod ports;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

pub use error::{CoreError, Result};

/// A point in time, UTC. Used for all `created_at` / `updated_at` fields so
/// the whole model shares one timestamp type.
pub type Timestamp = time::OffsetDateTime;

/// Who performed an action. Provenance and audit depend on this being explicit
/// everywhere a mutation can originate. See AGENTS.md "Agent Action".
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Actor {
    /// The human using the app.
    User,
    /// A named LLM agent (model/tool identifier goes in `name`).
    Agent { name: String },
    /// The system itself (migrations, indexers, derived-cache updates).
    System,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_round_trips_as_snake_case() {
        let user = Actor::User;
        let json = serde_json::to_string(&user).unwrap();
        let back: Actor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, user);

        let system = Actor::System;
        let json = serde_json::to_string(&system).unwrap();
        let back: Actor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, system);

        let agent = Actor::Agent {
            name: "claude-opus".to_string(),
        };
        let json = serde_json::to_string(&agent).unwrap();
        let back: Actor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, agent);
    }
}
