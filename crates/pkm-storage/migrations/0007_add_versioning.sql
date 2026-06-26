-- Migration 0007_add_versioning (STATUS.md task H2)
-- APPEND-ONLY: once this migration has shipped, never edit it; add 0008_*.sql.
--
-- Adds version tracking to all entities and creates an object_history table
-- for tracking changes, rollbacks, and sync operations.

-- Add version and updated_at to source table
ALTER TABLE source ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE source ADD COLUMN updated_at TEXT NOT NULL DEFAULT created_at;

-- Add version and updated_at to note table
ALTER TABLE note ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE note ADD COLUMN updated_at TEXT NOT NULL DEFAULT created_at;

-- Add version and updated_at to block table
ALTER TABLE block ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE block ADD COLUMN updated_at TEXT NOT NULL DEFAULT created_at;

-- Add version and updated_at to entity table
ALTER TABLE entity ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE entity ADD COLUMN updated_at TEXT NOT NULL DEFAULT created_at;

-- Add version and updated_at to link table
ALTER TABLE link ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE link ADD COLUMN updated_at TEXT NOT NULL DEFAULT created_at;

-- Add version and updated_at to view table
ALTER TABLE view ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE view ADD COLUMN updated_at TEXT NOT NULL DEFAULT created_at;

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
