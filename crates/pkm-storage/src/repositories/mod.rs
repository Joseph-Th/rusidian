//! SQLite implementations of the `pkm_core::ports` repository traits.
//!
//! One module per aggregate. Each holds a `Sqlite<Aggregate>Repo` that wraps a
//! connection handle and implements the corresponding core port. SQL stays in
//! these files; nothing outside `pkm-storage` sees it.
//!
//! Round-trip tests live in `tests/` and use `pkm_core::fixtures`.

pub mod sources;
pub mod notes;
pub mod agent_actions;

// TODO(B2): entities.rs, links.rs, views.rs as those ports are added to core.
