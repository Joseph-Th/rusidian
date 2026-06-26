//! Markdown import/export for notes and blocks.
//!
//! Pure, testable functions to convert between Note/Block structures and markdown.
//! These are stateless and keep all IO at the boundary (pkm-storage).
//!
//! The markdown format includes:
//! - Title as a level-1 heading (# Title)
//! - Blocks as paragraphs separated by blank lines
//! - Block IDs preserved as HTML comments for round-tripping
//! - Rich blocks (tables, math, media) serialized as standard markdown
//! - Complex UI (views, Kanban) serialized with fallback text + HTML comments
//! - Note metadata as YAML front matter

use crate::block::{Block, BlockContent};
use crate::id::{BlockId, NoteId};
use crate::media::{EmbedProvider, MediaType};
use crate::note::Note;
use crate::{Actor, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// YAML frontmatter for notes. Serialized as the first block in markdown files.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NoteFrontmatter {
    id: String,
    created_by: String,
    created_at: String,
    metadata: BTreeMap<String, serde_json::Value>,
}

/// Extract YAML frontmatter from the beginning of markdown text.
/// Returns (frontmatter, remaining_text). If no frontmatter, returns None for frontmatter.
fn extract_frontmatter(text: &str) -> (Option<NoteFrontmatter>, String) {
    if !text.starts_with("---\n") {
        return (None, text.to_string());
    }

    if let Some(end_pos) = text[4..].find("\n---\n") {
        let frontmatter_str = &text[4..4 + end_pos];
        let remaining = &text[4 + end_pos + 5..].to_string();

        match serde_yaml::from_str::<NoteFrontmatter>(frontmatter_str) {
            Ok(fm) => (Some(fm), remaining.to_string()),
            Err(_) => (None, text.to_string()),
        }
    } else {
        (None, text.to_string())
    }
}

/// Create YAML frontmatter from note metadata.
fn create_frontmatter(note: &Note) -> NoteFrontmatter {
    NoteFrontmatter {
        id: note.id.0.to_string(),
        created_by: format!("{:?}", note.created_by),
        created_at: note.created_at.to_string(),
        metadata: note.metadata.clone(),
    }
}

/// Serialize a single block to markdown. Rich block types are converted to
/// standard markdown (tables as GFM, math as $$, images as ![]()), and complex
/// blocks use HTML comments + fallback text so they're readable outside the app.
fn block_content_to_markdown(content: &BlockContent) -> String {
    match content {
        BlockContent::Markdown { text } => text.clone(),

        BlockContent::Math {
            expression,
            display_mode,
        } => {
            if *display_mode {
                format!("$$\n{}\n$$", expression)
            } else {
                format!("${expression}$")
            }
        }

        BlockContent::Media {
            hash_or_url,
            alt_text,
            media_type: _,
        } => format!("![{alt_text}]({hash_or_url})"),

        BlockContent::Table { headers, rows } => serialize_table(headers, rows),

        BlockContent::InternalEmbed {
            target,
            fallback_text,
        } => {
            // Internal embeds use HTML comment + blockquote fallback
            format!(
                "<!-- embed:internal:{:?} -->\n\n> **[Embedded: {}]**\n> \n> {}\n> \n> *[This is a dynamic view embedded here. Open in app to interact.]*",
                target,
                target_display_name(target),
                fallback_text.lines().map(|l| format!("> {}", l)).collect::<Vec<_>>().join("\n")
            )
        }

        BlockContent::ExternalEmbed { url, provider } => {
            // External embeds use HTML comment + link fallback
            format!(
                "<!-- embed:external:{}:{} -->\n\n[{}]({}) *[{}]*",
                provider.domain(),
                url,
                url,
                url,
                match provider {
                    EmbedProvider::YouTube => "YouTube Video",
                    EmbedProvider::Twitter => "Tweet",
                    EmbedProvider::GoogleDrive => "Google Drive Document",
                    EmbedProvider::Generic => "External Embed",
                }
            )
        }
    }
}

/// Serialize a table as GitHub-flavored markdown.
fn serialize_table(headers: &[String], rows: &[Vec<String>]) -> String {
    let mut result = String::new();

    // Header row
    result.push('|');
    for header in headers {
        result.push(' ');
        result.push_str(header);
        result.push_str(" |");
    }
    result.push('\n');

    // Separator row
    result.push('|');
    for _ in headers {
        result.push_str(" --- |");
    }
    result.push('\n');

    // Data rows
    for row in rows {
        result.push('|');
        for (i, cell) in row.iter().enumerate() {
            result.push(' ');
            // Escape pipes in cell content
            result.push_str(&cell.replace('|', "\\|"));
            result.push_str(" |");
            if i >= headers.len() - 1 {
                break;
            }
        }
        result.push('\n');
    }

    result.trim_end().to_string()
}

