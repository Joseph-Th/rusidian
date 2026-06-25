//! SQLite persistence for the agent action log.
//!
//! The action log is APPEND-ONLY: actions are never updated in place except to
//! advance their status (proposed -> accepted/applied/reverted/...). This table
//! is the audit trail behind every agent change (AGENTS.md "Versioning and
//! Audit Rules"). The `AgentActionRepo` port is added to core in task B2.

use rusqlite::Connection;

/// Agent-action persistence backed by SQLite. STUB — task B2/D2.
pub struct SqliteAgentActionRepo<'c> {
    pub conn: &'c Connection,
}

// TODO(B2): define AgentActionRepo in pkm_core::ports and implement it here
//           (append, get, list_by_target, set_status). Status transitions only;
//           never hard-delete an action.
