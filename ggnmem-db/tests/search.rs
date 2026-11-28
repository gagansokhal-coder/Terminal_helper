//! Tests for the search path (`Database::search_commands_v2`): FTS5 trigram
//! matching, the prefix and fuzzy scan, and ranking. Every test gets its own
//! temporary database and inserts commands through the normal insert path, so
//! the FTS triggers and run counts behave as they do in the daemon.

use ggnmem_db::{
    CommandId, Database, DatabaseConfig, MatchKind, NewCommand, NewSession, SearchOptions,
    SearchResult, SessionId,
};
use tempfile::NamedTempFile;

const BASE_TS: i64 = 1_725_000_000_000;

struct TestDb {
    database: Database,
    session: SessionId,
    // Declared last so the connection closes before the file is removed.
    _file: NamedTempFile,
}

impl TestDb {
    fn new() -> Self {
        let file = NamedTempFile::new().expect("temp db");
        let database = Database::open(&DatabaseConfig::new(file.path().to_path_buf()))
            .expect("database opens");
        let session = add_session(&database);
        Self {
            database,
            session,
            _file: file,
        }
    }

    /// Inserts `(command, cwd)` pairs, each one newer than the one before.
    fn with_commands(commands: &[(&str, &str)]) -> Self {
        let db = Self::new();
        for (offset, (command, cwd)) in (0_i64..).zip(commands) {
            db.run(command, cwd, BASE_TS + offset);
        }
        db
    }

    fn run(&self, command: &str, cwd: &str, completed_at_ms: i64) {
        self.run_in(&self.session, command, cwd, completed_at_ms);
    }

    fn run_in(&self, session: &SessionId, command: &str, cwd: &str, completed_at_ms: i64) {
        self.database
            .insert_command(&NewCommand {
                id: CommandId::new(),
                session_id: session.clone(),
                command: command.to_owned(),
                cwd: cwd.to_owned(),
                exit_code: Some(0),
                duration_ms: Some(10),
                started_at_ms: Some(completed_at_ms - 5),
                completed_at_ms,
            })
            .expect("command inserted");
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        self.search_with(&SearchOptions::new(query))
    }

    fn search_with(&self, options: &SearchOptions) -> Vec<SearchResult> {
        self.database
            .search_commands_v2(options)
            .unwrap_or_else(|error| panic!("search for {:?} failed: {error}", options.query))
    }
}

fn add_session(database: &Database) -> SessionId {
    let id = SessionId::new();
    database
        .insert_session(&NewSession {
            id: id.clone(),
            os_context: "linux".to_owned(),
            hostname: "devbox".to_owned(),
            shell: Some("zsh".to_owned()),
            started_at_ms: BASE_TS,
        })
        .expect("session inserted");
    id
}

/// Result commands in alphabetical order, for checks that ignore ranking.
fn sorted_commands(results: &[SearchResult]) -> Vec<&str> {
    let mut commands: Vec<&str> = results.iter().map(|r| r.command.as_str()).collect();
    commands.sort_unstable();
    commands
}

fn sample_db() -> TestDb {
    TestDb::with_commands(&[
        ("git status", "/work/app"),
        ("git commit -m wip", "/work/app"),
        ("git log --oneline", "/work/app"),
        ("docker compose up", "/work/infra"),
        ("cargo test", "/work/app"),
    ])
}

#[test]
fn keyword_returns_only_matching_commands() {
    let results = sample_db().search("docker");

    assert_eq!(sorted_commands(&results), ["docker compose up"]);
    assert_eq!(results[0].match_kind, MatchKind::Exact);
}

#[test]
fn keyword_matches_inside_a_word_and_ignores_case() {
    let db = sample_db();

    assert_eq!(sorted_commands(&db.search("ocke")), ["docker compose up"]);
    assert_eq!(sorted_commands(&db.search("DOCKER")), ["docker compose up"]);
}

#[test]
fn multi_word_query_requires_every_word_in_any_order() {
    let db = sample_db();

    assert_eq!(
        sorted_commands(&db.search("git commit")),
        ["git commit -m wip"]
    );
    assert_eq!(
        sorted_commands(&db.search("commit git")),
        ["git commit -m wip"]
    );
}

#[test]
fn quoted_phrase_matches_only_adjacent_words() {
    let db = TestDb::with_commands(&[
        ("docker compose up", "/work"),
        ("docker run compose-image", "/work"),
    ]);

    assert_eq!(db.search("docker compose").len(), 2);
    assert_eq!(
        sorted_commands(&db.search("\"docker compose\"")),
        ["docker compose up"]
    );
}

#[test]
fn short_query_matches_word_prefixes() {
    let results = sample_db().search("do");

    assert_eq!(sorted_commands(&results), ["docker compose up"]);
}

#[test]
fn single_character_query_gets_no_typo_tolerance() {
    let db = sample_db();

    assert_eq!(
        sorted_commands(&db.search("c")),
        ["cargo test", "docker compose up", "git commit -m wip",]
    );
    assert!(db.search("z").is_empty());
}

