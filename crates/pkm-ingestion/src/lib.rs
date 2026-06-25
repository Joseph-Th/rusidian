//! Ingestion pipeline (AGENTS.md "Ingestion Pipeline").
//!
//! Ingestion is a PIPELINE WITH EXPLICIT STATES, not an inbox folder. Each
//! stage produces derived content that links back to the immutable raw source.
//!
//! INVARIANTS:
//! - Raw capture is immutable unless the user explicitly edits it.
//! - Cleanup/summary/entity/link outputs are derived + reviewable, never
//!   silently promoted.
//! - Failed ingestion preserves diagnostics to allow retry.

pub use pkm_core::ingestion::IngestionState;
