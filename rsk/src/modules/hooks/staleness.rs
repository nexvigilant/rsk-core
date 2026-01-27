//! Staleness detection for files and directories.

use super::policy::{PolicyFile, matches_glob};
use super::validation::categorize_file;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

/// Result of checking a file's staleness
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StalenessResult {
    /// The file path that was checked
    pub path: String,
    /// Whether the file is considered stale
    pub is_stale: bool,
    /// Age of the file in days
    pub age_days: u64,
    /// Threshold that was applied
    pub threshold_days: u64,
    /// Category of the file
    pub category: String,
    /// Action to take for this file
    pub action: String,
}

impl StalenessResult {
    pub fn new(path: &str, category: &str) -> Self {
        Self {
            path: path.to_string(),
            is_stale: false,
            age_days: 0,
            threshold_days: 30,
            category: category.to_string(),
            action: "report".to_string(),
        }
    }
}

/// Get the age of a file in days
pub fn get_file_age_days(path: &Path) -> u64 {
    if let Ok(metadata) = fs::metadata(path) {
        if let Ok(modified) = metadata.modified() {
            let duration = SystemTime::now()
                .duration_since(modified)
                .unwrap_or_default();
            return duration.as_secs() / 86400;
        }
    }
    0
}

/// Check if a file is stale based on policy rules
pub fn check_staleness(path: &Path, policy: &PolicyFile) -> StalenessResult {
    let path_str = path.to_str().unwrap_or("");
    let category = categorize_file(path, policy);
    let mut result = StalenessResult::new(path_str, &category);

    // Get file age
    result.age_days = get_file_age_days(path);

    // Determine threshold from policy
    result.threshold_days = policy.default_staleness_days();

    // Check category-specific staleness from placement rules
    if let Some(rules) = &policy.placement_rules {
        if let Some(rule) = rules.get(&category) {
            if let Some(hours) = rule.staleness_hours {
                result.threshold_days = (hours / 24).max(1);
            }
            if let Some(days) = rule.staleness_days {
                result.threshold_days = days;
            }
        }
    }

    // Check path-specific rules
    if let Some(staleness) = &policy.staleness {
        if let Some(path_rules) = &staleness.path_rules {
            for (pattern, rule) in path_rules {
                if matches_glob(path_str, pattern) {
                    if let Some(days) = rule.days {
                        result.threshold_days = days;
                    }
                    if let Some(action) = &rule.action {
                        result.action = action.clone();
                    }
                    break;
                }
            }
        }

        // Check ignore patterns
        if let Some(ignore) = &staleness.ignore_patterns {
            for pattern in ignore {
                if matches_glob(path_str, pattern) {
                    result.action = "ignore".to_string();
                    break;
                }
            }
        }
    }

    // Determine if stale
    result.is_stale = result.age_days > result.threshold_days && result.action != "ignore";

    result
}

/// Summary of staleness analysis
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct StalenessSummary {
    pub total_files: usize,
    pub stale_count: usize,
    pub temp_files: usize,
    pub log_files: usize,
    pub build_artifacts: usize,
    pub other: usize,
}

impl StalenessSummary {
    pub fn add(&mut self, result: &StalenessResult) {
        self.total_files += 1;
        if result.is_stale {
            self.stale_count += 1;
            match result.category.as_str() {
                "temp_files" => self.temp_files += 1,
                "log_files" => self.log_files += 1,
                "build_artifacts" => self.build_artifacts += 1,
                _ => self.other += 1,
            }
        }
    }
}

/// Format staleness result for human-readable output
pub fn format_staleness_result(result: &StalenessResult) -> String {
    if result.is_stale {
        format!(
            "[Stale] {} ({} days old, threshold: {} days, action: {})",
            result.path, result.age_days, result.threshold_days, result.action
        )
    } else {
        format!(
            "[OK] {} ({} days old, threshold: {} days)",
            result.path, result.age_days, result.threshold_days
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_get_file_age() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let age = get_file_age_days(&file_path);
        assert_eq!(age, 0); // Just created, should be 0 days old
    }

    #[test]
    fn test_staleness_check_default() {
        let policy = PolicyFile::default();
        let result = check_staleness(Path::new("/test/file.txt"), &policy);

        assert_eq!(result.threshold_days, 30); // Default
        assert_eq!(result.action, "report");
    }

    #[test]
    fn test_staleness_summary() {
        let mut summary = StalenessSummary::default();

        let mut result1 = StalenessResult::new("/test/old.tmp", "temp_files");
        result1.is_stale = true;
        summary.add(&result1);

        let mut result2 = StalenessResult::new("/test/old.log", "log_files");
        result2.is_stale = true;
        summary.add(&result2);

        let result3 = StalenessResult::new("/test/new.py", "code_files");
        summary.add(&result3);

        assert_eq!(summary.total_files, 3);
        assert_eq!(summary.stale_count, 2);
        assert_eq!(summary.temp_files, 1);
        assert_eq!(summary.log_files, 1);
    }
}
