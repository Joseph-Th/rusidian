//! SQLite implementation of [`pkm_core::ports::EntityRepo`].

use rusqlite::{params, Connection};
use uuid::Uuid;

use pkm_core::entity::{Entity, EntityKind};
use pkm_core::id::EntityId;
use pkm_core::ports::EntityRepo;
use pkm_core::Result;
use pkm_core::{Actor, CoreError};

/// Entity persistence backed by SQLite.
pub struct SqliteEntityRepo<'c> {
    pub conn: &'c Connection,
}

impl EntityRepo for SqliteEntityRepo<'_> {
    fn create(&self, entity: &Entity) -> Result<()> {
        let kind_str = entity_kind_to_string(entity.kind);
        let created_by_json =
            serde_json::to_string(&entity.created_by).map_err(crate::StorageError::from)?;
        let aliases_json =
            serde_json::to_string(&entity.aliases).map_err(crate::StorageError::from)?;
        let created_at_str = entity
            .created_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());

        self.conn
            .execute(
                "INSERT INTO entity (id, kind, name, aliases, created_at, created_by, merged_into)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![
                    entity.id.to_string(),
                    kind_str,
                    entity.name,
                    aliases_json,
                    created_at_str,
                    created_by_json,
                    entity.merged_into.map(|id| id.to_string()),
                ],
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;
        Ok(())
    }

    fn get(&self, id: EntityId) -> Result<Option<Entity>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, kind, name, aliases, created_at, created_by, merged_into, version, updated_at
                 FROM entity WHERE id = ?",
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let kind_str: String = row.get(1)?;
            let name: String = row.get(2)?;
            let aliases_json: String = row.get(3)?;
            let created_at_str: String = row.get(4)?;
            let created_by_json: String = row.get(5)?;
            let merged_into_str: Option<String> = row.get(6)?;
            let version: i64 = row.get(7)?;
            let updated_at_str: String = row.get(8)?;

            Ok((
                id_str,
                kind_str,
                name,
                aliases_json,
                created_at_str,
                created_by_json,
                merged_into_str,
                version,
                updated_at_str,
            ))
        });

        match result {
            Ok(fields) => {
                let entity = build_entity_from_fields(fields).map_err(|e| {
                    let ce: CoreError = e.into();
                    ce
                })?;
                Ok(Some(entity))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                Err(ce)
            }
        }
    }

    fn set_merged_into(&self, loser_id: EntityId, survivor_id: EntityId) -> Result<()> {
        self.conn
            .execute(
                "UPDATE entity SET merged_into = ? WHERE id = ?",
                params![survivor_id.to_string(), loser_id.to_string()],
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;
        Ok(())
    }

    fn clear_merged_into(&self, entity_id: EntityId) -> Result<()> {
        self.conn
            .execute(
                "UPDATE entity SET merged_into = NULL WHERE id = ?",
                params![entity_id.to_string()],
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;
        Ok(())
    }
}

/// Pure mapping function: builds an Entity from extracted fields.
fn build_entity_from_fields(
    fields: (
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        i64,
        String,
    ),
) -> crate::Result<Entity> {
    let (
        id_str,
        kind_str,
        name,
        aliases_json,
        created_at_str,
        created_by_json,
        merged_into_str,
        version,
        updated_at_str,
    ) = fields;

    let id = Uuid::parse_str(&id_str).map(EntityId).map_err(|e| {
        crate::StorageError::Core(CoreError::Invariant(format!("invalid entity id: {}", e)))
    })?;

    let kind = parse_entity_kind(&kind_str);
    let created_by = parse_actor(&created_by_json);
    let aliases: Vec<String> = serde_json::from_str(&aliases_json).unwrap_or_else(|_| Vec::new());

    let created_at = time::OffsetDateTime::parse(
        &created_at_str,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| {
        crate::StorageError::Core(CoreError::Invariant(format!(
            "invalid timestamp: {}: {}",
            created_at_str, e
        )))
    })?;

    let updated_at = time::OffsetDateTime::parse(
        &updated_at_str,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| {
        crate::StorageError::Core(CoreError::Invariant(format!(
            "invalid timestamp: {}: {}",
            updated_at_str, e
        )))
    })?;

    let merged_into = merged_into_str.and_then(|s| Uuid::parse_str(&s).ok().map(EntityId));

    Ok(Entity {
        id,
        kind,
        name,
        aliases,
        created_by,
        created_at,
        merged_into,
        version: version as u32,
        updated_at,
    })
}

/// Convert EntityKind to the persisted snake_case string representation.
fn entity_kind_to_string(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Person => "person",
        EntityKind::Organization => "organization",
        EntityKind::Project => "project",
        EntityKind::Topic => "topic",
        EntityKind::Location => "location",
        EntityKind::Event => "event",
        EntityKind::Book => "book",
        EntityKind::Paper => "paper",
        EntityKind::Product => "product",
        EntityKind::Claim => "claim",
        EntityKind::Decision => "decision",
    }
}

/// Parse entity kind from string. Defaults to Topic if unrecognized.
fn parse_entity_kind(s: &str) -> EntityKind {
    match s {
        "person" => EntityKind::Person,
        "organization" => EntityKind::Organization,
        "project" => EntityKind::Project,
        "topic" => EntityKind::Topic,
        "location" => EntityKind::Location,
        "event" => EntityKind::Event,
        "book" => EntityKind::Book,
        "paper" => EntityKind::Paper,
        "product" => EntityKind::Product,
        "claim" => EntityKind::Claim,
        "decision" => EntityKind::Decision,
        _ => EntityKind::Topic,
    }
}

/// Parse actor from JSON. Defaults to User if unrecognized or malformed.
fn parse_actor(json: &str) -> Actor {
    serde_json::from_str(json).unwrap_or(Actor::User)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_entity_kinds() {
        assert_eq!(parse_entity_kind("person"), EntityKind::Person);
        assert_eq!(parse_entity_kind("organization"), EntityKind::Organization);
        assert_eq!(parse_entity_kind("project"), EntityKind::Project);
        assert_eq!(parse_entity_kind("invalid"), EntityKind::Topic);
    }

    #[test]
    fn entity_kind_round_trip() {
        let kinds = vec![
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
        ];
        for kind in kinds {
            let str = entity_kind_to_string(kind);
            assert_eq!(parse_entity_kind(str), kind);
        }
    }
}
