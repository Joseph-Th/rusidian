//! SQLite implementation of [`pkm_core::ports::LinkRepo`].

use rusqlite::{params, Connection};
use uuid::Uuid;

use pkm_core::id::{LinkId, ObjectRef};
use pkm_core::link::{Link, LinkType};
use pkm_core::ports::LinkRepo;
use pkm_core::review::ReviewState;
use pkm_core::Result;
use pkm_core::{Actor, CoreError};

/// Link persistence backed by SQLite.
pub struct SqliteLinkRepo<'c> {
    pub conn: &'c Connection,
}

impl LinkRepo for SqliteLinkRepo<'_> {
    fn create(&self, link: &Link) -> Result<()> {
        let link_type_str = link_type_to_string(link.link_type);
        let created_by_json =
            serde_json::to_string(&link.created_by).map_err(crate::StorageError::from)?;
        let created_at_str = link
            .created_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());
        let reviewed_str = review_state_to_string(link.reviewed);

        // Decompose ObjectRef into type and id
        let (from_type, from_id) = object_ref_to_parts(&link.from);
        let (to_type, to_id) = object_ref_to_parts(&link.to);

        self.conn
            .execute(
                "INSERT INTO link (id, from_type, from_id, to_type, to_id, link_type, created_at, created_by, reviewed, confidence)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    link.id.to_string(),
                    from_type,
                    from_id,
                    to_type,
                    to_id,
                    link_type_str,
                    created_at_str,
                    created_by_json,
                    reviewed_str,
                    link.confidence,
                ],
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;
        Ok(())
    }

    fn get(&self, link_id: LinkId) -> Result<Option<Link>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, from_type, from_id, to_type, to_id, link_type, created_at, created_by, reviewed, confidence
                 FROM link WHERE id = ?",
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;

        let result = stmt.query_row(params![link_id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let from_type: String = row.get(1)?;
            let from_id: String = row.get(2)?;
            let to_type: String = row.get(3)?;
            let to_id: String = row.get(4)?;
            let link_type: String = row.get(5)?;
            let created_at_str: String = row.get(6)?;
            let created_by_json: String = row.get(7)?;
            let reviewed_str: String = row.get(8)?;
            let confidence: Option<f32> = row.get(9)?;

            Ok((
                id_str,
                from_type,
                from_id,
                to_type,
                to_id,
                link_type,
                created_at_str,
                created_by_json,
                reviewed_str,
                confidence,
            ))
        });

        match result {
            Ok(fields) => {
                let link = build_link_from_fields(fields).map_err(|e| {
                    let ce: CoreError = e.into();
                    ce
                })?;
                Ok(Some(link))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                Err(ce)
            }
        }
    }

    fn get_by_to(&self, target: ObjectRef) -> Result<Vec<Link>> {
        let (to_type, to_id) = object_ref_to_parts(&target);

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, from_type, from_id, to_type, to_id, link_type, created_at, created_by, reviewed, confidence
                 FROM link WHERE to_type = ? AND to_id = ?",
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;

        let links = stmt
            .query_map(params![to_type, to_id], |row| {
                let id_str: String = row.get(0)?;
                let from_type: String = row.get(1)?;
                let from_id: String = row.get(2)?;
                let to_type: String = row.get(3)?;
                let to_id: String = row.get(4)?;
                let link_type: String = row.get(5)?;
                let created_at_str: String = row.get(6)?;
                let created_by_json: String = row.get(7)?;
                let reviewed_str: String = row.get(8)?;
                let confidence: Option<f32> = row.get(9)?;

                Ok((
                    id_str,
                    from_type,
                    from_id,
                    to_type,
                    to_id,
                    link_type,
                    created_at_str,
                    created_by_json,
                    reviewed_str,
                    confidence,
                ))
            })
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;

        let mut result = Vec::new();
        for fields in links {
            result.push(build_link_from_fields(fields).map_err(|e| {
                let ce: CoreError = e.into();
                ce
            })?);
        }
        Ok(result)
    }

    fn get_by_from(&self, source: ObjectRef) -> Result<Vec<Link>> {
        let (from_type, from_id) = object_ref_to_parts(&source);

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, from_type, from_id, to_type, to_id, link_type, created_at, created_by, reviewed, confidence
                 FROM link WHERE from_type = ? AND from_id = ?",
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;

        let links = stmt
            .query_map(params![from_type, from_id], |row| {
                let id_str: String = row.get(0)?;
                let from_type: String = row.get(1)?;
                let from_id: String = row.get(2)?;
                let to_type: String = row.get(3)?;
                let to_id: String = row.get(4)?;
                let link_type: String = row.get(5)?;
                let created_at_str: String = row.get(6)?;
                let created_by_json: String = row.get(7)?;
                let reviewed_str: String = row.get(8)?;
                let confidence: Option<f32> = row.get(9)?;

                Ok((
                    id_str,
                    from_type,
                    from_id,
                    to_type,
                    to_id,
                    link_type,
                    created_at_str,
                    created_by_json,
                    reviewed_str,
                    confidence,
                ))
            })
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;

        let mut result = Vec::new();
        for fields in links {
            result.push(build_link_from_fields(fields).map_err(|e| {
                let ce: CoreError = e.into();
                ce
            })?);
        }
        Ok(result)
    }

    fn set_to(&self, link_id: LinkId, new_to: ObjectRef) -> Result<()> {
        let (to_type, to_id) = object_ref_to_parts(&new_to);

        self.conn
            .execute(
                "UPDATE link SET to_type = ?, to_id = ? WHERE id = ?",
                params![to_type, to_id, link_id.to_string()],
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;
        Ok(())
    }

    fn set_from(&self, link_id: LinkId, new_from: ObjectRef) -> Result<()> {
        let (from_type, from_id) = object_ref_to_parts(&new_from);

        self.conn
            .execute(
                "UPDATE link SET from_type = ?, from_id = ? WHERE id = ?",
                params![from_type, from_id, link_id.to_string()],
            )
            .map_err(|e| {
                let se = crate::StorageError::from(e);
                let ce: CoreError = se.into();
                ce
            })?;
        Ok(())
    }
}

