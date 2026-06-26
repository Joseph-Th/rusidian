# STATUS

Single source of truth for *what to build next*. Read this **and** `AGENTS.md`
before touching code. `AGENTS.md` says *what the product is and what is
forbidden*; this file says *which concrete task is next and when it is done*.

> Note: `AGENTS.md` was revised (the longer Project-Identity / North-Star /
> 10-Principles version is current). Architecture decisions are recorded in
> `docs/adr/`. Schema is mirrored in `SCHEMA.md`.

---

## How to work (every agent, every task)

1. Read `AGENTS.md` and this file.
2. Pick **one** task whose `Depends on` is all ✅. Do not start a blocked task.
3. Set its status to 🔨 here before starting.
4. Make the **smallest coherent change** that meets the Acceptance Criteria.
   Do not expand scope; do not refactor unrelated code.
5. Run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
   All must pass.
6. Update the task to ✅ and leave a `Notes:` line for the next agent.
7. In your report, answer the AGENTS.md "Review Checklist for Every Change".

### Hard rules
- Stubs intentionally `unimplemented!()`. Implementing one means completing the
  task that owns it — keep the guardrail doc-comments above it.
- **Invariant enums** (`LinkType`, `EntityKind`, `ViewKind`, `IngestionState`,
  `ContentStatus`, `ReviewState`, `AgentActionStatus`, `OperationKind`, `Actor`)
  are *authoritative, not frozen*: closed to casual edits; a variant changes
  only via an ADR, plus a migration if the enum is persisted. Never add a
  stringly-typed field to dodge editing an enum.
- Never add a "bad operation" from AGENTS.md (`rewrite_vault`,
  `mutate_markdown_blob`, `delete_without_recovery`, …).
- **Only `pkm-storage` may use `rusqlite` / write SQL.** Other crates work
  through `pkm_core::ports`.
- Migrations are append-only. Schema change ⇒ new migration + `SCHEMA.md` update.
  Destructive op ⇒ a recovery path.
- New dependency? Answer the AGENTS.md Dependency Policy questions in your
  report and add it to `[workspace.dependencies]`, not a leaf crate.
- SQL tasks for cold agents must be **bounded**: an existing domain type, an
  explicit migration number, and a round-trip test. No "design the storage
  layer" tasks.

### Status legend
✅ done · 🔨 in progress · ⬜ not started · 🚫 blocked

---

## Crate map (what exists)

`cargo build`, `clippy -D warnings`, and `cargo test` are all green. Layout and
dependency direction are fixed by `docs/adr/0002`:

```
pkm-core       no internal deps — domain types, invariant enums,
               provenance/review/content-status, and the ports (traits).
pkm-storage    -> pkm-core      SQLite. The ONLY crate with rusqlite/SQL.
pkm-search     -> pkm-core      pure query parsing + ranking.
pkm-agent      -> pkm-core      typed, audited operations via ports.
pkm-ingestion  -> pkm-core      pipeline state machine.
pkm-app        -> all           Tauri shell (not created yet — task A0).
```

Enums are authoritative. Structs are minimal stubs with `TODO(<task>)` markers
that map 1:1 to the task ids below. Cross-cutting types live in `pkm-core`
(`provenance`, `review`, `ports`). Shared test data: `pkm_core::fixtures`
(enable via `features = ["fixtures"]` in a crate's `[dev-dependencies]`).

What does NOT exist yet: any persistence, any real operation, markdown I/O, the
Tauri app, real tests beyond one serde smoke test.

---

## Build order — the vertical-slice spine

Do these in order. The goal is a **working spine** early, not a pile of green
stubs. Each slice ends with a passing test.

1. **B1** — migration runner + `0001_init` schema + db open.
2. **A1** — clippy/test gate + first round-trip tests (partly done).
3. **C1, C3** — finish `Source` and `Link` fields needed by the schema.
4. **B2** — repository impls (`SqliteSourceRepo`, `SqliteNoteRepo`, …).
5. **S1** — slice: `create_source → persist → retrieve → export JSON` (test).
6. **C2** — block/note ordering + markdown shape.
7. **D3, D4** — ingestion transitions; binary attachments.
8. **D1, D2** — operation dispatch; diff + action log; rollback.
9. **S2** — slice: `propose block update → inspect diff → accept → rollback`.
10. **E1, E2** — markdown import/export; FTS retriever.
11. **F0** — typed view params + one view through the view model.
12. **A0** — Tauri shell wired to the service layer.

Do not start a later step while an earlier one is 🔨/🚫.

---

## Tasks

### Current

Phase 7 tasks — expansion beyond the core system:

### Phase 7: Expansion

1. **H3** — Sync protocol design. Conflict resolution, merge algorithm,
   transport contract. Depends on: none (✅).
   
   Subtasks:
   - H3a: Design conflict detection rules (version/timestamp based) (✅)
   - H3b: Define merge algorithm for diverged states (✅)
   - H3c: Design transport contract (sync request/response format) (✅)
   - H3d: Implement versioning checks for sync eligibility (✅)
   
   Notes: Created ADR 0006 with sync protocol design. Implemented SyncEligible
   trait in pkm-core with conflict detection algorithm (last-write-wins for 
   divergent timestamps, conflict for concurrent edits). Added SyncRef type for
   tracking version/timestamp pairs. All domain types (Note, Source, Entity, Link,
   View, Block) now implement SyncEligible. 14 tests added covering conflict
   detection, merge logic, and edge cases. All tests passing, clippy clean.

2. **H4** — Web UI mobile support. Make capture and basic editing work on mobile
   browsers. Depends on: A0 (Tauri shell), H0 (views) (🚫 blocked by A0).
   
   Subtasks:
   - H4a: Responsive layout for capture form
   - H4b: Touch-friendly edit controls
   - H4c: Mobile-optimized search interface
   - H4d: Test on mobile browsers

3. **H5** — Visual workspace: canvas/graph view. Spatial organization of entities
   and notes. Depends on: H0 (view system), A0 (Tauri shell) (🚫 blocked by A0).
   
   Subtasks:
   - H5a: Graph data model for spatial layout
   - H5b: Canvas rendering with position persistence
   - H5c: Interactive node dragging
   - H5d: Link visualization on canvas

Further: mobile native editor, public publishing, collaboration, plugin API.

---

## Deferred / Blocked Items

### A0 — Tauri shell with service commands (✅)

Implements a working desktop app with full CRUD operations for notes.

Completed:
- Tauri window and app structure ✅
- Service layer with all CRUD operations ✅
- Commands wired to Tauri handlers (create, list, get, update, delete) ✅
- HTML/CSS/JS frontend with full CRUD UI ✅
- Modal for viewing and editing notes ✅
- Metadata support in notes ✅
- command→service→repository→storage pipeline tested end-to-end ✅
- Migration 0008 adds metadata column to notes table ✅
- Comprehensive CRUD integration test (crud_workflow_end_to_end) ✅
- All tests passing, clippy clean ✅

Subtasks:
- A0a: Add get_note endpoint and view modal (✅)
- A0b: Add update_note command and inline editing UI (✅)
- A0c: Add delete_note command and UI (✅)
- A0d: Verify app runs and basic CRUD works end-to-end (✅)

A0 enables H4 (mobile UI) and H5 (visual workspace). Foundation is solid.
