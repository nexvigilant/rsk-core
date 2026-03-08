//! Policy definitions and YAML parsing for file organization rules.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur when loading or applying policies
#[derive(Error, Debug)]
pub enum PolicyError {
    #[error("Failed to read policy file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse policy YAML: {0}")]
    ParseError(#[from] serde_yaml::Error),
    #[error("Policy file not found: {0}")]
    NotFound(PathBuf),
    #[error("Invalid policy configuration: {0}")]
    InvalidConfig(String),
}

/// Complete policy file structure
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PolicyFile {
    pub version: Option<u32>,
    pub settings: Option<PolicySettings>,
    pub placement_rules: Option<HashMap<String, PlacementRule>>,
    pub staleness: Option<StalenessConfig>,
    pub forbidden_zones: Option<ForbiddenZones>,
    pub expected_structure: Option<HashMap<String, ProjectStructure>>,
}

/// Global policy settings
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PolicySettings {
    /// Policy enforcement mode: advisory, blocking, or logging
    pub mode: Option<String>,
    /// Action for stale files: prompt, report, or auto-archive
    pub stale_action: Option<String>,
    /// Directory for archived files
    pub archive_dir: Option<String>,
    /// Paths to monitor for organization
    pub monitor_paths: Option<Vec<String>>,
}

/// Rules for file placement by category
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlacementRule {
    /// File patterns that match this rule (e.g., "*.py", "test_*.rs")
    pub patterns: Vec<String>,
    /// Paths where these files should not be placed
    #[serde(default)]
    pub forbidden_paths: Vec<String>,
    /// Recommended paths for these files
    #[serde(default)]
    pub recommended_paths: Vec<String>,
    /// Patterns that are exceptions to the rule
    #[serde(default)]
    pub exceptions: Vec<String>,
    /// Warning message to display
    pub message: Option<String>,
    /// Severity level (e.g., "high" for security issues)
    pub severity: Option<String>,
    /// Staleness threshold in hours
    pub staleness_hours: Option<u64>,
    /// Staleness threshold in days
    pub staleness_days: Option<u64>,
}

/// Configuration for staleness detection
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct StalenessConfig {
    /// Default staleness threshold in days
    pub default_days: Option<u64>,
    /// Path-specific staleness rules
    pub path_rules: Option<HashMap<String, StalenessRule>>,
    /// Patterns to ignore for staleness
    pub ignore_patterns: Option<Vec<String>>,
}

/// Individual staleness rule
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StalenessRule {
    /// Threshold in days
    pub days: Option<u64>,
    /// Action to take: warn, prompt, report, ignore
    pub action: Option<String>,
}

/// Forbidden zones configuration
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ForbiddenZones {
    /// Paths where files should not be created
    pub paths: Option<Vec<String>>,
    /// Exceptions to forbidden zones
    pub exceptions: Option<Vec<String>>,
}

/// Expected project structure definition
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProjectStructure {
    /// Files that indicate this project type
    pub indicators: Option<Vec<String>>,
    /// Expected directories for this project type
    pub expected_dirs: Option<Vec<String>>,
}

impl PolicyFile {
    /// Load policy from the default location (~/.claude/file-policies.yaml)
    pub fn load_default() -> Result<Self, PolicyError> {
        let default_path = dirs::home_dir()
            .ok_or_else(|| PolicyError::InvalidConfig("Cannot determine home directory".into()))?
            .join(".claude/file-policies.yaml");

        Self::load(&default_path)
    }

    /// Load policy from a specific path
    pub fn load(path: &Path) -> Result<Self, PolicyError> {
        if !path.exists() {
            return Err(PolicyError::NotFound(path.to_path_buf()));
        }

        let content = fs::read_to_string(path)?;
        let policy: PolicyFile = serde_yaml::from_str(&content)?;
        Ok(policy)
    }

    /// Load policy or return default if not found
    pub fn load_or_default(path: Option<&Path>) -> Self {
        match path {
            Some(p) => Self::load(p).unwrap_or_default(),
            None => Self::load_default().unwrap_or_default(),
        }
    }

