CREATE TABLE IF NOT EXISTS retention_meta (
    id       INTEGER PRIMARY KEY CHECK (id = 1),
    last_cleanup_at_ms  INTEGER NOT NULL DEFAULT 0
);
INSERT OR IGNORE INTO retention_meta (id, last_cleanup_at_ms) VALUES (1, 0);
