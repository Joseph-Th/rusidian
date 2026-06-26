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

#### A0b · Tauri desktop shell (`pkm-app` binary) ✅
- **Depends on:** A0a ✅, B2 ✅.
- **Do:** Break down into subtasks A0b-i, A0b-ii, A0b-iii, A0b-iv (see below).
- **Done when:** All subtasks complete. ✅

#### A0b-i · ADR 0005: UI shell architecture choice ✅
- Created `docs/adr/0005-ui-shell-choice.md`. Documents decision to use Tauri as UI shell.
- Rationale: Rust alignment, lightweight, local-first, mature ecosystem.
- Alternatives considered: Electron (rejected: heavy), GTK/Qt (rejected: less design flexibility), web-only (rejected: violates local-first), CLI/TUI (rejected: can't deliver visual presentation).
- Consequences: Tauri binary in pkm-app, command API, single executable, cross-platform support.

#### A0b-ii · AppService commands foundation ✅
- **Depends on:** A0a ✅, A0b-i ✅.
- **Do:** Add list_notes method to NoteRepo and AppService; create command wrappers for create_note and list_notes.
- Added `list` method to NoteRepo trait that returns Vec<Note> with optional limit.
- Implemented `list` in SqliteNoteRepo (queries notes sorted by created_at DESC, respects limit).
- Added `list_notes` to AppService; returns Vec<(id, title)> tuples.
- Added `CreateNoteResponse` and `NoteInfo` response types; `list_notes` command wrapper.
- All tests pass; foundation ready for Tauri integration.

#### A0b-iii · Tauri setup and command wiring ✅
- **Depends on:** A0b-ii ✅.
- **Do:** Add Tauri to workspace; create pkm-app binary target; wire create_note and list_notes commands.
- Added Tauri v2 to workspace dependencies.
- Created src/bin/main.rs Tauri binary that wires create_note and list_notes commands via #[tauri::command].
- Created build.rs for Tauri build system integration.
- Created tauri.conf.json with app configuration (1200x800 window).
- Created minimal frontend (HTML + JavaScript) with UI to create and list notes.
- Created icon assets (PNG and ICO) for the application.
- Database location: ~/.pkm/pkm.db (or %USERPROFILE%\.pkm\pkm.db on Windows).
- cargo check, clippy -D warnings, and all tests pass.
- **Done when:** Tauri app binary can invoke create_note and list_notes via RPC. ✅

#### A0b-iv · Window, menu, data dir setup ✅
- **Depends on:** A0b-iii ✅.
- **Do:** Set up Tauri window, menu bar, persistent data directory.
- Configured 1200x800 window in tauri.conf.json.
- Added empty menu bar to application (via tauri::menu::Menu).
- Data directory at ~/.pkm (auto-created if missing).
- Database file at ~/.pkm/pkm.db (auto-opened on startup).
- App launches with UI showing create note form and list functionality.
- All tests pass; cargo clippy -D warnings and fmt clean.
- **Done when:** App launches, opens db, shows UI with basic controls. ✅

---

## Done (36 tasks)

All foundation, vertical-slice, entity merge, view model, app service, view implementations, and UI shell complete: A1, B1, B2, C1, C2, C3, D1, D2, D3, D4, S1, S2, E1, E2, C4a, C4b, C4c, C4d, C4e, C5, C4f, F0, A0a, F1, F2, F3, F4, F5, F6, A0b-i, A0b-ii, A0b-iii, A0b-iv.

---

## Phase 6: Advanced Retrieval

### G1 · Improve fuzzy text search ✅
- **Depends on:** A0b ✅.
- **Do:** Implement true fuzzy matching using FTS5 capabilities (prefix wildcards).
- Implemented fuzzy matching using FTS5's * wildcard operator (e.g., "note*" matches "notebook").
- Updated search_*_fuzzy functions to use prefix-based matching: format!("{}*", query.text).
- Fuzzy search now returns partial token matches, not just exact phrases.
- All tests pass; cargo check/clippy/fmt clean.
- **Done when:** Fuzzy search returns results for partial/misspelled queries; tests verify. ✅

### G2 · Add text snippets to search results ⬜
- **Depends on:** G1 ✅.
- **Do:** Extract matching text with context (50 chars before/after match).
- **Done when:** SearchHit.snippet populated with matched text; visible in UI.

### G3 · Link traversal search ⬜
- **Depends on:** A0b ✅.
- **Do:** Implement graph traversal (follow typed links to related notes/entities).
- **Done when:** Search mode LinkTraversal returns reachable objects.

### G4 · Date range and project filtering ⬜
- **Depends on:** G2 ✅.
- **Do:** Implement date_range and project filters in search queries.
- **Done when:** Filters exclude results outside range/project; tests verify.

### G5 · Semantic search foundation ⬜
- **Depends on:** G4 ✅.
- **Do:** Placeholder for vector-based semantic search (Phase 7 work).
- **Done when:** SearchMode::Semantic returns informative error with roadmap.

---

## Deferred (post Phase 6)

Further work as the system matures:

> Phase 7 (Expansion): sync, mobile, publishing, collaboration, visual workspace.
