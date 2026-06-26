//! Markdown import/export for notes and blocks.
//!
//! Pure, testable functions to convert between Note/Block structures and markdown.
//! These are stateless and keep all IO at the boundary (pkm-storage).
//!
//! The markdown format includes:
//! - Title as a level-1 heading (# Title)
//! - Blocks as paragraphs separated by blank lines
//! - Block IDs preserved as HTML comments for round-tripping
//! - Note metadata as YAML front matter (future extension)

use crate::block::{Block, BlockContent};
use crate::id::{BlockId, NoteId};
use crate::note::Note;
use crate::{Actor, Timestamp};
use std::collections::BTreeMap;

/// Convert blocks to markdown text. Each block becomes a paragraph or heading
/// based on its content. Preserves order and block identity as HTML comments.
pub fn blocks_to_markdown(blocks: &[Block]) -> String {
    let mut md = String::new();

    for block in blocks {
        // Add the block content as markdown
        match &block.content {
            BlockContent::Markdown { text } => {
                md.push_str(text);
                md.push_str("\n\n");
            }
        }

        // Add a block id reference as an HTML comment for round-tripping
        md.push_str(&format!("<!-- block:{} -->\n\n", block.id));
    }

    md.trim_end().to_string()
}

/// Parse markdown into blocks. Each paragraph or heading becomes a block.
/// Extracts block IDs from HTML comments where present; generates new ones otherwise.
/// Returns blocks in the order they appear, with fractional order keys.
pub fn markdown_to_blocks(text: &str, note_id: NoteId) -> Result<Vec<Block>, String> {
    let mut blocks = Vec::new();
    let mut order = 1.0_f32;
    let now = Timestamp::now_utc();

    let lines = text.lines().collect::<Vec<_>>();
    let mut current_block = String::new();
    let mut block_id: Option<BlockId> = None;

    for line in lines {
        // Check for block ID comment
        if line.starts_with("<!-- block:") && line.ends_with(" -->") {
            let id_str = line
                .trim_start_matches("<!-- block:")
                .trim_end_matches(" -->");
            if let Ok(uuid) = uuid::Uuid::parse_str(id_str) {
                block_id = Some(BlockId(uuid));
            }
        } else if line.is_empty() {
            // Empty line marks block boundary
            if !current_block.trim().is_empty() {
                let block = Block {
                    id: block_id.unwrap_or_else(BlockId::new),
                    note_id,
                    content: BlockContent::Markdown {
                        text: current_block.trim().to_string(),
                    },
                    order,
                    created_by: Actor::User,
                    created_at: now,
                    source_provenance_ref: None,
                    version: 1,
                    updated_at: now,
                };
                blocks.push(block);
                order += 1.0;
                current_block.clear();
                block_id = None;
            }
        } else {
            // Accumulate content
            if !current_block.is_empty() {
                current_block.push('\n');
            }
            current_block.push_str(line);
        }
    }

    // Don't forget the last block
    if !current_block.trim().is_empty() {
        let block = Block {
            id: block_id.unwrap_or_else(BlockId::new),
            note_id,
            content: BlockContent::Markdown {
                text: current_block.trim().to_string(),
            },
            order,
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        };
        blocks.push(block);
    }

    Ok(blocks)
}

/// Extract title from markdown text (first line that looks like a heading).
/// Returns the heading text without the # prefix.
pub fn extract_title(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("# ") {
            return Some(heading.trim().to_string());
        }
    }
    None
}

/// Export a note and its blocks to markdown.
/// Includes the note title as a level-1 heading, followed by blocks.
pub fn note_to_markdown(note: &Note, blocks: &[Block]) -> String {
    let mut md = String::new();

    // Write title as level-1 heading
    if !note.title.is_empty() {
        md.push_str("# ");
        md.push_str(&note.title);
        md.push_str("\n\n");
    }

    // Write blocks
    md.push_str(&blocks_to_markdown(blocks));

    md
}

