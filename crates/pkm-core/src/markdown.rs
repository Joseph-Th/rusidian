//! Markdown import/export for notes and blocks.
//!
//! Pure, testable functions to convert between Note/Block structures and markdown.
//! These are stateless and keep all IO at the boundary.
//!
//! The markdown format includes:
//! - Title as a level-1 heading (# Title)
//! - Blocks as paragraphs separated by blank lines
//! - Block IDs preserved as HTML comments for round-tripping
//! - Rich blocks (tables, math, media) serialized as standard markdown
//! - Complex UI (views, Kanban) serialized with fallback text + HTML comments
//! - Note metadata as YAML front matter

use crate::block::{Block, BlockContent};
use crate::id::{BlockId, NoteId, ObjectRef};
use crate::media::{EmbedProvider, MediaType};
use crate::note::{Note, NoteMetadata};
use crate::{Actor, Timestamp};
use serde::{Deserialize, Serialize};

/// YAML frontmatter for notes. Serialized as the first block in markdown files.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NoteFrontmatter {
    id: String,
    created_by: Actor,
    #[serde(with = "time::serde::rfc3339")]
    created_at: Timestamp,
    #[serde(default)]
    metadata: NoteMetadata,
}

/// Extract YAML frontmatter from the beginning of markdown text.
/// Returns Ok((frontmatter, remaining_text)).
/// - If frontmatter exists and parses: Ok(Some(fm), remaining)
/// - If no frontmatter delimiters: Ok(None, text)
/// - If frontmatter exists but is invalid YAML: Err("...parse error...")
/// Supports both Unix (\n) and Windows (\r\n) line endings.
fn extract_frontmatter(text: &str) -> Result<(Option<NoteFrontmatter>, String), String> {
    if !text.starts_with("---\n") && !text.starts_with("---\r\n") {
        return Ok((None, text.to_string()));
    }

    // Skip past the opening ---\n or ---\r\n
    let after_opener = if text.starts_with("---\r\n") {
        &text[5..]
    } else {
        &text[4..]
    };

    // Find the closing ---\n or ---\r\n.
    // Search for \n---\n or \n---\r\n; avoid matching the \n inside \r\n.
    let close_pos = after_opener
        .find("\n---\n")
        .or_else(|| after_opener.find("\n---\r\n"));

    if let Some(pos) = close_pos {
        let frontmatter_raw = &after_opener[..pos];

        let close_len = if after_opener[pos..].starts_with("\n---\r\n") {
            6
        } else {
            5
        };
        let remaining = &after_opener[pos + close_len..];

        // Normalize line endings only for the YAML block, not the whole doc
        let frontmatter_str = frontmatter_raw.replace("\r\n", "\n");

        match serde_yaml::from_str::<NoteFrontmatter>(&frontmatter_str) {
            Ok(fm) => Ok((Some(fm), remaining.to_string())),
            Err(e) => Err(format!(
                "Invalid YAML frontmatter: {}. Content:\n{}",
                e, frontmatter_raw
            )),
        }
    } else {
        Err("Unclosed YAML frontmatter: started with --- but no closing --- found".to_string())
    }
}

