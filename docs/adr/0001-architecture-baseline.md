# ADR 0001 — Architecture baseline

Status: accepted · Date: 2026-06-25

## Decision

Adopt the architecture assumptions from `AGENTS.md` as the concrete baseline:

- **Language:** Rust. **App model:** local-first desktop.
- **UI shell:** Tauri (decision deferred until STATUS task A0 — record a
  follow-up ADR if a different shell is chosen).
- **Storage:** embedded **SQLite** (`rusqlite`, bundled), schema-versioned via
  ordered migrations. Structured tables are the source of truth; markdown is an
  import/export format, not the database.
- **Search:** SQLite **FTS5** for keyword first; semantic/vector search added
  later behind the `Retriever` trait.
- **Workspace shape:** one Cargo workspace, one crate per concern
  (`core`/`storage`/`search`/`agent`/`ingestion`, later `app`). `core` is
  pure (no IO); IO stays at the edges.
- **Agent interface:** typed in-process operations + an append-only agent action
  log, with diffs and rollback. MCP/tool exposure comes after the local API is
  stable.

## Context

`AGENTS.md` fixes the product direction (local-first, structured, agent-safe,
presentation-first) and forbids the Obsidian plugin/markdown-vault model. The
scaffolding must encode those invariants in *types and crate boundaries* so
later, narrower agents cannot drift off course.

## Alternatives considered

- **Single crate.** Rejected: blurs the pure-core / IO-edge boundary the
  coding standards require.
- **Markdown files as the data model.** Rejected by invariant #2 and the
  "Forbidden Shortcuts" list.
- **A heavyweight ORM.** Rejected: hides SQL, complicates migrations, risks
  owning user data. We keep SQL explicit.

## Consequences

- Adding the Tauri shell is intentionally deferred so the foundation crates can
  be built and tested without UI weight.
- Every schema change needs a migration; every agent mutation needs an
  auditable action. These are enforced by review against `STATUS.md`.
- Swapping the UI shell or adding a vector index later is localized (UI in
  `app`, retrieval behind `Retriever`) and does not require rewriting `core`.
