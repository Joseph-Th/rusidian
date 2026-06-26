-- Migration 0006_add_project_field (STATUS.md task G4b)
-- APPEND-ONLY: once this migration has shipped, never edit it; add 0007_*.sql.
--
-- Adds optional project field to notes for efficient filtering by project/tag.
-- Existing notes default to NULL (unassigned).

ALTER TABLE note ADD COLUMN project TEXT;
