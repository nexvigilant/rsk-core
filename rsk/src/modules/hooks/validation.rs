//! File validation against policy rules.

use super::policy::{PolicyFile, expand_path, is_in_path, matches_glob};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Result of validating a file against policies
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationResult {
    /// The file path that was validated
    pub path: String,
    /// Whether the file placement is valid
    pub valid: bool,
    /// Category the file belongs to
    pub category: String,
    /// Warnings generated during validation
    pub warnings: Vec<ValidationWarning>,
    /// Suggestions for better placement
    pub suggestions: Vec<String>,
}

/// A warning generated during validation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationWarning {
    /// Severity level: info, warning, error, security
    pub level: String,
    /// Warning message
    pub message: String,
    /// Rule that triggered the warning
    pub rule: String,
}

impl ValidationResult {
    pub fn new(path: &str, category: &str) -> Self {
        Self {
            path: path.to_string(),
            valid: true,
            category: category.to_string(),
            warnings: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn add_warning(&mut self, level: &str, message: &str, rule: &str) {
        self.warnings.push(ValidationWarning {
            level: level.to_string(),
            message: message.to_string(),
            rule: rule.to_string(),
        });

        // Mark as invalid for security or error levels
        if level == "security" || level == "error" {
            self.valid = false;
        }
    }

    pub fn add_suggestion(&mut self, suggestion: &str) {
        self.suggestions.push(suggestion.to_string());
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn has_security_warnings(&self) -> bool {
        self.warnings.iter().any(|w| w.level == "security")
    }
}

/// Categorize a file based on policy rules or fallback heuristics
pub fn categorize_file(path: &Path, policy: &PolicyFile) -> String {
    let path_str = path.to_str().unwrap_or("");
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Check against policy placement rules
    if let Some(rules) = &policy.placement_rules {
        for (category, rule) in rules {
            for pattern in &rule.patterns {
                if matches_glob(path_str, pattern) || matches_glob(filename, pattern) {
                    return category.clone();
                }
            }
        }
    }

    // Fallback categorization by extension
    match path.extension().and_then(|e| e.to_str()) {
        Some(
            "py" | "js" | "ts" | "tsx" | "go" | "rs" | "java" | "c" | "cpp" | "rb" | "php" | "sh",
        ) => "code_files".to_string(),
        Some("md" | "rst" | "txt") => "docs".to_string(),
        Some("json" | "yaml" | "yml" | "toml" | "ini") => "config_files".to_string(),
        Some("tmp" | "bak" | "swp" | "swo") => "temp_files".to_string(),
        Some("log") => "log_files".to_string(),
        Some("o" | "pyc" | "pyo" | "class") => "build_artifacts".to_string(),
        Some("env") => "sensitive".to_string(),
        Some("pem" | "key" | "crt" | "p12") => "sensitive".to_string(),
        _ => {
            // Check filename patterns for sensitive files
            let lower = filename.to_lowercase();
            if lower.contains("secret")
                || lower.contains("credential")
                || lower.contains("password")
            {
                return "sensitive".to_string();
            }
            "other".to_string()
        }
    }
}

/// Validate a file against policy rules
pub fn validate_file(path: &Path, policy: &PolicyFile) -> ValidationResult {
    let path_str = path.to_str().unwrap_or("");
    let category = categorize_file(path, policy);
    let mut result = ValidationResult::new(path_str, &category);

    // Check forbidden zones
    if let Some(forbidden) = &policy.forbidden_zones {
        if let Some(paths) = &forbidden.paths {
            for forbidden_path in paths {
                if is_in_path(path_str, forbidden_path) {
                    // Check exceptions
                    let is_exception = forbidden
                        .exceptions
                        .as_ref()
                        .map(|exc| exc.iter().any(|e| matches_glob(path_str, e)))
                        .unwrap_or(false);

                    if !is_exception {
                        result.add_warning(
                            "warning",
                            &format!("File in forbidden zone: {}", forbidden_path),
                            "forbidden_zones",
                        );
                        result
                            .add_suggestion("Move to a project directory or appropriate location");
                    }
                }
            }
        }
    }

    // Check placement rules for this category
    if let Some(rules) = &policy.placement_rules {
        if let Some(rule) = rules.get(&category) {
            // Check forbidden paths
            for forbidden in &rule.forbidden_paths {
                if is_in_path(path_str, forbidden) {
                    let is_exception = rule.exceptions.iter().any(|e| matches_glob(path_str, e));

                    if !is_exception {
                        let level = rule
                            .severity
                            .as_ref()
                            .map(|s| if s == "high" { "security" } else { "warning" })
                            .unwrap_or("warning");

                        result.add_warning(
                            level,
                            rule.message
                                .as_deref()
                                .unwrap_or("File in forbidden location"),
                            &category,
                        );

                        if !rule.recommended_paths.is_empty() {
                            result.add_suggestion(&format!(
                                "Recommended locations: {}",
                                rule.recommended_paths.join(", ")
                            ));
                        }
                    }
                }
            }

            // Suggest recommended paths if not already in one
            if result.warnings.is_empty() && !rule.recommended_paths.is_empty() {
                let in_recommended = rule
                    .recommended_paths
                    .iter()
                    .any(|rp| is_in_path(path_str, rp));

                if !in_recommended {
                    result.add_suggestion(&format!(
                        "Consider placing in: {}",
                        rule.recommended_paths.join(", ")
                    ));
                }
            }
        }
    }

    result
}

/// Format validation result for human-readable output
pub fn format_validation_result(result: &ValidationResult) -> String {
    let mut output = String::new();

    for warning in &result.warnings {
        let prefix = match warning.level.as_str() {
            "security" => "[SECURITY]",
            "error" => "[ERROR]",
            _ => "[File Placement]",
        };
        output.push_str(&format!("{} {}\n", prefix, warning.message));
    }

    if let Some(suggestion) = result.suggestions.first() {
        output.push_str(&format!("  → {}\n", suggestion));
    }

    output.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_by_extension() {
        let policy = PolicyFile::default();

        assert_eq!(
            categorize_file(Path::new("/test/main.py"), &policy),
            "code_files"
        );
        assert_eq!(
            categorize_file(Path::new("/test/README.md"), &policy),
            "docs"
        );
        assert_eq!(
            categorize_file(Path::new("/test/config.json"), &policy),
            "config_files"
        );
        assert_eq!(
            categorize_file(Path::new("/test/app.log"), &policy),
            "log_files"
        );
    }

    #[test]
    fn test_categorize_sensitive() {
        let policy = PolicyFile::default();

        assert_eq!(
            categorize_file(Path::new("/test/.env"), &policy),
            "sensitive"
        );
        assert_eq!(
            categorize_file(Path::new("/test/secrets.json"), &policy),
            "sensitive"
        );
        assert_eq!(
            categorize_file(Path::new("/test/server.key"), &policy),
            "sensitive"
        );
    }

    #[test]
    fn test_validation_result_levels() {
        let mut result = ValidationResult::new("/test/file.py", "code_files");

        result.add_warning("warning", "Test warning", "test_rule");
        assert!(result.valid);

        result.add_warning("security", "Security issue", "security_rule");
        assert!(!result.valid);
        assert!(result.has_security_warnings());
    }
}
