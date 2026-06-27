-- Migration 0011_add_semantic_dates (Visualization H0)
-- APPEND-ONLY: once this migration has shipped, never edit it; add 0012_*.sql.
--
-- Adds semantic_date (RFC3339 formatted) to entity and note for timeline visualizations.
-- Semantic date represents when an event/entity actually occurred, not when it was captured.
--
-- - entity.semantic_date: When the entity/event actually occurred (NULL if unknown).
-- - Note metadata['semantic_date']: When the note's topic/event occurred (JSON string in metadata).

ALTER TABLE entity ADD COLUMN semantic_date TEXT NULL;

-- Create index for efficient timeline queries
CREATE INDEX idx_entity_semantic_date ON entity(semantic_date);
