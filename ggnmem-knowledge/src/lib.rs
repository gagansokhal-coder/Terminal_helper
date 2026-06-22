//! Offline knowledge base for ggnmem.
//!
//! Provides command suggestions, explanations, and learning mode.
//! All data is local — no cloud APIs, no LLMs.
//!
//! Sources:
//! 1. Built-in knowledge packs (compiled into binary via `include_str!`)
//! 2. User-defined packs under `~/.config/ggnmem/knowledge/*.json` or `*.toml`
//!
//! Both sources are loaded and merged at runtime.

pub mod entries;
pub mod search;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use entries::{
    format_confidence, AskResult, ExplainResult, FlagEntry, KnowledgeEntry, KnowledgePack,
    LearnCategory, LearnCommand, LearnResult, SimpleEntry,
};

// ─── Built-in knowledge packs (compiled into binary) ─────────────────────────

const BUILTIN_DOCKER: &str = include_str!("../data/docker.json");
const BUILTIN_GIT: &str = include_str!("../data/git.json");
const BUILTIN_LINUX: &str = include_str!("../data/linux.json");
const BUILTIN_CARGO: &str = include_str!("../data/cargo.json");
const BUILTIN_GO: &str = include_str!("../data/go.json");
const BUILTIN_KUBERNETES: &str = include_str!("../data/kubernetes.json");

/// The central knowledge base combining built-in and user packs.
pub struct KnowledgeBase {
    /// All entries from all loaded packs.
    entries: Vec<KnowledgeEntry>,
    /// Topic metadata: topic_name → description.
    topics: HashMap<String, String>,
    /// Sources of loaded packs: (name, source, entry_count).
    pack_sources: Vec<PackSource>,
    /// Errors encountered while loading user packs.
    load_errors: Vec<String>,
}

/// Information about a loaded knowledge pack.
#[derive(Debug, Clone)]
pub struct PackSource {
    /// Topic/pack name.
    pub name: String,
    /// Where this pack came from ("builtin" or file path).
    pub source: String,
    /// Number of entries loaded from this pack.
    pub entry_count: usize,
}

impl KnowledgeBase {
    /// Load the knowledge base from built-in packs and user directory.
    ///
    /// User packs are loaded from `~/.config/ggnmem/knowledge/` if the
    /// directory exists. Files can be `.json` or `.toml`.
    pub fn new() -> Self {
        let mut kb = Self {
            entries: Vec::new(),
            topics: HashMap::new(),
            pack_sources: Vec::new(),
            load_errors: Vec::new(),
        };

        // Load built-in packs.
        kb.load_builtin();

        // Load user packs.
        if let Some(user_dir) = user_knowledge_dir() {
            kb.load_user_dir(&user_dir);
        }

        kb
    }

    /// Load only built-in packs (no user directory).
    #[must_use]
    pub fn builtin_only() -> Self {
        let mut kb = Self {
            entries: Vec::new(),
            topics: HashMap::new(),
            pack_sources: Vec::new(),
            load_errors: Vec::new(),
        };
        kb.load_builtin();
        kb
    }

    /// Load user packs from a specific directory (for testing).
    #[must_use]
    pub fn with_user_dir(user_dir: &Path) -> Self {
        let mut kb = Self {
            entries: Vec::new(),
            topics: HashMap::new(),
            pack_sources: Vec::new(),
            load_errors: Vec::new(),
        };
        kb.load_builtin();
        kb.load_user_dir(user_dir);
        kb
    }

    /// Ask a natural language question and get command suggestions.
    ///
    /// Returns the top matches ranked by confidence.
    pub fn ask(&self, query: &str, limit: usize) -> Vec<AskResult> {
        let mut results = search::rank_entries(query, &self.entries, 0.15);
        results.truncate(limit);
        results
    }

    /// Explain a command: purpose, flags, examples.
    ///
    /// First tries exact match, then falls back to prefix/partial matching.
    pub fn explain(&self, command: &str) -> Option<ExplainResult> {
        let cmd_lower = command.to_lowercase();

        // Exact match.
        if let Some(entry) = self
            .entries
            .iter()
            .find(|e| e.command.to_lowercase() == cmd_lower)
        {
            return Some(entry_to_explain(entry));
        }

        // Prefix match (e.g., "docker" matches "docker ps", "docker images", etc.)
        // Return the entry whose command is the closest prefix match.
        let prefix_matches: Vec<&KnowledgeEntry> = self
            .entries
            .iter()
            .filter(|e| e.command.to_lowercase().starts_with(&cmd_lower))
            .collect();

        if prefix_matches.len() == 1 {
            return Some(entry_to_explain(prefix_matches[0]));
        }

        // Partial match: command contains the query or vice versa.
        self.entries
            .iter()
            .find(|e| {
                let el = e.command.to_lowercase();
                el.contains(&cmd_lower) || cmd_lower.contains(&el)
            })
            .map(entry_to_explain)
    }

