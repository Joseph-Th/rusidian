-- Migration 0014_standardize_block_order
-- APPEND-ONLY: once this migration has shipped, never edit it.
--
-- Standardize existing numeric block order values by zero-padding them.
UPDATE block
SET "order" = printf('%08d', CAST("order" * 1000 AS INTEGER))
WHERE typeof("order") = 'integer' OR typeof("order") = 'real';
