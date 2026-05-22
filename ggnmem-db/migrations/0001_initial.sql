PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    os_context TEXT NOT NULL,
    hostname TEXT NOT NULL,
    shell TEXT,
    started_at_ms INTEGER NOT NULL,
    ended_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS commands (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    command TEXT NOT NULL,
    normalized_command TEXT NOT NULL,
    cwd TEXT NOT NULL,
    exit_code INTEGER,
    duration_ms INTEGER,
    started_at_ms INTEGER,
    completed_at_ms INTEGER NOT NULL,
    content_hash TEXT NOT NULL UNIQUE,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS command_metadata (
    command_id TEXT PRIMARY KEY,
    run_count INTEGER NOT NULL DEFAULT 1 CHECK (run_count >= 1),
    first_seen_at_ms INTEGER NOT NULL,
    last_seen_at_ms INTEGER NOT NULL,
    last_exit_code INTEGER,
    last_duration_ms INTEGER,
    embedding_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (embedding_status IN ('pending', 'indexed', 'failed', 'skipped')),
    FOREIGN KEY(command_id) REFERENCES commands(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS command_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    command_id TEXT NOT NULL,
    queue_kind TEXT NOT NULL DEFAULT 'embedding'
        CHECK (queue_kind IN ('embedding', 'fts_rebuild', 'metadata_repair')),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'processing', 'failed')),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    available_at_ms INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    last_error TEXT,
    FOREIGN KEY(command_id) REFERENCES commands(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sessions_started_at_ms
    ON sessions(started_at_ms);

CREATE INDEX IF NOT EXISTS idx_commands_session_id
    ON commands(session_id);

CREATE INDEX IF NOT EXISTS idx_commands_completed_at_ms
    ON commands(completed_at_ms);

CREATE INDEX IF NOT EXISTS idx_commands_cwd
    ON commands(cwd);

CREATE INDEX IF NOT EXISTS idx_commands_content_hash
    ON commands(content_hash);

CREATE INDEX IF NOT EXISTS idx_command_metadata_last_seen_at_ms
    ON command_metadata(last_seen_at_ms);

CREATE INDEX IF NOT EXISTS idx_command_queue_status_available
    ON command_queue(status, available_at_ms);

CREATE VIRTUAL TABLE IF NOT EXISTS commands_fts USING fts5(
    command,
    cwd,
    content='commands',
    content_rowid='rowid',
    tokenize='trigram'
);

CREATE TRIGGER IF NOT EXISTS commands_fts_insert
AFTER INSERT ON commands
BEGIN
    INSERT INTO commands_fts(rowid, command, cwd)
    VALUES (new.rowid, new.command, new.cwd);
END;

CREATE TRIGGER IF NOT EXISTS commands_fts_delete
AFTER DELETE ON commands
BEGIN
    INSERT INTO commands_fts(commands_fts, rowid, command, cwd)
    VALUES ('delete', old.rowid, old.command, old.cwd);
END;

CREATE TRIGGER IF NOT EXISTS commands_fts_update
AFTER UPDATE ON commands
BEGIN
    INSERT INTO commands_fts(commands_fts, rowid, command, cwd)
    VALUES ('delete', old.rowid, old.command, old.cwd);
    INSERT INTO commands_fts(rowid, command, cwd)
    VALUES (new.rowid, new.command, new.cwd);
END;

CREATE TABLE IF NOT EXISTS vector_index_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    provider TEXT NOT NULL DEFAULT 'sqlite-vec',
    embedding_dim INTEGER NOT NULL DEFAULT 384,
    metric TEXT NOT NULL DEFAULT 'cosine',
    status TEXT NOT NULL DEFAULT 'placeholder'
);

INSERT OR IGNORE INTO vector_index_state(id)
VALUES (1);

-- sqlite-vec integration placeholder:
-- A later phase will register the sqlite-vec extension through C-FFI and create
-- the command embedding virtual table. This migration intentionally does not
-- create that virtual table yet so database initialization works before the
-- extension is linked.
--
-- Planned shape:
-- CREATE VIRTUAL TABLE command_embeddings USING vec0(
--     command_id TEXT PRIMARY KEY,
--     embedding FLOAT[384]
-- );
