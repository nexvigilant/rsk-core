//! Directory scanning for policy violations and stale files.

use super::policy::PolicyFile;
use super::staleness::{StalenessResult, StalenessSummary, check_staleness};
use super::validation::{ValidationResult, validate_file};
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

/// Results from scanning a directory
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScanResult {
    /// Total number of files scanned
    pub total_files: usize,
    /// Files with policy violations
    pub violations: Vec<ValidationResult>,
    /// Stale files found
    pub stale_files: Vec<StalenessResult>,
    /// Summary statistics
    pub summary: ScanSummary,
}

/// Summary of scan results
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ScanSummary {
    pub total_scanned: usize,
    pub placement_warnings: usize,
    pub security_warnings: usize,
    pub stale_files: usize,
}

/// Directories to skip during scanning
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    ".claude",
    "target",
    "build",
    "dist",
    ".cache",
];

/// Scan a directory for policy violations and stale files
pub fn scan_directory(path: &Path, max_depth: usize, policy: &PolicyFile) -> ScanResult {
    let mut violations = Vec::new();
    let mut stale_files = Vec::new();
    let mut total_files = 0;
    let mut staleness_summary = StalenessSummary::default();

    let walker = WalkDir::new(path)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip hidden directories and known skip dirs
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    if name.starts_with('.') && name != "." {
                        return false;
                    }
                    if SKIP_DIRS.contains(&name) {
                        return false;
                    }
                }
            }
            true
        });

    for entry in walker.filter_map(|e| e.ok()) {
        let entry_path = entry.path();

        if entry_path.is_file() {
            total_files += 1;

            // Validate file placement
            let validation = validate_file(entry_path, policy);
            if validation.has_warnings() {
                violations.push(validation);
            }

            // Check staleness
            let staleness = check_staleness(entry_path, policy);
            staleness_summary.add(&staleness);
            if staleness.is_stale {
                stale_files.push(staleness);
            }
        }
    }

    let summary = ScanSummary {
        total_scanned: total_files,
        placement_warnings: violations.len(),
        security_warnings: violations
            .iter()
            .filter(|v| v.has_security_warnings())
            .count(),
        stale_files: stale_files.len(),
    };

    ScanResult {
        total_files,
        violations,
        stale_files,
        summary,
    }
}

/// Scan options for customizing the scan
#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Maximum depth to scan
    pub max_depth: usize,
    /// Only check staleness (skip validation)
    pub staleness_only: bool,
    /// Only check validation (skip staleness)
    pub validation_only: bool,
    /// Additional directories to skip
    pub skip_dirs: Vec<String>,
}

impl ScanOptions {
    pub fn new() -> Self {
        Self {
            max_depth: 3,
            staleness_only: false,
            validation_only: false,
            skip_dirs: Vec::new(),
        }
    }

    pub fn with_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn staleness_only(mut self) -> Self {
        self.staleness_only = true;
        self
    }

    pub fn validation_only(mut self) -> Self {
        self.validation_only = true;
        self
    }
}

/// Scan with custom options
pub fn scan_with_options(path: &Path, options: &ScanOptions, policy: &PolicyFile) -> ScanResult {
    let mut violations = Vec::new();
    let mut stale_files = Vec::new();
    let mut total_files = 0;

    let skip_set: Vec<&str> = SKIP_DIRS
        .iter()
        .copied()
        .chain(options.skip_dirs.iter().map(String::as_str))
        .collect();

    let walker = WalkDir::new(path)
        .max_depth(options.max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    if name.starts_with('.') && name != "." {
                        return false;
                    }
                    if skip_set.contains(&name) {
                        return false;
                    }
                }
            }
            true
        });

    for entry in walker.filter_map(|e| e.ok()) {
        let entry_path = entry.path();

        if entry_path.is_file() {
            total_files += 1;

            if !options.staleness_only {
                let validation = validate_file(entry_path, policy);
                if validation.has_warnings() {
                    violations.push(validation);
                }
            }

            if !options.validation_only {
                let staleness = check_staleness(entry_path, policy);
                if staleness.is_stale {
                    stale_files.push(staleness);
                }
            }
        }
    }

    let summary = ScanSummary {
        total_scanned: total_files,
        placement_warnings: violations.len(),
        security_warnings: violations
            .iter()
            .filter(|v| v.has_security_warnings())
            .count(),
        stale_files: stale_files.len(),
    };

    ScanResult {
        total_files,
        violations,
        stale_files,
        summary,
    }
}

/// Format scan result for human-readable output
pub fn format_scan_result(result: &ScanResult) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "=== Scan Results ===\n\
         Total files: {}\n\
         Placement warnings: {}\n\
         Security warnings: {}\n\
         Stale files: {}\n\n",
        result.total_files,
        result.summary.placement_warnings,
        result.summary.security_warnings,
        result.summary.stale_files
    ));

    if !result.violations.is_empty() {
        output.push_str("Violations:\n");
        for v in &result.violations {
            for w in &v.warnings {
                output.push_str(&format!("  [{}] {}: {}\n", w.level, v.path, w.message));
            }
        }
        output.push('\n');
    }

    if !result.stale_files.is_empty() {
        output.push_str("Stale files:\n");
        for s in &result.stale_files {
            output.push_str(&format!(
                "  {} ({} days old, threshold: {} days)\n",
                s.path, s.age_days, s.threshold_days
            ));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{File, create_dir_all};
    use tempfile::tempdir;

    #[test]
    fn test_scan_empty_dir() {
        let dir = tempdir().unwrap();
        let policy = PolicyFile::default();

        let result = scan_directory(dir.path(), 3, &policy);

        assert_eq!(result.total_files, 0);
        assert!(result.violations.is_empty());
        assert!(result.stale_files.is_empty());
    }

    #[test]
    fn test_scan_with_files() {
        let dir = tempdir().unwrap();
        let policy = PolicyFile::default();

        // Create some test files
        File::create(dir.path().join("test.py")).unwrap();
        File::create(dir.path().join("config.json")).unwrap();

        let result = scan_directory(dir.path(), 3, &policy);

        assert_eq!(result.total_files, 2);
    }

    #[test]
    fn test_scan_skips_dirs() {
        let dir = tempdir().unwrap();
        let policy = PolicyFile::default();

        // Create a node_modules directory with a file
        let nm_dir = dir.path().join("node_modules");
        create_dir_all(&nm_dir).unwrap();
        File::create(nm_dir.join("package.json")).unwrap();

        // Create a regular file
        File::create(dir.path().join("main.py")).unwrap();

        let result = scan_directory(dir.path(), 3, &policy);

        // Should only find main.py, not the file in node_modules
        assert_eq!(result.total_files, 1);
    }

    #[test]
    fn test_scan_options() {
        let options = ScanOptions::new().with_depth(5).staleness_only();

        assert_eq!(options.max_depth, 5);
        assert!(options.staleness_only);
        assert!(!options.validation_only);
    }
}
