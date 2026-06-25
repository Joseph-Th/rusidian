-- Migration 0001_init (STATUS.md task B1)
-- APPEND-ONLY: once this migration has shipped, never edit it; add 0002_*.sql.
--
-- Initial schema: sources, notes, blocks, entities, links, views, agent_actions.
-- UUIDs are TEXT (RFC4122 format). Timestamps are TEXT (RFC3339, UTC).
-- Enums are TEXT (snake_case, matching serde #[serde(rename_all = "snake_case")]).

-- Migration version tracker. Every migration records itself here.
CREATE TABLE IF NOT EXISTS schema_version (
    version TEXT PRIMARY KEY,
    applied_at TEXT NOT NULL
);

-- Source: a raw piece of captured information.
CREATE TABLE IF NOT EXISTS source (
    id TEXT PRIMARY KEY,
    origin TEXT NOT NULL,
    title TEXT,
    raw_content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    created_by TEXT NOT NULL
);

-- Note: a durable knowledge object containing blocks.
CREATE TABLE IF NOT EXISTS note (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    created_by TEXT NOT NULL
);

-- Block: a stable, addressable unit inside a note.
CREATE TABLE IF NOT EXISTS block (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL,
    block_type TEXT NOT NULL,
    content TEXT NOT NULL,
    "order" REAL NOT NULL,
    created_at TEXT NOT NULL,
    created_by TEXT NOT NULL,
    FOREIGN KEY (note_id) REFERENCES note(id)
);

-- Entity: a normalized object the system recognizes and links.
CREATE TABLE IF NOT EXISTS entity (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    aliases TEXT NOT NULL,
    created_at TEXT NOT NULL,
    created_by TEXT NOT NULL
);

-- Link: a typed, directed edge from one object to another.
CREATE TABLE IF NOT EXISTS link (
    id TEXT PRIMARY KEY,
    from_type TEXT NOT NULL,
    from_id TEXT NOT NULL,
    to_type TEXT NOT NULL,
    to_id TEXT NOT NULL,
    link_type TEXT NOT NULL,
    created_at TEXT NOT NULL,
    created_by TEXT NOT NULL
);

-- View: a presentation-first rendering of structured knowledge.
CREATE TABLE IF NOT EXISTS view (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    params TEXT NOT NULL,
    created_at TEXT NOT NULL,
    created_by TEXT NOT NULL
);

-- AgentAction: append-only audit log of agent operations.
CREATE TABLE IF NOT EXISTS agent_action (
    id TEXT PRIMARY KEY,
    actor TEXT NOT NULL,
    operation TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    status TEXT NOT NULL,
    rationale TEXT NOT NULL,
    created_at TEXT NOT NULL,
    diff TEXT NOT NULL,
    rollback_of TEXT
);
