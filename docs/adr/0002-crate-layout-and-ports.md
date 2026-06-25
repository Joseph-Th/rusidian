# ADR 0002 — Crate naming, ports boundary, and cross-cutting types

Status: accepted · Date: 2026-06-25 · Supersedes parts of ADR 0001

## Context

Review of the initial scaffolding raised several structural risks for
cold-agent work: a crate literally named `core` (collides with Rust's std
`core`), cross-cutting concepts (content status, review state, provenance)
defined in the wrong layer, and an under-specified dependency graph that would
let `agent`/`search`/`ingestion` reach into SQLite.

## Decisions

1. **Crate family is `pkm-*`.** `core` is renamed `pkm-core` (lib `pkm_core`).
   Siblings: `pkm-storage`, `pkm-search`, `pkm-agent`, `pkm-ingestion`, later
   `pkm-app`. No crate is named `core`.

2. **Ports live in `pkm-core`.** Repository traits (`SourceRepo`, `NoteRepo`,
   …) and the `Retriever` trait live in `pkm_core::ports` and return
   `pkm_core::Result`. `pkm-storage` (and the SQLite `Retriever` impl) provide
   the implementations. Downstream crates depend on the **traits**, not on
   `pkm-storage`.

3. **Strict acyclic dependency graph.** `pkm-core` has no internal deps.
   `storage`/`search`/`agent`/`ingestion` depend only on `pkm-core`. Only
   `pkm-storage` may use `rusqlite` or write SQL. (`pkm-search` holds pure
   query/ranking logic; its SQLite-backed `Retriever` lives in `pkm-storage`.)

4. **Cross-cutting types belong in core.** `ContentStatus` and `Provenance`
   (`pkm_core::provenance`), `ReviewState` (`pkm_core::review`), `Actor`,
   `ObjectRef`, and the search query/hit/mode types are defined once in
   `pkm-core`. Search *filters* by content status; it does not define it.

5. **Invariant enums are authoritative, not frozen.** They are closed to casual
   edits. A variant changes only via an ADR; if the enum is persisted, the
   change also needs a migration. No stringly-typed escape-hatch fields.

6. **Storage layout + SQL discipline.** `pkm-storage` uses the
   `migrations/` + `src/db.rs` + `src/migrations.rs` + `src/repositories/*` +
   `tests/` layout. Migrations are append-only; `SCHEMA.md` mirrors the schema.
   Cold agents may write SQL only for a *bounded* repository task (existing
   domain type + explicit migration number + round-trip test).

7. **Shared fixtures.** `pkm_core::fixtures` (behind the `fixtures` feature)
   provides canonical example objects so tests across crates share real data.

## Consequences

- The db engine and search backend can be swapped without touching callers.
- Provenance/review/content-status cannot fork into per-crate definitions.
- `pkm-agent`/`pkm-ingestion`/`pkm-search` cannot accidentally depend on SQLite.
- More crates know about `pkm-core`, which is the intended single anchor.

## Alternatives considered

- *Keep `core` + define ports in `pkm-storage`.* Rejected: name collision risk,
  and it forces downstream crates to depend on the storage crate (pulling in
  rusqlite) just to see an interface.
- *ULIDs instead of UUIDv7.* Both are opaque + sortable; UUIDv7 is in the
  existing `uuid` dep and already serde-friendly. Revisit only if a concrete
  need (e.g. shorter ids in URLs) appears.
