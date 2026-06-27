//! SQLite implementation of [`pkm_core::ports::NoteRepo`].
//!
//! Hybrid markdown-first + SQLite-index architecture:
//! - Notes are written to disk as markdown files (vault_path)
//! - SQLite maintains an FTS5 index for fast search
//! - Blocks and note metadata are stored in both formats for redundancy

use std::path::PathBuf;
use rusqlite::{params, Connection};

use pkm_core::block::{Block, BlockContent};
use pkm_core::id::{BlockId, NoteId};
use pkm_core::markdown;
use pkm_core::note::Note;
use pkm_core::ports::NoteRepo;
use pkm_core::Result;
use crate::file_ops;

/// Note persistence backed by SQLite + markdown files.
/// The vault_path is the root directory where .md files are stored.
pub struct SqliteNoteRepo<'c> {
    pub conn: &'c Connection,
    pub vault_path: PathBuf,
}

impl SqliteNoteRepo<'_> {
    /// Helper: fetch all blocks for a note from the database.
    fn fetch_blocks(&self, note_id: NoteId) -> Result<Vec<Block>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, block_type, content, \"order\", created_at, created_by, version, updated_at \
                 FROM block WHERE note_id = ?1 ORDER BY \"order\"",
            )
            .map_err(crate::StorageError::from)?;

        let blocks: Result<Vec<Block>> = stmt
            .query_map(params![note_id.to_string()], |row| {
                let id_str: String = row.get(0)?;
                let _block_type: String = row.get(1)?;
                let content_json: String = row.get(2)?;
                let order: f32 = row.get(3)?;
                let created_at_str: String = row.get(4)?;
                let created_by_json: String = row.get(5)?;
                let version: i64 = row.get(6)?;
                let updated_at_str: String = row.get(7)?;

                Ok((
                    id_str,
                    content_json,
                    order,
                    created_at_str,
                    created_by_json,
                    version,
                    updated_at_str,
                ))
            })
            .map_err(crate::StorageError::from)?
            .map(|result| {
                let (
                    id_str,
                    content_json,
                    order,
                    created_at_str,
                    created_by_json,
                    version,
                    updated_at_str,
                ) = result.map_err(crate::StorageError::from)?;

                let uuid = uuid::Uuid::parse_str(&id_str)
                    .map_err(|_| pkm_core::CoreError::Invariant("invalid block uuid".into()))?;
                let id = BlockId(uuid);

                let content = serde_json::from_str(&content_json)?;
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

                Ok(Block {
                    id,
                    note_id,
                    content,
                    order,
                    created_by,
                    created_at,
                    source_provenance_ref: None,
                    version: u32::try_from(version).unwrap_or(1),
                    updated_at,
                })
            })
            .collect();

        blocks
    }
}

