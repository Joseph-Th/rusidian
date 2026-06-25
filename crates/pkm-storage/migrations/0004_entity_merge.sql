-- Migration 0004_entity_merge (STATUS.md task C4a)
-- APPEND-ONLY: once this migration has shipped, never edit it; add 0005_*.sql.
--
-- Adds support for non-lossy entity merges. When two entities merge, the loser
-- entity is marked with merged_into pointing to the survivor, preserving
-- history and enabling rollback.
--
-- - merged_into: If not NULL, this entity was merged into the entity with that id.
--   The survivor keeps this as NULL.

ALTER TABLE entity ADD COLUMN merged_into TEXT NULL;
