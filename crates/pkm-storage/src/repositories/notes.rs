//! SQLite implementation of [`pkm_core::ports::NoteRepo`].

use rusqlite::{params, Connection};
use std::collections::BTreeMap;

use pkm_core::block::{Block, BlockContent};
use pkm_core::id::{BlockId, NoteId};
use pkm_core::note::Note;
use pkm_core::ports::NoteRepo;
use pkm_core::Result;

/// Note persistence backed by SQLite.
pub struct SqliteNoteRepo<'c> {
    pub conn: &'c Connection,
}

impl NoteRepo for SqliteNoteRepo<'_> {
    fn create(&self, note: &Note) -> Result<()> {
        // Insert the note itself
        let created_at_str = note
            .created_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());
        let updated_at_str = note
            .updated_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());

        let created_by_json = serde_json::to_string(&note.created_by)?;

        self.conn
            .execute(
                "INSERT INTO note (id, title, created_at, created_by, version, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    note.id.to_string(),
                    note.title,
                    created_at_str,
                    created_by_json,
                    note.version,
                    updated_at_str,
                ],
            )
            .map_err(crate::StorageError::from)?;

        Ok(())
    }

    fn get(&self, id: NoteId) -> Result<Option<Note>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT title, created_at, created_by, version, updated_at FROM note WHERE id = ?1",
            )
            .map_err(crate::StorageError::from)?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            let title: String = row.get(0)?;
            let created_at_str: String = row.get(1)?;
            let created_by_json: String = row.get(2)?;
            let version: i64 = row.get(3)?;
            let updated_at_str: String = row.get(4)?;

            Ok((
                title,
                created_at_str,
                created_by_json,
                version,
                updated_at_str,
            ))
        });

        match result {
            Ok((title, created_at_str, created_by_json, version, updated_at_str)) => {
                let created_by = serde_json::from_str(&created_by_json)?;
                let created_at = time::OffsetDateTime::parse(
                    &created_at_str,
                    &time::format_description::well_known::Rfc3339,
                )
                .map_err(|_| pkm_core::CoreError::Invariant("invalid timestamp".into()))?;
                let updated_at = time::OffsetDateTime::parse(
                    &updated_at_str,
                    &time::format_description::well_known::Rfc3339,
                )
                .map_err(|_| pkm_core::CoreError::Invariant("invalid timestamp".into()))?;

                // Fetch block IDs for this note
                let mut block_stmt = self
                    .conn
                    .prepare("SELECT id FROM block WHERE note_id = ?1 ORDER BY \"order\"")
                    .map_err(crate::StorageError::from)?;

                let block_ids: Result<Vec<BlockId>> = block_stmt
                    .query_map(params![id.to_string()], |row| {
                        let block_id_str: String = row.get(0)?;
                        Ok(block_id_str)
                    })
                    .map_err(crate::StorageError::from)?
                    .map(|result| {
                        let block_id_str = result.map_err(crate::StorageError::from)?;
                        let uuid = uuid::Uuid::parse_str(&block_id_str).map_err(|_| {
                            pkm_core::CoreError::Invariant("invalid block uuid".into())
                        })?;
                        Ok(BlockId(uuid))
                    })
                    .collect();

                let blocks = block_ids?;

                Ok(Some(Note {
                    id,
                    title,
                    blocks,
                    metadata: BTreeMap::new(), // TODO(S2): add metadata to schema
                    created_at,
                    created_by,
                    version: version as u32,
                    updated_at,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(crate::StorageError::from(e).into()),
        }
    }

    fn list(&self, limit: Option<usize>) -> Result<Vec<Note>> {
        let query = "SELECT id, title, created_at, created_by, version, updated_at FROM note ORDER BY created_at DESC";
        let mut stmt = self
            .conn
            .prepare(query)
            .map_err(crate::StorageError::from)?;

        let notes: Result<Vec<Note>> = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let title: String = row.get(1)?;
                let created_at_str: String = row.get(2)?;
                let created_by_json: String = row.get(3)?;
                let version: i64 = row.get(4)?;
                let updated_at_str: String = row.get(5)?;

                Ok((
                    id_str,
                    title,
                    created_at_str,
                    created_by_json,
                    version,
                    updated_at_str,
                ))
            })
            .map_err(crate::StorageError::from)?
            .take(limit.unwrap_or(usize::MAX))
            .map(|result| {
                let (id_str, title, created_at_str, created_by_json, version, updated_at_str) =
                    result.map_err(crate::StorageError::from)?;

                let uuid = uuid::Uuid::parse_str(&id_str)
                    .map_err(|_| pkm_core::CoreError::Invariant("invalid note uuid".into()))?;
                let id = NoteId(uuid);

                let created_by = serde_json::from_str(&created_by_json)?;
                let created_at = time::OffsetDateTime::parse(
                    &created_at_str,
                    &time::format_description::well_known::Rfc3339,
                )
                .map_err(|_| pkm_core::CoreError::Invariant("invalid timestamp".into()))?;
                let updated_at = time::OffsetDateTime::parse(
                    &updated_at_str,
                    &time::format_description::well_known::Rfc3339,
                )
                .map_err(|_| pkm_core::CoreError::Invariant("invalid timestamp".into()))?;

                Ok(Note {
                    id,
                    title,
                    blocks: vec![],
                    metadata: BTreeMap::new(),
                    created_at,
                    created_by,
                    version: version as u32,
                    updated_at,
                })
            })
            .collect();

        notes
    }

    fn update_block(
        &self,
        note_id: NoteId,
        block_id: BlockId,
        new_content: BlockContent,
    ) -> Result<Block> {
        // Serialize the new content as JSON
        let content_json = serde_json::to_string(&new_content)?;

        // Update the block
        self.conn
            .execute(
                "UPDATE block SET content = ?1 WHERE id = ?2 AND note_id = ?3",
                params![content_json, block_id.to_string(), note_id.to_string()],
            )
            .map_err(crate::StorageError::from)?;

        // Retrieve the updated block to return it
        let mut stmt = self
            .conn
            .prepare("SELECT \"order\", created_at, created_by FROM block WHERE id = ?1")
            .map_err(crate::StorageError::from)?;

        let (order, created_at_str, created_by_json) = stmt
            .query_row(params![block_id.to_string()], |row| {
                let order: f32 = row.get(0)?;
                let created_at_str: String = row.get(1)?;
                let created_by_json: String = row.get(2)?;

                Ok((order, created_at_str, created_by_json))
            })
            .map_err(crate::StorageError::from)?;

        let created_by = serde_json::from_str(&created_by_json)?;
        let created_at = time::OffsetDateTime::parse(
            &created_at_str,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|_| pkm_core::CoreError::Invariant("invalid timestamp".into()))?;

        Ok(Block {
            id: block_id,
            note_id,
            content: new_content,
            order,
            created_by,
            created_at,
            source_provenance_ref: None,
            version: 1,
            updated_at: created_at,
        })
    }
}

impl SqliteNoteRepo<'_> {
    pub fn insert_block(&self, block: &Block) -> Result<()> {
        let created_at_str = block
            .created_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());
        let updated_at_str = block
            .updated_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());

        let created_by_json = serde_json::to_string(&block.created_by)?;
        let content_json = serde_json::to_string(&block.content)?;

        self.conn
            .execute(
                "INSERT INTO block (id, note_id, block_type, content, \"order\", created_at, created_by, version, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    block.id.to_string(),
                    block.note_id.to_string(),
                    "markdown", // TODO: extract from BlockContent enum
                    content_json,
                    block.order,
                    created_at_str,
                    created_by_json,
                    block.version,
                    updated_at_str,
                ],
            )
            .map_err(crate::StorageError::from)?;

        Ok(())
    }
}
