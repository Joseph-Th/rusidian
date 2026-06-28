//! SQLite persistence for the agent action log.
//!
//! The action log is APPEND-ONLY: actions are never updated in place except to
//! advance their status (proposed -> accepted/applied/reverted/...). This table
//! is the audit trail behind every agent change (AGENTS.md "Versioning and
//! Audit Rules").

use rusqlite::{params, Connection};

use pkm_core::agent_action::AgentAction;
use pkm_core::id::AgentActionId;
use pkm_core::ports::AgentActionRepo;
use pkm_core::Result;

/// Agent-action persistence backed by SQLite. Implements the append-only audit log.
pub struct SqliteAgentActionRepo<'c> {
    pub conn: &'c Connection,
}

impl<'c> AgentActionRepo for SqliteAgentActionRepo<'c> {
    fn create(&self, action: &AgentAction) -> Result<()> {
        // Serialize the diff as JSON.
        let diff_json = serde_json::to_string(&action.diff)?;

        // Serialize the actor as JSON.
        let actor_json = serde_json::to_string(&action.actor)?;

        // Serialize the operation as JSON.
        let operation_json = serde_json::to_string(&action.operation)?;

        // Serialize the status as JSON (snake_case).
        let status_json = serde_json::to_string(&action.status)?;

        // Extract target type and id from ObjectRef.
        let (target_type, target_id) = match action.target {
            pkm_core::id::ObjectRef::Source(id) => ("source", id.to_string()),
            pkm_core::id::ObjectRef::Note(id) => ("note", id.to_string()),
            pkm_core::id::ObjectRef::Block(id) => ("block", id.to_string()),
            pkm_core::id::ObjectRef::Entity(id) => ("entity", id.to_string()),
            pkm_core::id::ObjectRef::Link(id) => ("link", id.to_string()),
            pkm_core::id::ObjectRef::View(id) => ("view", id.to_string()),
            pkm_core::id::ObjectRef::AgentAction(id) => ("agent_action", id.to_string()),
        };

        // Store created_at as RFC3339 string.
        let created_at_str = action
            .created_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());

        // Store rollback_of as UUID string if present.
        let rollback_of_str = action.rollback_of.map(|id| id.to_string());

        self.conn
            .execute(
                "INSERT INTO agent_action
                (id, actor, operation, target_type, target_id, status, rationale, created_at, diff, rollback_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    action.id.to_string(),
                    actor_json,
                    operation_json,
                    target_type,
                    target_id,
                    status_json,
                    action.rationale,
                    created_at_str,
                    diff_json,
                    rollback_of_str,
                ],
            )
            .map_err(crate::StorageError::from)?;

        Ok(())
    }

    fn get(&self, id: AgentActionId) -> Result<Option<AgentAction>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT actor, operation, target_type, target_id, status, rationale, created_at, diff, rollback_of
             FROM agent_action WHERE id = ?1",
            )
            .map_err(crate::StorageError::from)?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            let actor_json: String = row.get(0)?;
            let operation_json: String = row.get(1)?;
            let target_type: String = row.get(2)?;
            let target_id: String = row.get(3)?;
            let status_json: String = row.get(4)?;
            let rationale: String = row.get(5)?;
            let created_at_str: String = row.get(6)?;
            let diff_json: String = row.get(7)?;
            let rollback_of_str: Option<String> = row.get(8)?;

            Ok((
                actor_json,
                operation_json,
                target_type,
                target_id,
                status_json,
                rationale,
                created_at_str,
                diff_json,
                rollback_of_str,
            ))
        });

        match result {
            Ok((
                actor_json,
                operation_json,
                target_type,
                target_id,
                status_json,
                rationale,
                created_at_str,
                diff_json,
                rollback_of_str,
            )) => {
                // Deserialize JSON fields
                let actor = serde_json::from_str(&actor_json)?;
                let operation = serde_json::from_str(&operation_json)?;

                // Reconstruct ObjectRef from target_type and target_id
                let target = {
                    let uuid = uuid::Uuid::parse_str(&target_id)
                        .map_err(|_| pkm_core::CoreError::Invariant("invalid uuid".into()))?;
                    match target_type.as_str() {
                        "source" => pkm_core::id::ObjectRef::Source(pkm_core::id::SourceId(uuid)),
                        "note" => pkm_core::id::ObjectRef::Note(pkm_core::id::NoteId(uuid)),
                        "block" => pkm_core::id::ObjectRef::Block(pkm_core::id::BlockId(uuid)),
                        "entity" => pkm_core::id::ObjectRef::Entity(pkm_core::id::EntityId(uuid)),
                        "link" => pkm_core::id::ObjectRef::Link(pkm_core::id::LinkId(uuid)),
                        "view" => pkm_core::id::ObjectRef::View(pkm_core::id::ViewId(uuid)),
                        _ => {
                            return Err(pkm_core::CoreError::Invariant(
                                "invalid target type".into(),
                            ))
                        }
                    }
                };

                let status = serde_json::from_str(&status_json)?;
                let diff = serde_json::from_str(&diff_json)?;
                let created_at = time::OffsetDateTime::parse(
                    &created_at_str,
                    &time::format_description::well_known::Rfc3339,
                )
                .map_err(|_| pkm_core::CoreError::Invariant("invalid timestamp".into()))?;
                let rollback_of = rollback_of_str
                    .and_then(|s| uuid::Uuid::parse_str(&s).ok())
                    .map(AgentActionId);

                Ok(Some(AgentAction {
                    id,
                    actor,
                    operation,
                    target,
                    status,
                    rationale,
                    created_at,
                    diff,
                    rollback_of,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(crate::StorageError::from(e).into()),
        }
    }

    fn set_status(
        &self,
        id: AgentActionId,
        new_status: pkm_core::agent_action::AgentActionStatus,
    ) -> Result<()> {
        let status_json = serde_json::to_string(&new_status)?;

        self.conn
            .execute(
                "UPDATE agent_action SET status = ?1 WHERE id = ?2",
                params![status_json, id.to_string()],
            )
            .map_err(crate::StorageError::from)?;

        Ok(())
    }

    fn set_diff(&self, id: AgentActionId, diff: serde_json::Value) -> Result<()> {
        let diff_json = serde_json::to_string(&diff)?;

        self.conn
            .execute(
                "UPDATE agent_action SET diff = ?1 WHERE id = ?2",
                params![diff_json, id.to_string()],
            )
            .map_err(crate::StorageError::from)?;

        Ok(())
    }
}

// TODO(D2): list/filter (by target, by status, by actor, by date range), batch get.
