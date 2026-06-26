-- Migration 0008_add_note_metadata (A0: Tauri app CRUD)
-- APPEND-ONLY: once this migration has shipped, never edit it; add 0009_*.sql.
--
-- Adds metadata support to notes as a JSON column.

-- Add metadata to note table (JSON string, empty object by default)
ALTER TABLE note ADD COLUMN metadata TEXT NOT NULL DEFAULT '{}';