/// Convert ObjectRef to (type_str, id_str) for storage.
fn object_ref_to_parts(obj: &ObjectRef) -> (&'static str, String) {
    match obj {
        ObjectRef::Source(id) => ("source", id.to_string()),
        ObjectRef::Note(id) => ("note", id.to_string()),
        ObjectRef::Block(id) => ("block", id.to_string()),
        ObjectRef::Entity(id) => ("entity", id.to_string()),
        ObjectRef::Link(id) => ("link", id.to_string()),
        ObjectRef::View(id) => ("view", id.to_string()),
    }
}

/// Reconstruct ObjectRef from (type_str, id_str).
fn parts_to_object_ref(type_str: &str, id_str: &str) -> crate::Result<ObjectRef> {
    let id = Uuid::parse_str(id_str).map_err(|e| {
        crate::StorageError::Core(CoreError::Invariant(format!("invalid object id: {}", e)))
    })?;

    Ok(match type_str {
        "source" => ObjectRef::Source(pkm_core::id::SourceId(id)),
        "note" => ObjectRef::Note(pkm_core::id::NoteId(id)),
        "block" => ObjectRef::Block(pkm_core::id::BlockId(id)),
        "entity" => ObjectRef::Entity(pkm_core::id::EntityId(id)),
        "link" => ObjectRef::Link(pkm_core::id::LinkId(id)),
        "view" => ObjectRef::View(pkm_core::id::ViewId(id)),
        _ => {
            return Err(crate::StorageError::Core(CoreError::Invariant(format!(
                "unknown object type: {}",
                type_str
            ))))
        }
    })
}