#[test]
fn fuzzy_stage_tolerates_a_typo_in_a_single_word() {
    let db = sample_db();

    assert_eq!(sorted_commands(&db.search("dockr")), ["docker compose up"]);
    assert_eq!(sorted_commands(&db.search("carg0")), ["cargo test"]);
    assert!(db.search("kubectl").is_empty());
}

#[test]
fn empty_and_whitespace_queries_return_nothing() {
    let db = sample_db();

    for query in ["", " ", "\t  \n"] {
        assert!(db.search(query).is_empty(), "query {query:?}");
    }
}

#[test]
fn special_characters_do_not_return_errors() {
    let db = sample_db();
    let queries = [
        "*",
        "\"",
        "'",
        "\\",
        "(",
        ")",
        ":",
        "^",
        "-",
        "+",
        ".",
        "%",
        "git \"",
        "ls *",
        "it's",
        "a\\b",
        "--oneline",
        "./run.sh",
        "foo:bar",
        "NOT",
        "AND OR",
        "\"\"\"",
        "'; DROP TABLE commands; --",
    ];

    for query in queries {
        // `search` panics with the query if the database returns an error.
        db.search(query);
    }
    assert_eq!(db.database.count_commands().expect("count"), 5);
}

#[test]
fn punctuation_is_searched_as_literal_text() {
    let db = TestDb::with_commands(&[
        ("git log --oneline", "/work"),
        ("./scripts/deploy.sh staging", "/work"),
        ("echo \"hello world", "/work"),
        ("ls *.rs", "/work"),
        ("cat notes.txt", "/work"),
    ]);

    for (query, expected) in [
        ("--oneline", "git log --oneline"),
        ("./scripts/deploy.sh", "./scripts/deploy.sh staging"),
        ("echo \"hello", "echo \"hello world"),
        ("ls *.rs", "ls *.rs"),
        ("notes.txt", "cat notes.txt"),
    ] {
        assert_eq!(
            sorted_commands(&db.search(query)),
            [expected],
            "query {query:?}"
        );
    }
}

#[test]
fn unicode_commands_are_found() {
    let db = TestDb::with_commands(&[
        ("echo café au lait", "/home/josé"),
        ("grep नमस्ते notes.txt", "/work"),
        ("mkdir 日本語フォルダ", "/work"),
        ("git commit -m '🚀 release'", "/work"),
    ]);

    for (query, expected) in [
        ("café", "echo café au lait"),
        ("नमस्ते", "grep नमस्ते notes.txt"),
        ("日本語", "mkdir 日本語フォルダ"),
        ("🚀 release", "git commit -m '🚀 release'"),
    ] {
        assert_eq!(
            sorted_commands(&db.search(query)),
            [expected],
            "query {query:?}"
        );
    }
}

#[test]
fn same_command_from_two_sessions_is_one_result() {
    let db = TestDb::new();
    let other_session = add_session(&db.database);
    db.run("cargo build", "/work", BASE_TS);
    db.run_in(&other_session, "cargo build", "/work", BASE_TS + 1_000);

    let results = db.search("cargo");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].run_count, 2);
    assert_eq!(results[0].completed_at_ms, BASE_TS + 1_000);
}

#[test]
fn same_command_in_two_directories_stays_separate() {
    let db = TestDb::with_commands(&[("cargo build", "/work/a"), ("cargo build", "/work/b")]);

    let results = db.search("cargo");

    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.run_count == 1));
}

#[test]
fn more_frequent_command_ranks_first_when_recency_is_equal() {
    let db = TestDb::new();
    for _ in 0..3 {
        db.run("make lint", "/work", BASE_TS);
    }
    db.run("make test", "/work", BASE_TS);

    let results = db.search("make");

    assert_eq!(results[0].command, "make lint");
    assert_eq!(results[0].run_count, 3);
    assert!(results[0].score > results[1].score);
}

#[test]
fn newer_command_ranks_first_when_everything_else_is_equal() {
    let db = TestDb::with_commands(&[("make lint", "/work"), ("make test", "/work")]);

    let results = db.search("make");

    assert_eq!(results[0].command, "make test");
}

#[test]
fn cwd_boost_orders_by_how_much_of_the_path_is_shared() {
    let db = TestDb::new();
    for cwd in ["/work/app/api", "/srv/other", "/work/app/web"] {
        db.run("npm start", cwd, BASE_TS);
    }

    let results = db.search_with(&SearchOptions::new("npm").with_cwd("/work/app/api"));
    let order: Vec<&str> = results.iter().map(|r| r.cwd.as_str()).collect();

    assert_eq!(order, ["/work/app/api", "/work/app/web", "/srv/other"]);
}

#[test]
fn limit_caps_the_number_of_results() {
    let db = sample_db();

    let results = db.search_with(&SearchOptions::new("git").with_limit(2));

    assert_eq!(results.len(), 2);
}