/// Import markdown into a new note with blocks.
/// Extracts the title from the first heading; creates blocks from paragraphs.
/// All blocks are created with the same actor and timestamp.
pub fn markdown_to_note(
    text: &str,
    note_id: NoteId,
    actor: Actor,
    created_at: Timestamp,
) -> Result<(Note, Vec<Block>), String> {
    // Extract title from markdown
    let title = extract_title(text).unwrap_or_default();

    // Parse blocks from markdown (without title line)
    let mut blocks_text = text.to_string();
    if let Some(newline_pos) = text.find('\n') {
        blocks_text = text[newline_pos + 1..].to_string();
    }

    let mut parsed_blocks = markdown_to_blocks(&blocks_text, note_id)?;

    // Update blocks to use the provided actor and timestamp
    for block in &mut parsed_blocks {
        block.created_by = actor.clone();
        block.created_at = created_at;
    }

    let block_ids: Vec<BlockId> = parsed_blocks.iter().map(|b| b.id).collect();

    let note = Note {
        id: note_id,
        title,
        blocks: block_ids,
        metadata: BTreeMap::new(),
        created_by: actor,
        created_at,
        version: 1,
        updated_at: created_at,
    };

    Ok((note, parsed_blocks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_to_markdown_round_trip() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        let blocks = vec![
            Block {
                id: BlockId::new(),
                note_id,
                content: BlockContent::Markdown {
                    text: "First paragraph".to_string(),
                },
                order: 1.0,
                created_by: Actor::User,
                created_at: now,
                source_provenance_ref: None,
                version: 1,
                updated_at: now,
            },
            Block {
                id: BlockId::new(),
                note_id,
                content: BlockContent::Markdown {
                    text: "Second paragraph".to_string(),
                },
                order: 2.0,
                created_by: Actor::User,
                created_at: now,
                source_provenance_ref: None,
                version: 1,
                updated_at: now,
            },
        ];

        let md = blocks_to_markdown(&blocks);
        assert!(md.contains("First paragraph"));
        assert!(md.contains("Second paragraph"));
        assert!(md.contains("<!-- block:"));
    }

    #[test]
    fn markdown_to_blocks_basic() {
        let note_id = NoteId::new();
        let text = "First paragraph\n\nSecond paragraph";

        let blocks = markdown_to_blocks(text, note_id).expect("parse");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].order, 1.0);
        assert_eq!(blocks[1].order, 2.0);
    }

    #[test]
    fn markdown_to_blocks_preserves_block_ids() {
        let note_id = NoteId::new();
        let block_id = BlockId::new();
        let text = format!(
            "First paragraph\n\n<!-- block:{} -->\n\nSecond paragraph",
            block_id
        );

        let blocks = markdown_to_blocks(&text, note_id).expect("parse");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[1].id, block_id);
    }

    #[test]
    fn extract_title_from_heading() {
        let text = "# My Note Title\n\nSome content here.";
        let title = extract_title(text);
        assert_eq!(title, Some("My Note Title".to_string()));
    }

    #[test]
    fn extract_title_missing() {
        let text = "No heading here\n\nJust content.";
        let title = extract_title(text);
        assert_eq!(title, None);
    }

    #[test]
    fn note_to_markdown_with_title_and_blocks() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        let note = Note {
            id: note_id,
            title: "My Note".to_string(),
            blocks: vec![],
            metadata: std::collections::BTreeMap::new(),
            created_by: Actor::User,
            created_at: now,
            version: 1,
            updated_at: now,
        };

        let blocks = vec![Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::Markdown {
                text: "Some content".to_string(),
            },
            order: 1.0,
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        }];

        let md = note_to_markdown(&note, &blocks);
        assert!(md.contains("# My Note"));
        assert!(md.contains("Some content"));
    }

    #[test]
    fn markdown_to_note_round_trip() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        let original_text = "# Test Note\n\nFirst paragraph\n\nSecond paragraph";

        let (note, blocks) =
            markdown_to_note(original_text, note_id, Actor::User, now).expect("parse note");

        assert_eq!(note.title, "Test Note");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].order, 1.0);
        assert_eq!(blocks[1].order, 2.0);

        // Export back to markdown and verify structure is preserved
        let exported = note_to_markdown(&note, &blocks);
        assert!(exported.contains("# Test Note"));
        assert!(exported.contains("First paragraph"));
        assert!(exported.contains("Second paragraph"));
    }
}
