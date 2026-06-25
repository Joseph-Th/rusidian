-- Migration 0002_extend_source (STATUS.md task B2 prep - extends C1)
-- APPEND-ONLY: once this migration has shipped, never edit it; add 0003_*.sql.
--
-- Extends the source table with fields added in task C1:
-- - captured_at: when the source was captured (Timestamp, RFC3339)
-- - content_hash: SHA256 hash for deduplication (String)
-- - ingestion_state: pipeline state (enum, snake_case)
--
-- Note: We use CURRENT_TIMESTAMP which returns RFC3339 format natively in sqlite.
-- The 'Z' suffix ensures UTC timezone is explicit.

ALTER TABLE source ADD COLUMN captured_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
ALTER TABLE source ADD COLUMN content_hash TEXT NOT NULL DEFAULT '';
ALTER TABLE source ADD COLUMN ingestion_state TEXT NOT NULL DEFAULT 'captured';
