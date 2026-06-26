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

1. **H4** — Web UI mobile support. Make capture and basic editing work on mobile
   browsers. Depends on: A0 (Tauri shell) (✅), H0 (views) (✅).
   
   Subtasks:
   - H4a: Responsive layout for capture form (✅)
   - H4b: Touch-friendly edit controls (✅)
   - H4c: Mobile-optimized search interface (✅)
   - H4d: Test on mobile browsers (⬜)
   
   Notes (H4a-H4b): Added mobile-first CSS with viewport meta tag. Media queries
   for tablets (768px) and phones (480px). Touch-friendly button/input heights
   (min 44px). Flexible layouts with flexbox. Modal full-width on mobile.
   Improved padding/margins for touch targets. All tests passing (110).
   
   Notes (H4c): Added search command to Tauri backend using pkm-search parsing
   and SqliteRetriever. Search results display with mobile-optimized styling
   (44px+ touch targets). Green left border for search results, distinct from
   notes (blue). Enter key triggers search. Fuzzy text search by default. All 122
   tests passing.

2. **H5** — Visual workspace: canvas/graph view. Spatial organization of entities
   and notes. Depends on: H0 (views) (✅), A0 (Tauri shell) (✅).
   
   Subtasks:
   - H5a: Graph data model for spatial layout (✅)
   - H5b: Canvas rendering with position persistence (⬜)
   - H5c: Interactive node dragging (⬜)
   - H5d: Link visualization on canvas (⬜)
   
   Notes (H5a): Added GraphView kind with spatial layout support. GraphLayoutType
   enum (ForceDirected, Hierarchical, Circular, Custom). NodePosition struct for
   storing (x,y) coordinates. GraphViewParams with layout configuration. Integrated
   into view rendering pipeline. 3 new tests added, all passing (108 total).

Further: mobile native editor, public publishing, collaboration, plugin API.

---

## Completed

### A0 — Tauri shell with service commands (✅)

Tauri desktop app with full CRUD for notes. Enables H4 and H5.

- A0a: Get note endpoint and view modal (✅)
- A0b: Update note command and inline editing UI (✅)
- A0c: Delete note command and UI (✅)
- A0d: Verify end-to-end CRUD workflow (✅)

### H0 — Presentation-first views (✅)

View system for reading queue, review queue, dossier, timeline, source map,
entity pages, and more. Each view type has typed parameters and rendering logic.
Integrated into Tauri app service with create/list/get/render commands.

- H0a: ViewKind enum and ReadingQueue parameters (✅)
- H0b: DefaultViewModel rendering for reading_queue (✅)
- H0c: ReviewQueue view with filtering (✅)
- H0d: Dossier view (entity-centered) (✅)
- H0e: Timeline view with reverse chronological ordering (✅)
- H0f: SourceMap view for tracing provenance chains (✅)
- H0g: Service layer integration (create_view, list_views, get_view, render_view) (✅)
- H0h: Added SourceRepo.list() method for view rendering (✅)

Notes: All 12 view kinds defined with typed parameters. DefaultViewModel
provides placeholder implementations (full filtering deferred to future tasks).
Service methods wired to Tauri commands. Tests passing: 110 total, 66 in pkm-core.

### H3 — Sync protocol design (✅)

Conflict detection and merge algorithm for diverged states.

- H3a: Conflict detection rules (version/timestamp based) (✅)
- H3b: Merge algorithm for diverged states (✅)
- H3c: Transport contract (sync request/response format) (✅)
- H3d: Versioning checks for sync eligibility (✅)

ADR 0006 documents the design. SyncEligible trait implemented with
last-write-wins for divergent timestamps, conflict for concurrent edits.
