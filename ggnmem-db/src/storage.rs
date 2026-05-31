use std::time::Instant;

use rusqlite::{named_params, Connection, ErrorCode, OptionalExtension};

use crate::{
    config::DatabaseConfig,
    connection,
    domain::{CommandId, CommandRecord, NewCommand, NewSession, SessionId, SessionRecord},
    error::DbResult,
    filter,
    hash::{content_hash, normalize_command},
};

/// Statistics returned after a cleanup operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleanupStats {
    /// Number of internal command rows deleted.
    pub removed: u64,
    /// Number of commands remaining after cleanup.
    pub remaining: u64,
}

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(config: &DatabaseConfig) -> DbResult<Self> {
        Ok(Self {
            connection: connection::open_database(config)?,
        })
    }

    #[must_use]
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn insert_session(&self, session: &NewSession) -> DbResult<()> {
        self.connection.execute(
            r#"
            INSERT OR IGNORE INTO sessions (
                id,
                os_context,
                hostname,
                shell,
                started_at_ms,
                created_at_ms
            )
            VALUES (
                :id,
                :os_context,
                :hostname,
                :shell,
                :started_at_ms,
                :created_at_ms
            )
            "#,
            named_params! {
                ":id": session.id.as_str(),
                ":os_context": session.os_context.as_str(),
                ":hostname": session.hostname.as_str(),
                ":shell": session.shell.as_deref(),
                ":started_at_ms": session.started_at_ms,
                ":created_at_ms": session.started_at_ms,
            },
        )?;
        Ok(())
    }

    pub fn insert_command(&self, command: &NewCommand) -> DbResult<()> {
        let normalized_command = normalize_command(&command.command);
        let hash = content_hash(&command.command, &command.cwd);

        self.connection.execute(
            r#"
            INSERT INTO commands (
                id,
                session_id,
                command,
                normalized_command,
                cwd,
                exit_code,
                duration_ms,
                started_at_ms,
                completed_at_ms,
                content_hash,
                created_at_ms,
                updated_at_ms
            )
            VALUES (
                :id,
                :session_id,
                :command,
                :normalized_command,
                :cwd,
                :exit_code,
                :duration_ms,
                :started_at_ms,
                :completed_at_ms,
                :content_hash,
                :created_at_ms,
                :updated_at_ms
            )
            ON CONFLICT(content_hash) DO UPDATE SET
                updated_at_ms = excluded.updated_at_ms,
                exit_code = excluded.exit_code,
                duration_ms = excluded.duration_ms,
                started_at_ms = excluded.started_at_ms,
                completed_at_ms = excluded.completed_at_ms
            "#,
            named_params! {
                ":id": command.id.as_str(),
                ":session_id": command.session_id.as_str(),
                ":command": command.command.as_str(),
                ":normalized_command": normalized_command,
                ":cwd": command.cwd.as_str(),
                ":exit_code": command.exit_code,
                ":duration_ms": command.duration_ms,
                ":started_at_ms": command.started_at_ms,
                ":completed_at_ms": command.completed_at_ms,
                ":content_hash": hash,
                ":created_at_ms": command.completed_at_ms,
                ":updated_at_ms": command.completed_at_ms,
            },
        )?;

        self.connection.execute(
            r#"
            INSERT INTO command_metadata (
                command_id,
                first_seen_at_ms,
                last_seen_at_ms,
                last_exit_code,
                last_duration_ms
            )
            SELECT
                id,
                completed_at_ms,
                completed_at_ms,
                exit_code,
                duration_ms
            FROM commands
            WHERE content_hash = :content_hash
            ON CONFLICT(command_id) DO UPDATE SET
                run_count = command_metadata.run_count + 1,
                last_seen_at_ms = excluded.last_seen_at_ms,
                last_exit_code = excluded.last_exit_code,
                last_duration_ms = excluded.last_duration_ms
            "#,
            named_params! {
                ":content_hash": content_hash(&command.command, &command.cwd),
            },
        )?;

        self.connection.execute(
            r#"
            INSERT INTO command_queue (
                command_id,
                available_at_ms,
                created_at_ms,
                updated_at_ms
            )
            SELECT
                id,
                completed_at_ms,
                completed_at_ms,
                completed_at_ms
            FROM commands
            WHERE content_hash = :content_hash
            "#,
            named_params! {
                ":content_hash": content_hash(&command.command, &command.cwd),
            },
        )?;

        Ok(())
    }

    pub fn list_recent_commands(&self, limit: u32) -> DbResult<Vec<CommandRecord>> {
        let mut stmt = self.connection.prepare(
            r#"
            SELECT
                id,
                session_id,
                command,
                normalized_command,
                cwd,
                exit_code,
                duration_ms,
                started_at_ms,
                completed_at_ms,
                content_hash,
                created_at_ms,
                updated_at_ms
            FROM commands
            ORDER BY completed_at_ms DESC
            LIMIT :limit
            "#,
        )?;

        let rows = stmt.query_map(named_params! { ":limit": limit }, |row| {
            Ok(CommandRecord {
                id: CommandId::from_storage(row.get::<_, String>(0)?),
                session_id: SessionId::from_storage(row.get::<_, String>(1)?),
                command: row.get(2)?,
                normalized_command: row.get(3)?,
                cwd: row.get(4)?,
                exit_code: row.get(5)?,
                duration_ms: row.get(6)?,
                started_at_ms: row.get(7)?,
                completed_at_ms: row.get(8)?,
                content_hash: row.get(9)?,
                created_at_ms: row.get(10)?,
                updated_at_ms: row.get(11)?,
            })
        })?;

        let mut commands = Vec::new();
        for row in rows {
            commands.push(row?);
        }
        Ok(commands)
    }

    pub fn count_commands(&self) -> DbResult<u64> {
        let count: u64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM commands", [], |row| row.get(0))?;
        Ok(count)
    }

    /// List all commands as `(id, command_text)` pairs for the embedding indexer.
    ///
    /// Returns every unique command in the database.  The indexer
    /// cross-references these IDs against the vector store to find
    /// un-indexed commands.
    pub fn list_commands_for_indexing(&self) -> DbResult<Vec<(String, String)>> {
        let mut stmt = self
            .connection
            .prepare("SELECT id, command FROM commands ORDER BY completed_at_ms ASC")?;
        let pairs: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(pairs)
    }

    /// Look up command details by command ID.
    ///
    /// Used by semantic search to enrich vector matches with metadata.
    pub fn get_command_by_id(&self, id: &str) -> DbResult<Option<CommandRecord>> {
        self.connection
            .query_row(
                r#"
                SELECT
                    id,
                    session_id,
                    command,
                    normalized_command,
                    cwd,
                    exit_code,
                    duration_ms,
                    started_at_ms,
                    completed_at_ms,
                    content_hash,
                    created_at_ms,
                    updated_at_ms
                FROM commands
                WHERE id = :id
                "#,
                named_params! { ":id": id },
                |row| {
                    Ok(CommandRecord {
                        id: CommandId::from_storage(row.get::<_, String>(0)?),
                        session_id: SessionId::from_storage(row.get::<_, String>(1)?),
                        command: row.get(2)?,
                        normalized_command: row.get(3)?,
                        cwd: row.get(4)?,
                        exit_code: row.get(5)?,
                        duration_ms: row.get(6)?,
                        started_at_ms: row.get(7)?,
                        completed_at_ms: row.get(8)?,
                        content_hash: row.get(9)?,
                        created_at_ms: row.get(10)?,
                        updated_at_ms: row.get(11)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn get_session(&self, id: &SessionId) -> DbResult<Option<SessionRecord>> {
        self.connection
            .query_row(
                r#"
                SELECT id, os_context, hostname, shell, started_at_ms, ended_at_ms, created_at_ms
                FROM sessions
                WHERE id = :id
                "#,
                named_params! { ":id": id.as_str() },
                |row| {
                    Ok(SessionRecord {
                        id: SessionId::from_storage(row.get::<_, String>(0)?),
                        os_context: row.get(1)?,
                        hostname: row.get(2)?,
                        shell: row.get(3)?,
                        started_at_ms: row.get(4)?,
                        ended_at_ms: row.get(5)?,
                        created_at_ms: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn get_command_by_hash(&self, hash: &str) -> DbResult<Option<CommandRecord>> {
        self.connection
            .query_row(
                r#"
                SELECT
                    id,
                    session_id,
                    command,
                    normalized_command,
                    cwd,
                    exit_code,
                    duration_ms,
                    started_at_ms,
                    completed_at_ms,
                    content_hash,
                    created_at_ms,
                    updated_at_ms
                FROM commands
                WHERE content_hash = :content_hash
                "#,
                named_params! { ":content_hash": hash },
                |row| {
                    Ok(CommandRecord {
                        id: CommandId::from_storage(row.get::<_, String>(0)?),
                        session_id: SessionId::from_storage(row.get::<_, String>(1)?),
                        command: row.get(2)?,
                        normalized_command: row.get(3)?,
                        cwd: row.get(4)?,
                        exit_code: row.get(5)?,
                        duration_ms: row.get(6)?,
                        started_at_ms: row.get(7)?,
                        completed_at_ms: row.get(8)?,
                        content_hash: row.get(9)?,
                        created_at_ms: row.get(10)?,
                        updated_at_ms: row.get(11)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    // ─── Phase 5 backward compat ─────────────────────────────────────────────

    /// Simple search (Phase 5 backward-compatible entry point).
    /// Delegates to `search_commands_v2` with default options.
    pub fn search_commands(
        &self,
        query: &str,
        limit: u32,
    ) -> DbResult<Vec<crate::domain::SearchResult>> {
        let opts = crate::domain::SearchOptions::new(query).with_limit(limit);
        self.search_commands_v2(&opts)
    }

    // ─── Phase 6 intelligent search ──────────────────────────────────────────

    /// Intelligent search with multi-strategy matching and weighted scoring.
    ///
    /// Strategy cascade:
    /// 1. FTS5 trigram MATCH (requires 3+ char query)
    /// 2. Prefix scan (for short queries like "gi" → "git")
    /// 3. Edit-distance fuzzy matching (typo tolerance)
    ///
    /// Scoring (configurable via `SearchOptions::weights`):
    /// - 50% relevance (exact > prefix > partial > fuzzy)
    /// - 20% recency
    /// - 20% frequency
    /// - 10% cwd similarity
    pub fn search_commands_v2(
        &self,
        opts: &crate::domain::SearchOptions,
    ) -> DbResult<Vec<crate::domain::SearchResult>> {
        use crate::domain::MatchKind;
        use crate::fuzzy;

        let trimmed = opts.query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let query_lower = trimmed.to_lowercase();
        let fetch_limit = opts.limit.saturating_mul(8).max(200);

        // ── Stage 1: FTS5 trigram candidates (only for 3+ char queries) ──
        let mut candidates = if trimmed.len() >= 3 {
            self.fts_search(trimmed, fetch_limit)?
        } else {
            Vec::new()
        };

        // ── Stage 2: Prefix + fuzzy scan ─────────────────────────────────
        // Always run if we have fewer results than requested.
        // This catches short queries ("gi" → "git") and typos ("gt" → "git").
        if candidates.len() < opts.limit as usize {
            let max_dist = fuzzy::max_distance_for_query(trimmed.len());
            let scan_results =
                self.prefix_and_fuzzy_scan(trimmed, max_dist, fetch_limit, &candidates)?;
            candidates.extend(scan_results);
        }

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        // ── Classify match kinds ─────────────────────────────────────────
        for c in &mut candidates {
            c.match_kind = classify_match(&query_lower, &c.command, &c.cwd);
        }

        // ── If recent_only, skip scoring and sort by time ────────────────
        if opts.recent_only {
            candidates.sort_by_key(|c| std::cmp::Reverse(c.completed_at_ms));
            candidates.truncate(opts.limit as usize);
            // Assign monotonically decreasing scores for display purposes.
            let len = candidates.len() as f64;
            for (i, c) in candidates.iter_mut().enumerate() {
                c.score = 1.0 - (i as f64 / len.max(1.0));
            }
            return Ok(candidates);
        }

        // ── Compute normalized scores ────────────────────────────────────
        let (min_ts, max_ts) = candidates.iter().fold((i64::MAX, i64::MIN), |(lo, hi), c| {
            (lo.min(c.completed_at_ms), hi.max(c.completed_at_ms))
        });
        let max_run_count = candidates.iter().map(|c| c.run_count).max().unwrap_or(1);

        let w = &opts.weights;

        for c in &mut candidates {
            // ── Ranking protection: internal commands → score = 0 ─────
            if filter::is_internal_command(&c.command) {
                c.score = 0.0;
                continue;
            }

            // Match quality score [0.0, 1.0].
            let match_score = match c.match_kind {
                MatchKind::Exact => 1.0,
                MatchKind::Prefix => 0.75,
                MatchKind::Partial => 0.5,
                MatchKind::Fuzzy => 0.25,
            };

            // Recency score [0.0, 1.0].
            let recency_score = if max_ts == min_ts {
                1.0
            } else {
                (c.completed_at_ms - min_ts) as f64 / (max_ts - min_ts) as f64
            };

            // Frequency score [0.0, 1.0].
            let frequency_score = if max_run_count <= 1 {
                0.0
            } else {
                (c.run_count as f64 - 1.0) / (max_run_count as f64 - 1.0)
            };

            // CWD similarity score [0.0, 1.0].
            let cwd_score = if let Some(ref search_cwd) = opts.cwd {
                fuzzy::cwd_similarity(search_cwd, &c.cwd)
            } else {
                0.0
            };

            c.score = w.exact_weight * match_score
                + w.recency_weight * recency_score
                + w.frequency_weight * frequency_score
                + w.cwd_weight * cwd_score;
        }

        // Sort by score descending, tie-break on recency.
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.completed_at_ms.cmp(&a.completed_at_ms))
        });

        candidates.truncate(opts.limit as usize);
        Ok(candidates)
    }

    /// FTS5 trigram search for substring hits.
    fn fts_search(&self, query: &str, limit: u32) -> DbResult<Vec<crate::domain::SearchResult>> {
        use crate::domain::{MatchKind, SearchResult};

        let mut stmt = self.connection.prepare(
            r#"
            SELECT
                c.command,
                c.cwd,
                c.exit_code,
                c.duration_ms,
                c.completed_at_ms,
                COALESCE(m.run_count, 1) AS run_count
            FROM commands_fts f
            JOIN commands c ON c.rowid = f.rowid
            LEFT JOIN command_metadata m ON m.command_id = c.id
            WHERE commands_fts MATCH :query
            ORDER BY c.completed_at_ms DESC
            LIMIT :fetch_limit
            "#,
        )?;

        let rows = stmt.query_map(
            named_params! {
                ":query": query,
                ":fetch_limit": limit,
            },
            |row| {
                Ok(SearchResult {
                    command: row.get(0)?,
                    cwd: row.get(1)?,
                    exit_code: row.get(2)?,
                    duration_ms: row.get(3)?,
                    completed_at_ms: row.get(4)?,
                    run_count: row.get(5)?,
                    match_kind: MatchKind::Partial, // will be reclassified
                    score: 0.0,
                })
            },
        )?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Prefix + fuzzy scan: scan recent commands and match via prefix or edit distance.
    /// Only adds results not already present in `existing`.
    ///
    /// This handles:
    /// - Short prefix queries: "gi" → matches "git status" (prefix match)
    /// - Typo queries: "gt" → matches "git status" (edit distance = 1)
    /// - Combined: catches anything FTS5 trigram misses
    fn prefix_and_fuzzy_scan(
        &self,
        query: &str,
        max_dist: usize,
        limit: u32,
        existing: &[crate::domain::SearchResult],
    ) -> DbResult<Vec<crate::domain::SearchResult>> {
        use crate::domain::{MatchKind, SearchResult};
        use crate::fuzzy;

        let mut stmt = self.connection.prepare(
            r#"
            SELECT
                c.command,
                c.cwd,
                c.exit_code,
                c.duration_ms,
                c.completed_at_ms,
                COALESCE(m.run_count, 1) AS run_count
            FROM commands c
            LEFT JOIN command_metadata m ON m.command_id = c.id
            ORDER BY c.completed_at_ms DESC
            LIMIT :limit
            "#,
        )?;

        let rows = stmt.query_map(named_params! { ":limit": limit }, |row| {
            let command: String = row.get(0)?;
            let cwd: String = row.get(1)?;
            let exit_code: Option<i32> = row.get(2)?;
            let duration_ms: Option<i64> = row.get(3)?;
            let completed_at_ms: i64 = row.get(4)?;
            let run_count: u64 = row.get(5)?;
            Ok((
                command,
                cwd,
                exit_code,
                duration_ms,
                completed_at_ms,
                run_count,
            ))
        })?;

        // Dedup against existing results.
        let existing_keys: std::collections::HashSet<(&str, &str)> = existing
            .iter()
            .map(|r| (r.command.as_str(), r.cwd.as_str()))
            .collect();

        let mut results = Vec::new();
        for row in rows {
            let (command, cwd, exit_code, duration_ms, completed_at_ms, run_count) = row?;

            if existing_keys.contains(&(command.as_str(), cwd.as_str())) {
                continue;
            }

            // Strategy 1: Prefix match ("gi" matches "git" token).
            if fuzzy::prefix_match_tokens(query, &command) {
                results.push(SearchResult {
                    command,
                    cwd,
                    exit_code,
                    duration_ms,
                    completed_at_ms,
                    run_count,
                    match_kind: MatchKind::Prefix,
                    score: 0.0,
                });
                continue;
            }

            // Strategy 2: Fuzzy edit-distance match ("gt" matches "git").
            if max_dist > 0 && fuzzy::fuzzy_match_tokens(query, &command, max_dist).is_some() {
                results.push(SearchResult {
                    command,
                    cwd,
                    exit_code,
                    duration_ms,
                    completed_at_ms,
                    run_count,
                    match_kind: MatchKind::Fuzzy,
                    score: 0.0,
                });
            }
        }

        Ok(results)
    }

    // ─── Phase 6C: Database Cleanup ──────────────────────────────────────────

    /// Remove previously indexed internal commands from the database.
    ///
    /// Deletes rows from `commands`, `command_metadata`, `command_queue`, and
    /// the FTS index where the command text matches known internal patterns
    /// (e.g. `ggnmem %`, `history`, `clear`, `exit`).
    ///
    /// Disk compaction is handled by the explicit optimize path.
    pub fn cleanup_internal_commands(&self) -> DbResult<CleanupStats> {
        let before = self.count_commands()?;

        // SQL WHERE clause shared across all related tables.
        // Matches: ggnmem *, ggnmem-*, bare ggnmem, and shell noise.
        const INTERNAL_WHERE: &str = r#"
            command LIKE 'ggnmem %'
            OR command LIKE 'ggnmem-%'
            OR command = 'ggnmem'
            OR LOWER(TRIM(command)) IN ('history', 'clear', 'exit', 'logout', 'reset')
        "#;

        // Delete from command_queue (references command_id).
        // Best-effort: table may not have matching rows.
        let _ = self.connection.execute_batch(&format!(
            "DELETE FROM command_queue WHERE command_id IN (SELECT id FROM commands WHERE {INTERNAL_WHERE});"
        ));

        // Delete from command_metadata (references command_id).
        // Best-effort: table may not have matching rows.
        let _ = self.connection.execute_batch(&format!(
            "DELETE FROM command_metadata WHERE command_id IN (SELECT id FROM commands WHERE {INTERNAL_WHERE});"
        ));

        // Delete from commands (main table).
        // The commands_fts_delete trigger automatically removes matching FTS entries.
        self.connection
            .execute_batch(&format!("DELETE FROM commands WHERE {INTERNAL_WHERE};"))?;

        let remaining = self.count_commands()?;
        let removed = before.saturating_sub(remaining);

        Ok(CleanupStats { removed, remaining })
    }

    // ─── Phase 11: Database Optimization ─────────────────────────────────────

    /// Run `PRAGMA optimize` + `ANALYZE` and compact free pages when safe.
    pub fn optimize(&self) -> DbResult<crate::domain::OptimizeStats> {
        let before_size_bytes = self.database_size_bytes()?;
        let start = Instant::now();

        self.connection.execute_batch("PRAGMA optimize; ANALYZE;")?;

        let vacuum_ran = if self.connection.is_autocommit() && self.freelist_count()? > 0 {
            match self.connection.execute_batch("VACUUM;") {
                Ok(()) => true,
                Err(rusqlite::Error::SqliteFailure(error, _))
                    if matches!(
                        error.code,
                        ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
                    ) =>
                {
                    false
                }
                Err(error) => return Err(error.into()),
            }
        } else {
            false
        };

        self.connection
            .execute_batch("PRAGMA wal_checkpoint(PASSIVE);")?;

        let after_size_bytes = self.database_size_bytes()?;
        let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        let now = crate::time::unix_epoch_millis();
        self.set_last_optimize_at_ms(now)?;

        Ok(crate::domain::OptimizeStats {
            before_size_bytes,
            after_size_bytes,
            elapsed_ms,
            vacuum_ran,
        })
    }

    /// Run `VACUUM` to defragment and compact the database file.
    pub fn vacuum(&self) -> DbResult<()> {
        self.connection.execute_batch("VACUUM;")?;
        Ok(())
    }

    /// Collect database-level statistics.
    pub fn db_stats(&self) -> DbResult<crate::domain::DbStats> {
        let command_count = self.count_commands()?;
        let session_count: u64 =
            self.connection
                .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;
        let metadata_count: u64 =
            self.connection
                .query_row("SELECT COUNT(*) FROM command_metadata", [], |row| {
                    row.get(0)
                })?;
        let queue_count: u64 =
            self.connection
                .query_row("SELECT COUNT(*) FROM command_queue", [], |row| row.get(0))?;
        let page_size = self.page_size()?;
        let page_count = self.page_count()?;
        let freelist_count = self.freelist_count()?;
        let db_size_bytes = page_size * page_count;

        // FTS shadow table row count (best-effort).
        let fts_row_count: u64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM commands_fts", [], |row| row.get(0))
            .unwrap_or(0);
        let duplicate_count_estimate = self.duplicate_count_estimate()?;
        let last_optimize_at_ms = self.get_last_optimize_at_ms()?;

        Ok(crate::domain::DbStats {
            command_count,
            session_count,
            metadata_count,
            queue_count,
            db_size_bytes,
            page_size,
            page_count,
            freelist_count,
            fts_row_count,
            duplicate_count_estimate,
            last_optimize_at_ms,
        })
    }

    /// Collect high-level usage statistics for `ggnmem stats`.
    pub fn usage_stats(&self) -> DbResult<crate::domain::UsageStats> {
        let total_commands = self.count_commands()?;
        let unique_commands: u64 = self.connection.query_row(
            "SELECT COUNT(DISTINCT normalized_command) FROM commands",
            [],
            |row| row.get(0),
        )?;
        let total_sessions: u64 =
            self.connection
                .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;
        let searches_performed = self.searches_performed()?;
        let deduplicated_commands: u64 = self.connection.query_row(
            "SELECT COUNT(*) FROM command_metadata WHERE run_count > 1",
            [],
            |row| row.get(0),
        )?;

        // Top 10 most-used commands by run_count.
        let mut stmt = self.connection.prepare(
            r#"
            SELECT c.normalized_command, m.run_count
            FROM command_metadata m
            JOIN commands c ON c.id = m.command_id
            ORDER BY m.run_count DESC
            LIMIT 10
            "#,
        )?;
        let most_used: Vec<(String, u64)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
            })?
            .filter_map(Result::ok)
            .collect();

        // DB file size (best-effort from PRAGMA).
        let db_size_bytes = self.database_size_bytes()?;
        let (last_cleanup_at_ms, last_cleanup_removed, last_cleanup_remaining) =
            self.cleanup_history()?;
        let last_optimize_at_ms = self.get_last_optimize_at_ms()?;

        Ok(crate::domain::UsageStats {
            total_commands,
            unique_commands,
            total_sessions,
            searches_performed,
            deduplicated_commands,
            most_used,
            db_size_bytes,
            last_cleanup_at_ms,
            last_cleanup_removed,
            last_cleanup_remaining,
            last_optimize_at_ms,
        })
    }

    // ─── Phase 11: Cleanup Engine ────────────────────────────────────────────

    /// Remove commands that failed (non-zero exit code) and were only run once.
    pub fn cleanup_failed(&self) -> DbResult<CleanupStats> {
        let before = self.count_commands()?;

        self.connection.execute_batch(
            r#"
            DELETE FROM command_queue WHERE command_id IN (
                SELECT c.id FROM commands c
                JOIN command_metadata m ON m.command_id = c.id
                WHERE c.exit_code IS NOT NULL AND c.exit_code != 0
                AND m.run_count = 1
            );
            DELETE FROM command_metadata WHERE command_id IN (
                SELECT c.id FROM commands c
                JOIN command_metadata m ON m.command_id = c.id
                WHERE c.exit_code IS NOT NULL AND c.exit_code != 0
                AND m.run_count = 1
            );
            DELETE FROM commands WHERE id IN (
                SELECT c.id FROM commands c
                LEFT JOIN command_metadata m ON m.command_id = c.id
                WHERE c.exit_code IS NOT NULL AND c.exit_code != 0
                AND (m.run_count IS NULL OR m.run_count = 1)
            );
            "#,
        )?;

        let remaining = self.count_commands()?;
        Ok(CleanupStats {
            removed: before.saturating_sub(remaining),
            remaining,
        })
    }

    /// Remove commands older than the given number of days.
    pub fn cleanup_older_than(&self, days: u32) -> DbResult<CleanupStats> {
        let before = self.count_commands()?;
        let cutoff_ms = crate::time::unix_epoch_millis() - (days as i64 * 24 * 60 * 60 * 1000);

        self.connection.execute(
            "DELETE FROM command_queue WHERE command_id IN (SELECT id FROM commands WHERE completed_at_ms < ?1)",
            [cutoff_ms],
        )?;
        self.connection.execute(
            "DELETE FROM command_metadata WHERE command_id IN (SELECT id FROM commands WHERE completed_at_ms < ?1)",
            [cutoff_ms],
        )?;
        self.connection.execute(
            "DELETE FROM commands WHERE completed_at_ms < ?1",
            [cutoff_ms],
        )?;

        let remaining = self.count_commands()?;
        Ok(CleanupStats {
            removed: before.saturating_sub(remaining),
            remaining,
        })
    }

    /// Remove true duplicate rows (same content_hash appearing more than once).
    /// Should be rare because of the UNIQUE constraint, but handles edge cases.
    pub fn cleanup_duplicates(&self) -> DbResult<CleanupStats> {
        let before = self.count_commands()?;

        // Keep the row with the latest updated_at_ms for each content_hash.
        self.connection.execute_batch(
            r#"
            DELETE FROM command_queue WHERE command_id IN (
                SELECT id FROM commands
                WHERE rowid NOT IN (
                    SELECT MIN(rowid) FROM commands GROUP BY content_hash
                )
            );
            DELETE FROM command_metadata WHERE command_id IN (
                SELECT id FROM commands
                WHERE rowid NOT IN (
                    SELECT MIN(rowid) FROM commands GROUP BY content_hash
                )
            );
            DELETE FROM commands
            WHERE rowid NOT IN (
                SELECT MIN(rowid) FROM commands GROUP BY content_hash
            );
            "#,
        )?;

        let remaining = self.count_commands()?;
        Ok(CleanupStats {
            removed: before.saturating_sub(remaining),
            remaining,
        })
    }

    /// Enforce retention policy: remove commands older than max_age_days,
    /// then trim to max_commands if still over the limit.
    pub fn cleanup_by_retention(
        &self,
        max_age_days: u32,
        max_commands: u64,
    ) -> DbResult<CleanupStats> {
        let before = self.count_commands()?;

        // Step 1: age-based cleanup.
        if max_age_days > 0 {
            let cutoff_ms =
                crate::time::unix_epoch_millis() - (max_age_days as i64 * 24 * 60 * 60 * 1000);
            self.connection.execute(
                "DELETE FROM command_queue WHERE command_id IN (SELECT id FROM commands WHERE completed_at_ms < ?1)",
                [cutoff_ms],
            )?;
            self.connection.execute(
                "DELETE FROM command_metadata WHERE command_id IN (SELECT id FROM commands WHERE completed_at_ms < ?1)",
                [cutoff_ms],
            )?;
            self.connection.execute(
                "DELETE FROM commands WHERE completed_at_ms < ?1",
                [cutoff_ms],
            )?;
        }

        // Step 2: count-based trim — keep only the N most recent.
        if max_commands > 0 {
            let current = self.count_commands()?;
            if current > max_commands {
                self.connection.execute(
                    r#"
                    DELETE FROM command_queue WHERE command_id IN (
                        SELECT id FROM commands
                        ORDER BY completed_at_ms DESC
                        LIMIT -1 OFFSET ?1
                    )
                    "#,
                    [max_commands],
                )?;
                self.connection.execute(
                    r#"
                    DELETE FROM command_metadata WHERE command_id IN (
                        SELECT id FROM commands
                        ORDER BY completed_at_ms DESC
                        LIMIT -1 OFFSET ?1
                    )
                    "#,
                    [max_commands],
                )?;
                self.connection.execute(
                    r#"
                    DELETE FROM commands WHERE id NOT IN (
                        SELECT id FROM commands
                        ORDER BY completed_at_ms DESC
                        LIMIT ?1
                    )
                    "#,
                    [max_commands],
                )?;
            }
        }

        let remaining = self.count_commands()?;
        Ok(CleanupStats {
            removed: before.saturating_sub(remaining),
            remaining,
        })
    }

    /// Dispatch cleanup based on the requested mode.
    pub fn cleanup_by_mode(&self, mode: &crate::domain::CleanupMode) -> DbResult<CleanupStats> {
        use crate::domain::CleanupMode;
        let stats = match mode {
            CleanupMode::Internal => self.cleanup_internal_commands(),
            CleanupMode::Duplicates => self.cleanup_duplicates(),
            CleanupMode::Failed => self.cleanup_failed(),
            CleanupMode::OlderThan(days) => self.cleanup_older_than(*days),
            CleanupMode::Retention {
                max_age_days,
                max_commands,
            } => self.cleanup_by_retention(*max_age_days, *max_commands),
        }?;
        self.record_cleanup(crate::time::unix_epoch_millis(), stats)?;
        Ok(stats)
    }

    /// Run the daemon-owned automatic cleanup policy.
    pub fn run_automatic_cleanup(
        &self,
        max_age_days: u32,
        max_commands: u64,
    ) -> DbResult<CleanupStats> {
        let internal = self.cleanup_internal_commands()?;
        let retention = self.cleanup_by_retention(max_age_days, max_commands)?;
        let stats = CleanupStats {
            removed: internal.removed + retention.removed,
            remaining: retention.remaining,
        };
        self.record_cleanup(crate::time::unix_epoch_millis(), stats)?;
        Ok(stats)
    }

    // ─── Phase 11: Hybrid Retention Scheduling ───────────────────────────────

    /// Get the timestamp (ms since epoch) of the last cleanup run.
    pub fn get_last_cleanup_at_ms(&self) -> DbResult<i64> {
        self.connection
            .query_row(
                "SELECT last_cleanup_at_ms FROM retention_meta WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    /// Update the last cleanup timestamp to now.
    pub fn set_last_cleanup_at_ms(&self, now_ms: i64) -> DbResult<()> {
        self.connection.execute(
            "UPDATE retention_meta SET last_cleanup_at_ms = ?1 WHERE id = 1",
            [now_ms],
        )?;
        self.connection.execute(
            "UPDATE maintenance_meta SET last_cleanup_at_ms = ?1 WHERE id = 1",
            [now_ms],
        )?;
        Ok(())
    }

    pub fn record_cleanup(&self, now_ms: i64, stats: CleanupStats) -> DbResult<()> {
        self.set_last_cleanup_at_ms(now_ms)?;
        let removed = stats.removed.min(i64::MAX as u64) as i64;
        let remaining = stats.remaining.min(i64::MAX as u64) as i64;
        self.connection.execute(
            r#"
            UPDATE maintenance_meta
            SET
                last_cleanup_removed = ?1,
                last_cleanup_remaining = ?2
            WHERE id = 1
            "#,
            [removed, remaining],
        )?;
        Ok(())
    }

    pub fn record_search_performed(&self) -> DbResult<()> {
        self.connection.execute(
            r#"
            UPDATE maintenance_meta
            SET searches_performed = searches_performed + 1
            WHERE id = 1
            "#,
            [],
        )?;
        Ok(())
    }

    fn page_size(&self) -> DbResult<u64> {
        Ok(self
            .connection
            .pragma_query_value(None, "page_size", |row| row.get(0))?)
    }

    fn page_count(&self) -> DbResult<u64> {
        Ok(self
            .connection
            .pragma_query_value(None, "page_count", |row| row.get(0))?)
    }

    fn freelist_count(&self) -> DbResult<u64> {
        Ok(self
            .connection
            .pragma_query_value(None, "freelist_count", |row| row.get(0))?)
    }

    fn database_size_bytes(&self) -> DbResult<u64> {
        Ok(self.page_size()? * self.page_count()?)
    }

    fn duplicate_count_estimate(&self) -> DbResult<u64> {
        let count: i64 = self.connection.query_row(
            "SELECT COALESCE(SUM(run_count - 1), 0) FROM command_metadata WHERE run_count > 1",
            [],
            |row| row.get(0),
        )?;
        Ok(count.max(0) as u64)
    }

    fn searches_performed(&self) -> DbResult<u64> {
        let count: i64 = self.connection.query_row(
            "SELECT searches_performed FROM maintenance_meta WHERE id = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(count.max(0) as u64)
    }

    fn cleanup_history(&self) -> DbResult<(i64, u64, u64)> {
        let (last_cleanup_at_ms, removed, remaining): (i64, i64, i64) = self.connection.query_row(
            r#"
            SELECT
                last_cleanup_at_ms,
                last_cleanup_removed,
                last_cleanup_remaining
            FROM maintenance_meta
            WHERE id = 1
            "#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        Ok((
            last_cleanup_at_ms,
            removed.max(0) as u64,
            remaining.max(0) as u64,
        ))
    }

    fn get_last_optimize_at_ms(&self) -> DbResult<i64> {
        Ok(self.connection.query_row(
            "SELECT last_optimize_at_ms FROM maintenance_meta WHERE id = 1",
            [],
            |row| row.get(0),
        )?)
    }

    fn set_last_optimize_at_ms(&self, now_ms: i64) -> DbResult<()> {
        self.connection.execute(
            "UPDATE maintenance_meta SET last_optimize_at_ms = ?1 WHERE id = 1",
            [now_ms],
        )?;
        Ok(())
    }
}

/// Classify how well a query matches a command+cwd pair.
fn classify_match(query_lower: &str, command: &str, cwd: &str) -> crate::domain::MatchKind {
    use crate::domain::MatchKind;

    let cmd_lower = command.to_lowercase();
    let cwd_lower = cwd.to_lowercase();

    // Exact: query appears verbatim as a substring.
    if cmd_lower.contains(query_lower) || cwd_lower.contains(query_lower) {
        return MatchKind::Exact;
    }

    // Prefix: query is a prefix of any whitespace-delimited token.
    for token in cmd_lower.split_whitespace() {
        if token.starts_with(query_lower) {
            return MatchKind::Prefix;
        }
    }

    // Check if the original FTS5 MATCH found this via trigrams
    // (if we're here, the query was in the FTS5 result set but didn't
    // match by exact substring or prefix — trigram partial match).
    //
    // Otherwise it's a fuzzy match (set by the caller).
    MatchKind::Partial
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use crate::{
        config::DatabaseConfig,
        domain::{CommandId, MatchKind, NewCommand, NewSession, SearchOptions, SessionId},
        hash::content_hash,
    };

    use super::*;

    /// Helper to create a test database with a session and insert commands.
    fn setup_db_with_commands(
        commands: &[(&str, &str, i64)],
    ) -> (NamedTempFile, Database, SessionId) {
        let temp = NamedTempFile::new().expect("temp db");
        let database = Database::open(&DatabaseConfig::new(temp.path().to_path_buf()))
            .expect("database opens");
        let session_id = SessionId::new();
        let base_ts = commands.first().map(|c| c.2).unwrap_or(1_725_000_000_000);

        database
            .insert_session(&NewSession {
                id: session_id.clone(),
                os_context: "linux".to_owned(),
                hostname: "devbox".to_owned(),
                shell: Some("zsh".to_owned()),
                started_at_ms: base_ts,
            })
            .expect("session inserted");

        for (cmd, cwd, ts) in commands {
            database
                .insert_command(&NewCommand {
                    id: CommandId::new(),
                    session_id: session_id.clone(),
                    command: cmd.to_string(),
                    cwd: cwd.to_string(),
                    exit_code: Some(0),
                    duration_ms: Some(10),
                    started_at_ms: Some(*ts - 5),
                    completed_at_ms: *ts,
                })
                .expect("command inserted");
        }

        (temp, database, session_id)
    }

    #[test]
    fn test_retention_meta_defaults_to_zero() {
        let temp = NamedTempFile::new().expect("temp db");
        let database = Database::open(&DatabaseConfig::new(temp.path().to_path_buf()))
            .expect("database opens");

        assert_eq!(
            database
                .get_last_cleanup_at_ms()
                .expect("last cleanup timestamp"),
            0
        );
    }

    #[test]
    fn test_set_and_get_last_cleanup() {
        let temp = NamedTempFile::new().expect("temp db");
        let database = Database::open(&DatabaseConfig::new(temp.path().to_path_buf()))
            .expect("database opens");
        let timestamp = 1_725_123_456_789_i64;

        database
            .set_last_cleanup_at_ms(timestamp)
            .expect("timestamp updates");

        assert_eq!(
            database
                .get_last_cleanup_at_ms()
                .expect("last cleanup timestamp"),
            timestamp
        );
    }

    #[test]
    fn database_initializes_and_persists_command_contract() {
        let temp = NamedTempFile::new().expect("temp db");
        let database = Database::open(&DatabaseConfig::new(temp.path().to_path_buf()))
            .expect("database opens");
        let session_id = SessionId::new();
        let started_at_ms = 1_725_000_000_000;

        database
            .insert_session(&NewSession {
                id: session_id.clone(),
                os_context: "linux".to_owned(),
                hostname: "devbox".to_owned(),
                shell: Some("zsh".to_owned()),
                started_at_ms,
            })
            .expect("session inserted");

        database
            .insert_command(&NewCommand {
                id: CommandId::new(),
                session_id,
                command: "git   status".to_owned(),
                cwd: "/tmp/project".to_owned(),
                exit_code: Some(0),
                duration_ms: Some(12),
                started_at_ms: Some(started_at_ms + 10),
                completed_at_ms: started_at_ms + 22,
            })
            .expect("command inserted");

        let stored = database
            .get_command_by_hash(&content_hash("git status", "/tmp/project"))
            .expect("command query")
            .expect("command exists");

        assert_eq!(stored.normalized_command, "git status");
        assert_eq!(stored.cwd, "/tmp/project");
    }

    #[test]
    fn search_commands_returns_matching_results() {
        let base_ts = 1_725_000_000_000_i64;
        let (_temp, database, _) = setup_db_with_commands(&[
            ("docker ps", "/home/user", base_ts + 100),
            ("docker build .", "/home/user/project", base_ts + 200),
            ("git status", "/home/user/project", base_ts + 300),
            ("git log --oneline", "/home/user/project", base_ts + 400),
            ("cargo build", "/home/user/project", base_ts + 500),
            ("cargo test", "/home/user/project", base_ts + 600),
        ]);

        // Search for "docker": should return the 2 docker commands.
        let results = database
            .search_commands("docker", 20)
            .expect("search works");
        assert_eq!(results.len(), 2, "should find 2 docker commands");
        assert!(
            results.iter().all(|r| r.command.contains("docker")),
            "all results should contain 'docker'"
        );

        // Search for "git": should return the 2 git commands.
        let results = database.search_commands("git", 20).expect("search works");
        assert_eq!(results.len(), 2, "should find 2 git commands");

        // Search for "cargo": should return the 2 cargo commands.
        let results = database.search_commands("cargo", 20).expect("search works");
        assert_eq!(results.len(), 2, "should find 2 cargo commands");

        // Empty query should return nothing.
        let results = database.search_commands("", 20).expect("search works");
        assert!(results.is_empty(), "empty query returns no results");

        // Query with no matches.
        let results = database
            .search_commands("nonexistent_xyz", 20)
            .expect("search works");
        assert!(results.is_empty(), "no-match query returns no results");

        // Limit is respected.
        let results = database.search_commands("docker", 1).expect("search works");
        assert_eq!(results.len(), 1, "limit=1 should return 1 result");
    }

    #[test]
    fn search_v2_fuzzy_matches_typos() {
        let base_ts = 1_725_000_000_000_i64;
        let (_temp, database, _) = setup_db_with_commands(&[
            ("docker ps", "/home/user", base_ts + 100),
            ("docker build .", "/home/user/project", base_ts + 200),
            ("git status", "/home/user/project", base_ts + 300),
        ]);

        // Typo: "dockr" should fuzzy-match "docker" commands.
        let opts = SearchOptions::new("dockr").with_limit(20);
        let results = database
            .search_commands_v2(&opts)
            .expect("fuzzy search works");
        assert!(
            !results.is_empty(),
            "fuzzy search should find docker commands for 'dockr'"
        );
        assert!(
            results.iter().any(|r| r.command.contains("docker")),
            "should match docker commands"
        );
    }

    #[test]
    fn search_v2_prefix_short_query() {
        let base_ts = 1_725_000_000_000_i64;
        let (_temp, database, _) = setup_db_with_commands(&[
            ("git status", "/home/user/project", base_ts + 100),
            ("git log --oneline", "/home/user/project", base_ts + 200),
            ("docker ps", "/home/user", base_ts + 300),
            ("cargo build", "/home/user/project", base_ts + 400),
        ]);

        // "gi" should match git commands via prefix scan.
        let opts = SearchOptions::new("gi").with_limit(20);
        let results = database
            .search_commands_v2(&opts)
            .expect("prefix search works");
        assert!(
            !results.is_empty(),
            "'gi' should find git commands via prefix"
        );
        assert!(
            results.iter().all(|r| r.command.starts_with("git")),
            "all results should be git commands"
        );

        // "dock" should match docker commands via prefix.
        let opts = SearchOptions::new("dock").with_limit(20);
        let results = database
            .search_commands_v2(&opts)
            .expect("prefix search works");
        assert!(!results.is_empty(), "'dock' should find docker commands");

        // "car" should match cargo commands.
        let opts = SearchOptions::new("car").with_limit(20);
        let results = database
            .search_commands_v2(&opts)
            .expect("prefix search works");
        assert!(!results.is_empty(), "'car' should find cargo commands");
    }

    #[test]
    fn search_v2_edit_distance_short_query() {
        let base_ts = 1_725_000_000_000_i64;
        let (_temp, database, _) = setup_db_with_commands(&[
            ("git status", "/home/user/project", base_ts + 100),
            ("git log --oneline", "/home/user/project", base_ts + 200),
            ("docker ps", "/home/user", base_ts + 300),
        ]);

        // "gt" should fuzzy-match "git" (edit distance = 1).
        let opts = SearchOptions::new("gt").with_limit(20);
        let results = database
            .search_commands_v2(&opts)
            .expect("edit distance search works");
        assert!(
            !results.is_empty(),
            "'gt' should find git commands via edit distance"
        );
        assert!(
            results.iter().any(|r| r.command.contains("git")),
            "should match git commands"
        );
    }

    #[test]
    fn search_v2_compose_expansion() {
        let base_ts = 1_725_000_000_000_i64;
        let (_temp, database, _) = setup_db_with_commands(&[
            ("docker compose up", "/home/user/project", base_ts + 100),
            ("docker compose down", "/home/user/project", base_ts + 200),
            ("docker ps", "/home/user", base_ts + 300),
        ]);

        // "compose" should match docker compose commands.
        let opts = SearchOptions::new("compose").with_limit(20);
        let results = database
            .search_commands_v2(&opts)
            .expect("compose search works");
        assert!(
            results.len() >= 2,
            "'compose' should find docker compose commands, got {}",
            results.len()
        );
    }

    #[test]
    fn search_v2_cwd_boosting() {
        let base_ts = 1_725_000_000_000_i64;
        // Use the same timestamp so recency is equal — cwd weight breaks the tie.
        let (_temp, database, _) = setup_db_with_commands(&[
            ("git status", "/home/user/project-a", base_ts + 100),
            ("git status", "/home/user/project-b", base_ts + 100),
        ]);

        // Search with cwd matching project-a should boost that result.
        let opts = SearchOptions::new("git")
            .with_limit(20)
            .with_cwd("/home/user/project-a");
        let results = database
            .search_commands_v2(&opts)
            .expect("cwd search works");
        assert_eq!(results.len(), 2);
        // The project-a result should rank higher due to cwd similarity.
        assert_eq!(results[0].cwd, "/home/user/project-a");
    }

    #[test]
    fn search_v2_recent_only_mode() {
        let base_ts = 1_725_000_000_000_i64;
        let (_temp, database, _) = setup_db_with_commands(&[
            ("docker ps", "/home/user", base_ts + 100),
            ("docker build .", "/home/user/project", base_ts + 200),
            ("docker compose up", "/home/user/project", base_ts + 300),
        ]);

        let opts = SearchOptions::new("docker")
            .with_limit(20)
            .with_recent_only(true);
        let results = database
            .search_commands_v2(&opts)
            .expect("recent_only search works");
        assert_eq!(results.len(), 3);
        // Should be sorted by recency, newest first.
        assert!(results[0].completed_at_ms >= results[1].completed_at_ms);
        assert!(results[1].completed_at_ms >= results[2].completed_at_ms);
    }

    #[test]
    fn search_v2_exact_matches_rank_highest() {
        let base_ts = 1_725_000_000_000_i64;
        let (_temp, database, _) = setup_db_with_commands(&[
            ("docker ps", "/home/user", base_ts + 100),
            ("git status", "/home/user", base_ts + 200),
            ("docker compose up", "/home/user", base_ts + 300),
        ]);

        let opts = SearchOptions::new("docker").with_limit(20);
        let results = database.search_commands_v2(&opts).expect("search works");
        // All docker results should be exact matches.
        for r in &results {
            assert!(r.command.contains("docker"));
            assert_eq!(r.match_kind, MatchKind::Exact);
        }
    }

    #[test]
    fn search_v2_scores_are_populated() {
        let base_ts = 1_725_000_000_000_i64;
        let (_temp, database, _) = setup_db_with_commands(&[
            ("docker ps", "/home/user", base_ts + 100),
            ("docker build .", "/home/user/project", base_ts + 200),
        ]);

        let opts = SearchOptions::new("docker").with_limit(20);
        let results = database.search_commands_v2(&opts).expect("search works");
        for r in &results {
            assert!(r.score > 0.0, "score should be positive, got {}", r.score);
            assert!(r.score <= 1.0, "score should be <= 1.0, got {}", r.score);
        }
    }

    #[test]
    fn benchmark_search_1000_commands() {
        use std::time::Instant;

        let temp = NamedTempFile::new().expect("temp db");
        let database = Database::open(&DatabaseConfig::new(temp.path().to_path_buf()))
            .expect("database opens");
        let session_id = SessionId::new();
        let base_ts = 1_725_000_000_000_i64;

        database
            .insert_session(&NewSession {
                id: session_id.clone(),
                os_context: "linux".to_owned(),
                hostname: "devbox".to_owned(),
                shell: Some("zsh".to_owned()),
                started_at_ms: base_ts,
            })
            .expect("session inserted");

        // Generate 1200 synthetic commands.
        let tool_names = [
            "docker",
            "git",
            "cargo",
            "npm",
            "kubectl",
            "terraform",
            "ansible",
            "python",
            "rustc",
            "gcc",
            "make",
            "cmake",
            "curl",
            "wget",
            "ssh",
            "scp",
            "rsync",
            "find",
            "grep",
            "sed",
            "awk",
            "cat",
            "less",
            "vim",
        ];
        let subcommands = [
            "build", "run", "test", "deploy", "status", "log", "push", "pull", "commit",
            "checkout", "init", "install", "update", "list", "exec", "stop", "start", "restart",
            "version", "help",
        ];
        let cwds = [
            "/home/user/project-a",
            "/home/user/project-b",
            "/opt/services/api",
            "/var/log",
            "/tmp/scratch",
        ];

        for i in 0..1200 {
            let tool = tool_names[i % tool_names.len()];
            let sub = subcommands[i % subcommands.len()];
            let cwd = cwds[i % cwds.len()];
            let cmd = format!("{tool} {sub} --flag-{i}");

            database
                .insert_command(&NewCommand {
                    id: CommandId::new(),
                    session_id: session_id.clone(),
                    command: cmd,
                    cwd: cwd.to_string(),
                    exit_code: Some(if i % 10 == 0 { 1 } else { 0 }),
                    duration_ms: Some((i as i64 % 500) + 10),
                    started_at_ms: Some(base_ts + (i as i64 * 1000)),
                    completed_at_ms: base_ts + (i as i64 * 1000) + 500,
                })
                .expect("command inserted");
        }

        let total = database.count_commands().expect("count");
        assert!(
            total >= 1200,
            "should have at least 1200 commands, got {total}"
        );

        // Benchmark search queries.
        let queries = ["docker", "git", "cargo", "dockr", "kubectl"];
        for query in &queries {
            let start = Instant::now();
            let opts = SearchOptions::new(*query).with_limit(20);
            let results = database.search_commands_v2(&opts).expect("search");
            let elapsed = start.elapsed();

            assert!(
                elapsed.as_millis() < 100,
                "search for '{query}' took {}ms (target <100ms)",
                elapsed.as_millis()
            );
            assert!(
                !results.is_empty(),
                "search for '{query}' should return results"
            );
            // Verify scores are sorted descending.
            for w in results.windows(2) {
                assert!(
                    w[0].score >= w[1].score,
                    "results should be sorted by score descending"
                );
            }
        }

        // Benchmark with cwd filter.
        let start = Instant::now();
        let opts = SearchOptions::new("docker")
            .with_limit(20)
            .with_cwd("/home/user/project-a");
        let _results = database.search_commands_v2(&opts).expect("cwd search");
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 100,
            "cwd-boosted search took {}ms (target <100ms)",
            elapsed.as_millis()
        );
    }

    // ─── Phase 6C: Cleanup & ranking protection ─────────────────────────────

    #[test]
    fn cleanup_removes_internal_commands() {
        let base_ts = 1_725_000_000_000_i64;
        let (_temp, database, _) = setup_db_with_commands(&[
            ("git status", "/home/user", base_ts + 100),
            ("ggnmem search docker", "/home/user", base_ts + 200),
            ("ggnmem recent", "/home/user", base_ts + 300),
            ("ggnmem doctor", "/home/user", base_ts + 400),
            ("docker ps", "/home/user", base_ts + 500),
            ("cargo build", "/home/user/project", base_ts + 600),
        ]);

        let before = database.count_commands().expect("count before");
        assert_eq!(before, 6, "should have 6 commands before cleanup");

        let stats = database
            .cleanup_internal_commands()
            .expect("cleanup succeeds");

        assert_eq!(stats.removed, 3, "should remove 3 ggnmem commands");
        assert_eq!(stats.remaining, 3, "should have 3 commands remaining");

        // Verify the right commands survived.
        let remaining = database.list_recent_commands(10).expect("list");
        let cmds: Vec<&str> = remaining.iter().map(|r| r.command.as_str()).collect();
        assert!(cmds.contains(&"git status"), "git status should survive");
        assert!(cmds.contains(&"docker ps"), "docker ps should survive");
        assert!(cmds.contains(&"cargo build"), "cargo build should survive");
        assert!(
            !cmds.iter().any(|c| c.starts_with("ggnmem")),
            "no ggnmem commands should survive"
        );
    }

    #[test]
    fn cleanup_is_idempotent() {
        let base_ts = 1_725_000_000_000_i64;
        let (_temp, database, _) = setup_db_with_commands(&[
            ("ggnmem search docker", "/home/user", base_ts + 100),
            ("git status", "/home/user", base_ts + 200),
        ]);

        let stats1 = database.cleanup_internal_commands().expect("first cleanup");
        assert_eq!(stats1.removed, 1);

        let stats2 = database
            .cleanup_internal_commands()
            .expect("second cleanup");
        assert_eq!(stats2.removed, 0, "second cleanup should find nothing");
        assert_eq!(stats2.remaining, 1);
    }

    #[test]
    fn ranking_protection_scores_internal_commands_zero() {
        let base_ts = 1_725_000_000_000_i64;
        // Bypass the ingestion filter by inserting directly.
        let (_temp, database, _) = setup_db_with_commands(&[
            ("git status", "/home/user/project", base_ts + 100),
            ("git log --oneline", "/home/user/project", base_ts + 200),
            ("ggnmem search git", "/home/user", base_ts + 300),
        ]);

        let opts = SearchOptions::new("git").with_limit(20);
        let results = database.search_commands_v2(&opts).expect("search works");

        // The ggnmem command should have score = 0 and appear last.
        for r in &results {
            if r.command.starts_with("ggnmem") {
                assert_eq!(
                    r.score, 0.0,
                    "internal command should have score 0, got {}",
                    r.score
                );
            } else {
                assert!(
                    r.score > 0.0,
                    "normal command should have positive score, got {}",
                    r.score
                );
            }
        }
    }

    // ─── Phase 11: Stress Test ───────────────────────────────────────────────

    #[test]
    #[ignore] // Run manually with: cargo test stress_test_100k_commands -- --ignored
    fn stress_test_100k_commands() {
        let temp = NamedTempFile::new().expect("temp file");
        let path = temp.path().to_path_buf();
        let database = Database::open(&crate::config::DatabaseConfig::new(path)).expect("db");

        let base_ts = 1_700_000_000_000_i64;
        let session = crate::domain::NewSession {
            id: crate::domain::SessionId::from_storage("stress-session".to_string()),
            os_context: "linux".to_string(),
            hostname: "test".to_string(),
            shell: Some("bash".to_string()),
            started_at_ms: base_ts,
        };
        database.insert_session(&session).expect("insert session");

        let start_insert = Instant::now();
        for i in 0..100_000 {
            let cmd_text = format!("git commit -m \"stress test insert {}\"", i);
            database
                .insert_command(&crate::domain::NewCommand {
                    id: crate::domain::CommandId::from_storage(format!("cmd-{}", i)),
                    session_id: crate::domain::SessionId::from_storage(
                        "stress-session".to_string(),
                    ),
                    command: cmd_text,
                    cwd: "/home/user/project".to_string(),
                    exit_code: Some(0),
                    duration_ms: Some(10),
                    started_at_ms: Some(base_ts + i as i64),
                    completed_at_ms: base_ts + i as i64,
                })
                .unwrap();
        }
        let insert_elapsed = start_insert.elapsed();

        // Optimize
        let start_opt = Instant::now();
        database.optimize().unwrap();
        let opt_elapsed = start_opt.elapsed();

        // Search
        let start_search = Instant::now();
        let opts = SearchOptions::new("stress test insert 9999").with_limit(10);
        let results = database.search_commands_v2(&opts).unwrap();
        let search_elapsed = start_search.elapsed();

        assert!(!results.is_empty(), "should find the test command");

        println!(
            "100k commands stress test:\n  insert: {:?}\n  optimize+vacuum: {:?}\n  search: {:?}",
            insert_elapsed, opt_elapsed, search_elapsed
        );
    }
}
