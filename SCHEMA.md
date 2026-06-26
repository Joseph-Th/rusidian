# SCHEMA

The authoritative description of the persisted schema. Update this file in the
**same change** as any migration (`crates/pkm-storage/migrations/NNNN_*.sql`).
If code and this file disagree, the migrations are the truth and this file is a
bug — fix it.

## Rules

- Migrations are **append-only**. Never edit a shipped migration; add the next
  numbered file.
- Every migration is recorded in `schema_version`.
- Invariant enums (`LinkType`, `EntityKind`, `ViewKind`, `IngestionState`,
  `ContentStatus`, `ReviewState`, `AgentActionStatus`, `OperationKind`,
  `Actor`) are persisted as their `snake_case` serde strings. Changing a variant
  requires an ADR **and** a migration.
- Ids are UUIDv7 stored as TEXT. Timestamps are RFC3339 TEXT (UTC).

## Current schema

Migrations applied in order:

1. `0001_init` (STATUS task B1): initial aggregates
2. `0002_extend_source` (STATUS task B2 / task C1): adds source ingestion pipeline fields
3. `0003_fts5_indexing` (STATUS task E2): adds FTS5 virtual tables
4. `0004_entity_merge` (STATUS task C4a): adds merged_into column to entity table
5. `0005_link_review_state` (STATUS task C4d): adds reviewed and confidence to link table
6. `0006_add_project_field` (STATUS task G4b): adds project column to note table for filtering

| Table | Backs (core type) | Key columns | Notes |
|-------|-------------------|-------------|-------|
| `schema_version` | — | `version` (PK), `applied_at` | migration ledger; every migration records itself |
| `source` | `source::Source` | `id` (PK) | raw content write-once; `origin` (TEXT, enum), `title`, `raw_content`, `created_at` (RFC3339), `created_by`, `captured_at` (RFC3339), `content_hash`, `ingestion_state` (enum) |
| `note` | `note::Note` | `id` (PK) | `title`, `created_at` (RFC3339), `created_by`, `project` (TEXT NULL) |
| `block` | `block::Block` | `id` (PK), `note_id` (FK) | `block_type` (enum), `content`, `order` (REAL, fractional for insert-between), `created_at`, `created_by` |
| `entity` | `entity::Entity` | `id` (PK) | `kind` (enum), `name`, `aliases` (JSON), `created_at`, `created_by`, `merged_into` (TEXT NULL, references `entity(id)`) |
| `link` | `link::Link` | `id` (PK) | `from_type`/`from_id` (ObjectRef), `to_type`/`to_id` (ObjectRef), `link_type` (enum), `created_at`, `created_by`, `reviewed` (enum), `confidence` (REAL NULL) |
| `view` | `view::View` | `id` (PK) | `kind` (enum), `title`, `params` (JSON), `created_at`, `created_by` |
| `agent_action` | `agent_action::AgentAction` | `id` (PK) | `actor` (JSON), `operation` (enum), `target_type`/`target_id` (ObjectRef), `status` (enum), `rationale`, `created_at`, `diff` (JSON), `rollback_of` (FK or NULL) |

### Storage notes

- **UUIDs:** TEXT (RFC4122 string format, e.g., `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`)
- **Timestamps:** TEXT (RFC3339 format, e.g., `2025-06-25T14:26:00Z`)
- **Enums:** TEXT (snake_case per serde serialization, e.g., `related_to`, `proposed`, `user_authored`)
- **JSON:** TEXT (serde_json; used for complex types like aliases, params, actor, diff)
- **Foreign keys:** PRAGMA foreign_keys=ON enforced; soft deletes preserve recovery
- **Concurrency:** PRAGMA journal_mode=WAL for better concurrent access
- **Busy timeout:** 5 seconds to avoid immediate failures under contention

### What is NOT yet persisted

Tasks D1-D4, E1 will add fields (e.g., `captured_at`, `content_hash`, `ingestion_state` on Source; provenance/review state on blocks; etc.). Schema evolves via new migrations.
