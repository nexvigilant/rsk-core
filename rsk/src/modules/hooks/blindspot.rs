//! Blindspot check generation for self-review reminders.

use super::policy::PolicyFile;
use super::validation::categorize_file;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Types of blindspot checks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlindspotType {
    /// Code file - check assumptions, edge cases, failures
    Code,
    /// Plan/documentation - check alternatives, dependencies, assumptions
    Plan,
    /// Configuration - check syntax, secrets, compatibility
    Config,
    /// Sensitive file - security review
    Sensitive,
    /// Documentation - check accuracy
    Docs,
    /// Test file - check coverage, assertions
    Test,
    /// Other/unknown file type
    Other,
}

impl BlindspotType {
    /// Determine blindspot type from file category
    pub fn from_category(category: &str) -> Self {
        match category {
            "code_files" => BlindspotType::Code,
            "test_files" => BlindspotType::Test,
            "config_files" => BlindspotType::Config,
            "sensitive" => BlindspotType::Sensitive,
            "docs" => BlindspotType::Docs,
            _ => {
                // Check for plan files
                if category.contains("plan") {
                    BlindspotType::Plan
                } else {
                    BlindspotType::Other
                }
            }
        }
    }

    /// Determine blindspot type from file path
    pub fn from_path(path: &Path, policy: &PolicyFile) -> Self {
        let category = categorize_file(path, policy);

        // Check if it's a plan file by name
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        if filename.contains("plan") || filename.contains("todo") {
            return BlindspotType::Plan;
        }

        Self::from_category(&category)
    }
}

/// A blindspot check reminder
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlindspotCheck {
    /// Type of check
    pub check_type: BlindspotType,
    /// File path that triggered the check
    pub path: String,
    /// Category of the file
    pub category: String,
    /// Check items to review
    pub items: Vec<String>,
    /// Formatted message for display
    pub message: String,
}

impl BlindspotCheck {
    /// Generate a blindspot check for a file
    pub fn for_file(path: &Path, policy: &PolicyFile) -> Self {
        let category = categorize_file(path, policy);
        let check_type = BlindspotType::from_path(path, policy);
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");

        let (items, message) = match check_type {
            BlindspotType::Code => (
                vec![
                    "Assumptions: What must be true for this to work?".to_string(),
                    "Edge cases: null, empty, boundaries, malformed input?".to_string(),
                    "Failure modes: Where does this break first?".to_string(),
                    "Deletable: Any unnecessary complexity?".to_string(),
                ],
                format!(
                    "[Blindspot Check] Code modified: {}. Self-review: assumptions, edge cases, failure modes, deletable complexity. State confidence.",
                    filename
                ),
            ),
            BlindspotType::Test => (
                vec![
                    "Coverage: All branches and edge cases tested?".to_string(),
                    "Assertions: Are assertions specific enough?".to_string(),
                    "Isolation: Test independent of external state?".to_string(),
                    "Readability: Is the test self-documenting?".to_string(),
                ],
                format!(
                    "[Blindspot Check] Test modified: {}. Verify: coverage complete, assertions specific, tests isolated.",
                    filename
                ),
            ),
            BlindspotType::Plan => (
                vec![
                    "Alternatives: Were other approaches considered?".to_string(),
                    "Dependencies: What could block this?".to_string(),
                    "Steps: Are all steps explicit and ordered?".to_string(),
                    "Assumptions: What's being taken for granted?".to_string(),
                ],
                format!(
                    "[Plan Review] Plan modified: {}. Verify: alternatives considered, dependencies identified, steps complete, assumptions explicit.",
                    filename
                ),
            ),
            BlindspotType::Config => (
                vec![
                    "Syntax: Is the format valid?".to_string(),
                    "Secrets: Any exposed credentials?".to_string(),
                    "Compatibility: Backwards compatible?".to_string(),
                    "Required: All required fields present?".to_string(),
                ],
                format!(
                    "[Config Check] Config modified: {}. Verify: syntax valid, no exposed secrets, backwards compatible.",
                    filename
                ),
            ),
            BlindspotType::Sensitive => (
                vec![
                    "Gitignore: Is this file gitignored?".to_string(),
                    "Exposure: Could this leak to logs or output?".to_string(),
                    "Access: Are permissions restrictive enough?".to_string(),
                    "Rotation: Is there a rotation/expiry policy?".to_string(),
                ],
                format!(
                    "[SECURITY] Sensitive file modified: {}. VERIFY: gitignored, not logged, permissions restricted.",
                    filename
                ),
            ),
            BlindspotType::Docs => (
                vec![
                    "Accuracy: Does this match current behavior?".to_string(),
                    "Completeness: Any missing sections?".to_string(),
                    "Examples: Are examples up to date?".to_string(),
                ],
                format!(
                    "[Doc Check] Documentation modified: {}. Verify: accurate, complete, examples current.",
                    filename
                ),
            ),
            BlindspotType::Other => (
                vec!["Review: Does this file need to exist?".to_string()],
                format!("[File Check] File modified: {}.", filename),
            ),
        };

        Self {
            check_type,
            path: path.to_str().unwrap_or("").to_string(),
            category,
            items,
            message,
        }
    }

