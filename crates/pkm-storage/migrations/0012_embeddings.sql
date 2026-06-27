-- Migration 0012_embeddings (Visualization H0)
-- APPEND-ONLY: once this migration has shipped, never edit it; add 0013_*.sql.
--
-- Embeddings table for semantic clustering and similarity search.
-- Stores dense vector representations (as serialized arrays) for notes and entities.
--
-- - object_type: Type of object (note, entity, source, block)
-- - object_id: The ID of the object (UUID string)
-- - embedding: BLOB containing serialized f32 vector (row-major order)
-- - model_id: Optional identifier for the embedding model used
-- - created_at: When the embedding was generated

CREATE TABLE object_embeddings (
    object_type TEXT NOT NULL,
    object_id TEXT NOT NULL,
    embedding BLOB NOT NULL,
    model_id TEXT NOT NULL DEFAULT 'default',
    created_at TEXT NOT NULL,
    PRIMARY KEY (object_type, object_id)
);

-- Create index for efficient lookups
CREATE INDEX idx_embeddings_created ON object_embeddings(created_at);
