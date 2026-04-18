//! Integration Patrol — detects pub symbols that are declared but not consumed
//! at the expected ring level. Guardian-triggered via patrol-guardian.sh hook.
//!
//! Ring model:
//!   Ring 0: same module's tests only (self-referential)
//!   Ring 1: sibling modules within microgram/ (library use)
//!   Ring 2: CLI handlers or chain_registry test infrastructure (feature use)
//!
//! A classification config (patrol.yaml) declares which symbols are "features"
//! (must reach Ring 2) vs "library" (Ring 1 sufficient). Unclassified symbols
//! that reach only Ring 0 are flagged as UNCLASSIFIED — forcing the author
//! to decide before claiming completion.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// Strip `#[cfg(test)]` module blocks from Rust source content.
/// This ensures Ring 1/2 content searches don't match test-only references.
fn strip_test_blocks(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut depth: usize = 0;
    let mut in_test_block = false;
    let mut prev_line_was_cfg_test = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("#[cfg(test)]") {
            prev_line_was_cfg_test = true;
            continue;
        }

        if prev_line_was_cfg_test && trimmed.starts_with("mod ") {
            in_test_block = true;
            depth = 0;
        }
        prev_line_was_cfg_test = false;

        if in_test_block {
            for ch in trimmed.chars() {
                if ch == '{' {
                    depth += 1;
                }
                if ch == '}' {
                    depth = depth.saturating_sub(1);
                }
            }
            if depth == 0 && trimmed.contains('}') {
                in_test_block = false;
            }
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }
    result
}

/// Classification of a pub symbol's expected reach
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum SymbolClass {
    Feature,
    Library,
    Unclassified,
}

/// Ring level a symbol actually reaches
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Ring {
    /// Only referenced in same module's #[cfg(test)]
    Ring0,
    /// Referenced by sibling modules within microgram/
    Ring1,
    /// Referenced by CLI handlers or chain_registry
    Ring2,
}

/// A single patrol finding
#[derive(Debug, Clone, Serialize)]
pub struct PatrolFinding {
    pub symbol: String,
    pub classification: SymbolClass,
    pub actual_ring: Ring,
    pub expected_ring: Ring,
    pub verdict: PatrolVerdict,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PatrolVerdict {
    /// Symbol reaches its expected ring — no action needed
    Ok,
    /// Feature symbol doesn't reach Ring 2
    Unwired,
    /// Unclassified symbol at Ring 0 — needs classification
    Unclassified,
    /// Symbol listed in patrol.yaml but not found in source code
    StaleConfig,
}

/// Full patrol report
#[derive(Debug, Clone, Serialize)]
pub struct PatrolReport {
    pub clean: bool,
    pub total_symbols: usize,
    pub ok: usize,
    pub unwired: usize,
    pub unclassified: usize,
    pub stale: usize,
    pub findings: Vec<PatrolFinding>,
}

/// Classification config loaded from patrol.yaml
#[derive(Debug, Deserialize)]
struct PatrolConfig {
    #[serde(default)]
    features: Vec<String>,
    #[serde(default)]
    library: Vec<String>,
}

/// Extract pub free function names from a Rust source file.
/// Skips methods (functions with `&self`, `&mut self`, or `self` as first param)
/// since methods are tied to their struct's usage, not independently invocable.
fn extract_pub_functions(source: &str) -> Vec<String> {
    let mut fns = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("pub fn ") {
            // Skip methods — they have self as first param
            if trimmed.contains("&self")
                || trimmed.contains("&mut self")
                || trimmed.contains("(self")
            {
                continue;
            }
            // Extract function name: "pub fn foo(" → "foo"
            // skip "pub fn "
            let end = rest.find(['(', '<', ' ']).unwrap_or(rest.len());
            let name = &rest[..end];
            if !name.is_empty() {
                fns.push(name.to_string());
            }
        }
    }
    fns
}

/// Count occurrences of a symbol in a file's content (word-boundary aware)
fn count_references(content: &str, symbol: &str) -> usize {
    content
        .match_indices(symbol)
        .filter(|(pos, _)| {
            // Check word boundary before
            let before_ok = *pos == 0 || {
                let prev = content.as_bytes()[pos - 1];
                !prev.is_ascii_alphanumeric() && prev != b'_'
            };
            // Check word boundary after
            let after_pos = pos + symbol.len();
            let after_ok = after_pos >= content.len() || {
                let next = content.as_bytes()[after_pos];
                !next.is_ascii_alphanumeric() && next != b'_'
            };
            before_ok && after_ok
        })
        .count()
}

