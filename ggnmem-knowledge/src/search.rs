//! Fuzzy search engine for the knowledge base.
//!
//! Performs deterministic keyword/n-gram matching against knowledge entries
//! to find the best command match for a natural language query.
//!
//! No LLMs, no cloud APIs — pure string similarity scoring.

use crate::entries::{AskResult, KnowledgeEntry};

/// Score a query against a single knowledge entry.
///
/// Returns a confidence score between 0.0 and 1.0.
///
/// Scoring factors:
/// 1. Exact alias match (1.0)
/// 2. Alias word overlap (0.5-0.95)
/// 3. Command substring match (0.4-0.8)
/// 4. Description word overlap (0.3-0.7)
/// 5. Category/topic match bonus (+0.05)
pub fn score_entry(query: &str, entry: &KnowledgeEntry) -> f64 {
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();

    if query_words.is_empty() {
        return 0.0;
    }

    let mut best_score: f64 = 0.0;

    // 1. Check exact alias match.
    for alias in &entry.aliases {
        let alias_lower = alias.to_lowercase();
        if alias_lower == query_lower {
            return 1.0; // Perfect match.
        }

        // 2. Word overlap with aliases.
        let alias_words: Vec<&str> = alias_lower.split_whitespace().collect();
        let overlap = word_overlap(&query_words, &alias_words);
        if overlap > best_score {
            best_score = overlap;
        }

        // Substring containment (alias in query or query in alias).
        if query_lower.contains(&alias_lower) || alias_lower.contains(&query_lower) {
            let containment = 0.85;
            if containment > best_score {
                best_score = containment;
            }
        }
    }

    // 3. Command substring match.
    let cmd_lower = entry.command.to_lowercase();
    let cmd_words: Vec<&str> = cmd_lower.split_whitespace().collect();
    if cmd_lower == query_lower {
        let score = 0.95;
        if score > best_score {
            best_score = score;
        }
    } else if query_lower.contains(&cmd_lower) || cmd_lower.contains(&query_lower) {
        let score = 0.80;
        if score > best_score {
            best_score = score;
        }
    } else {
        let cmd_overlap = word_overlap(&query_words, &cmd_words);
        let score = cmd_overlap * 0.75;
        if score > best_score {
            best_score = score;
        }
    }

    // 4. Description word overlap.
    let desc_lower = entry.description.to_lowercase();
    let desc_words: Vec<&str> = desc_lower.split_whitespace().collect();
    let desc_overlap = word_overlap(&query_words, &desc_words);
    let desc_score = desc_overlap * 0.70;
    if desc_score > best_score {
        best_score = desc_score;
    }

    // 5. Trigram similarity boost for fuzzy matching.
    let trigram_score = best_alias_trigram_score(&query_lower, entry) * 0.75;
    if trigram_score > best_score {
        best_score = trigram_score;
    }

    // 6. Category/topic bonus: if query words include the topic or category name.
    let topic_lower = entry.topic.to_lowercase();
    let cat_lower = entry.category.to_lowercase();
    if query_words
        .iter()
        .any(|w| *w == topic_lower || *w == cat_lower)
    {
        best_score = (best_score + 0.05).min(1.0);
    }

    best_score
}

/// Rank all entries by relevance to a query.
///
/// Returns entries sorted by descending score, filtered to minimum threshold.
pub fn rank_entries(query: &str, entries: &[KnowledgeEntry], min_score: f64) -> Vec<AskResult> {
    let mut scored: Vec<(f64, &KnowledgeEntry)> = entries
        .iter()
        .map(|e| (score_entry(query, e), e))
        .filter(|(score, _)| *score >= min_score)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    scored
        .into_iter()
        .map(|(score, entry)| AskResult {
            command: entry.command.clone(),
            description: entry.description.clone(),
            category: entry.category.clone(),
            topic: entry.topic.clone(),
            confidence: score,
            source: "Knowledge Base".to_owned(),
        })
        .collect()
}

/// Calculate word overlap ratio between two word sets.
///
/// Returns the Jaccard-like similarity: intersection / min(len_a, len_b).
/// Using min instead of union gives higher scores for short queries
/// that fully overlap with longer aliases.
fn word_overlap(query_words: &[&str], target_words: &[&str]) -> f64 {
    if query_words.is_empty() || target_words.is_empty() {
        return 0.0;
    }

    let matching = query_words
        .iter()
        .filter(|qw| target_words.iter().any(|tw| tw == *qw))
        .count();

    if matching == 0 {
        // Try partial word matching (prefix matching).
        let partial = query_words
            .iter()
            .filter(|qw| {
                qw.len() >= 3
                    && target_words
                        .iter()
                        .any(|tw| tw.starts_with(*qw) || qw.starts_with(tw))
            })
            .count();

        if partial == 0 {
            return 0.0;
        }

        return (partial as f64 / query_words.len().min(target_words.len()) as f64) * 0.7;
    }

    matching as f64 / query_words.len().min(target_words.len()) as f64
}

