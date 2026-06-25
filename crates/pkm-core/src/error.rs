//! Core error type. Storage/agent/search crates define their own errors and
//! convert into or out of this as needed. No panics in application logic
//! (AGENTS.md Coding Standards) — return `Result` instead.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    /// A domain invariant was violated (e.g. empty required field, illegal
    /// state transition). Carries a human-readable explanation.
    #[error("invariant violated: {0}")]
    Invariant(String),

    /// A referenced object id was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Serialization / deserialization failure.
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}
