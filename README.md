# rusidian

A local-first, Rust-based, **agent-native knowledge workbench**: structured
storage, source provenance, safe/audited agent operations, multi-mode
retrieval, and presentation-first views.

This is **not** an Obsidian clone, a markdown editor, or an AI chat app. See
[`AGENTS.md`](AGENTS.md) for the product thesis and non-negotiable invariants.

## Repo layout

```
crates/
  pkm-core/       domain model — types, ids, invariant enums, provenance,
                  review state, and the ports (traits). No IO, no deps.
  pkm-storage/    embedded SQLite persistence + migrations. Only crate with SQL.
  pkm-search/     pure query parsing + ranking.
  pkm-agent/      typed, audited, reversible agent operations.
  pkm-ingestion/  capture→promote pipeline state machine.
docs/adr/         architecture decision records.
SCHEMA.md         persisted-schema overview (mirrors migrations).
```

`crates/pkm-app` (Tauri desktop shell) is added later — see STATUS task A0.

### Dependency graph (strict, acyclic — see `docs/adr/0002`)

```
pkm-core   <- everything. No internal deps.
pkm-storage    -> pkm-core    implements core ports; OWNS rusqlite/SQL
pkm-search     -> pkm-core    pure logic; FTS impl lives in pkm-storage
pkm-agent      -> pkm-core    operates via &dyn ...Repo ports
pkm-ingestion  -> pkm-core
pkm-app        -> all         wires concrete impls to ports (later)
```

Only `pkm-storage` may touch SQLite. Cross-cutting types (provenance, review
state, content status, actor, ids, search query/hit) live in `pkm-core` so every
layer shares one definition.

## Working on this project

1. Read [`AGENTS.md`](AGENTS.md) — product invariants and forbidden shortcuts.
2. Read [`STATUS.md`](STATUS.md) — pick the next unblocked task.
3. Build / verify:

   ```sh
   cargo build
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```

The codebase is scaffolding: most crate functions `unimplemented!()` on purpose
and carry `TODO(<task-id>)` markers that map to tasks in `STATUS.md`.
