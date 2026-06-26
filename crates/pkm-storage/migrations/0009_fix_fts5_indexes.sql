-- Migration 0009_fix_fts5_indexes
-- APPEND-ONLY: once this migration has shipped, never edit it; add 0010_*.sql.
--
-- The original FTS5 tables in 0003 used `content_rowid = 'id'` with TEXT UUID
-- ids. SQLite FTS5 rowids must be INTEGER; the coercion silently set every row's
-- rowid to 0, making search return wrong results. Those tables also had no
-- triggers, so notes/sources created after migration were never indexed.
--
-- Fix: drop and recreate as standalone FTS5 tables with an explicit UNINDEXED
-- 'id' TEXT column. Add AFTER INSERT/UPDATE/DELETE triggers on every content
-- table to keep the FTS indexes in sync at all times.

DROP TABLE IF EXISTS note_fts;
DROP TABLE IF EXISTS block_fts;
DROP TABLE IF EXISTS source_fts;
DROP TABLE IF EXISTS entity_fts;

-- Standalone FTS5 tables — no content= or content_rowid options.
-- 'id' is UNINDEXED so we can filter by it without it affecting search ranking.
CREATE VIRTUAL TABLE note_fts   USING fts5(id UNINDEXED, title, body);
CREATE VIRTUAL TABLE block_fts  USING fts5(id UNINDEXED, body);
CREATE VIRTUAL TABLE source_fts USING fts5(id UNINDEXED, title, body);
CREATE VIRTUAL TABLE entity_fts USING fts5(id UNINDEXED, name, aliases);

-- Populate from current data.
INSERT INTO note_fts(id, title, body)
    SELECT id, title, '' FROM note;

INSERT INTO block_fts(id, body)
    SELECT id, content FROM block;

INSERT INTO source_fts(id, title, body)
    SELECT id, COALESCE(title, ''), raw_content FROM source;

INSERT INTO entity_fts(id, name, aliases)
    SELECT id, name, aliases FROM entity;

-- Note triggers.
CREATE TRIGGER note_fts_insert AFTER INSERT ON note BEGIN
    INSERT INTO note_fts(id, title, body) VALUES (new.id, new.title, '');
END;
CREATE TRIGGER note_fts_update AFTER UPDATE ON note BEGIN
    DELETE FROM note_fts WHERE id = old.id;
    INSERT INTO note_fts(id, title, body) VALUES (new.id, new.title, '');
END;
CREATE TRIGGER note_fts_delete AFTER DELETE ON note BEGIN
    DELETE FROM note_fts WHERE id = old.id;
END;

-- Block triggers.
CREATE TRIGGER block_fts_insert AFTER INSERT ON block BEGIN
    INSERT INTO block_fts(id, body) VALUES (new.id, new.content);
END;
CREATE TRIGGER block_fts_update AFTER UPDATE ON block BEGIN
    DELETE FROM block_fts WHERE id = old.id;
    INSERT INTO block_fts(id, body) VALUES (new.id, new.content);
END;
CREATE TRIGGER block_fts_delete AFTER DELETE ON block BEGIN
    DELETE FROM block_fts WHERE id = old.id;
END;

-- Source triggers.
CREATE TRIGGER source_fts_insert AFTER INSERT ON source BEGIN
    INSERT INTO source_fts(id, title, body) VALUES (new.id, COALESCE(new.title, ''), new.raw_content);
END;
CREATE TRIGGER source_fts_update AFTER UPDATE ON source BEGIN
    DELETE FROM source_fts WHERE id = old.id;
    INSERT INTO source_fts(id, title, body) VALUES (new.id, COALESCE(new.title, ''), new.raw_content);
END;
CREATE TRIGGER source_fts_delete AFTER DELETE ON source BEGIN
    DELETE FROM source_fts WHERE id = old.id;
END;

-- Entity triggers.
CREATE TRIGGER entity_fts_insert AFTER INSERT ON entity BEGIN
    INSERT INTO entity_fts(id, name, aliases) VALUES (new.id, new.name, new.aliases);
END;
CREATE TRIGGER entity_fts_update AFTER UPDATE ON entity BEGIN
    DELETE FROM entity_fts WHERE id = old.id;
    INSERT INTO entity_fts(id, name, aliases) VALUES (new.id, new.name, new.aliases);
END;
CREATE TRIGGER entity_fts_delete AFTER DELETE ON entity BEGIN
    DELETE FROM entity_fts WHERE id = old.id;
END;
