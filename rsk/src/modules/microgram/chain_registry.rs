//! Chain Registry — named chain definitions with end-to-end test cases.

use crate::modules::decision_engine::Value;
use super::{Microgram, load_all};
use super::chain::{chain, chain_accumulate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A named chain definition loaded from YAML
#[derive(Debug, Clone, Deserialize)]
pub struct ChainDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    /// Relative path to micrograms directory (from chain file location)
    #[serde(default = "default_mcg_dir")]
    pub micrograms_dir: String,
    /// Ordered list of microgram names in the chain
    pub steps: Vec<String>,
    /// End-to-end test cases for the chain
    #[serde(default)]
    pub tests: Vec<ChainTestCase>,
    /// Use context accumulation mode (merge outputs into running context)
    #[serde(default)]
    pub accumulate: bool,
}

fn default_version() -> String { "0.1.0".to_string() }
fn default_mcg_dir() -> String { "../micrograms".to_string() }

/// A test case for a chain definition
#[derive(Debug, Clone, Deserialize)]
pub struct ChainTestCase {
    #[serde(default)]
    pub name: String,
    pub input: HashMap<String, Value>,
    pub expect: HashMap<String, Value>,
}

/// Result of testing one chain definition
#[derive(Debug, Clone, Serialize)]
pub struct ChainTestResult {
    pub chain_name: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<SingleChainTestResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SingleChainTestResult {
    pub name: String,
    pub passed: bool,
    pub input: HashMap<String, Value>,
    pub expected: HashMap<String, Value>,
    pub actual: HashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mismatch: Option<String>,
}

impl ChainDefinition {
    /// Load a chain definition from a YAML file
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        serde_yaml::from_str(&content).map_err(|e| format!("Parse error in {}: {e}", path.display()))
    }

    /// Resolve the micrograms directory relative to the chain file
    pub fn resolve_mcg_dir(&self, chain_file: &Path) -> std::path::PathBuf {
        chain_file
            .parent()
            .unwrap_or(Path::new("."))
            .join(&self.micrograms_dir)
    }

    /// Run all chain test cases
    pub fn test(&self, micrograms: &[Microgram]) -> ChainTestResult {
        let mut results = Vec::with_capacity(self.tests.len());
        let mut passed = 0;

        // Resolve the ordered micrograms for this chain
        let ordered: Vec<&Microgram> = self.steps.iter().filter_map(|name| {
            micrograms.iter().find(|mg| mg.name == *name)
        }).collect();

        if ordered.len() != self.steps.len() {
            let missing: Vec<&String> = self.steps.iter()
                .filter(|s| !micrograms.iter().any(|mg| mg.name == **s))
                .collect();
            return ChainTestResult {
                chain_name: self.name.clone(),
                total: self.tests.len(),
                passed: 0,
                failed: self.tests.len(),
                results: self.tests.iter().map(|t| SingleChainTestResult {
                    name: t.name.clone(),
                    passed: false,
                    input: t.input.clone(),
                    expected: t.expect.clone(),
                    actual: HashMap::new(),
                    mismatch: Some(format!("Missing micrograms: {:?}", missing)),
                }).collect(),
            };
        }

        let owned: Vec<Microgram> = ordered.into_iter().cloned().collect();

        for test in &self.tests {
            let chain_result = if self.accumulate {
                chain_accumulate(&owned, test.input.clone())
            } else {
                chain(&owned, test.input.clone())
            };
            let actual = chain_result.final_output;

            let mut mismatch = None;
            let mut test_passed = true;

            for (key, expected_val) in &test.expect {
                match actual.get(key) {
                    Some(actual_val) if actual_val == expected_val => {}
                    Some(actual_val) => {
                        mismatch = Some(format!(
                            "{}: expected {:?}, got {:?}", key, expected_val, actual_val
                        ));
                        test_passed = false;
                    }
                    None => {
                        mismatch = Some(format!(
                            "{}: expected {:?}, not in output", key, expected_val
                        ));
                        test_passed = false;
                    }
                }
            }

            if test_passed { passed += 1; }

            results.push(SingleChainTestResult {
                name: test.name.clone(),
                passed: test_passed,
                input: test.input.clone(),
                expected: test.expect.clone(),
                actual,
                mismatch,
            });
        }

        ChainTestResult {
            chain_name: self.name.clone(),
            total: self.tests.len(),
            passed,
            failed: self.tests.len() - passed,
            results,
        }
    }
}

/// Load all chain definitions from a directory
pub fn load_chains(dir: &Path) -> Result<Vec<(ChainDefinition, std::path::PathBuf)>, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read chain dir: {e}"))?;

    let mut chains = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Dir entry error: {e}"))?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
            match ChainDefinition::load(&path) {
                Ok(def) => chains.push((def, path)),
                Err(e) => eprintln!("Warning: skipping {}: {e}", path.display()),
            }
        }
    }

    chains.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    Ok(chains)
}

/// Test all chain definitions in a directory
pub fn test_chains(chains_dir: &Path) -> Result<Vec<ChainTestResult>, String> {
    let chains = load_chains(chains_dir)?;
    let mut results = Vec::with_capacity(chains.len());

    for (def, path) in &chains {
        let mcg_dir = def.resolve_mcg_dir(path);
        let micrograms = load_all(&mcg_dir)?;
        results.push(def.test(&micrograms));
    }

    Ok(results)
}
