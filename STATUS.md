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

### Spine slices (prove the architecture end-to-end)

#### S1 · Vertical slice: source round-trip + JSON export ⬜
- **Depends on:** B1, B2, C1.
- **Do:** A test (and a thin service fn) that creates a `Source`, persists it
  via `SourceRepo`, reads it back equal, and exports it to JSON. Use
  `pkm_core::fixtures::sample_source`.
- **Done when:** One integration test green covering create→persist→get→export.

#### S2 · Vertical slice: propose → diff → accept → rollback ⬜
- **Depends on:** D1, D2, B2.
- **Do:** A test that proposes an `UpdateBlock`, inspects the recorded diff,
  accepts/applies it, then rolls it back to the exact prior state.
- **Done when:** Test proves the full agent-action lifecycle + rollback.

---

### A — Project meta

#### A1 · Dev gate + first tests ✅
- **Depends on:** —
- **Done so far:** serde round-trip smoke test in `pkm-core/src/link.rs`;
  `clippy --all-targets -- -D warnings` is clean.
- **Completed:** Added round-trip tests for EntityKind, ViewKind, ReviewState,
  AgentActionStatus, OperationKind, Actor, ContentStatus, ObjectRef, and
  IngestionState. All 10 invariant enums now have snake_case round-trip tests.
  All tests pass; clippy is clean.
- **Notes:** Needed to add serde_json as dev-dependency to pkm-ingestion.
  Each test verifies serialize→deserialize round-trip with correct snake_case
  format per AGENTS.md invariant enum policy.

#### A0 · Tauri desktop shell (`pkm-app`) ⬜
- **Depends on:** S1, S2, E2.
- **Do:** Add `crates/pkm-app` (Tauri), wire as a workspace member. Expose the
  agent/search/storage services as Tauri commands. NO business logic in the UI
  layer. Write an ADR confirming/!revising the UI-shell choice.
- **Done when:** App launches, opens the db, creates + lists a note through the
  real services.

---

### B — Storage

#### B1 · Migration runner + `0001_init` + db open 🔨
- **Depends on:** —
- **Files:** `pkm-storage/src/db.rs`, `.../migrations.rs`,
  `pkm-storage/migrations/0001_init.sql`, `SCHEMA.md`.
- **Do:** Implement `open()` (pragmas: WAL, foreign_keys=ON, busy_timeout, then
  `migrations::run`). Implement `run()` with a `schema_version` table, applied
  transactionally + idempotently. Write `0001_init` creating typed tables for
  source/note/block/entity/link/view/agent_action. Fill in `SCHEMA.md`.
- **Done when:** Tests: fresh open succeeds; `open` twice is idempotent;
  `schema_version` records `0001_init`.

#### B2 · Repository implementations ⬜
- **Depends on:** B1, C1, C3 (and C2 for blocks).
- **Files:** `pkm-storage/src/repositories/*`; add ports to `pkm_core::ports`.
- **Do:** Implement `SqliteSourceRepo`, `SqliteNoteRepo`; add + implement
  `EntityRepo`, `LinkRepo`, `ViewRepo`, `AgentActionRepo` (append-only). Raw
  source content write-once except an explicit audited user edit. Soft/
  recoverable deletes. Pure row↔type mapping fns, unit-tested.
- **Done when:** Round-trip tests (create→get equal) pass for every aggregate;
  soft-delete keeps a recovery path.

---

### C — Domain model completion

#### C1 · Flesh out `Source` ⬜
- **Files:** `pkm-core/src/source.rs`.
- **Do:** Add `captured_at`, `content_hash`, `ingestion_state: IngestionState`,
  attachment reference (see D4). Document raw-content write-once.
- **Done when:** serde round-trip test; no existing field removed.

#### C2 · Block ordering + Note metadata + markdown shape ⬜
- **Files:** `pkm-core/src/block.rs`, `note.rs`.
- **Do:** Choose block ordering (recommend a fractional/`order` key, not array
  index — document it). Add typed `metadata` to `Note` (NOT a frontmatter
  blob), `created_by/at`, block-level provenance ref.
- **Done when:** Ordering stable under insert-between; reorder test passes.

