//! Generic text processing utilities.
//!
//! This module provides general-purpose text manipulation functions:
//! - Tokenization and word frequency analysis
//! - Text normalization and slugification
//! - N-gram extraction
//! - Compressibility analysis (Shannon entropy)

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

// ═══════════════════════════════════════════════════════════════════════════
// PRECOMPILED REGEX PATTERNS (compiled once, reused forever)
// ═══════════════════════════════════════════════════════════════════════════

/// Tokenizer pattern - matches word characters
pub(crate) static RE_TOKENIZE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"[\w]+").ok());

/// Whitespace collapse pattern
pub(crate) static RE_WHITESPACE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\s+").ok());

/// Slug cleanup pattern
pub(crate) static RE_SLUG_SPECIAL: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"[^a-z0-9\s-]").ok());

// ═══════════════════════════════════════════════════════════════════════════
// TEXT PROCESSING TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Result of text tokenization
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenizeResult {
    pub tokens: Vec<String>,
    pub count: usize,
    pub unique_count: usize,
}

/// Result of text normalization
#[derive(Debug, Serialize, Deserialize)]
pub struct NormalizeResult {
    pub text: String,
    pub original_length: usize,
    pub normalized_length: usize,
}

/// Result of word frequency analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct WordFrequencyResult {
    pub frequencies: HashMap<String, usize>,
    pub total_words: usize,
    pub unique_words: usize,
    pub top_words: Vec<(String, usize)>,
}

/// Result of text compression ratio analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct CompressionAnalysis {
    pub original_chars: usize,
    pub unique_chars: usize,
    pub entropy_estimate: f64,
    pub compressibility: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// TEXT PROCESSING FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Tokenize text into words
///
/// Splits on whitespace and punctuation, filters empty tokens.
pub fn tokenize(text: &str) -> TokenizeResult {
    let tokens: Vec<String> = RE_TOKENIZE
        .as_ref()
        .map(|re| {
            re.find_iter(text)
                .map(|m: regex::Match| m.as_str().to_string())
                .collect()
        })
        .unwrap_or_default();

    let unique: std::collections::HashSet<_> = tokens.iter().collect();

    TokenizeResult {
        count: tokens.len(),
        unique_count: unique.len(),
        tokens,
    }
}

/// Normalize text for comparison
///
/// Converts to lowercase, removes extra whitespace, optionally removes punctuation.
pub fn normalize(text: &str, remove_punctuation: bool) -> NormalizeResult {
    let original_length = text.len();

    // Convert to lowercase
    let mut normalized = text.to_lowercase();

    // Remove punctuation if requested
    if remove_punctuation {
        normalized = normalized
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect();
    }

    // Collapse whitespace using precompiled regex
    normalized = RE_WHITESPACE
        .as_ref()
        .map(|re| re.replace_all(&normalized, " ").trim().to_string())
        .unwrap_or_else(|| normalized.split_whitespace().collect::<Vec<_>>().join(" "));

    NormalizeResult {
        normalized_length: normalized.len(),
        text: normalized,
        original_length,
    }
}

/// Calculate word frequencies in text
///
/// Returns frequency map and top N most common words.
pub fn word_frequency(text: &str, top_n: usize) -> WordFrequencyResult {
    let tokens = tokenize(text);
    let mut frequencies: HashMap<String, usize> = HashMap::new();

    for token in &tokens.tokens {
        let lower = token.to_lowercase();
        *frequencies.entry(lower).or_insert(0) += 1;
    }

    // Get top N words
    let mut freq_vec: Vec<(String, usize)> =
        frequencies.iter().map(|(k, v)| (k.clone(), *v)).collect();
    freq_vec.sort_by(|a, b| b.1.cmp(&a.1));
    let top_words: Vec<(String, usize)> = freq_vec.into_iter().take(top_n).collect();

    WordFrequencyResult {
        total_words: tokens.count,
        unique_words: frequencies.len(),
        top_words,
        frequencies,
    }
}

