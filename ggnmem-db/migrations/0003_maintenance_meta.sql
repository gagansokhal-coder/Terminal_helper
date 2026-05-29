CREATE TABLE IF NOT EXISTS maintenance_meta (
    id                       INTEGER PRIMARY KEY CHECK (id = 1),
    last_optimize_at_ms       INTEGER NOT NULL DEFAULT 0,
    last_cleanup_at_ms        INTEGER NOT NULL DEFAULT 0,
    last_cleanup_removed      INTEGER NOT NULL DEFAULT 0,
    last_cleanup_remaining    INTEGER NOT NULL DEFAULT 0,
    searches_performed        INTEGER NOT NULL DEFAULT 0
);

INSERT OR IGNORE INTO maintenance_meta (
    id,
    last_optimize_at_ms,
    last_cleanup_at_ms,
    last_cleanup_removed,
    last_cleanup_remaining,
    searches_performed
)
VALUES (1, 0, 0, 0, 0, 0);

UPDATE maintenance_meta
SET last_cleanup_at_ms = COALESCE(
    (SELECT last_cleanup_at_ms FROM retention_meta WHERE id = 1),
    last_cleanup_at_ms
)
WHERE id = 1;
