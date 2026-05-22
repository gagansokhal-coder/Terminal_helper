//! Lightweight fuzzy string matching utilities.
//!
//! Provides edit-distance (Levenshtein) calculation and a fuzzy scoring
//! function for command search without any AI or external dependencies.

/// Compute the Levenshtein edit distance between two strings.
/// Uses the classic O(min(m,n)) space dynamic-programming algorithm.
#[must_use]
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    // Ensure we iterate over the shorter string in the inner loop.
    let (short, long) = if a_bytes.len() <= b_bytes.len() {
        (a_bytes, b_bytes)
    } else {
        (b_bytes, a_bytes)
    };

    let mut prev_row: Vec<usize> = (0..=short.len()).collect();
    let mut curr_row: Vec<usize> = vec![0; short.len() + 1];

    for (i, &long_byte) in long.iter().enumerate() {
        curr_row[0] = i + 1;
        for (j, &short_byte) in short.iter().enumerate() {
            let cost = if long_byte == short_byte { 0 } else { 1 };
            curr_row[j + 1] = (prev_row[j] + cost)
                .min(prev_row[j + 1] + 1)
                .min(curr_row[j] + 1);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[short.len()]
}

/// Check whether `query` fuzzy-matches `text` within the given maximum
/// edit-distance threshold.
///
/// We check each whitespace-delimited token in `text` independently, so
/// that "dockr" can match "docker" even if the full text is "docker ps".
///
/// Returns `Some(distance)` for the best matching token, or `None` if
/// no token is within the threshold.
#[must_use]
pub fn fuzzy_match_tokens(query: &str, text: &str, max_distance: usize) -> Option<usize> {
    let query_lower = query.to_lowercase();

    let mut best: Option<usize> = None;
    for token in text.split_whitespace() {
        let token_lower = token.to_lowercase();

        // Skip tokens that are much shorter/longer than the query — they
        // can never be within max_distance.
        let len_diff = if query_lower.len() >= token_lower.len() {
            query_lower.len() - token_lower.len()
        } else {
            token_lower.len() - query_lower.len()
        };
        if len_diff > max_distance {
            continue;
        }

        let dist = edit_distance(&query_lower, &token_lower);
        if dist <= max_distance {
            match best {
                Some(prev) if dist < prev => best = Some(dist),
                None => best = Some(dist),
                _ => {}
            }
        }
    }

    best
}

/// Compute the maximum edit distance allowed for a query of the given length.
/// Shorter queries get less tolerance; longer queries get more.
#[must_use]
pub fn max_distance_for_query(query_len: usize) -> usize {
    match query_len {
        0..=2 => 0, // no tolerance for very short queries
        3..=4 => 1, // 1 typo for 3-4 char queries
        5..=7 => 2, // 2 typos for 5-7 char queries
        _ => 3,     // max 3 typos for 8+ char queries
    }
}

/// Compute a cwd similarity score between 0.0 and 1.0.
///
/// - 1.0 if the paths are identical
/// - Proportional to the number of shared path components otherwise
/// - 0.0 if no components are shared
#[must_use]
pub fn cwd_similarity(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }

    let a_parts: Vec<&str> = a.split('/').filter(|s| !s.is_empty()).collect();
    let b_parts: Vec<&str> = b.split('/').filter(|s| !s.is_empty()).collect();

    if a_parts.is_empty() || b_parts.is_empty() {
        return 0.0;
    }

    let shared = a_parts
        .iter()
        .zip(b_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let max_len = a_parts.len().max(b_parts.len());
    if max_len == 0 {
        return 0.0;
    }

    shared as f64 / max_len as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_basic() {
        assert_eq!(edit_distance("docker", "docker"), 0);
        assert_eq!(edit_distance("docker", "dockr"), 1);
        assert_eq!(edit_distance("docker", "doker"), 1); // single deletion
        assert_eq!(edit_distance("docker", "dokcer"), 2); // transposition = 2 edits
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("abc", ""), 3);
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn fuzzy_match_tokens_finds_typos() {
        assert_eq!(fuzzy_match_tokens("dockr", "docker ps", 2), Some(1));
        assert_eq!(fuzzy_match_tokens("dokcer", "docker ps", 2), Some(2));
        assert_eq!(fuzzy_match_tokens("xyz", "docker ps", 2), None);
    }

    #[test]
    fn fuzzy_match_tokens_exact_match() {
        assert_eq!(fuzzy_match_tokens("docker", "docker ps", 2), Some(0));
    }

    #[test]
    fn cwd_similarity_identical() {
        assert_eq!(
            cwd_similarity("/home/user/project", "/home/user/project"),
            1.0
        );
    }

    #[test]
    fn cwd_similarity_partial() {
        let sim = cwd_similarity("/home/user/project", "/home/user/other");
        assert!(sim > 0.5, "shared /home/user should be >0.5, got {sim}");
        assert!(sim < 1.0);
    }

    #[test]
    fn cwd_similarity_none() {
        let sim = cwd_similarity("/opt/app", "/home/user");
        assert_eq!(sim, 0.0);
    }
}