    /// Learn about a topic: returns commands grouped by category, ordered by difficulty.
    pub fn learn(&self, topic: &str) -> Option<LearnResult> {
        let topic_lower = topic.to_lowercase();

        let topic_entries: Vec<&KnowledgeEntry> = self
            .entries
            .iter()
            .filter(|e| e.topic.to_lowercase() == topic_lower)
            .collect();

        if topic_entries.is_empty() {
            return None;
        }

        let description = self
            .topics
            .get(&topic_lower)
            .cloned()
            .unwrap_or_else(|| format!("{topic} commands"));

        // Group by category.
        let mut cat_map: HashMap<String, Vec<LearnCommand>> = HashMap::new();
        for entry in &topic_entries {
            cat_map
                .entry(entry.category.clone())
                .or_default()
                .push(LearnCommand {
                    command: entry.command.clone(),
                    description: entry.description.clone(),
                    difficulty: entry.difficulty,
                });
        }

        // Sort commands within each category by difficulty.
        let mut categories: Vec<LearnCategory> = cat_map
            .into_iter()
            .map(|(name, mut commands)| {
                commands.sort_by_key(|c| c.difficulty);
                LearnCategory { name, commands }
            })
            .collect();

        // Sort categories alphabetically.
        categories.sort_by(|a, b| a.name.cmp(&b.name));

        Some(LearnResult {
            topic: topic_lower,
            description,
            categories,
        })
    }

    /// List all available topics.
    pub fn topics(&self) -> Vec<(String, String)> {
        let mut topics: Vec<(String, String)> = self.topics.clone().into_iter().collect();
        topics.sort_by(|a, b| a.0.cmp(&b.0));
        topics
    }

    /// Total number of entries in the knowledge base.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get all pack sources for diagnostics.
    pub fn pack_sources(&self) -> &[PackSource] {
        &self.pack_sources
    }

    /// Get any errors encountered while loading user packs.
    pub fn load_errors(&self) -> &[String] {
        &self.load_errors
    }

    /// Get the user knowledge directory path.
    pub fn user_dir() -> Option<PathBuf> {
        user_knowledge_dir()
    }

    // ─── Private loading methods ─────────────────────────────────────────

    fn load_builtin(&mut self) {
        let builtins: &[(&str, &str)] = &[
            ("docker", BUILTIN_DOCKER),
            ("git", BUILTIN_GIT),
            ("linux", BUILTIN_LINUX),
            ("cargo", BUILTIN_CARGO),
            ("go", BUILTIN_GO),
            ("kubernetes", BUILTIN_KUBERNETES),
        ];

        for (name, json_str) in builtins {
            if let Ok(pack) = serde_json::from_str::<KnowledgePack>(json_str) {
                let count = pack.entries.len();
                self.topics
                    .insert(pack.topic.to_lowercase(), pack.description.clone());
                self.entries.extend(pack.entries);
                self.pack_sources.push(PackSource {
                    name: name.to_string(),
                    source: "builtin".to_owned(),
                    entry_count: count,
                });
            }
        }
    }

    fn load_user_dir(&mut self, dir: &Path) {
        if !dir.is_dir() {
            return;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            match path.extension().and_then(|e| e.to_str()) {
                Some("json") => self.load_json_file(&path),
                Some("toml") => self.load_toml_file(&path),
                _ => {} // Skip other files.
            }
        }
    }

    fn load_json_file(&mut self, path: &Path) {
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                self.load_errors
                    .push(format!("{}: failed to read: {e}", path.display()));
                return;
            }
        };

        // Try full KnowledgePack format first.
        if let Ok(pack) = serde_json::from_str::<KnowledgePack>(&contents) {
            let count = pack.entries.len();
            self.topics
                .insert(pack.topic.to_lowercase(), pack.description.clone());
            self.entries.extend(pack.entries);
            self.pack_sources.push(PackSource {
                name: pack.topic,
                source: path.display().to_string(),
                entry_count: count,
            });
            return;
        }

        // Fallback: try simple array format [{command, description, ...}].
        let topic_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("custom")
            .to_owned();

        if let Ok(simple_entries) = serde_json::from_str::<Vec<entries::SimpleEntry>>(&contents) {
            if simple_entries.is_empty() {
                self.load_errors
                    .push(format!("{}: empty entries array", path.display()));
                return;
            }
            let count = simple_entries.len();
            let topic_lower = topic_name.to_lowercase();
            self.topics
                .entry(topic_lower.clone())
                .or_insert_with(|| format!("Custom: {topic_name}"));
            for entry in simple_entries {
                self.entries.push(entry.into_knowledge_entry(&topic_lower));
            }
            self.pack_sources.push(PackSource {
                name: topic_name,
                source: path.display().to_string(),
                entry_count: count,
            });
            return;
        }

        // Neither format matched.
        self.load_errors.push(format!(
            "{}: invalid format. Expected a KnowledgePack object or an array of {{command, description}} entries.",
            path.display()
        ));
    }

    fn load_toml_file(&mut self, path: &Path) {
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                self.load_errors
                    .push(format!("{}: failed to read: {e}", path.display()));
                return;
            }
        };
        if let Ok(pack) = toml::from_str::<KnowledgePack>(&contents) {
            let count = pack.entries.len();
            self.topics
                .insert(pack.topic.to_lowercase(), pack.description.clone());
            self.entries.extend(pack.entries);
            self.pack_sources.push(PackSource {
                name: pack.topic,
                source: path.display().to_string(),
                entry_count: count,
            });
        } else {
            self.load_errors.push(format!(
                "{}: failed to parse as TOML KnowledgePack",
                path.display()
            ));
        }
    }
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a `KnowledgeEntry` to an `ExplainResult`.
fn entry_to_explain(entry: &KnowledgeEntry) -> ExplainResult {
    ExplainResult {
        command: entry.command.clone(),
        purpose: entry.description.clone(),
        category: entry.category.clone(),
        topic: entry.topic.clone(),
        flags: entry.flags.clone(),
        examples: entry.examples.clone(),
    }
}