/// Run the integration patrol on the microgram module.
///
/// `module_dir` is the path to `rsk/src/modules/microgram/`
/// `cli_dir` is the path to `rsk/src/cli/`
///
/// Ring 2 scope is fixed: `cli/handlers/`, `cli/actions.rs`, and
/// `microgram/chain_registry.rs`. If new Ring 2 consumers are added
/// (e.g., new CLI entry points outside `handlers/`), they must be
/// registered here explicitly.
pub fn run_patrol(module_dir: &Path, cli_dir: &Path) -> Result<PatrolReport, String> {
    // Load classification config
    let config_path = module_dir.join("patrol.yaml");
    let config: PatrolConfig = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Cannot read patrol.yaml: {e}"))?;
        serde_yaml::from_str(&content).map_err(|e| format!("Parse error in patrol.yaml: {e}"))?
    } else {
        PatrolConfig {
            features: vec![],
            library: vec![],
        }
    };

    let features: HashSet<String> = config.features.into_iter().collect();
    let library: HashSet<String> = config.library.into_iter().collect();

    // Structural invariant: module must be flat (no subdirectories).
    // If subdirectories exist, collect_rs_files would silently miss them,
    // producing false-negative patrol results. Fail loud instead.
    guard_flat_module(module_dir)?;

    // Collect all pub functions from source files in the module
    let mut all_pub_fns: Vec<String> = Vec::new();
    let source_files = collect_rs_files(module_dir)?;

    for path in &source_files {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        all_pub_fns.extend(extract_pub_functions(&content));
    }

    // Read re-exports from mod.rs to know which functions are part of the public API
    // Uses word-boundary matching (not substring contains) to avoid false positives
    // where short names like "run" match inside "run_patrol"
    let mod_rs = module_dir.join("mod.rs");
    let mod_content =
        std::fs::read_to_string(&mod_rs).map_err(|e| format!("Cannot read mod.rs: {e}"))?;
    let reexported: HashSet<String> = all_pub_fns
        .iter()
        .filter(|name| count_references(&mod_content, name) > 0)
        .cloned()
        .collect();

    // Only patrol re-exported functions (the public API contract)
    let symbols_to_check: Vec<String> = reexported.into_iter().collect();

    // Load Ring 1 content (sibling modules, excluding test blocks)
    // Test-only references from siblings are Ring 0 from the caller's perspective,
    // not Ring 1. Strip #[cfg(test)] mod tests { ... } blocks.
    let mut ring1_content = String::new();
    for path in &source_files {
        let fname = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        if fname == "mod.rs" {
            // mod.rs re-exports don't count as Ring 1 usage
            continue;
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        ring1_content.push_str(&strip_test_blocks(&content));
        ring1_content.push('\n');
    }

    // Load Ring 2 content (CLI handlers + chain_registry)
    let mut ring2_content = String::new();
    let handlers_dir = cli_dir.join("handlers");
    if handlers_dir.exists() {
        for path in collect_rs_files(&handlers_dir)? {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
            ring2_content.push_str(&content);
            ring2_content.push('\n');
        }
    }
    // chain_registry.rs is also Ring 2 (test infrastructure)
    let registry_path = module_dir.join("chain_registry.rs");
    if registry_path.exists() {
        let content = std::fs::read_to_string(&registry_path)
            .map_err(|e| format!("Cannot read chain_registry.rs: {e}"))?;
        ring2_content.push_str(&content);
    }
    // actions.rs is Ring 2 (CLI definitions)
    let actions_path = cli_dir.join("actions.rs");
    if actions_path.exists() {
        let content = std::fs::read_to_string(&actions_path)
            .map_err(|e| format!("Cannot read actions.rs: {e}"))?;
        ring2_content.push_str(&content);
    }

    // Defect 3 fix: Detect stale YAML entries — symbols classified in patrol.yaml
    // that no longer exist as pub functions in any source file.
    let all_pub_set: HashSet<&str> = all_pub_fns.iter().map(|s| s.as_str()).collect();
    let mut stale_findings: Vec<PatrolFinding> = Vec::new();
    for name in features.iter().chain(library.iter()) {
        if !all_pub_set.contains(name.as_str()) {
            stale_findings.push(PatrolFinding {
                symbol: name.clone(),
                classification: if features.contains(name) {
                    SymbolClass::Feature
                } else {
                    SymbolClass::Library
                },
                actual_ring: Ring::Ring0,
                expected_ring: Ring::Ring0,
                verdict: PatrolVerdict::StaleConfig,
            });
        }
    }

    // Classify and check each symbol
    let mut findings = Vec::new();
    let mut ok_count = 0;
    let mut unwired_count = 0;
    let mut unclassified_count = 0;
    let stale_count = stale_findings.len();

    // Build a deterministic ordering
    let mut sorted_symbols: Vec<String> = symbols_to_check;
    sorted_symbols.sort();

    for symbol in &sorted_symbols {
        let ring2_refs = count_references(&ring2_content, symbol);
        let ring1_refs = count_references(&ring1_content, symbol);

        let actual_ring = if ring2_refs > 0 {
            Ring::Ring2
        } else if ring1_refs > 0 {
            Ring::Ring1
        } else {
            Ring::Ring0
        };

        let classification = if features.contains(symbol) {
            SymbolClass::Feature
        } else if library.contains(symbol) {
            SymbolClass::Library
        } else {
            SymbolClass::Unclassified
        };

        let expected_ring = match &classification {
            SymbolClass::Feature => Ring::Ring2,
            SymbolClass::Library => Ring::Ring1,
            SymbolClass::Unclassified => Ring::Ring1, // minimum for re-exported
        };

        let verdict = match (&classification, &actual_ring) {
            (SymbolClass::Feature, Ring::Ring2) => PatrolVerdict::Ok,
            (SymbolClass::Feature, _) => PatrolVerdict::Unwired,
            (SymbolClass::Library, Ring::Ring0) => PatrolVerdict::Unwired,
            (SymbolClass::Library, _) => PatrolVerdict::Ok,
            (SymbolClass::Unclassified, Ring::Ring0) => PatrolVerdict::Unclassified,
            (SymbolClass::Unclassified, _) => PatrolVerdict::Ok,
        };

        match &verdict {
            PatrolVerdict::Ok => ok_count += 1,
            PatrolVerdict::Unwired => unwired_count += 1,
            PatrolVerdict::Unclassified => unclassified_count += 1,
            PatrolVerdict::StaleConfig => {} // only produced in stale_findings, not here
        }

        findings.push(PatrolFinding {
            symbol: symbol.clone(),
            classification,
            actual_ring,
            expected_ring,
            verdict,
        });
    }

    // Append stale config findings
    findings.extend(stale_findings);

    Ok(PatrolReport {
        clean: unwired_count == 0 && unclassified_count == 0 && stale_count == 0,
        total_symbols: findings.len(),
        ok: ok_count,
        unwired: unwired_count,
        unclassified: unclassified_count,
        stale: stale_count,
        findings,
    })
}

/// Guard that the module directory contains no subdirectories with .rs files.
/// A subdirectory would be invisible to `collect_rs_files`, causing silent
/// false negatives in the patrol report. This converts that into a loud error.
fn guard_flat_module(dir: &Path) -> Result<(), String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("Cannot read dir {}: {e}", dir.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Dir entry error: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            // Check if the subdirectory contains any .rs files
            let has_rs = std::fs::read_dir(&path)
                .map(|rd| {
                    rd.flatten()
                        .any(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
                })
                .unwrap_or(false);
            if has_rs {
                return Err(format!(
                    "Subdirectory {} contains .rs files — patrol requires flat module structure. \
                     Either move files to the module root or make collect_rs_files recursive.",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

/// Collect all .rs files in a directory (non-recursive).
fn collect_rs_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("Cannot read dir {}: {e}", dir.display()))?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Dir entry error: {e}"))?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Convenience: resolve standard paths and run patrol
pub fn run_patrol_default(project_root: &Path) -> Result<PatrolReport, String> {
    let module_dir = project_root.join("rsk/src/modules/microgram");
    let cli_dir = project_root.join("rsk/src/cli");
    run_patrol(&module_dir, &cli_dir)
}