/// Analyze text compressibility
///
/// Estimates how compressible the text is based on character distribution.
pub fn analyze_compressibility(text: &str) -> CompressionAnalysis {
    let chars: Vec<char> = text.chars().collect();
    let original_chars = chars.len();

    if original_chars == 0 {
        return CompressionAnalysis {
            original_chars: 0,
            unique_chars: 0,
            entropy_estimate: 0.0,
            compressibility: "empty".to_string(),
        };
    }

    // Count character frequencies
    let mut char_freq: HashMap<char, usize> = HashMap::new();
    for c in &chars {
        *char_freq.entry(*c).or_insert(0) += 1;
    }

    let unique_chars = char_freq.len();

    // Calculate Shannon entropy estimate
    #[allow(clippy::as_conversions)] // usize→f64 for entropy calculation
    let total = original_chars as f64;
    let entropy: f64 = char_freq
        .values()
        .map(|&count| {
            #[allow(clippy::as_conversions)] // usize→f64 for probability
            let p = count as f64 / total;
            if p > 0.0 { -p * p.log2() } else { 0.0 }
        })
        .sum();

    // Determine compressibility category
    let compressibility = if entropy < 2.0 {
        "highly_compressible"
    } else if entropy < 4.0 {
        "moderately_compressible"
    } else if entropy < 6.0 {
        "low_compressibility"
    } else {
        "incompressible"
    }
    .to_string();

    CompressionAnalysis {
        original_chars,
        unique_chars,
        entropy_estimate: (entropy * 100.0).round() / 100.0,
        compressibility,
    }
}

/// Extract n-grams from text
///
/// Returns character or word n-grams based on mode.
pub fn extract_ngrams(text: &str, n: usize, word_mode: bool) -> Vec<String> {
    if word_mode {
        let tokens = tokenize(text);
        if tokens.tokens.len() < n {
            return vec![];
        }
        tokens.tokens.windows(n).map(|w| w.join(" ")).collect()
    } else {
        let chars: Vec<char> = text.chars().collect();
        if chars.len() < n {
            return vec![];
        }
        chars.windows(n).map(|w| w.iter().collect()).collect()
    }
}

/// Truncate text to maximum length with ellipsis
pub fn truncate(text: &str, max_len: usize, ellipsis: &str) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }

    let truncate_at = max_len.saturating_sub(ellipsis.len());
    let mut result: String = text.chars().take(truncate_at).collect();
    result.push_str(ellipsis);
    result
}

/// Slugify text for URLs/filenames
///
/// Converts to lowercase, replaces spaces with dashes, removes special chars.
pub fn slugify(text: &str) -> String {
    let normalized = text.to_lowercase();
    let cleaned = RE_SLUG_SPECIAL
        .as_ref()
        .map(|re| re.replace_all(&normalized, "").into_owned())
        .unwrap_or(normalized);
    RE_WHITESPACE
        .as_ref()
        .map(|re| re.replace_all(&cleaned, "-").trim_matches('-').to_string())
        .unwrap_or_else(|| cleaned.split_whitespace().collect::<Vec<_>>().join("-"))
        .to_string()
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let result = tokenize("Hello world! This is a test.");
        assert_eq!(result.count, 6);
        assert_eq!(
            result.tokens,
            vec!["Hello", "world", "This", "is", "a", "test"]
        );
    }

    #[test]
    fn test_tokenize_empty() {
        let result = tokenize("");
        assert_eq!(result.count, 0);
        assert!(result.tokens.is_empty());
    }

    #[test]
    fn test_tokenize_unicode() {
        let result = tokenize("日本語 test émoji");
        assert_eq!(result.count, 3);
    }

    #[test]
    fn test_normalize_basic() {
        let result = normalize("  Hello   WORLD  ", false);
        assert_eq!(result.text, "hello world");
    }

    #[test]
    fn test_normalize_strip_punctuation() {
        let result = normalize("Hello, World!", true);
        assert_eq!(result.text, "hello world");
    }

    #[test]
    fn test_word_frequency() {
        let result = word_frequency("the cat sat on the mat", 3);
        assert_eq!(result.total_words, 6);
        assert_eq!(result.unique_words, 5);
        assert_eq!(result.top_words[0], ("the".to_string(), 2));
    }

    #[test]
    fn test_analyze_compressibility_low_entropy() {
        let result = analyze_compressibility("aaaaaaaaaa");
        assert!(result.entropy_estimate < 1.0);
        assert_eq!(result.compressibility, "highly_compressible");
    }

    #[test]
    fn test_analyze_compressibility_high_entropy() {
        let result = analyze_compressibility("abcdefghijklmnopqrstuvwxyz");
        assert!(result.entropy_estimate > 4.0);
    }

    #[test]
    fn test_extract_ngrams_chars() {
        let ngrams = extract_ngrams("hello", 2, false);
        assert_eq!(ngrams, vec!["he", "el", "ll", "lo"]);
    }

    #[test]
    fn test_extract_ngrams_words() {
        let ngrams = extract_ngrams("the quick brown fox", 2, true);
        assert_eq!(ngrams, vec!["the quick", "quick brown", "brown fox"]);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello world", 8, "..."), "hello...");
        assert_eq!(truncate("hi", 10, "..."), "hi");
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("Test 123 @#$ Slug"), "test-123-slug");
        assert_eq!(slugify("  Multiple   Spaces  "), "multiple-spaces");
    }
}
