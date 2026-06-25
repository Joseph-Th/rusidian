//! pkm-agent: the typed, audited operation layer (AGENTS.md Phase 3 +
//! "Agent-Native API Principles").
//!
//! This is the safety boundary. Rules it enforces:
//! - Agents act ONLY through typed operations ([`pkm_core::agent_action::
//!   OperationKind`]); never by rewriting arbitrary files/blobs.
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

use pkm_core::agent_action::{AgentAction, OperationKind};
use pkm_core::Actor;

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

/// A request to perform a typed operation. The payload is operation-specific;
/// its typed schema per `OperationKind` is task D1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRequest {
    pub actor: Actor,
    pub operation: OperationKind,
    pub rationale: String,
    /// Operation-specific arguments. Task D1 replaces this with a typed enum.
    pub payload: serde_json::Value,
}

/// Whether an operation may be applied directly or must be proposed for review.
/// STUB — task D1 defines the real risk classification. Default to "propose".
pub fn requires_review(_op: OperationKind) -> bool {
    // TODO(D1): mechanical ops (indexing, derived caches) => false; everything
    //           that touches user-facing knowledge => true.
    true
}

/// Execute (or propose) an operation, producing an auditable `AgentAction`.
/// STUB — task D1/D2. Must: validate, compute before/after diff, persist the
/// action via the AgentActionRepo port, and only apply when `!requires_review`.
/// Real signature will take the relevant `&dyn ...Repo` port(s) as arguments.
pub fn execute(_req: OperationRequest) -> Result<AgentAction> {
    // TODO(D1): dispatch on req.operation -> typed handler -> AgentAction.
    unimplemented!("operation dispatch — STATUS.md task D1")
}
