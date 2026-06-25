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

Nothing is persisted yet. The initial schema lands in migration `0001_init`
(STATUS task B1). When B1 is implemented, document each table here:

| Table | Backs (core type) | Key columns | Notes |
|-------|-------------------|-------------|-------|
| `schema_version` | — | `version`, `applied_at` | migration ledger |
| `source` | `source::Source` | TBD (B1) | raw content write-once |
| `note` | `note::Note` | TBD | |
| `block` | `block::Block` | TBD | ordered within note |
| `entity` | `entity::Entity` | TBD | aliases, merge target |
| `link` | `link::Link` | TBD | typed; from/to are ObjectRef |
| `view` | `view::View` | TBD | kind + params |
| `agent_action` | `agent_action::AgentAction` | TBD | append-only audit log |
