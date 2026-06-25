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

### Active — Next to work on

#### E2 · Keyword/FTS retriever + ranking + filters ✅
- **Depends on:** B1 ✅, B2 ✅.
- **Files:** `pkm-search/src/lib.rs` (parse/rank); the `Retriever` impl over
  SQLite FTS5 lives in `pkm-storage`.
- **Notes:** Implemented parse_query() for quoted phrases, bare terms, and field filters (type:, status:, reviewed:, date:, project:). Implemented rank() with ContentStatus-aware scoring (UserAuthored > Reviewed > RawSource > others). Expanded SearchQuery/SearchHit with filters and score/snippet fields. Created migration 0003 for FTS5 virtual tables on notes, blocks, sources, entities. Implemented SqliteRetriever with ExactText/FuzzyText search and filter application. Left Semantic and LinkTraversal unimplemented. 8 parse/rank tests pass; migration tests pass; all content status preserved throughout retrieval pipeline.

#### E1 · Markdown import/export ✅
- **Depends on:** C2 ✅, B2 ✅.
- **Notes:** Implemented pure markdown parsing in pkm-core/src/markdown.rs. Functions: blocks_to_markdown(), markdown_to_blocks(), note_to_markdown(), markdown_to_note(), extract_title(). All preserve block IDs via HTML comments for round-tripping. Tested: 7 unit tests covering title extraction, block parsing, block ID preservation, and note-level round-trip (# Title + paragraphs). Note-to-markdown includes title as level-1 heading. Markdown-to-note extracts title and creates blocks with fractional ordering. Folder import (walk directory, create sources) deferred to follow-up as it requires coordination with storage layer for provenance tracking.

#### C4 · Entity merge semantics ⬜
- **Depends on:** B2 ✅.
- **Progress:** Entity struct updated with merged_into, created_by, created_at fields. Tests pass.
- **Do:** Non-lossy merge: survivor id, losers marked merged-into, all
  links/aliases re-pointed, original recoverable. Write an ADR.
- **Remaining work:**
  - C4a: Schema migration 0004 for entity table (add merged_into, created_by, created_at columns)
  - C4b: SqliteEntityRepo impl + round-trip tests
  - C4c: Entity merge operation (MergeEntity in Operation enum)
  - C4d: Link re-pointing logic (move links from loser to survivor)
  - C4e: Rollback for merge (via AgentAction)
  - C4f: Write ADR 0004 for merge semantics
- **Done when:** Test: merge keeps every alias + re-points every link; rollback
  restores pre-merge state.

#### C5 · Finalize `Provenance` ⬜
- **Files:** `pkm-core/src/provenance.rs`.
- **Do:** Add `originating_action: Option<AgentActionId>` and extraction span
  fields. Ensure derived content always has non-empty `derived_from`.
- **Done when:** A derived block/link/entity can be traced to source + action.

#### F0 · Typed view parameters + view model ⬜
- **Depends on:** C-series.
- **Files:** `pkm-core/src/view.rs` (+ a render boundary).
- **Do:** Replace `params: Value` with typed params per `ViewKind`. Define a
  `ViewModel`/render trait so every view goes through the shared model (no
  one-off views — AGENTS.md red flag).
- **Done when:** One view (recommend `ReadingQueue` or `ReviewQueue`) renders
  from stored data through the view model, with tests.

#### A0 · Tauri desktop shell (`pkm-app`) ⬜
- **Depends on:** S1 ✅, S2 ✅, E2.
- **Do:** Add `crates/pkm-app` (Tauri), wire as a workspace member. Expose the
  agent/search/storage services as Tauri commands. NO business logic in the UI
  layer. Write an ADR confirming/!revising the UI-shell choice.
- **Done when:** App launches, opens the db, creates + lists a note through the
  real services.

---

## Done

#### A1 · Clippy/test gate + round-trip tests ✅
- **Notes:** Implemented round-trip tests for all invariant enums (LinkType, EntityKind, ViewKind, IngestionState, ContentStatus, ReviewState, AgentActionStatus, OperationKind, Actor).

#### B1 · Migration runner + 0001_init schema + db open ✅
- **Notes:** Implemented migration infrastructure with automatic runner. Schema includes tables for sources, notes, blocks, entities, links, and action logs.

#### B2 · Repository impls ✅
- **Notes:** Implemented SqliteSourceRepo, SqliteNoteRepo, SqliteBlockRepo, SqliteEntityRepo, SqliteLinkRepo with round-trip persistence tests.

#### C1 · Source fields (ingestion state, timestamps) ✅
- **Notes:** Added ingestion state, captured timestamp, processed timestamp, extraction status, and URL/locator fields to Source.

#### C2 · Block ordering + Note metadata + markdown shape ✅
- **Notes:** Implemented block ordering within notes, note metadata fields, and markdown-compatible block types (paragraph, heading, quote, etc.).

#### C3 · Link provenance and review state ✅
- **Notes:** Added provenance and review state to Link struct; confidence field for inferred relationships.

#### D1 · Typed operation dispatch + risk classification ✅
- **Depends on:** B2.
- **Notes:** Implemented 16-variant Operation enum. requires_review() classifies: ParseSource/GenerateSummary are mechanical (false); all others are knowledge ops (true). execute() creates AgentAction records with Proposed status.

#### D2 · Diff representation + action log persistence ✅
- **Depends on:** D1 ✅, B2.
- **Notes:** Wrote ADR 0003 (full snapshots over patches). Added AgentActionRepo trait; implemented SqliteAgentActionRepo. Action log is append-only and fully auditable.

#### D3 · Ingestion transition table ✅
- **Notes:** Implemented ingestion state machine transitions with validation.

#### D4 · Binary attachments (content-addressed blob store) ✅
- **Notes:** Implemented content-addressed blob storage for binary attachments with integrity verification.

#### S1 · Vertical slice: source round-trip + JSON export ✅
- **Notes:** Implemented full create_source → persist → retrieve → export JSON workflow with round-trip tests.

#### S2 · Vertical slice: propose → diff → accept → rollback ✅
- **Depends on:** D1 ✅, D2 ✅, B2 ✅.
- **Notes:** Implemented full agent-action lifecycle: propose UpdateBlock operations, apply them (status transition), and rollback. Added NoteRepo::update_block and AgentActionRepo::set_status/set_diff methods. Test verifies end-to-end proposal → acceptance → rollback with action status tracking.

---

> The remaining concrete views (Dossier, Timeline, ProjectDashboard, SourceMap,
> DecisionLog, PersonProfile, EntityPage, BriefingPage, OpenQuestions,
> ActionList) become one task each (F1…Fn), all depending on F0. Add them when
> F0 lands; build none before F0.