/// Create YAML frontmatter from note metadata.
fn create_frontmatter(note: &Note) -> NoteFrontmatter {
    NoteFrontmatter {
        id: note.id.0.to_string(),
        created_by: note.created_by.clone(),
        created_at: note.created_at,
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
            media_type,
        } => {
            let image_md = format!("![{alt_text}]({hash_or_url})");
            match media_type {
                MediaType::Image => image_md,
                non_image => format!("<!-- media:{:?} -->\n{}", non_image, image_md),
            }
        }

        BlockContent::Table { headers, rows } => serialize_table(headers, rows),

        BlockContent::InternalEmbed {
            target,
            fallback_text,
        } => {
            // Internal embeds use HTML comment + blockquote fallback
            format!(
                "<!-- embed:internal:{} -->\n> **[Embedded: {}]**\n> \n> {}\n> \n> *[This is a dynamic view embedded here. Open in app to interact.]*",
                object_ref_to_embed_string(target),
                target_display_name(target),
                fallback_text.lines().map(|l| format!("> {}", l)).collect::<Vec<_>>().join("\n")
            )
        }

        BlockContent::ExternalEmbed { url, provider } => {
            // External embeds use HTML comment + link fallback
            format!(
                "<!-- embed:external:{}:{} -->\n[{}]({}) *[{}]*",
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
/// Pads shorter rows with empty cells and preserves extra cells in longer rows
/// (padding headers with empty strings as needed) to prevent data loss.
fn serialize_table(headers: &[String], rows: &[Vec<String>]) -> String {
    let mut result = String::new();

    if headers.is_empty() && rows.iter().all(|r| r.is_empty()) {
        return result;
    }

    // Determine the maximum column count across headers and all rows
    let max_cols = rows.iter().fold(headers.len(), |max, row| max.max(row.len()));

    // Pad headers to max_cols if rows have more columns
    let padded_headers: Vec<&str> = headers.iter().map(|s| s.as_str()).chain(std::iter::repeat("")).take(max_cols).collect();

    // Header row
    result.push('|');
    for header in &padded_headers {
        result.push(' ');
        result.push_str(header);
        result.push_str(" |");
    }
    result.push('\n');

    // Separator row
    result.push('|');
    for _ in &padded_headers {
        result.push_str(" --- |");
    }
    result.push('\n');

    // Data rows
    for row in rows {
        result.push('|');
        for i in 0..max_cols {
            result.push(' ');
            let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
            let sanitized = cell.replace('|', "\\|").replace('\n', "<br>");
            result.push_str(&sanitized);
            result.push_str(" |");
        }
        result.push('\n');
    }

    result.trim_end().to_string()
}

fn object_ref_to_embed_string(target: &ObjectRef) -> String {
    match target {
        ObjectRef::Source(id) => format!("source:{}", id),
        ObjectRef::Note(id) => format!("note:{}", id),
        ObjectRef::Block(id) => format!("block:{}", id),
        ObjectRef::Entity(id) => format!("entity:{}", id),
        ObjectRef::Link(id) => format!("link:{}", id),
        ObjectRef::View(id) => format!("view:{}", id),
    }
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
        // Add a block id reference as an HTML comment for round-tripping.
        // Format: <!-- block:uuid -->
        md.push_str(&format!("<!-- block:{} -->\n\n", block.id));

        // Serialize block content
        let content_md = block_content_to_markdown(&block.content);
        md.push_str(&content_md);
        md.push_str("\n\n");
    }

    md.trim_end().to_string()
}

/// Deserialize a table from markdown-like content.
/// This is a best-effort parser for GFM tables found in markdown.
/// Properly handles escaped pipes: \| is treated as literal content, not a separator.
/// Tolerates missing leading/trailing pipes and varying separator styles.
fn deserialize_table(text: &str) -> Option<(Vec<String>, Vec<Vec<String>>)> {
    let raw_lines: Vec<&str> = text.lines().collect();
    if raw_lines.len() < 3 {
        return None;
    }

    // Trim each line and normalize: add leading/trailing pipe if missing so
    // split_table_row works uniformly.
    let normalize_row = |line: &str| -> String {
        let t = line.trim();
        let mut s = if t.starts_with('|') {
            t.to_string()
        } else {
            format!("|{}", t)
        };
        if !s.ends_with('|') {
            s.push('|');
        }
        s
    };

    let header_line = normalize_row(raw_lines[0]);

    let headers: Vec<String> = split_table_row(&header_line)
        .into_iter()
        .map(|s| s.trim().to_string())
        .collect();

    if headers.is_empty() {
        return None;
    }

    // Check for separator line (line 2) — must contain at least one dash segment
    let sep_line = raw_lines[1].trim();
    if !sep_line.contains('-') {
        return None;
    }

    // Parse data rows
    let mut rows = Vec::new();
    for line in &raw_lines[2..] {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.contains('|') {
            break;
        }

        let normal = normalize_row(trimmed);
        let mut row: Vec<String> = split_table_row(&normal)
            .into_iter()
            .map(|s| s.trim().replace("\\|", "|").replace("<br>", "\n"))
            .collect();

        row.resize(headers.len(), String::new());
        rows.push(row);
    }

    Some((headers, rows))
}

/// Split a table row on unescaped pipes.
/// Pipes preceded by backslash (\|) are not treated as separators.
fn split_table_row(line: &str) -> Vec<String> {
    let line = line.trim_start_matches('|').trim_end_matches('|');
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'|') {
            current.push('\\');
            current.push('|');
            chars.next();
        } else if ch == '|' {
            parts.push(current);
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    parts.push(current);
    parts
}

/// Parse markdown into blocks. Recognizes rich block types (tables, math, media)
/// and converts them back to their BlockContent variants. Falls back to Markdown
/// blocks for unrecognized content. Block IDs are preserved from HTML comments:
/// <!-- block:uuid -->
///
/// Uses an internal pure function with parameterized ID generation to ensure testability.
pub fn markdown_to_blocks(text: &str, note_id: NoteId) -> Result<Vec<Block>, String> {
    markdown_to_blocks_internal(text, note_id, &mut BlockId::new)
}

/// Internal implementation with parameterized ID generation for testability.
/// Allows tests to inject deterministic ID generators instead of using system timestamps.
fn markdown_to_blocks_internal(
    text: &str,
    note_id: NoteId,
    id_gen: &mut dyn FnMut() -> BlockId,
) -> Result<Vec<Block>, String> {
    let mut blocks = Vec::new();
    let now = Timestamp::now_utc();

    let lines = text.lines().collect::<Vec<_>>();
    let mut i = 0;
    let mut pending_block_id: Option<BlockId> = None;

    while i < lines.len() {
        let line = lines[i];

        // Check for block ID comment: <!-- block:uuid -->
        if line.starts_with("<!-- block:") && line.ends_with(" -->") {
            let comment_content = line
                .trim_start_matches("<!-- block:")
                .trim_end_matches(" -->");

            // Split on space to extract id (we ignore any extra legacy order info)
            let parts: Vec<&str> = comment_content.split_whitespace().collect();

            if let Some(id_str) = parts.first() {
                if let Ok(uuid) = uuid::Uuid::parse_str(id_str) {
                    pending_block_id = Some(BlockId(uuid));
                }
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
        let mut in_math_fence = false;
        let mut in_code_fence = false;

        while i < lines.len() {
            let current = lines[i];
            let trimmed = current.trim();

            // Toggle fences if we hit a boundary line
            if trimmed.starts_with("```") {
                if !trimmed[3..].ends_with("```") || trimmed == "```" {
                    in_code_fence = !in_code_fence;
                }
            }
            if trimmed.starts_with("$$") {
                if !trimmed[2..].ends_with("$$") || trimmed == "$$" {
                    in_math_fence = !in_math_fence;
                }
            }

            let is_fence_active = in_code_fence || in_math_fence;

            if !is_fence_active && (current.is_empty() || (current.starts_with("<!-- block:") && current.ends_with(" -->"))) {
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
        let content = if let Some((media_type, image_text)) = try_extract_media_type_comment(&block_text) {
            if let Some((url, alt)) = try_parse_image(&image_text) {
                BlockContent::Media {
                    hash_or_url: url,
                    alt_text: alt,
                    media_type,
                }
            } else {
                // Media type comment present but no image follows — treat as markdown
                BlockContent::Markdown {
                    text: block_text.trim().to_string(),
                }
            }
        } else if let Some((headers, rows)) = deserialize_table(&block_text) {
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
        } else if let Some((target, fallback)) = try_parse_internal_embed(&block_text) {
            BlockContent::InternalEmbed {
                target,
                fallback_text: fallback,
            }
        } else if let Some((url, provider)) = try_parse_external_embed(&block_text) {
            BlockContent::ExternalEmbed {
                url,
                provider,
            }
        } else {
            BlockContent::Markdown {
                text: block_text.trim().to_string(),
            }
        };

        let block = Block {
            id: pending_block_id.take().unwrap_or_else(|| id_gen()),
            note_id,
            content,
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

/// Try to parse internal embed (e.g., <!-- embed:internal:view:uuid -->).
fn try_parse_internal_embed(text: &str) -> Option<(ObjectRef, String)> {
    let trimmed = text.trim();
    if !trimmed.starts_with("<!-- embed:internal:") {
        return None;
    }
    
    let end_comment = trimmed.find(" -->")?;
    let inner = &trimmed["<!-- embed:internal:".len()..end_comment];
    
    let parts: Vec<&str> = inner.split(':').collect();
    if parts.len() < 2 { return None; }
    
    let obj_type = parts[0];
    let obj_id = parts[1..].join(":");
    let fallback_raw = trimmed[end_comment + 4..].trim().to_string();
    // Strip generated > blockquote prefixes and boilerplate to prevent
    // exponential corruption on re-serialization
    let fallback = fallback_raw
        .lines()
        .map(|l| {
            let s = l.trim_start();
            if s.starts_with("> ") {
                &s[2..]
            } else if s == ">" {
                ""
            } else {
                s
            }
        })
        .filter(|l| {
            !l.trim_start().starts_with("**[Embedded:")
            && !l.trim_start().starts_with("*[This is a dynamic view")
        })
        .collect::<Vec<&str>>()
        .join("\n")
        .trim()
        .to_string();
    
    if let Ok(uuid) = uuid::Uuid::parse_str(&obj_id) {
        let target = match obj_type {
            "note" => ObjectRef::Note(crate::id::NoteId(uuid)),
            "view" => ObjectRef::View(crate::id::ViewId(uuid)),
            "entity" => ObjectRef::Entity(crate::id::EntityId(uuid)),
            "source" => ObjectRef::Source(crate::id::SourceId(uuid)),
            "block" => ObjectRef::Block(crate::id::BlockId(uuid)),
            _ => return None,
        };
        return Some((target, fallback));
    }
    None
}

/// Try to parse external embed.
fn try_parse_external_embed(text: &str) -> Option<(String, EmbedProvider)> {
    let trimmed = text.trim();
    if !trimmed.starts_with("<!-- embed:external:") {
        return None;
    }
    let end_comment = trimmed.find(" -->")?;
    let inner = &trimmed["<!-- embed:external:".len()..end_comment];
    
    if let Some(colon) = inner.find(':') {
        let url = inner[colon + 1..].to_string();
        let provider = EmbedProvider::from_url(&url);
        return Some((url, provider));
    }
    None
}

/// Try to parse inline math (e.g., $expression$).
fn try_parse_inline_math(text: &str) -> Option<String> {
    if text.starts_with('$') && text.ends_with('$') && !text.starts_with("$$") {
        Some(text[1..text.len() - 1].to_string())
    } else {
        None
    }
}

/// Try to extract a media type comment from the start of block text.
/// Returns (media_type, remaining_text) if a comment like `<!-- media:video -->` is found.
fn try_extract_media_type_comment(text: &str) -> Option<(MediaType, String)> {
    let trimmed = text.trim_start();
    if let Some(rest) = trimmed.strip_prefix("<!-- media:") {
        if let Some(end) = rest.find("-->") {
            let type_str = rest[..end].trim();
            let media_type = match type_str.to_lowercase().as_str() {
                "audio" => MediaType::Audio,
                "video" => MediaType::Video,
                "pdf" => MediaType::Pdf,
                _ => return None,
            };
            let remaining = rest[end + 3..].trim().to_string();
            if !remaining.is_empty() {
                return Some((media_type, remaining));
            }
        }
    }
    None
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
/// Returns (cleaned_title, exact_heading_line) so the caller can remove the exact line.
pub fn extract_title(text: &str) -> Option<(String, String)> {
    if let Some(first_line) = text.lines().find(|l| !l.trim().is_empty()) {
        if let Some(heading) = first_line.strip_prefix("# ") {
            return Some((heading.trim().to_string(), first_line.to_string()));
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
    // If frontmatter exists but is invalid YAML, propagate the error
    // to prevent silent metadata deletion
    let (frontmatter, remaining_text) = extract_frontmatter(text)?;

    // Use frontmatter values if available, otherwise use provided parameters
    let (final_note_id, final_actor, final_created_at, metadata) = if let Some(ref fm) = frontmatter {
        (
            uuid::Uuid::parse_str(&fm.id).map(NoteId).unwrap_or(note_id),
            fm.created_by.clone(),
            fm.created_at,
            fm.metadata.clone(),
        )
    } else {
        (note_id, actor, created_at, NoteMetadata::default())
    };

    // Extract title from remaining markdown
    let (title, heading_line) = extract_title(&remaining_text).unwrap_or_default();

    // Strip the title heading so it is not duplicated as a block
    let mut blocks_text = remaining_text.trim_start().to_string();
    if !title.is_empty() && !heading_line.is_empty() {
        blocks_text = blocks_text.replacen(&heading_line, "", 1).trim_start().to_string();
    }

    let mut parsed_blocks = markdown_to_blocks(&blocks_text, final_note_id)?;

    // Update blocks to use the historical actor and timestamp from frontmatter
    for block in &mut parsed_blocks {
        block.created_by = final_actor.clone();
        block.created_at = final_created_at;
    }

    let block_ids: Vec<BlockId> = parsed_blocks.iter().map(|b| b.id).collect();

    let note = Note {
        id: final_note_id,
        title,
        blocks: block_ids,
        metadata,
        created_by: final_actor,
        created_at: final_created_at,
        version: 1,
        updated_at: final_created_at,
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
        let (title, line) = extract_title(text).unwrap();
        assert_eq!(title, "My Note Title");
        assert_eq!(line, "# My Note Title");
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
            metadata: NoteMetadata::default(),
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
        let metadata = NoteMetadata {
            project: Some("MyProject".to_string()),
            priority: Some(1),
            ..Default::default()
        };

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
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        }];

        let md = blocks_to_markdown(&blocks);
        assert!(md.contains("if a \\| b"));
    }

    #[test]
    fn table_with_mismatched_row_lengths() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        // Row with fewer cells than headers should be padded with empty cells
        let blocks = vec![Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::Table {
                headers: vec!["A".to_string(), "B".to_string(), "C".to_string()],
                rows: vec![vec!["1".to_string(), "2".to_string()]], // Only 2 cells
            },
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        }];

        let md = blocks_to_markdown(&blocks);
        // Row should be padded to 3 cells: "| 1 | 2 |  |"
        assert!(md.contains("| 1 | 2 |  |"), "Short row should be padded with empty cells");
    }

    #[test]
    fn table_with_empty_cells() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        let blocks = vec![Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::Table {
                headers: vec!["Name".to_string(), "Value".to_string()],
                rows: vec![
                    vec!["Item1".to_string(), "".to_string()],
                    vec!["".to_string(), "100".to_string()],
                ],
            },
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        }];

        let md = blocks_to_markdown(&blocks);
        assert!(md.contains("| Item1 |  |")); // Empty cell rendered as just spaces
        assert!(md.contains("|  | 100 |")); // Empty cell rendered as just spaces
    }

    #[test]
    fn empty_table_renders_without_data() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        let blocks = vec![Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::Table {
                headers: vec!["A".to_string(), "B".to_string()],
                rows: vec![],
            },
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        }];

        let md = blocks_to_markdown(&blocks);
        assert!(md.contains("| A | B |")); // Headers still render
        assert!(md.contains("| --- | --- |")); // Separator still renders
        // Should contain header, separator, comment, but no data
        let line_count = md.lines().count();
        assert!(line_count >= 3, "Table should have at least header, separator, and comment");
    }

    #[test]
    fn malformed_image_syntax_falls_back_to_markdown() {
        let note_id = NoteId::new();
        let text = "![alt without close bracket(url)";

        let blocks = markdown_to_blocks(text, note_id).expect("parse");
        assert_eq!(blocks.len(), 1);
        // Should fall back to Markdown block
        match &blocks[0].content {
            BlockContent::Markdown { .. } => (),
            other => panic!("Expected Markdown block, got {:?}", other),
        }
    }

    #[test]
    fn valid_image_with_special_characters_in_alt() {
        let note_id = NoteId::new();
        let text = "![My (special) & image](image.png)";

        let blocks = markdown_to_blocks(text, note_id).expect("parse");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].content {
            BlockContent::Media {
                hash_or_url,
                alt_text,
                media_type,
            } => {
                assert_eq!(hash_or_url, "image.png");
                assert_eq!(alt_text, "My (special) & image");
                assert_eq!(*media_type, MediaType::Image);
            }
            _ => panic!("Expected Media block"),
        }
    }

    #[test]
    fn math_with_newlines_in_display_mode() {
        let note_id = NoteId::new();
        let now = Timestamp::now_utc();

        let blocks = vec![Block {
            id: BlockId::new(),
            note_id,
            content: BlockContent::Math {
                expression: "x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}".to_string(),
                display_mode: true,
            },
            created_by: Actor::User,
            created_at: now,
            source_provenance_ref: None,
            version: 1,
            updated_at: now,
        }];

        let md = blocks_to_markdown(&blocks);
        assert!(md.contains("$$"));
        assert!(md.contains("x = \\frac"));
    }

    #[test]
    fn inline_math_with_backslashes() {
        let note_id = NoteId::new();
        let text = "$\\alpha + \\beta$";

        let blocks = markdown_to_blocks(text, note_id).expect("parse");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].content {
            BlockContent::Math {
                expression,
                display_mode,
            } => {
                assert_eq!(expression, "\\alpha + \\beta");
                assert!(!display_mode);
            }
            _ => panic!("Expected Math block"),
        }
    }
}
