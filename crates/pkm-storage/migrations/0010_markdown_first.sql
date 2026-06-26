-- Migration 0010_markdown_first (Markdown-first architecture)
-- APPEND-ONLY: once this migration has shipped, never edit it; add 0011_*.sql.
--
-- Adds file_path column to note table to track where markdown files are stored.
-- This enables the hybrid markdown-file + SQLite-index architecture where:
-- - Notes are written to disk as markdown files
-- - SQLite maintains an FTS5 index for fast search
-- - block and note content columns remain for FTS5 indexing

-- Add file_path to note table (TEXT, NOT NULL with empty string default)
ALTER TABLE note ADD COLUMN file_path TEXT NOT NULL DEFAULT '';
