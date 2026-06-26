-- Migration 0007_add_versioning (STATUS.md task H2)
-- APPEND-ONLY: once this migration has shipped, never edit it; add 0008_*.sql.
--
-- Adds version tracking to all entities and creates an object_history table
-- for tracking changes, rollbacks, and sync operations.

-- Add version to all tables
ALTER TABLE source ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE note ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE block ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE entity ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE link ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE view ADD COLUMN version INTEGER NOT NULL DEFAULT 1;

-- Add updated_at to all tables (initially set to created_at for existing rows)
ALTER TABLE source ADD COLUMN updated_at TEXT NOT NULL DEFAULT '2024-01-01T00:00:00Z';
UPDATE source SET updated_at = created_at;

ALTER TABLE note ADD COLUMN updated_at TEXT NOT NULL DEFAULT '2024-01-01T00:00:00Z';
UPDATE note SET updated_at = created_at;

ALTER TABLE block ADD COLUMN updated_at TEXT NOT NULL DEFAULT '2024-01-01T00:00:00Z';
UPDATE block SET updated_at = created_at;

ALTER TABLE entity ADD COLUMN updated_at TEXT NOT NULL DEFAULT '2024-01-01T00:00:00Z';
UPDATE entity SET updated_at = created_at;

ALTER TABLE link ADD COLUMN updated_at TEXT NOT NULL DEFAULT '2024-01-01T00:00:00Z';
UPDATE link SET updated_at = created_at;

ALTER TABLE view ADD COLUMN updated_at TEXT NOT NULL DEFAULT '2024-01-01T00:00:00Z';
UPDATE view SET updated_at = created_at;

-- object_history: tracks all changes to entities for rollback, sync, and audit.
-- Records when each version was created, by whom, and what changed.
CREATE TABLE object_history (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,  -- 'source', 'note', 'block', 'entity', 'link', 'view'
  entity_id TEXT NOT NULL,
  version INTEGER NOT NULL,
  actor TEXT NOT NULL,        -- JSON serialized Actor
  timestamp TEXT NOT NULL,    -- RFC3339
  operation TEXT NOT NULL,    -- 'created', 'updated', 'deleted', 'merged', 'rollback'
  previous_version INTEGER,   -- NULL for creation, version number it reverted from for rollback
  diff TEXT,                  -- JSON: changes made (optional, for space efficiency)
  UNIQUE (entity_type, entity_id, version)
);

-- Index for efficient history queries
CREATE INDEX idx_object_history_entity ON object_history(entity_type, entity_id);
CREATE INDEX idx_object_history_timestamp ON object_history(timestamp);
