//! Knowledge base entry types and JSON schema.
//!
//! Defines the `KnowledgeEntry` structure used across all knowledge packs.
//! Each entry represents a single command or tool with its description,
//! aliases (natural language queries), flags, and usage examples.

use serde::{Deserialize, Serialize};

/// A single knowledge base entry describing a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    /// The canonical command string (e.g., "docker ps").
    pub command: String,

    /// Short one-line description of what the command does.
    pub description: String,

    /// Category within the topic (e.g., "containers", "networking").
    pub category: String,

    /// The parent topic (e.g., "docker", "git").
    pub topic: String,

    /// Natural language aliases / queries that map to this command.
    /// Used for fuzzy matching in `ask` mode.
    /// e.g., ["show running containers", "list containers", "what containers are running"]
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Common flags with descriptions.
    /// e.g., [("-a", "Show all containers"), ("--format", "Pretty-print using Go template")]
    #[serde(default)]
    pub flags: Vec<FlagEntry>,

    /// Usage examples.
    /// e.g., ["docker ps -a", "docker ps --format '{{.Names}}'"]
    #[serde(default)]
    pub examples: Vec<String>,

    /// Difficulty level for learning mode ordering.
    /// 1 = beginner, 2 = intermediate, 3 = advanced.
    #[serde(default = "default_difficulty")]
    pub difficulty: u8,
}

/// A command flag with its description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagEntry {
    /// The flag string (e.g., "-a", "--all").
    pub flag: String,
    /// Description of what the flag does.
    pub description: String,
}

/// A knowledge pack: a collection of entries for a topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgePack {
    /// Topic name (e.g., "docker", "git").
    pub topic: String,
    /// Human-readable description of the topic.
    pub description: String,
    /// Version of this knowledge pack.
    #[serde(default = "default_version")]
    pub version: String,
    /// All command entries in this pack.
    pub entries: Vec<KnowledgeEntry>,
}

/// Simplified entry format for user-created packs.
///
/// Users can write packs as simple arrays of `{command, description}` or
/// `{query, command, description}` without needing the full `KnowledgePack`
/// wrapper. This struct handles the minimal format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleEntry {
    /// The command string.
    pub command: String,
    /// Short description.
    pub description: String,
    /// Optional natural language query that maps to this command.
    #[serde(default)]
    pub query: Option<String>,
    /// Optional category.
    #[serde(default)]
    pub category: Option<String>,
    /// Optional topic override.
    #[serde(default)]
    pub topic: Option<String>,
}

impl SimpleEntry {
    /// Convert to a full `KnowledgeEntry`, deriving topic from filename.
    pub fn into_knowledge_entry(self, default_topic: &str) -> KnowledgeEntry {
        let topic = self.topic.unwrap_or_else(|| default_topic.to_owned());
        let category = self
            .category
            .unwrap_or_else(|| "general".to_owned());
        let aliases = match self.query {
            Some(q) => vec![q],
            None => Vec::new(),
        };
        KnowledgeEntry {
            command: self.command,
            description: self.description,
            category,
            topic,
            aliases,
            flags: Vec::new(),
            examples: Vec::new(),
            difficulty: 1,
        }
    }
}

/// Result from an `ask` query.
#[derive(Debug, Clone)]
pub struct AskResult {
    /// The suggested command.
    pub command: String,
    /// Short explanation of the command.
    pub description: String,
    /// Category within the topic.
    pub category: String,
    /// The topic this came from (e.g., "docker").
    pub topic: String,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f64,
    /// Source label (e.g., "Knowledge Base").
    pub source: String,
}

/// Result from an `explain` query.
#[derive(Debug, Clone)]
pub struct ExplainResult {
    /// The command being explained.
    pub command: String,
    /// Purpose / description.
    pub purpose: String,
    /// Category within its topic.
    pub category: String,
    /// The topic this came from.
    pub topic: String,
    /// Common flags with descriptions.
    pub flags: Vec<FlagEntry>,
    /// Typical usage examples.
    pub examples: Vec<String>,
}

/// Result from a `learn` query.
#[derive(Debug, Clone)]
pub struct LearnResult {
    /// Topic name.
    pub topic: String,
    /// Topic description.
    pub description: String,
    /// Commands grouped by category, ordered by difficulty.
    pub categories: Vec<LearnCategory>,
}

/// A group of commands within a learning topic.
#[derive(Debug, Clone)]
pub struct LearnCategory {
    /// Category name (e.g., "containers", "images").
    pub name: String,
    /// Commands in this category, ordered by difficulty.
    pub commands: Vec<LearnCommand>,
}

/// A command entry in learning mode.
#[derive(Debug, Clone)]
pub struct LearnCommand {
    /// The command string.
    pub command: String,
    /// Short description.
    pub description: String,
    /// Difficulty level.
    pub difficulty: u8,
}

/// Format a confidence score as a qualitative + numeric label.
///
/// Thresholds:
/// - High   = 90-100%
/// - Medium = 70-89%
/// - Low    = 0-69%
pub fn format_confidence(confidence: f64) -> String {
    let pct = (confidence * 100.0).round() as u32;
    let label = if pct >= 90 {
        "High"
    } else if pct >= 70 {
        "Medium"
    } else {
        "Low"
    };
    format!("{label} ({pct}%)")
}

fn default_difficulty() -> u8 {
    1
}

fn default_version() -> String {
    "1.0".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_confidence_high() {
        assert_eq!(format_confidence(0.95), "High (95%)");
        assert_eq!(format_confidence(1.0), "High (100%)");
        assert_eq!(format_confidence(0.90), "High (90%)");
    }

    #[test]
    fn format_confidence_medium() {
        assert_eq!(format_confidence(0.89), "Medium (89%)");
        assert_eq!(format_confidence(0.70), "Medium (70%)");
        assert_eq!(format_confidence(0.75), "Medium (75%)");
    }

    #[test]
    fn format_confidence_low() {
        assert_eq!(format_confidence(0.69), "Low (69%)");
        assert_eq!(format_confidence(0.50), "Low (50%)");
        assert_eq!(format_confidence(0.0), "Low (0%)");
    }

    #[test]
    fn knowledge_entry_deserialize() {
        let json = r#"{
            "command": "docker ps",
            "description": "List running containers",
            "category": "containers",
            "topic": "docker",
            "aliases": ["show running containers", "list containers"],
            "flags": [{"flag": "-a", "description": "Show all containers"}],
            "examples": ["docker ps -a"],
            "difficulty": 1
        }"#;
        let entry: KnowledgeEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.command, "docker ps");
        assert_eq!(entry.aliases.len(), 2);
        assert_eq!(entry.flags.len(), 1);
        assert_eq!(entry.difficulty, 1);
    }

    #[test]
    fn knowledge_pack_deserialize() {
        let json = r#"{
            "topic": "docker",
            "description": "Docker container management",
            "entries": [
                {
                    "command": "docker ps",
                    "description": "List running containers",
                    "category": "containers",
                    "topic": "docker",
                    "aliases": ["show running containers"],
                    "flags": [],
                    "examples": []
                }
            ]
        }"#;
        let pack: KnowledgePack = serde_json::from_str(json).unwrap();
        assert_eq!(pack.topic, "docker");
        assert_eq!(pack.entries.len(), 1);
    }
}