/// Pure mapping function: builds a Link from extracted fields.
fn build_link_from_fields(
    fields: (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<f32>,
    ),
) -> crate::Result<Link> {
    let (
        id_str,
        from_type,
        from_id,
        to_type,
        to_id,
        link_type,
        created_at_str,
        created_by_json,
        reviewed_str,
        confidence,
    ) = fields;

    let id = Uuid::parse_str(&id_str).map(LinkId).map_err(|e| {
        crate::StorageError::Core(CoreError::Invariant(format!("invalid link id: {}", e)))
    })?;

    let from = parts_to_object_ref(&from_type, &from_id)?;
    let to = parts_to_object_ref(&to_type, &to_id)?;
    let link_type_enum = parse_link_type(&link_type);
    let created_by = parse_actor(&created_by_json);
    let reviewed = parse_review_state(&reviewed_str);

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

    Ok(Link {
        id,
        from,
        to,
        link_type: link_type_enum,
        created_by,
        created_at,
        reviewed,
        confidence,
    })
}

/// Convert LinkType to the persisted snake_case string representation.
fn link_type_to_string(link_type: LinkType) -> &'static str {
    match link_type {
        LinkType::RelatedTo => "related_to",
        LinkType::Cites => "cites",
        LinkType::Supports => "supports",
        LinkType::Contradicts => "contradicts",
        LinkType::Summarizes => "summarizes",
        LinkType::DerivedFrom => "derived_from",
        LinkType::Mentions => "mentions",
        LinkType::PartOf => "part_of",
        LinkType::DependsOn => "depends_on",
        LinkType::DecidedIn => "decided_in",
        LinkType::AssignedTo => "assigned_to",
        LinkType::FollowsUp => "follows_up",
    }
}

/// Parse link type from string. Defaults to RelatedTo if unrecognized.
fn parse_link_type(s: &str) -> LinkType {
    match s {
        "related_to" => LinkType::RelatedTo,
        "cites" => LinkType::Cites,
        "supports" => LinkType::Supports,
        "contradicts" => LinkType::Contradicts,
        "summarizes" => LinkType::Summarizes,
        "derived_from" => LinkType::DerivedFrom,
        "mentions" => LinkType::Mentions,
        "part_of" => LinkType::PartOf,
        "depends_on" => LinkType::DependsOn,
        "decided_in" => LinkType::DecidedIn,
        "assigned_to" => LinkType::AssignedTo,
        "follows_up" => LinkType::FollowsUp,
        _ => LinkType::RelatedTo,
    }
}

/// Convert ReviewState to the persisted snake_case string representation.
fn review_state_to_string(state: ReviewState) -> &'static str {
    match state {
        ReviewState::Proposed => "proposed",
        ReviewState::Accepted => "accepted",
        ReviewState::Rejected => "rejected",
    }
}

/// Parse review state from string. Defaults to Proposed if unrecognized.
fn parse_review_state(s: &str) -> ReviewState {
    match s {
        "proposed" => ReviewState::Proposed,
        "accepted" => ReviewState::Accepted,
        "rejected" => ReviewState::Rejected,
        _ => ReviewState::Proposed,
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
    fn parse_link_types() {
        assert_eq!(parse_link_type("related_to"), LinkType::RelatedTo);
        assert_eq!(parse_link_type("cites"), LinkType::Cites);
        assert_eq!(parse_link_type("invalid"), LinkType::RelatedTo);
    }

    #[test]
    fn link_type_round_trip() {
        let types = vec![
            LinkType::RelatedTo,
            LinkType::Cites,
            LinkType::Supports,
            LinkType::Contradicts,
            LinkType::Summarizes,
            LinkType::DerivedFrom,
            LinkType::Mentions,
            LinkType::PartOf,
            LinkType::DependsOn,
            LinkType::DecidedIn,
            LinkType::AssignedTo,
            LinkType::FollowsUp,
        ];
        for link_type in types {
            let str = link_type_to_string(link_type);
            assert_eq!(parse_link_type(str), link_type);
        }
    }

    #[test]
    fn review_state_round_trip() {
        let states = vec![
            ReviewState::Proposed,
            ReviewState::Accepted,
            ReviewState::Rejected,
        ];
        for state in states {
            let str = review_state_to_string(state);
            assert_eq!(parse_review_state(str), state);
        }
    }
}