    /// Generate a blindspot check for a subagent task
    pub fn for_subagent(subagent_type: &str, description: &str) -> Self {
        let (check_type, items, message) = match subagent_type.to_lowercase().as_str() {
            "plan" => (
                BlindspotType::Plan,
                vec![
                    "Alternatives: Were other approaches considered?".to_string(),
                    "Dependencies: What could block this?".to_string(),
                    "Steps: Are all steps explicit?".to_string(),
                ],
                format!(
                    "[Subagent Review - Plan] \"{}\". Verify: alternatives considered, dependencies identified, steps complete.",
                    truncate_str(description, 80)
                ),
            ),
            "explore" => (
                BlindspotType::Other,
                vec![
                    "Sources: Are findings from reliable locations?".to_string(),
                    "Completeness: Any gaps in the search?".to_string(),
                ],
                format!(
                    "[Subagent Review - Research] \"{}\". Verify: sources reliable, findings complete.",
                    truncate_str(description, 80)
                ),
            ),
            "bash" => (
                BlindspotType::Other,
                vec![
                    "Exit status: Was it successful?".to_string(),
                    "Output: Does it match expectations?".to_string(),
                    "Side effects: Any unintended changes?".to_string(),
                ],
                format!(
                    "[Subagent Review - Command] \"{}\". Check: exit status, output, side effects.",
                    truncate_str(description, 80)
                ),
            ),
            _ => {
                // Infer from description
                let desc_lower = description.to_lowercase();
                if desc_lower.contains("implement")
                    || desc_lower.contains("write")
                    || desc_lower.contains("create")
                    || desc_lower.contains("fix")
                {
                    (
                        BlindspotType::Code,
                        vec![
                            "Code assumptions: Valid?".to_string(),
                            "Edge cases: Handled?".to_string(),
                            "Tests: Needed/updated?".to_string(),
                        ],
                        format!(
                            "[Subagent Review - Implementation] \"{}\". Verify: assumptions valid, edge cases handled, tests needed?",
                            truncate_str(description, 80)
                        ),
                    )
                } else {
                    (
                        BlindspotType::Other,
                        vec!["Output: Meets requirements?".to_string()],
                        format!(
                            "[Subagent Review] \"{}\". Quick check: output meets requirements?",
                            truncate_str(description, 80)
                        ),
                    )
                }
            }
        };

        Self {
            check_type,
            path: String::new(),
            category: subagent_type.to_string(),
            items,
            message,
        }
    }
}

/// Truncate a string to a maximum length, adding "..." if truncated
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blindspot_type_from_category() {
        assert_eq!(
            BlindspotType::from_category("code_files"),
            BlindspotType::Code
        );
        assert_eq!(
            BlindspotType::from_category("test_files"),
            BlindspotType::Test
        );
        assert_eq!(
            BlindspotType::from_category("config_files"),
            BlindspotType::Config
        );
        assert_eq!(
            BlindspotType::from_category("sensitive"),
            BlindspotType::Sensitive
        );
    }

    #[test]
    fn test_blindspot_check_code() {
        let policy = PolicyFile::default();
        let check = BlindspotCheck::for_file(Path::new("/test/main.py"), &policy);

        assert_eq!(check.check_type, BlindspotType::Code);
        assert!(!check.items.is_empty());
        assert!(check.message.contains("Code modified"));
    }

    #[test]
    fn test_blindspot_check_plan() {
        let policy = PolicyFile::default();
        let check = BlindspotCheck::for_file(Path::new("/test/implementation-plan.md"), &policy);

        assert_eq!(check.check_type, BlindspotType::Plan);
        assert!(check.message.contains("Plan"));
    }

    #[test]
    fn test_subagent_check() {
        let check = BlindspotCheck::for_subagent("Plan", "design new authentication system");

        assert_eq!(check.check_type, BlindspotType::Plan);
        assert!(check.message.contains("Subagent Review"));
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("short", 10), "short");
        assert_eq!(truncate_str("this is a long string", 10), "this is...");
    }
}