/// Get a display name for an ObjectRef for use in fallback text.
fn target_display_name(target: &crate::id::ObjectRef) -> String {
    match target {
        crate::id::ObjectRef::Note(id) => format!("Note {}", id),
        crate::id::ObjectRef::View(id) => format!("View {}", id),
        crate::id::ObjectRef::Block(id) => format!("Block {}", id),
        crate::id::ObjectRef::Entity(id) => format!("Entity {}", id),
        crate::id::ObjectRef::Link(id) => format!("Link {}", id),
        crate::id::ObjectRef::Source(id) => format!("Source {}", id),
    }
}

/// Convert blocks to markdown text. Each block is serialized to standard markdown
/// so the file remains readable in any markdown editor. Block IDs are preserved
/// as HTML comments for round-tripping.
pub fn blocks_to_markdown(blocks: &[Block]) -> String {
    let mut md = String::new();

    for block in blocks {
        // Serialize block content
        let content_md = block_content_to_markdown(&block.content);
        md.push_str(&content_md);
        md.push_str("\n\n");

        // Add a block id reference as an HTML comment for round-tripping
        md.push_str(&format!("<!-- block:{} -->\n\n", block.id));
    }

    md.trim_end().to_string()
}

/// Deserialize a table from markdown-like content.
/// This is a best-effort parser for GFM tables found in markdown.
fn deserialize_table(text: &str) -> Option<(Vec<String>, Vec<Vec<String>>)> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 3 {
        return None;
    }

    // Try to parse first line as header row
    let header_line = lines[0];
    if !header_line.starts_with('|') || !header_line.ends_with('|') {
        return None;
    }

    let headers: Vec<String> = header_line
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|s| s.trim().to_string())
        .collect();

    // Check for separator line (line 2)
    let sep_line = lines[1];
    if !sep_line.contains("---") {
        return None;
    }

    // Parse data rows
    let mut rows = Vec::new();
    for line in &lines[2..] {
        if !line.starts_with('|') || !line.ends_with('|') {
            break;
        }

        let row: Vec<String> = line
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(|s| s.trim().replace("\\|", "|"))
            .collect();

        if row.len() == headers.len() {
            rows.push(row);
        }
    }

    Some((headers, rows))
}

/// Parse markdown into blocks. Recognizes rich block types (tables, math, media)
/// and converts them back to their BlockContent variants. Falls back to Markdown
/// blocks for unrecognized content. Block IDs (<!-- block:uuid -->) are associated
/// with the block that follows them.
pub fn markdown_to_blocks(text: &str, note_id: NoteId) -> Result<Vec<Block>, String> {
    let mut blocks = Vec::new();
    let mut order = 1.0_f32;
    let now = Timestamp::now_utc();

    let lines = text.lines().collect::<Vec<_>>();
    let mut i = 0;
    let mut pending_block_id: Option<BlockId> = None;

    while i < lines.len() {
        let line = lines[i];

        // Check for block ID comment — if found, store it for the next block
        if line.starts_with("<!-- block:") && line.ends_with(" -->") {
            let id_str = line
                .trim_start_matches("<!-- block:")
                .trim_end_matches(" -->");
            if let Ok(uuid) = uuid::Uuid::parse_str(id_str) {
                pending_block_id = Some(BlockId(uuid));
            }
            i += 1;
            continue;
        }

        // Skip blank lines
        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        // Collect block content until next blank line or block ID comment
        let mut block_text = String::new();
        while i < lines.len() {
            let current = lines[i];
            if current.is_empty() || (current.starts_with("<!-- block:") && current.ends_with(" -->")) {
                break;
            }
            if !block_text.is_empty() {
                block_text.push('\n');
            }
            block_text.push_str(current);
            i += 1;
        }

        if block_text.trim().is_empty() {
            continue;
        }

        // Try to parse the block content as a rich type; fall back to Markdown
        let content = if let Some((headers, rows)) = deserialize_table(&block_text) {
            BlockContent::Table { headers, rows }
        } else if block_text.starts_with("$$") && block_text.ends_with("$$") {
            let expr = block_text
                .trim_start_matches("$$")
                .trim_end_matches("$$")
                .trim()
                .to_string();
            BlockContent::Math {
                expression: expr,
                display_mode: true,
            }
        } else if let Some(caps) = try_parse_inline_math(&block_text) {
            BlockContent::Math {
                expression: caps,
                display_mode: false,
            }
        } else if let Some((url, alt)) = try_parse_image(&block_text) {
            BlockContent::Media {
                hash_or_url: url,
                alt_text: alt,
                media_type: MediaType::Image,
            }
        } else {
            BlockContent::Markdown {
                text: block_text.trim().to_string(),
            }
        };

        let block = Block {
            id: pending_block_id.take().unwrap_or_else(BlockId::new),
            note_id,
            content,
            order,
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        };
        blocks.push(block);
        order += 1.0;
    }

    Ok(blocks)
}

