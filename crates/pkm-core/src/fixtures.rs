//! Shared test fixtures — concrete example objects.
//!
//! Cold agents drift less when they have real examples to test against. These
//! constructors build canonical instances of each primitive. They are available
//! in this crate's own tests, and in OTHER crates' tests via the `fixtures`
//! feature: `pkm-core = { workspace = true, features = ["fixtures"] }` under
//! `[dev-dependencies]`.
//!
//! Keep these type-checked (real constructors, not JSON blobs) so they cannot
//! drift away from the types. Extend as new primitives gain fields.
//!
//! NOTE: these call `..Id::new()` and read the clock for timestamps, so they
//! are NOT deterministic. Use them for round-trip/shape tests, not for tests
//! asserting exact ids/times.

use crate::block::{Block, BlockContent};
use crate::id::{BlockId, NoteId, ObjectRef, SourceId, ViewId};
use crate::ingestion::IngestionState;
use crate::media::{EmbedProvider, MediaType};
use crate::note::Note;
use crate::source::{Source, SourceOrigin};
use crate::{Actor, Timestamp};
use std::collections::BTreeMap;

/// A minimal example [`Source`] (a pasted-text capture).
pub fn sample_source() -> Source {
    let raw = "Raw captured text that must never be overwritten.";
    let now = Timestamp::now_utc();
    Source {
        id: SourceId::new(),
        origin: SourceOrigin::PastedText,
        title: Some("Example source".to_string()),
        raw_content: raw.to_string(),
        captured_at: now,
        content_hash: "5b2c6b6c1d5e0a1b7c9d3e8f2a4c6e9a".to_string(),
        ingestion_state: IngestionState::Captured,
        created_by: Actor::User,
        created_at: now,
        version: 1,
        updated_at: now,
    }
}

/// A minimal example [`Block`] (markdown text).
pub fn sample_block() -> Block {
    let now = Timestamp::now_utc();
    Block {
        id: BlockId::new(),
        note_id: NoteId::new(),
        content: BlockContent::Markdown {
            text: "Example block content".to_string(),
        },
        order: 1.0,
        created_by: Actor::User,
        created_at: now,
        source_provenance_ref: None,
        version: 1,
        updated_at: now,
    }
}

/// A minimal example [`Note`] with a single block.
pub fn sample_note() -> Note {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "example_key".to_string(),
        serde_json::json!("example_value"),
    );

    let now = Timestamp::now_utc();
    Note {
        id: NoteId::new(),
        title: "Example note".to_string(),
        blocks: vec![BlockId::new()],
        metadata,
        created_by: Actor::User,
        created_at: now,
        version: 1,
        updated_at: now,
    }
}

/// A [`Block`] containing a math equation (display mode).
pub fn sample_math_block() -> Block {
    let now = Timestamp::now_utc();
    Block {
        id: BlockId::new(),
        note_id: NoteId::new(),
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
    }
}

/// A [`Block`] containing a table.
pub fn sample_table_block() -> Block {
    let now = Timestamp::now_utc();
    Block {
        id: BlockId::new(),
        note_id: NoteId::new(),
        content: BlockContent::Table {
            headers: vec!["Name".to_string(), "Value".to_string()],
            rows: vec![
                vec!["Alpha".to_string(), "1".to_string()],
                vec!["Beta".to_string(), "2".to_string()],
            ],
        },
        order: 1.0,
        created_by: Actor::User,
        created_at: now,
        source_provenance_ref: None,
        version: 1,
        updated_at: now,
    }
}

/// A [`Block`] containing an image.
pub fn sample_image_block() -> Block {
    let now = Timestamp::now_utc();
    Block {
        id: BlockId::new(),
        note_id: NoteId::new(),
        content: BlockContent::Media {
            hash_or_url: "image-abc123.png".to_string(),
            alt_text: "Example image".to_string(),
            media_type: MediaType::Image,
        },
        order: 1.0,
        created_by: Actor::User,
        created_at: now,
        source_provenance_ref: None,
        version: 1,
        updated_at: now,
    }
}

/// A [`Block`] containing an embedded YouTube video.
pub fn sample_youtube_embed_block() -> Block {
    let now = Timestamp::now_utc();
    Block {
        id: BlockId::new(),
        note_id: NoteId::new(),
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
    }
}

/// A [`Block`] containing an internal embed (e.g., embedding another View).
pub fn sample_internal_embed_block() -> Block {
    let now = Timestamp::now_utc();
    Block {
        id: BlockId::new(),
        note_id: NoteId::new(),
        content: BlockContent::InternalEmbed {
            target: ObjectRef::View(ViewId::new()),
            fallback_text: "Embedded View: Project Dashboard".to_string(),
        },
        order: 1.0,
        created_by: Actor::User,
        created_at: now,
        source_provenance_ref: None,
        version: 1,
        updated_at: now,
    }
}

// TODO(fixtures): add sample_entity(), sample_typed_link(),
// sample_summary_proposal(), sample_agent_action() as those types gain
// their full fields (tasks C5, D1-D2). Tests in storage, agent, search,
// and ingestion should consume these.