/// Calculate the best trigram similarity score across all aliases.
fn best_alias_trigram_score(query: &str, entry: &KnowledgeEntry) -> f64 {
    let query_trigrams = trigrams(query);
    if query_trigrams.is_empty() {
        return 0.0;
    }

    let mut best = 0.0f64;

    for alias in &entry.aliases {
        let alias_trigrams = trigrams(&alias.to_lowercase());
        if alias_trigrams.is_empty() {
            continue;
        }
        let intersection = query_trigrams
            .iter()
            .filter(|t| alias_trigrams.contains(t))
            .count();
        let union = query_trigrams.len() + alias_trigrams.len() - intersection;
        if union > 0 {
            let score = intersection as f64 / union as f64;
            if score > best {
                best = score;
            }
        }
    }

    // Also compare against the command itself.
    let cmd_trigrams = trigrams(&entry.command.to_lowercase());
    if !cmd_trigrams.is_empty() {
        let intersection = query_trigrams
            .iter()
            .filter(|t| cmd_trigrams.contains(t))
            .count();
        let union = query_trigrams.len() + cmd_trigrams.len() - intersection;
        if union > 0 {
            let score = intersection as f64 / union as f64;
            if score > best {
                best = score;
            }
        }
    }

    best
}

/// Extract character trigrams from text.
fn trigrams(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 3 {
        return vec![];
    }
    chars.windows(3).map(|w| w.iter().collect()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entries::FlagEntry;

    fn make_entry(command: &str, description: &str, aliases: &[&str]) -> KnowledgeEntry {
        KnowledgeEntry {
            command: command.to_owned(),
            description: description.to_owned(),
            category: "general".to_owned(),
            topic: "docker".to_owned(),
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
            flags: vec![],
            examples: vec![],
            difficulty: 1,
        }
    }

    #[test]
    fn exact_alias_match_scores_perfect() {
        let entry = make_entry(
            "docker ps",
            "List running containers",
            &["show running containers"],
        );
        let score = score_entry("show running containers", &entry);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn partial_alias_overlap_scores_high() {
        let entry = make_entry(
            "docker ps",
            "List running containers",
            &["show running containers", "list containers"],
        );
        let score = score_entry("running containers", &entry);
        assert!(score > 0.7, "expected > 0.7, got {score}");
    }

    #[test]
    fn command_match_scores_well() {
        let entry = make_entry(
            "git status",
            "Show working tree status",
            &["check git changes"],
        );
        let score = score_entry("git status", &entry);
        assert!(score > 0.9, "expected > 0.9, got {score}");
    }

    #[test]
    fn unrelated_query_scores_low() {
        let entry = make_entry(
            "docker ps",
            "List running containers",
            &["show running containers"],
        );
        let score = score_entry("compile rust project", &entry);
        assert!(score < 0.3, "expected < 0.3, got {score}");
    }

    #[test]
    fn rank_entries_returns_sorted() {
        let entries = vec![
            make_entry(
                "docker ps",
                "List running containers",
                &["show running containers"],
            ),
            make_entry(
                "docker images",
                "List images",
                &["show docker images", "list images"],
            ),
            make_entry(
                "git status",
                "Show working tree status",
                &["check git changes"],
            ),
        ];

        let results = rank_entries("show running containers", &entries, 0.1);
        assert!(!results.is_empty());
        assert_eq!(results[0].command, "docker ps");
    }

    #[test]
    fn trigrams_works() {
        let t = trigrams("docker");
        assert_eq!(t.len(), 4); // doc, ock, cke, ker
        assert!(t.contains(&"doc".to_owned()));
    }

    #[test]
    fn word_overlap_full_match() {
        let a = vec!["show", "running", "containers"];
        let b = vec!["show", "running", "containers"];
        let score = word_overlap(&a, &b);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn word_overlap_partial() {
        let a = vec!["show", "containers"];
        let b = vec!["show", "running", "containers"];
        let score = word_overlap(&a, &b);
        assert!((score - 1.0).abs() < f64::EPSILON); // 2/min(2,3) = 1.0
    }

    #[test]
    fn word_overlap_none() {
        let a = vec!["compile", "rust"];
        let b = vec!["show", "running", "containers"];
        let score = word_overlap(&a, &b);
        assert!(score < 0.01);
    }

    #[test]
    fn rank_entries_filters_by_min_score() {
        let entries = vec![
            make_entry(
                "docker ps",
                "List running containers",
                &["show running containers"],
            ),
            make_entry(
                "git status",
                "Show working tree status",
                &["check git changes"],
            ),
        ];
        let results = rank_entries("show running containers", &entries, 0.5);
        // Only docker ps should pass the threshold.
        assert!(results.iter().all(|r| r.confidence >= 0.5));
    }
}