fn user_knowledge_dir() -> Option<PathBuf> {
    ggnmem_paths::knowledge_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_knowledge_loads() {
        let kb = KnowledgeBase::builtin_only();
        assert!(kb.entry_count() > 0, "expected entries, got 0");
    }

    #[test]
    fn topics_are_loaded() {
        let kb = KnowledgeBase::builtin_only();
        let topics = kb.topics();
        assert!(
            topics.len() >= 6,
            "expected >= 6 topics, got {}",
            topics.len()
        );
    }

    #[test]
    fn ask_docker_ps() {
        let kb = KnowledgeBase::builtin_only();
        let results = kb.ask("show running containers", 5);
        assert!(
            !results.is_empty(),
            "expected results for 'show running containers'"
        );
        assert_eq!(results[0].command, "docker ps");
    }

    #[test]
    fn ask_git_status() {
        let kb = KnowledgeBase::builtin_only();
        let results = kb.ask("check git changes", 5);
        assert!(
            !results.is_empty(),
            "expected results for 'check git changes'"
        );
        assert_eq!(results[0].command, "git status");
    }

    #[test]
    fn ask_cargo_build() {
        let kb = KnowledgeBase::builtin_only();
        let results = kb.ask("build rust project", 5);
        assert!(
            !results.is_empty(),
            "expected results for 'build rust project'"
        );
        assert!(results[0].command.contains("cargo build"));
    }

    #[test]
    fn explain_docker_ps() {
        let kb = KnowledgeBase::builtin_only();
        let result = kb.explain("docker ps");
        assert!(result.is_some(), "expected explain result for 'docker ps'");
        let r = result.unwrap();
        assert_eq!(r.command, "docker ps");
        assert!(!r.purpose.is_empty());
    }

    #[test]
    fn explain_git_status() {
        let kb = KnowledgeBase::builtin_only();
        let result = kb.explain("git status");
        assert!(result.is_some(), "expected explain result for 'git status'");
    }

    #[test]
    fn learn_docker() {
        let kb = KnowledgeBase::builtin_only();
        let result = kb.learn("docker");
        assert!(result.is_some(), "expected learn result for 'docker'");
        let r = result.unwrap();
        assert!(!r.categories.is_empty());
    }

    #[test]
    fn learn_nonexistent_topic() {
        let kb = KnowledgeBase::builtin_only();
        assert!(kb.learn("nonexistent").is_none());
    }

    #[test]
    fn user_packs_loaded() {
        let tmp = tempfile::TempDir::new().unwrap();
        let knowledge_dir = tmp.path();

        // Create a user pack.
        let pack = r#"{
            "topic": "mytools",
            "description": "My custom tools",
            "entries": [
                {
                    "command": "mytool run",
                    "description": "Run my custom tool",
                    "category": "general",
                    "topic": "mytools",
                    "aliases": ["run my tool"],
                    "flags": [],
                    "examples": ["mytool run --verbose"]
                }
            ]
        }"#;
        std::fs::write(knowledge_dir.join("mytools.json"), pack).unwrap();

        let kb = KnowledgeBase::with_user_dir(knowledge_dir);

        // Should find the user entry.
        let results = kb.ask("run my tool", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].command, "mytool run");
    }

    #[test]
    fn user_toml_pack_loaded() {
        let tmp = tempfile::TempDir::new().unwrap();
        let knowledge_dir = tmp.path();

        let pack = r#"
topic = "mytools"
description = "My custom tools"

[[entries]]
command = "mytool check"
description = "Run checks"
category = "general"
topic = "mytools"
aliases = ["check my tool"]
flags = []
examples = ["mytool check --all"]
"#;
        std::fs::write(knowledge_dir.join("mytools.toml"), pack).unwrap();

        let kb = KnowledgeBase::with_user_dir(knowledge_dir);

        let results = kb.ask("check my tool", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].command, "mytool check");
    }
}
