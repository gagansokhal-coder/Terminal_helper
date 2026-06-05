-- Phase 13: Add hybrid/semantic search metrics to maintenance_meta.
-- Running-average latency: total_search_latency_ms / searches_performed.
ALTER TABLE maintenance_meta ADD COLUMN hybrid_searches      INTEGER NOT NULL DEFAULT 0;
ALTER TABLE maintenance_meta ADD COLUMN semantic_searches     INTEGER NOT NULL DEFAULT 0;
ALTER TABLE maintenance_meta ADD COLUMN total_search_latency_ms INTEGER NOT NULL DEFAULT 0;