impl NoteRepo for SqliteNoteRepo<'_> {
    fn create(&self, note: &Note) -> Result<()> {
        // STEP 1: Fetch blocks to write the markdown file
        let blocks = self.fetch_blocks(note.id)?;

        // STEP 2: Generate markdown
        let markdown_text = markdown::note_to_markdown(note, &blocks);
        let file_path = self.vault_path.join(note.file_name());

        // STEP 3: Write to temp file (transactional pattern)
        let temp_path = file_ops::write_to_temp_file(&self.vault_path, &note.id.to_string(), &markdown_text)
            .map_err(|e| pkm_core::CoreError::Invariant(format!("failed to write temp file: {}", e)))?;

        // STEP 4: Insert the note into SQLite
        let created_at_str = note
            .created_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());
        let updated_at_str = note
            .updated_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());
        let metadata_json = serde_json::to_string(&note.metadata)?;
        let created_by_json = serde_json::to_string(&note.created_by)?;
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| pkm_core::CoreError::Invariant("invalid file path".into()))?;

        // If SQLite fails, abort temp file
        if let Err(e) = self.conn.execute(
            "INSERT INTO note (id, title, created_at, created_by, version, updated_at, metadata, file_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                note.id.to_string(),
                note.title,
                created_at_str,
                created_by_json,
                note.version,
                updated_at_str,
                metadata_json,
                file_path_str,
            ],
        ) {
            let _ = file_ops::abort_temp_file(&temp_path);
            return Err(crate::StorageError::from(e).into());
        }

        // STEP 5: Commit temp file (atomic rename)
        file_ops::commit_temp_file(&temp_path, &file_path)
            .map_err(|e| pkm_core::CoreError::Invariant(format!("failed to commit file: {}", e)))?;

        Ok(())
    }

    fn get(&self, id: NoteId) -> Result<Option<Note>> {
        // Query SQLite to get all metadata including version
        let mut stmt = self
            .conn
            .prepare(
                "SELECT file_path, created_at, created_by, version, updated_at FROM note WHERE id = ?1",
            )
            .map_err(crate::StorageError::from)?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            let file_path: String = row.get(0)?;
            let created_at_str: String = row.get(1)?;
            let created_by_json: String = row.get(2)?;
            let version: i64 = row.get(3)?;
            let updated_at_str: String = row.get(4)?;

            Ok((file_path, created_at_str, created_by_json, version, updated_at_str))
        });

        match result {
            Ok((file_path, created_at_str, created_by_json, version, updated_at_str)) => {
                // Read the markdown file from disk
                let markdown_text = std::fs::read_to_string(&file_path)
                    .map_err(|e| pkm_core::CoreError::Invariant(
                        format!("failed to read note file {}: {}", file_path, e)
                    ))?;

                // Parse the markdown into a Note and Blocks using the frontmatter
                let created_at = pkm_core::Timestamp::now_utc();
                let (mut note, _blocks) = markdown::markdown_to_note(
                    &markdown_text,
                    id,
                    pkm_core::Actor::User,
                    created_at,
                )
                .map_err(|e| pkm_core::CoreError::Invariant(
                    format!("failed to parse markdown: {}", e)
                ))?;

                // Merge in database metadata: version, updated_at, and created timestamps
                note.version = u32::try_from(version).unwrap_or(1);

                let created_by = serde_json::from_str(&created_by_json)?;
                note.created_by = created_by;

                let created_at = time::OffsetDateTime::parse(
                    &created_at_str,
                    &time::format_description::well_known::Rfc3339,
                )
                .map_err(|_| pkm_core::CoreError::Invariant("invalid timestamp".into()))?;
                note.created_at = created_at;

                let updated_at = time::OffsetDateTime::parse(
                    &updated_at_str,
                    &time::format_description::well_known::Rfc3339,
                )
                .map_err(|_| pkm_core::CoreError::Invariant("invalid timestamp".into()))?;
                note.updated_at = updated_at;

                // Fetch full blocks from the database (they may have been updated independently)
                let blocks = self.fetch_blocks(id)?;

                // Reconstruct block IDs in the correct order
                let block_ids: Vec<BlockId> = blocks.iter().map(|b| b.id).collect();
                note.blocks = block_ids;

                Ok(Some(note))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(crate::StorageError::from(e).into()),
        }
    }

    fn list(&self, limit: Option<usize>) -> Result<Vec<Note>> {
        let query = match limit {
            Some(n) => format!(
                "SELECT id, title, created_at, created_by, version, updated_at, metadata \
                 FROM note ORDER BY created_at DESC LIMIT {}",
                n
            ),
            None => "SELECT id, title, created_at, created_by, version, updated_at, metadata \
                     FROM note ORDER BY created_at DESC"
                .to_string(),
        };
        let mut stmt = self
            .conn
            .prepare(&query)
            .map_err(crate::StorageError::from)?;

        let notes: Result<Vec<Note>> = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let title: String = row.get(1)?;
                let created_at_str: String = row.get(2)?;
                let created_by_json: String = row.get(3)?;
                let version: i64 = row.get(4)?;
                let updated_at_str: String = row.get(5)?;
                let metadata_json: String = row.get(6)?;

                Ok((
                    id_str,
                    title,
                    created_at_str,
                    created_by_json,
                    version,
                    updated_at_str,
                    metadata_json,
                ))
            })
            .map_err(crate::StorageError::from)?
            .map(|result| {
                let (
                    id_str,
                    title,
                    created_at_str,
                    created_by_json,
                    version,
                    updated_at_str,
                    metadata_json,
                ) = result.map_err(crate::StorageError::from)?;

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

                let metadata_value: serde_json::Value = serde_json::from_str(&metadata_json)?;
                let metadata = metadata_value
                    .as_object()
                    .cloned()
                    .map(|obj| obj.into_iter().collect())
                    .unwrap_or_default();

                Ok(Note {
                    id,
                    title,
                    blocks: vec![],
                    metadata,
                    created_at,
                    created_by,
                    version: u32::try_from(version).unwrap_or(1),
                    updated_at,
                })
            })
            .collect();

        notes
    }

    fn update(&self, note: &Note) -> Result<()> {
        // STEP 1: Fetch blocks for this note
        let blocks = self.fetch_blocks(note.id)?;

        // STEP 2: Generate markdown
        let markdown_text = markdown::note_to_markdown(note, &blocks);
        let file_path = self.vault_path.join(note.file_name());

        // STEP 3: Write to temp file (transactional pattern)
        let temp_path = file_ops::write_to_temp_file(&self.vault_path, &note.id.to_string(), &markdown_text)
            .map_err(|e| pkm_core::CoreError::Invariant(format!("failed to write temp file: {}", e)))?;

        // STEP 4: Update the note in SQLite
        let updated_at_str = note
            .updated_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());

        let metadata_json = serde_json::to_string(&note.metadata)?;
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| pkm_core::CoreError::Invariant("invalid file path".into()))?;

        let rows = match self.conn.execute(
            "UPDATE note SET title = ?1, version = ?2, updated_at = ?3, metadata = ?4, file_path = ?5 WHERE id = ?6",
            params![
                note.title,
                note.version as i64,
                updated_at_str,
                metadata_json,
                file_path_str,
                note.id.to_string(),
            ],
        ) {
            Ok(rows) => rows,
            Err(e) => {
                let _ = file_ops::abort_temp_file(&temp_path);
                return Err(crate::StorageError::from(e).into());
            }
        };

        if rows == 0 {
            let _ = file_ops::abort_temp_file(&temp_path);
            return Err(pkm_core::CoreError::Invariant(
                format!("note {} not found", note.id),
            ));
        }

        // STEP 5: Commit temp file (atomic rename)
        file_ops::commit_temp_file(&temp_path, &file_path)
            .map_err(|e| pkm_core::CoreError::Invariant(format!("failed to commit file: {}", e)))?;

        Ok(())
    }

    fn delete(&self, id: NoteId) -> Result<()> {
        // Delete child blocks first (no ON DELETE CASCADE in schema).
        self.conn
            .execute("DELETE FROM block WHERE note_id = ?1", params![id.to_string()])
            .map_err(crate::StorageError::from)?;
        self.conn
            .execute("DELETE FROM note WHERE id = ?1", params![id.to_string()])
            .map_err(crate::StorageError::from)?;
        Ok(())
    }

    fn update_block(
        &self,
        note_id: NoteId,
        block_id: BlockId,
        new_content: BlockContent,
    ) -> Result<Block> {
        let now = pkm_core::Timestamp::now_utc();

        // STEP 1: Query SQLite for the file path
        let file_path_str: String = self
            .conn
            .query_row(
                "SELECT file_path FROM note WHERE id = ?1",
                params![note_id.to_string()],
                |row| row.get(0),
            )
            .map_err(crate::StorageError::from)?;

        // STEP 2: Read the full markdown file
        let markdown_text = std::fs::read_to_string(&file_path_str)
            .map_err(|e| pkm_core::CoreError::Invariant(
                format!("failed to read note file: {}", e)
            ))?;

        // STEP 3: Parse the markdown file into a Note and Blocks
        let (_note, mut parsed_blocks) = markdown::markdown_to_note(
            &markdown_text,
            note_id,
            pkm_core::Actor::User,
            now,
        )
        .map_err(|e| pkm_core::CoreError::Invariant(
            format!("failed to parse markdown: {}", e)
        ))?;

        // STEP 4: Find the block and update it
        let mut updated_block = None;
        for block in &mut parsed_blocks {
            if block.id == block_id {
                block.content = new_content.clone();
                block.version += 1;
                block.updated_at = now;
                updated_block = Some(block.clone());
                break;
            }
        }

        let updated_block = updated_block
            .ok_or_else(|| pkm_core::CoreError::Invariant(
                format!("block {} not found in note {}", block_id, note_id)
            ))?;

        // STEP 5: Fetch the full note from the database
        let mut stmt = self
            .conn
            .prepare(
                "SELECT title, created_at, created_by, version, updated_at, metadata FROM note WHERE id = ?1",
            )
            .map_err(crate::StorageError::from)?;

        let (title, created_at_str, created_by_json, version, updated_at_str, metadata_json) = stmt
            .query_row(params![note_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(crate::StorageError::from)?;

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

        let metadata_value: serde_json::Value = serde_json::from_str(&metadata_json)?;
        let metadata = metadata_value
            .as_object()
            .cloned()
            .map(|obj| obj.into_iter().collect())
            .unwrap_or_default();

        let block_ids: Vec<BlockId> = parsed_blocks.iter().map(|b| b.id).collect();

        let note = Note {
            id: note_id,
            title,
            blocks: block_ids,
            metadata,
            created_by,
            created_at,
            version: version as u32,
            updated_at,
        };

        // STEP 6: Re-serialize the whole note and write to temp file
        let new_markdown = markdown::note_to_markdown(&note, &parsed_blocks);
        let temp_path = file_ops::write_to_temp_file(&self.vault_path, &note_id.to_string(), &new_markdown)
            .map_err(|e| pkm_core::CoreError::Invariant(
                format!("failed to write temp file: {}", e)
            ))?;

        // STEP 7: Update the block in SQLite
        let content_json = serde_json::to_string(&updated_block.content)?;
        let updated_at_str = updated_block
            .updated_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());

        if let Err(e) = self.conn.execute(
            "UPDATE block SET content = ?1, version = ?2, updated_at = ?3 WHERE id = ?4 AND note_id = ?5",
            params![
                content_json,
                updated_block.version as i64,
                updated_at_str,
                block_id.to_string(),
                note_id.to_string()
            ],
        ) {
            let _ = file_ops::abort_temp_file(&temp_path);
            return Err(crate::StorageError::from(e).into());
        }

        // STEP 8: Commit temp file (atomic rename)
        file_ops::commit_temp_file(&temp_path, std::path::Path::new(&file_path_str))
            .map_err(|e| pkm_core::CoreError::Invariant(
                format!("failed to commit file: {}", e)
            ))?;

        Ok(updated_block)
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
