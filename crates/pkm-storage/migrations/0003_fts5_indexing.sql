-- Migration 0003_fts5_indexing (STATUS.md task E2)
-- APPEND-ONLY: once shipped, never edit; add 0004_*.sql.
--
-- Creates FTS5 virtual tables for full-text search across notes, blocks,
-- sources, and entities. FTS5 enables efficient keyword and phrase search.
-- Each table tracks the object type and id so results can be mapped back.

-- FTS5 virtual table for notes
CREATE VIRTUAL TABLE note_fts USING fts5(
    title,
    content,
    content = 'note',
    content_rowid = 'id'
);

-- FTS5 virtual table for blocks
CREATE VIRTUAL TABLE block_fts USING fts5(
    content,
    content = 'block',
    content_rowid = 'id'
);

-- FTS5 virtual table for sources
CREATE VIRTUAL TABLE source_fts USING fts5(
    title,
    raw_content,
    content = 'source',
    content_rowid = 'id'
);

-- FTS5 virtual table for entities
CREATE VIRTUAL TABLE entity_fts USING fts5(
    name,
    aliases,
    content = 'entity',
    content_rowid = 'id'
);

-- Populate the FTS5 tables with existing data
INSERT INTO note_fts(rowid, title, content)
SELECT id, title, '' FROM note;

INSERT INTO block_fts(rowid, content)
SELECT id, content FROM block;

INSERT INTO source_fts(rowid, title, raw_content)
SELECT id, title, raw_content FROM source;

INSERT INTO entity_fts(rowid, name, aliases)
SELECT id, name, aliases FROM entity;
