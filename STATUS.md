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

### Next to work on

#### F3 · ReviewQueue view ✅
- Added `ingestion_state_filter` to ReviewQueueParams to allow filtering by ingestion state.
- Updated `render_review_queue` to filter sources by AwaitingReview or other states.
- Added helper methods `with_state()` and `with_limit()` for fluent configuration.
- 2 tests verify: params serialization and filtering by AwaitingReview status.
- All tests pass; ready for integration with UI layer.

#### F4 · ProjectDashboard view ⬜
- **Depends on:** F0 ✅.
- **Do:** Implement ProjectDashboard view. Group notes by project, show status summary.
- **Done when:** Test: project dashboard aggregates and displays project status.

#### F5 · SourceMap view ⬜
- **Depends on:** F0 ✅.
- **Do:** Implement SourceMap view. Show source citations, trace knowledge provenance.
- **Done when:** Test: source map displays link chain from raw source to final note.

#### F6 · DecisionLog, PersonProfile, EntityPage, BriefingPage, OpenQuestions, ActionList ⬜
- **Depends on:** F0 ✅.
- **Do:** Implement remaining views (DecisionLog, PersonProfile, EntityPage, BriefingPage, OpenQuestions, ActionList).
- **Done when:** All views render and have basic tests.

#### A0b · Tauri desktop shell (`pkm-app` binary) ⬜
- **Depends on:** A0a ✅, B2 ✅.
- **Do:** Add Tauri main.rs binary that wires AppService and exposes commands.
  Set up window, menu, data dir. Write ADR confirming UI-shell choice.
- **Done when:** App launches, opens db, creates + lists note via frontend commands.

---

## Done (25 tasks)

All foundation, vertical-slice, entity merge, view model, app service, timeline, and dossier views complete: A1, B1, B2, C1, C2, C3, D1, D2, D3, D4, S1, S2, E1, E2, C4a, C4b, C4c, C4d, C4e, C5, C4f, F0, A0a, F1, F2.

---

## Deferred (post F1+, A0)

Further work as the system matures:

> Advanced retrieval (Phase 6): hybrid search, semantic search, entity-aware
> retrieval, typed link traversal, citation-aware retrieval, stale-content detection.
> Expansion (Phase 7): sync, mobile, publishing, collaboration, visual workspace.
