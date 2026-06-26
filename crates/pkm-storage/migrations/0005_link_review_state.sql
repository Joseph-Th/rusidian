-- Migration 0005_link_review_state (STATUS.md task C4d)
-- APPEND-ONLY: once this migration has shipped, never edit it; add 0006_*.sql.
--
-- Adds review state and confidence to links to support link provenance tracking.
-- All existing links default to Proposed review state (not yet reviewed by user).

ALTER TABLE link ADD COLUMN reviewed TEXT NOT NULL DEFAULT 'proposed';
ALTER TABLE link ADD COLUMN confidence REAL;
