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
use crate::id::{BlockId, NoteId, SourceId};
use crate::ingestion::IngestionState;
use crate::note::Note;
use crate::source::{Source, SourceOrigin};
use crate::{Actor, Timestamp};
use std::collections::BTreeMap;

/// A minimal example [`Source`] (a pasted-text capture).
pub fn sample_source() -> Source {
    let raw = "Raw captured text that must never be overwritten.";
    Source {
        id: SourceId::new(),
        origin: SourceOrigin::PastedText,
        title: Some("Example source".to_string()),
        raw_content: raw.to_string(),
        captured_at: Timestamp::now_utc(),
        content_hash: "5b2c6b6c1d5e0a1b7c9d3e8f2a4c6e9a".to_string(),
        ingestion_state: IngestionState::Captured,
        created_by: Actor::User,
    }
}

/// A minimal example [`Block`] (markdown text).
pub fn sample_block() -> Block {
    Block {
        id: BlockId::new(),
        note_id: NoteId::new(),
        content: BlockContent::Markdown {
            text: "Example block content".to_string(),
        },
        order: 1.0,
        created_by: Actor::User,
        created_at: Timestamp::now_utc(),
        source_provenance_ref: None,
    }
}

/// A minimal example [`Note`] with a single block.
pub fn sample_note() -> Note {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "example_key".to_string(),
        serde_json::json!("example_value"),
    );

    Note {
        id: NoteId::new(),
        title: "Example note".to_string(),
        blocks: vec![BlockId::new()],
        metadata,
        created_by: Actor::User,
        created_at: Timestamp::now_utc(),
    }
}

// TODO(fixtures): add sample_entity(), sample_typed_link(),
// sample_summary_proposal(), sample_agent_action() as those types gain
// their full fields (tasks C5, D1-D2). Tests in storage, agent, search,
// and ingestion should consume these.