    /// Get the enforcement mode
    pub fn mode(&self) -> &str {
        self.settings
            .as_ref()
            .and_then(|s| s.mode.as_deref())
            .unwrap_or("advisory")
    }

    /// Get the stale action
    pub fn stale_action(&self) -> &str {
        self.settings
            .as_ref()
            .and_then(|s| s.stale_action.as_deref())
            .unwrap_or("report")
    }

    /// Get default staleness days
    pub fn default_staleness_days(&self) -> u64 {
        self.staleness
            .as_ref()
            .and_then(|s| s.default_days)
            .unwrap_or(30)
    }

    /// Check if a pattern should be ignored for staleness
    pub fn is_staleness_ignored(&self, path: &str) -> bool {
        self.staleness
            .as_ref()
            .and_then(|s| s.ignore_patterns.as_ref())
            .map(|patterns| patterns.iter().any(|p| matches_glob(path, p)))
            .unwrap_or(false)
    }
}

impl PolicySettings {
    pub fn mode(&self) -> &str {
        self.mode.as_deref().unwrap_or("advisory")
    }
}

/// Simple glob pattern matching
pub fn matches_glob(path: &str, pattern: &str) -> bool {
    let filename = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    // Handle ** for recursive matching
    if pattern.contains("**") {
        let base = pattern.replace("**", "");
        if path.contains(&base) {
            return true;
        }
    }

    // Simple wildcard matching
    if pattern.starts_with('*') && pattern.len() > 1 {
        let suffix = &pattern[1..];
        if filename.ends_with(suffix) || path.ends_with(suffix) {
            return true;
        }
    }

    if pattern.ends_with('*') && pattern.len() > 1 {
        let prefix = &pattern[..pattern.len() - 1];
        if filename.starts_with(prefix) || path.starts_with(prefix) {
            return true;
        }
    }

    // Exact match
    filename == pattern || path.ends_with(pattern)
}

/// Expand ~ to home directory
pub fn expand_path(path: &str) -> String {
    if path.starts_with("~/")
        && let Some(home) = dirs::home_dir()
    {
        return path.replacen("~", home.to_str().unwrap_or(""), 1);
    }
    path.to_string()
}

/// Check if a file path is within a given directory path
pub fn is_in_path(file_path: &str, check_path: &str) -> bool {
    let expanded_file = expand_path(file_path);
    let expanded_check = expand_path(check_path);

    // Handle home directory root special case
    if check_path == "~/"
        && let Some(home) = dirs::home_dir()
    {
        let home_str = home.to_str().unwrap_or("");
        let parent = Path::new(&expanded_file)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        return parent == home_str;
    }

    // Handle ** for recursive matching
    if expanded_check.contains("**") {
        let base = expanded_check.replace("**", "");
        return expanded_file.contains(&base);
    }

    expanded_file.starts_with(&expanded_check) || expanded_file.contains(&expanded_check)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_glob_extension() {
        assert!(matches_glob("/home/user/test.py", "*.py"));
        assert!(matches_glob("main.rs", "*.rs"));
        assert!(!matches_glob("test.py", "*.rs"));
    }

    #[test]
    fn test_matches_glob_prefix() {
        assert!(matches_glob("test_main.py", "test_*"));
        assert!(!matches_glob("main_test.py", "test_*"));
    }

    #[test]
    fn test_is_in_path_home() {
        // This test depends on having a home directory
        if dirs::home_dir().is_some() {
            let home = dirs::home_dir().unwrap();
            let file_in_home = format!("{}/test.py", home.display());
            assert!(is_in_path(&file_in_home, "~/"));

            let file_in_subdir = format!("{}/projects/test.py", home.display());
            assert!(!is_in_path(&file_in_subdir, "~/"));
        }
    }

    #[test]
    fn test_expand_path() {
        if let Some(home) = dirs::home_dir() {
            let expanded = expand_path("~/test");
            assert!(expanded.starts_with(home.to_str().unwrap()));
            assert!(expanded.ends_with("test"));
        }
    }
}
