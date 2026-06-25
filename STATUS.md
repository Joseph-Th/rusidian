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

#### C4a · Entity merge: schema migration 0004 ✅
- **Depends on:** B2 ✅.
- **Do:** Add merged_into (nullable EntityId), created_by, created_at columns to entity table.
  Update schema version. Backfill created_at/created_by from existing data.
- **Done when:** Migration runs; existing entities load with new fields; schema.md updated.

#### C4b · Entity merge: SqliteEntityRepo persistence ✅
- **Notes:** Added EntityRepo trait to ports.rs with create(), get(), set_merged_into(). Implemented SqliteEntityRepo in entities.rs with full SQL persistence for all Entity fields including merged_into. Added 2 round-trip tests: entity_create_and_get_round_trip and entity_merge_sets_merged_into. Both tests pass; all entity fields survive persist/retrieve cycle.

#### C4c · Entity merge: Operation enum + MergeEntity ⬜
- **Depends on:** C4b.
- **Do:** Add MergeEntity variant to Operation enum (with survivor_id and loser_id).
  Requires review (knowledge operation). Update execute() dispatcher.
- **Done when:** Operation serializes; requires_review() returns true; tests pass.

#### C4d · Entity merge: Link re-pointing ⬜
- **Depends on:** C4c.
- **Do:** When merge is accepted, re-point all links pointing to loser_id → survivor_id.
  Preserve link provenance/review state. Update AgentAction.apply().
- **Done when:** Test: merge creates AgentAction; applying it re-points all links.

#### C4e · Entity merge: Rollback path ⬜
- **Depends on:** C4d.
- **Do:** Implement rollback_merge in AgentActionRepo. Restore merged_into→NULL,
  re-point links back to loser. Verify pre-merge state is fully recoverable.
- **Done when:** Test: merge → accept → rollback restores all state.

#### C4f · ADR 0004: Entity merge semantics ⬜
- **Depends on:** C4e.
- **Do:** Document merge design (non-lossy, survivor/loser, re-pointing strategy,
  rollback mechanics). Link to tests.
- **Done when:** ADR written; answers why this design over alternatives.

#### C5 · Finalize `Provenance` ✅
- **Notes:** Added `originating_action: Option<AgentActionId>` and `extraction_span: Option<ExtractionSpan>` fields to Provenance. ExtractionSpan tracks byte offsets (start/end) for precise source location. Added invariant documentation that derived content must have non-empty `derived_from`. 7 tests verify round-trip serialization, span arithmetic, and provenance traceability.

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

## Done (14 tasks)

All foundation and vertical-slice work complete: A1, B1, B2, C1, C2, C3, D1, D2, D3, D4, S1, S2, E1, E2.

---

## Deferred

These will become tasks once C4, C5, F0 land:

> The remaining concrete views (Dossier, Timeline, ProjectDashboard, SourceMap,
> DecisionLog, PersonProfile, EntityPage, BriefingPage, OpenQuestions,
> ActionList) become one task each (F1…Fn), all depending on F0. Add them when
> F0 lands; build none before F0.
