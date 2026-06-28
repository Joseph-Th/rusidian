-- Migration 0013_add_ingestion_queue
-- APPEND-ONLY: once this migration has shipped, never edit it.
--
-- Ingestion queue table for rate-limited bulk ingestion.

CREATE TABLE ingestion_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    processed_at TEXT,
    error_message TEXT
);