#### C3 · Link provenance ⬜
- **Files:** `pkm-core/src/link.rs`.
- **Do:** Add `created_by: Actor`, `created_at`, `reviewed: ReviewState`,
  optional `confidence`. Keep `from/to/link_type` stable.
- **Done when:** Inferred vs. user-confirmed links are distinguishable.

#### C4 · Entity merge semantics ⬜
- **Depends on:** B2.
- **Do:** Non-lossy merge: survivor id, losers marked merged-into, all
  links/aliases re-pointed, original recoverable. Write an ADR.
- **Done when:** Test: merge keeps every alias + re-points every link; rollback
  restores pre-merge state.

#### C5 · Finalize `Provenance` ⬜
- **Files:** `pkm-core/src/provenance.rs`.
- **Do:** Add `originating_action: Option<AgentActionId>` and extraction span
  fields. Ensure derived content always has non-empty `derived_from`.
- **Done when:** A derived block/link/entity can be traced to source + action.

---

### D — Agent safety + ingestion

#### D1 · Typed operation dispatch + risk classification ⬜
- **Depends on:** B2.
- **Files:** `pkm-agent/src/lib.rs`.
- **Do:** Replace `payload: Value` with a typed `Operation` enum (one variant
  per `OperationKind`). Make `execute` take the needed `&dyn ...Repo` ports.
  Implement `requires_review` honestly (mechanical→direct; knowledge→proposed).
- **Done when:** Each op produces a recorded `AgentAction`; knowledge ops
  default to `Proposed`; tests cover the propose-vs-apply split.

#### D2 · Diff representation + action log persistence ⬜
- **Depends on:** D1, B2.
- **Do:** Decide before/after representation (structured patch vs. snapshot) —
  **write `docs/adr/0003-agent-diff.md`**. Persist via `AgentActionRepo`.
  Implement `RollbackAction`.
- **Done when:** apply→rollback restores prior state exactly; log is append-only.

#### D3 · Ingestion transition table ⬜
- **Files:** `pkm-ingestion/src/lib.rs`.
- **Do:** Implement `IngestionState::can_transition_to` with the real allowed
  edges; reject illegal jumps. Raw capture immutable; failures keep diagnostics.
- **Done when:** Tests cover every legal transition + a sample of rejected ones.

#### D4 · Binary attachments ⬜
- **Depends on:** B1.
- **Do:** Content-addressed local blob store (hash→file under app data dir);
  `Source` references attachments by hash. No DB blobs for large files.
- **Done when:** Store + fetch by hash round-trips; dedupe by hash works.

---

### E — Import/export + search

#### E1 · Markdown import/export ⬜
- **Depends on:** C2, B2.
- **Do:** Pure `Note ⇄ markdown` functions (blocks ↔ markdown spans); import a
  markdown folder into sources/notes with provenance. Keep parsing pure+tested.
- **Done when:** Round-trip note→markdown→note preserves content + block ids
  where possible; folder import creates sources with provenance.

#### E2 · Keyword/FTS retriever + ranking + filters ⬜
- **Depends on:** B1, B2.
- **Files:** `pkm-search/src/lib.rs` (parse/rank); the `Retriever` impl over
  SQLite FTS5 lives in `pkm-storage`.
- **Do:** Implement `ExactText`/`FuzzyText` via FTS5. Every `SearchHit` sets its
  `ContentStatus`. Add filters (date, type, review-state, project). Leave
  `Semantic` an explicit unimplemented variant.
- **Done when:** Index + ranking tests pass; unreviewed content is flagged,
  never returned as settled knowledge.

---

### F — Presentation views

#### F0 · Typed view parameters + view model ⬜
- **Depends on:** C-series.
- **Files:** `pkm-core/src/view.rs` (+ a render boundary).
- **Do:** Replace `params: Value` with typed params per `ViewKind`. Define a
  `ViewModel`/render trait so every view goes through the shared model (no
  one-off views — AGENTS.md red flag).
- **Done when:** One view (recommend `ReadingQueue` or `ReviewQueue`) renders
  from stored data through the view model, with tests.

> The remaining concrete views (Dossier, Timeline, ProjectDashboard, SourceMap,
> DecisionLog, PersonProfile, EntityPage, BriefingPage, OpenQuestions,
> ActionList) become one task each (F1…Fn), all depending on F0. Add them when
> F0 lands; build none before F0.