/// Try to parse inline math (e.g., $expression$).
fn try_parse_inline_math(text: &str) -> Option<String> {
    if text.starts_with('$') && text.ends_with('$') && !text.starts_with("$$") {
        Some(text[1..text.len() - 1].to_string())
    } else {
        None
    }
}

/// Try to parse image markdown (e.g., ![alt](url)).
fn try_parse_image(text: &str) -> Option<(String, String)> {
    if !text.starts_with("![") {
        return None;
    }
    if let Some(close_bracket) = text.find(']') {
        if close_bracket + 1 < text.len() && text.chars().nth(close_bracket + 1) == Some('(') {
            if let Some(close_paren) = text[close_bracket + 2..].find(')') {
                let alt = text[2..close_bracket].to_string();
                let url = text[close_bracket + 2..close_bracket + 2 + close_paren].to_string();
                return Some((url, alt));
            }
        }
    }
    None
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
/// Includes YAML frontmatter with note metadata, then the title as a level-1 heading, followed by blocks.
pub fn note_to_markdown(note: &Note, blocks: &[Block]) -> String {
    let mut md = String::new();

    // Write YAML frontmatter
    let frontmatter = create_frontmatter(note);
    if let Ok(yaml_str) = serde_yaml::to_string(&frontmatter) {
        md.push_str("---\n");
        md.push_str(&yaml_str);
        md.push_str("---\n\n");
    }

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
/// Extracts YAML frontmatter if present (uses id and metadata from it).
/// Falls back to provided note_id and defaults if no frontmatter.
/// Extracts the title from the first heading; creates blocks from paragraphs.
pub fn markdown_to_note(
    text: &str,
    note_id: NoteId,
    actor: Actor,
    created_at: Timestamp,
) -> Result<(Note, Vec<Block>), String> {
    // Extract frontmatter if present
    let (frontmatter, remaining_text) = extract_frontmatter(text);

    // Use frontmatter id if available, otherwise use provided note_id
    let final_note_id = if let Some(ref fm) = frontmatter {
        uuid::Uuid::parse_str(&fm.id)
            .map(NoteId)
            .unwrap_or(note_id)
    } else {
        note_id
    };

    // Use frontmatter metadata if available, otherwise empty
    let metadata = frontmatter
        .as_ref()
        .map(|fm| fm.metadata.clone())
        .unwrap_or_default();

    // Extract title from remaining markdown
    let title = extract_title(&remaining_text).unwrap_or_default();

    // Parse blocks from markdown (without title line)
    let mut blocks_text = remaining_text.to_string();
    if let Some(newline_pos) = remaining_text.find('\n') {
        blocks_text = remaining_text[newline_pos + 1..].to_string();
    }

    let mut parsed_blocks = markdown_to_blocks(&blocks_text, final_note_id)?;

    // Update blocks to use the provided actor and timestamp
    for block in &mut parsed_blocks {
        block.created_by = actor.clone();
        block.created_at = created_at;
    }

    let block_ids: Vec<BlockId> = parsed_blocks.iter().map(|b| b.id).collect();

    let note = Note {
        id: final_note_id,
        title,
        blocks: block_ids,
        metadata,
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

    #[test]
    fn yaml_frontmatter_round_trip() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();
        let mut metadata = BTreeMap::new();
        metadata.insert("project".to_string(), serde_json::json!("MyProject"));
        metadata.insert("priority".to_string(), serde_json::json!(1));

        let original_note = Note {
            id: note_id,
            title: "Test Note with Metadata".to_string(),
            blocks: vec![],
            metadata,
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

        // Export to markdown with frontmatter
        let exported = note_to_markdown(&original_note, &blocks);

        // Verify frontmatter is present
        assert!(exported.starts_with("---\n"));
        assert!(exported.contains(&note_id.0.to_string()));
        assert!(exported.contains("MyProject"));

        // Re-parse the markdown
        let dummy_id = NoteId::new();
        let (imported_note, _imported_blocks) =
            markdown_to_note(&exported, dummy_id, Actor::User, now).expect("parse note");

        // Verify ID and metadata are preserved
        assert_eq!(imported_note.id, original_note.id);
        assert_eq!(imported_note.title, original_note.title);
        assert_eq!(imported_note.metadata, original_note.metadata);
    }

    #[test]
    fn serialize_math_block_display_mode() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        let blocks = vec![Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::Math {
                expression: "E = mc^2".to_string(),
                display_mode: true,
            },
            order: 1.0,
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        }];

        let md = blocks_to_markdown(&blocks);
        assert!(md.contains("$$\nE = mc^2\n$$"));
    }

    #[test]
    fn parse_math_block_display_mode() {
        let note_id = NoteId::new();
        let text = "$$\nE = mc^2\n$$";

        let blocks = markdown_to_blocks(text, note_id).expect("parse");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].content {
            BlockContent::Math {
                expression,
                display_mode,
            } => {
                assert_eq!(expression, "E = mc^2");
                assert!(*display_mode);
            }
            _ => panic!("Expected Math block"),
        }
    }

    #[test]
    fn serialize_table_block() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        let blocks = vec![Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::Table {
                headers: vec!["Name".to_string(), "Price".to_string()],
                rows: vec![
                    vec!["Apple".to_string(), "$1".to_string()],
                    vec!["Orange".to_string(), "$2".to_string()],
                ],
            },
            order: 1.0,
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        }];

        let md = blocks_to_markdown(&blocks);
        assert!(md.contains("| Name | Price |"));
        assert!(md.contains("| Apple | $1 |"));
        assert!(md.contains("| Orange | $2 |"));
    }

    #[test]
    fn parse_table_block() {
        let note_id = NoteId::new();
        let text = "| Name | Price |\n| --- | --- |\n| Apple | $1 |\n| Orange | $2 |";

        let blocks = markdown_to_blocks(text, note_id).expect("parse");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].content {
            BlockContent::Table { headers, rows } => {
                assert_eq!(headers, &vec!["Name".to_string(), "Price".to_string()]);
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0], vec!["Apple".to_string(), "$1".to_string()]);
            }
            _ => panic!("Expected Table block"),
        }
    }

    #[test]
    fn serialize_media_block() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        let blocks = vec![Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::Media {
                hash_or_url: "image-hash-abc123.png".to_string(),
                alt_text: "A beautiful sunset".to_string(),
                media_type: MediaType::Image,
            },
            order: 1.0,
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        }];

        let md = blocks_to_markdown(&blocks);
        assert!(md.contains("![A beautiful sunset](image-hash-abc123.png)"));
    }

    #[test]
    fn parse_media_block() {
        let note_id = NoteId::new();
        let text = "![A sunset](sunset.png)";

        let blocks = markdown_to_blocks(text, note_id).expect("parse");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].content {
            BlockContent::Media {
                hash_or_url,
                alt_text,
                media_type,
            } => {
                assert_eq!(hash_or_url, "sunset.png");
                assert_eq!(alt_text, "A sunset");
                assert_eq!(*media_type, MediaType::Image);
            }
            _ => panic!("Expected Media block"),
        }
    }

    #[test]
    fn serialize_external_embed_block() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        let blocks = vec![Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::ExternalEmbed {
                url: "https://youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
                provider: EmbedProvider::YouTube,
            },
            order: 1.0,
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        }];

        let md = blocks_to_markdown(&blocks);
        assert!(md.contains("<!-- embed:external:youtube.com:https://youtube.com/watch?v=dQw4w9WgXcQ -->"));
        assert!(md.contains("youtube.com"));
        assert!(md.contains("[https://youtube.com/watch?v=dQw4w9WgXcQ](https://youtube.com/watch?v=dQw4w9WgXcQ)"));
    }

    #[test]
    fn table_with_pipe_characters_escapes_properly() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        let blocks = vec![Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::Table {
                headers: vec!["Code".to_string()],
                rows: vec![vec!["if a | b".to_string()]],
            },
            order: 1.0,
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        }];

        let md = blocks_to_markdown(&blocks);
        assert!(md.contains("if a \\| b"));
    }
}
