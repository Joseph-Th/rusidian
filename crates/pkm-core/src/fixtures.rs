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

use crate::id::SourceId;
use crate::ingestion::IngestionState;
use crate::source::{Source, SourceOrigin};
use crate::{Actor, Timestamp};

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

// TODO(fixtures): add sample_note(), sample_block_tree(), sample_entity(),
// sample_typed_link(), sample_summary_proposal(), sample_agent_action() as
// those types gain their full fields (tasks C1-C5, D1-D2). Tests in storage,
// agent, search, and ingestion should consume these.
