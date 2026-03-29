//! Epistemic Rigor Validation Module
//!
//! Validates claims for epistemic rigor: proper sourcing, uncertainty markers,
//! and transparent limitation acknowledgment.
//!
//! # Overview
//!
//! This module provides tools to detect overconfident language and suggest
//! more epistemically honest alternatives. It supports the 100% Rust mandate
//! by replacing Python regex-based validation.
//!
//! # Example
//!
//! ```rust
//! use rsk::modules::epistemic::validate_claim;
//!
//! let result = validate_claim("This will always work perfectly.");
//! assert!(!result.valid);
//! assert!(!result.issues.is_empty());
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Confidence level for a claim
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

/// Result of epistemic validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpistemicResult {
    /// Original claim text
    pub claim: String,
    /// Whether the claim passes validation
    pub valid: bool,
    /// Issues found in the claim
    pub issues: Vec<String>,
    /// Suggestions for improvement
    pub suggestions: Vec<String>,
    /// Overall confidence level
    pub confidence_level: ConfidenceLevel,
    /// Overconfident words detected
    pub detected_words: Vec<String>,
}

/// Overconfident language patterns (compiled once)
static OVERCONFIDENT_WORDS: &[&str] = &[
    "always",
    "never",
    "definitely",
    "certainly",
    "obviously",
    "clearly",
    "undoubtedly",
    "unquestionably",
    "absolutely",
    "proven",
    "guaranteed",
];

/// Pre-compiled regex patterns for overconfident words
static OVERCONFIDENT_PATTERNS: LazyLock<Vec<(String, Regex)>> = LazyLock::new(|| {
    OVERCONFIDENT_WORDS
        .iter()
        .filter_map(|word| {
            Regex::new(&format!(r"(?i)\b{word}\b"))
                .ok()
                .map(|re| (word.to_string(), re))
        })
        .collect()
});

/// Citation pattern (looks for [citations] or explicit "source" mentions)
static CITATION_PATTERN: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"\[|\bsource\b|\bcitation\b|\breference\b|\baccording to\b").ok()
});

/// Validate a claim for epistemic rigor.
///
/// Checks for:
/// - Unsupported certainty language ("always", "never", "definitely", etc.)
/// - Missing uncertainty markers
/// - Citation presence
///
/// # Arguments
///
/// * `claim` - The claim text to validate
///
/// # Returns
///
/// `EpistemicResult` with issues, suggestions, and confidence level
pub fn validate_claim(claim: &str) -> EpistemicResult {
    let mut issues = Vec::new();
    let mut suggestions = Vec::new();
    let mut detected_words = Vec::new();

    // Check for overconfident language
    for (word, pattern) in OVERCONFIDENT_PATTERNS.iter() {
        if pattern.is_match(claim) {
            issues.push(format!("Uses overconfident language: '{word}'"));
            detected_words.push(word.clone());
        }
    }

    // Add suggestion if overconfident language found
    if !detected_words.is_empty() {
        suggestions.push(
            "Consider hedging with 'typically', 'often', 'in most cases', or 'evidence suggests'"
                .to_string(),
        );
    }

    // Check for citation markers
    let has_citation = CITATION_PATTERN
        .as_ref()
        .map(|re| re.is_match(claim))
        .unwrap_or(false);
    if !has_citation {
        suggestions.push("Consider adding citations or sources".to_string());
    }

    let valid = issues.is_empty();
    let confidence_level = if valid {
        ConfidenceLevel::High
    } else if issues.len() < 3 {
        ConfidenceLevel::Medium
    } else {
        ConfidenceLevel::Low
    };

    EpistemicResult {
        claim: claim.to_string(),
        valid,
        issues,
        suggestions,
        confidence_level,
        detected_words,
    }
}

/// Batch validate multiple claims.
///
/// # Arguments
///
/// * `claims` - Slice of claim strings to validate
///
/// # Returns
///
/// Vector of `EpistemicResult` for each claim
pub fn validate_claims(claims: &[&str]) -> Vec<EpistemicResult> {
    claims.iter().map(|c| validate_claim(c)).collect()
}

/// Suggest replacements for overconfident language.
///
/// Returns a mapping of overconfident word → suggested alternatives.
pub fn get_hedging_suggestions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("always", "typically, usually, often"),
        ("never", "rarely, seldom, in few cases"),
        ("definitely", "likely, probably, evidence suggests"),
        ("certainly", "appears to, seems to, likely"),
        ("obviously", "it appears, evidence indicates"),
        ("clearly", "the data suggests, observations indicate"),
        ("undoubtedly", "with high confidence, strongly suggests"),
        ("unquestionably", "with strong evidence, highly likely"),
        ("absolutely", "in most cases, with high probability"),
        ("proven", "supported by evidence, validated"),
        ("guaranteed", "expected with high confidence"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_claim() {
        let result = validate_claim("This typically works in most cases.");
        assert!(result.valid);
        assert!(result.issues.is_empty());
        assert_eq!(result.confidence_level, ConfidenceLevel::High);
    }

    #[test]
    fn test_overconfident_claim() {
        let result = validate_claim("This will always work perfectly.");
        assert!(!result.valid);
        assert!(!result.issues.is_empty());
        assert!(result.detected_words.contains(&"always".to_string()));
    }

    #[test]
    fn test_multiple_overconfident_words() {
        let result = validate_claim("This is obviously and definitely true.");
        assert!(!result.valid);
        assert_eq!(result.issues.len(), 2);
        assert!(result.detected_words.contains(&"obviously".to_string()));
        assert!(result.detected_words.contains(&"definitely".to_string()));
    }

    #[test]
    fn test_citation_suggestion() {
        let result = validate_claim("The sky is blue.");
        assert!(result.valid); // No overconfident words
        assert!(result.suggestions.iter().any(|s| s.contains("citation")));
    }

    #[test]
    fn test_with_citation() {
        let result = validate_claim("According to [Smith 2024], this often occurs.");
        assert!(result.valid);
        // Citation suggestion should NOT appear
        assert!(!result.suggestions.iter().any(|s| s.contains("citation")));
    }

    #[test]
    fn test_case_insensitive() {
        let result = validate_claim("This NEVER happens.");
        assert!(!result.valid);
        assert!(result.detected_words.contains(&"never".to_string()));
    }

    #[test]
    fn test_confidence_levels() {
        // High confidence (no issues)
        let high = validate_claim("This typically works.");
        assert_eq!(high.confidence_level, ConfidenceLevel::High);

        // Medium confidence (1-2 issues)
        let medium = validate_claim("This always works.");
        assert_eq!(medium.confidence_level, ConfidenceLevel::Medium);

        // Low confidence (3+ issues)
        let low = validate_claim("This is obviously, definitely, and certainly true.");
        assert_eq!(low.confidence_level, ConfidenceLevel::Low);
    }

    #[test]
    fn test_hedging_suggestions() {
        let suggestions = get_hedging_suggestions();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|(word, _)| *word == "always"));
    }

    #[test]
    fn test_batch_validation() {
        let claims = vec!["This always works.", "This typically works."];
        let results = validate_claims(&claims);
        assert_eq!(results.len(), 2);
        assert!(!results[0].valid);
        assert!(results[1].valid);
    }
}
