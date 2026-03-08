use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LevenshteinResult {
    pub distance: usize,
    pub similarity: f64,
    pub source_len: usize,
    pub target_len: usize,
}

/// Compute Levenshtein edit distance between two strings.
/// Uses Wagner-Fischer algorithm with O(min(m,n)) space optimization.
pub fn levenshtein_distance(source: &str, target: &str) -> usize {
    let source_chars: Vec<char> = source.chars().collect();
    let target_chars: Vec<char> = target.chars().collect();

    let m = source_chars.len();
    let n = target_chars.len();

    // Early termination cases
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Ensure we iterate over the shorter string for space efficiency
    let (shorter, longer, short_len, long_len) = if m <= n {
        (&source_chars, &target_chars, m, n)
    } else {
        (&target_chars, &source_chars, n, m)
    };

    // Single row for space optimization: O(min(m,n)) instead of O(m*n)
    let mut prev_row: Vec<usize> = (0..=short_len).collect();
    let mut curr_row: Vec<usize> = vec![0; short_len + 1];

    for i in 1..=long_len {
        curr_row[0] = i;

        for j in 1..=short_len {
            let cost = if longer[i - 1] == shorter[j - 1] {
                0
            } else {
                1
            };

            curr_row[j] = (prev_row[j] + 1) // deletion
                .min(curr_row[j - 1] + 1) // insertion
                .min(prev_row[j - 1] + cost); // substitution
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[short_len]
}

/// Compute Levenshtein distance with full result including similarity ratio.
pub fn levenshtein(source: &str, target: &str) -> LevenshteinResult {
    let distance = levenshtein_distance(source, target);
    let max_len = source.chars().count().max(target.chars().count());
    let similarity = if max_len == 0 {
        1.0
    } else {
        #[allow(clippy::as_conversions)] // usize→f64 for similarity ratio
        let sim = 1.0 - (distance as f64 / max_len as f64);
        sim
    };

    LevenshteinResult {
        distance,
        similarity: (similarity * 10000.0).round() / 10000.0, // 4 decimal places
        source_len: source.chars().count(),
        target_len: target.chars().count(),
    }
}

/// Batch fuzzy search: find best matches for a query against candidates.
/// Returns candidates sorted by similarity (descending).
#[derive(Debug, Serialize, Deserialize)]
pub struct FuzzyMatch {
    pub candidate: String,
    pub distance: usize,
    pub similarity: f64,
}

pub fn fuzzy_search(query: &str, candidates: &[String], limit: usize) -> Vec<FuzzyMatch> {
    let mut matches: Vec<FuzzyMatch> = candidates
        .iter()
        .map(|c| {
            let result = levenshtein(query, c);
            FuzzyMatch {
                candidate: c.clone(),
                distance: result.distance,
                similarity: result.similarity,
            }
        })
        .collect();

    // Sort by similarity descending, then by candidate name for stability
    matches.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.candidate.cmp(&b.candidate))
    });

    matches.truncate(limit);
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // LEVENSHTEIN DISTANCE: POSITIVE TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_identical_strings() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn test_one_substitution() {
        assert_eq!(levenshtein_distance("hello", "hallo"), 1);
    }

    #[test]
    fn test_one_insertion() {
        assert_eq!(levenshtein_distance("hello", "helllo"), 1);
    }

    #[test]
    fn test_one_deletion() {
        assert_eq!(levenshtein_distance("hello", "helo"), 1);
    }

    #[test]
    fn test_multiple_operations() {
        // "kitten" -> "sitting" requires 3 operations
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_completely_different() {
        assert_eq!(levenshtein_distance("abc", "xyz"), 3);
    }

    // ═══════════════════════════════════════════════════════════════
    // LEVENSHTEIN DISTANCE: EDGE CASES
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_empty_strings() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "xyz"), 3);
    }

    #[test]
    fn test_single_char() {
        assert_eq!(levenshtein_distance("a", "a"), 0);
        assert_eq!(levenshtein_distance("a", "b"), 1);
        assert_eq!(levenshtein_distance("a", ""), 1);
    }

    #[test]
    fn test_case_sensitive() {
        assert_eq!(levenshtein_distance("Hello", "hello"), 1);
        assert_eq!(levenshtein_distance("ABC", "abc"), 3);
    }

    #[test]
    fn test_symmetry() {
        // Distance should be the same regardless of order
        assert_eq!(
            levenshtein_distance("abc", "def"),
            levenshtein_distance("def", "abc")
        );
    }

    #[test]
    fn test_unicode() {
        // Japanese characters
        assert_eq!(levenshtein_distance("こんにちは", "こんばんは"), 2);
    }

    #[test]
    fn test_emoji() {
        assert_eq!(levenshtein_distance("👋🌍", "👋🌎"), 1);
        assert_eq!(levenshtein_distance("🎉", "🎉"), 0);
    }

    #[test]
    fn test_mixed_unicode_ascii() {
        assert_eq!(levenshtein_distance("hello世界", "hello世界"), 0);
        assert_eq!(levenshtein_distance("hello世界", "hello世间"), 1);
    }

    // ═══════════════════════════════════════════════════════════════
    // LEVENSHTEIN RESULT: SIMILARITY TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_similarity() {
        let result = levenshtein("hello", "hallo");
        assert_eq!(result.distance, 1);
        assert_eq!(result.similarity, 0.8); // 1 - 1/5
    }

    #[test]
    fn test_similarity_perfect_match() {
        let result = levenshtein("test", "test");
        assert_eq!(result.distance, 0);
        assert_eq!(result.similarity, 1.0);
    }

    #[test]
    fn test_similarity_no_match() {
        let result = levenshtein("abc", "xyz");
        assert_eq!(result.distance, 3);
        assert_eq!(result.similarity, 0.0);
    }

    #[test]
    fn test_similarity_empty_both() {
        let result = levenshtein("", "");
        assert_eq!(result.similarity, 1.0); // Both empty = identical
    }

    #[test]
    fn test_result_lengths() {
        let result = levenshtein("hello", "hi");
        assert_eq!(result.source_len, 5);
        assert_eq!(result.target_len, 2);
    }

    // ═══════════════════════════════════════════════════════════════
    // FUZZY SEARCH: POSITIVE TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_fuzzy_search_basic() {
        let candidates = vec![
            "commit".to_string(),
            "comment".to_string(),
            "comet".to_string(),
        ];
        let results = fuzzy_search("comit", &candidates, 3);

        assert_eq!(results.len(), 3);
        // "commit" should be first (distance 1)
        assert_eq!(results[0].candidate, "commit");
        assert_eq!(results[0].distance, 1);
    }

    #[test]
    fn test_fuzzy_search_exact_match() {
        let candidates = vec!["hello".to_string(), "world".to_string(), "help".to_string()];
        let results = fuzzy_search("hello", &candidates, 3);

        assert_eq!(results[0].candidate, "hello");
        assert_eq!(results[0].distance, 0);
        assert_eq!(results[0].similarity, 1.0);
    }

    #[test]
    fn test_fuzzy_search_limit() {
        let candidates = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string(),
        ];
        let results = fuzzy_search("x", &candidates, 2);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_fuzzy_search_sorted_by_similarity() {
        let candidates = vec![
            "apple".to_string(), // distance 5 from "xyz"
            "xyzzy".to_string(), // distance 2 from "xyz"
            "xyz".to_string(),   // distance 0 from "xyz"
        ];
        let results = fuzzy_search("xyz", &candidates, 3);

        assert_eq!(results[0].candidate, "xyz");
        assert_eq!(results[1].candidate, "xyzzy");
        assert_eq!(results[2].candidate, "apple");
    }

    // ═══════════════════════════════════════════════════════════════
    // FUZZY SEARCH: EDGE CASES
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_fuzzy_search_empty_candidates() {
        let candidates: Vec<String> = vec![];
        let results = fuzzy_search("test", &candidates, 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_fuzzy_search_empty_query() {
        let candidates = vec!["a".to_string(), "ab".to_string(), "abc".to_string()];
        let results = fuzzy_search("", &candidates, 3);

        // Shorter strings should rank higher (closer to empty)
        assert_eq!(results[0].candidate, "a");
    }

    #[test]
    fn test_fuzzy_search_limit_zero() {
        let candidates = vec!["a".to_string(), "b".to_string()];
        let results = fuzzy_search("a", &candidates, 0);
        assert!(results.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════
    // STRESS TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_long_strings() {
        let a = "a".repeat(100);
        let b = "b".repeat(100);
        assert_eq!(levenshtein_distance(&a, &b), 100);
    }

    #[test]
    fn test_long_similar_strings() {
        let a = "a".repeat(1000);
        let mut b = "a".repeat(999);
        b.push('b');
        assert_eq!(levenshtein_distance(&a, &b), 1);
    }

    #[test]
    fn test_fuzzy_search_many_candidates() {
        let candidates: Vec<String> = (0..100).map(|i| format!("item_{}", i)).collect();
        let results = fuzzy_search("item_50", &candidates, 5);

        assert_eq!(results.len(), 5);
        assert_eq!(results[0].candidate, "item_50");
        assert_eq!(results[0].distance, 0);
    }
}
