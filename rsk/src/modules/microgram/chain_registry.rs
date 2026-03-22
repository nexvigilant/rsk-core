//! Chain Registry — named chain definitions with end-to-end test cases.

use crate::modules::decision_engine::Value;
use super::{Microgram, load_all};
use super::chain::{chain, chain_accumulate, chain_loop, LoopResult};
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
    /// Optional per-step expected decision paths (macrotest pattern).
    /// Each entry is the expected path for one chain step. If fewer entries
    /// than chain steps, uncovered steps are not checked.
    #[serde(default)]
    pub expect_paths: Vec<Vec<String>>,
}

/// Result of testing one chain definition
#[derive(Debug, Clone, Serialize)]
pub struct ChainTestResult {
    pub chain_name: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<SingleChainTestResult>,
    /// Primitive signature chain validation (conservation law check)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_validation: Option<super::signature_validator::SignatureValidation>,
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
                    mismatch: Some(format!("Missing micrograms: {missing:?}")),
                }).collect(),
                signature_validation: None,
            };
        }

        let owned: Vec<Microgram> = ordered.into_iter().cloned().collect();

        for test in &self.tests {
            let chain_result = if self.accumulate {
                chain_accumulate(&owned, test.input.clone(), false)
            } else {
                chain(&owned, test.input.clone(), false)
            };
            let actual = chain_result.final_output.clone();

            let mut mismatch = None;
            let mut test_passed = true;

            for (key, expected_val) in &test.expect {
                match actual.get(key) {
                    Some(actual_val) if actual_val == expected_val => {}
                    Some(actual_val) => {
                        mismatch = Some(format!(
                            "{key}: expected {expected_val:?}, got {actual_val:?}"
                        ));
                        test_passed = false;
                    }
                    None => {
                        mismatch = Some(format!(
                            "{key}: expected {expected_val:?}, not in output"
                        ));
                        test_passed = false;
                    }
                }
            }

            // Path snapshot verification (macrotest pattern):
            // if expect_paths is non-empty, verify each step's decision path
            if !test.expect_paths.is_empty() {
                for (step_i, expected_path) in test.expect_paths.iter().enumerate() {
                    if step_i >= chain_result.steps.len() {
                        break;
                    }
                    let actual_path = &chain_result.steps[step_i].path;
                    if actual_path != expected_path {
                        mismatch = Some(format!(
                            "path[{}] ({}): expected {:?}, got {:?}",
                            step_i, chain_result.steps[step_i].name,
                            expected_path, actual_path
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
            signature_validation: None,
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

// ─────────────────────────────────────────────────────────────────────
// Process Factory — a Process is a Chain + ρ(loop) + ∂(governor) + π(trajectory)
// ─────────────────────────────────────────────────────────────────────

use super::PrimitiveSignature;

/// A process definition: a chain that loops with governor control.
///
/// Primitive grounding: ×(→, ρ, ∂)
/// - → (Causality): the chain pipeline
/// - ρ (Recursion): the feedback loop
/// - ∂ (Boundary): the governor gate that decides halt/continue
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default = "default_mcg_dir")]
    pub micrograms_dir: String,
    /// Ordered list of microgram names in the chain body
    pub steps: Vec<String>,
    /// Use context accumulation (always true for processes — loops need state)
    #[serde(default = "default_true")]
    pub accumulate: bool,

    // ── ρ (loop configuration) ──
    /// Maximum iterations before forced halt
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// Detect convergence (ρ-fixpoint: output == previous output)
    #[serde(default = "default_true")]
    pub detect_convergence: bool,

    // ── ∂ (governor gate) ──
    /// Output field to check for halt condition
    #[serde(default)]
    pub halt_field: Option<String>,
    /// Value that triggers halt (JSON-compatible)
    #[serde(default)]
    pub halt_value: Option<Value>,
    /// Name of a governor microgram to append to the chain (e.g., "loop-governor")
    #[serde(default)]
    pub governor: Option<String>,

    // ── π (trajectory) ──
    /// Whether to include full trajectory in output
    #[serde(default = "default_true")]
    pub trajectory: bool,

    // ── primitive signature ──
    #[serde(default)]
    pub primitive_signature: Option<PrimitiveSignature>,

    /// End-to-end test cases
    #[serde(default)]
    pub tests: Vec<ProcessTestCase>,
}

fn default_true() -> bool { true }
fn default_max_iterations() -> usize { 10 }

/// Test case for a process — checks final state after loop completes
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessTestCase {
    #[serde(default)]
    pub name: String,
    pub input: HashMap<String, Value>,
    /// Expected fields in final_state
    pub expect: HashMap<String, Value>,
    /// Expected number of iterations (optional)
    #[serde(default)]
    pub expect_iterations: Option<usize>,
    /// Expected halt reason type (optional): "MaxIterations", "HaltCondition", "Convergence", "ChainFailure"
    #[serde(default)]
    pub expect_halt: Option<String>,
}

/// Result of testing a process definition
#[derive(Debug, Clone, Serialize)]
pub struct ProcessTestResult {
    pub process_name: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<SingleProcessTestResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SingleProcessTestResult {
    pub name: String,
    pub passed: bool,
    pub iterations: usize,
    pub halt_reason: String,
    pub expected: HashMap<String, Value>,
    pub actual: HashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mismatch: Option<String>,
}

impl ProcessDefinition {
    /// Load a process definition from a YAML file
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        serde_yaml::from_str(&content)
            .map_err(|e| format!("Parse error in {}: {e}", path.display()))
    }

    /// Resolve the micrograms directory relative to the process file
    pub fn resolve_mcg_dir(&self, process_file: &Path) -> std::path::PathBuf {
        process_file
            .parent()
            .unwrap_or(Path::new("."))
            .join(&self.micrograms_dir)
    }

    /// Build the effective step list: chain steps + optional governor
    fn effective_steps(&self) -> Vec<String> {
        let mut steps = self.steps.clone();
        if let Some(ref gov) = self.governor {
            steps.push(gov.clone());
        }
        steps
    }

    /// Run the process: resolve micrograms, wire governor, execute loop
    pub fn run(&self, micrograms: &[Microgram], input: HashMap<String, Value>) -> Result<LoopResult, String> {
        let effective = self.effective_steps();
        let mut ordered = Vec::with_capacity(effective.len());

        for step_name in &effective {
            match micrograms.iter().find(|mg| mg.name == *step_name) {
                Some(mg) => ordered.push(mg.clone()),
                None => return Err(format!("Microgram '{step_name}' not found")),
            }
        }

        // Determine halt field/value — from explicit config or governor convention
        let halt_field = self.halt_field.as_deref()
            .or_else(|| if self.governor.is_some() { Some("loop_action") } else { None });
        let halt_value_owned = self.halt_value.clone()
            .or_else(|| if self.governor.is_some() { Some(Value::String("HALT".to_string())) } else { None });
        let halt_value = halt_value_owned.as_ref();

        Ok(chain_loop(&ordered, input, self.max_iterations, halt_field, halt_value, false))
    }

    /// Run all process test cases
    pub fn test(&self, micrograms: &[Microgram]) -> ProcessTestResult {
        let mut results = Vec::with_capacity(self.tests.len());
        let mut passed = 0;

        for test in &self.tests {
            let loop_result = match self.run(micrograms, test.input.clone()) {
                Ok(r) => r,
                Err(e) => {
                    results.push(SingleProcessTestResult {
                        name: test.name.clone(),
                        passed: false,
                        iterations: 0,
                        halt_reason: "LoadError".to_string(),
                        expected: test.expect.clone(),
                        actual: HashMap::new(),
                        mismatch: Some(e),
                    });
                    continue;
                }
            };

            let halt_reason = match &loop_result.halt_reason {
                super::chain::LoopHalt::MaxIterations => "MaxIterations".to_string(),
                super::chain::LoopHalt::HaltCondition { field, .. } => format!("HaltCondition({field})"),
                super::chain::LoopHalt::Convergence { iteration } => format!("Convergence({iteration})"),
                super::chain::LoopHalt::ChainFailure { iteration, step } => format!("ChainFailure({iteration}, {step})"),
            };

            let mut mismatch = None;
            let mut test_passed = true;

            // Check expected output fields
            for (key, expected_val) in &test.expect {
                match loop_result.final_state.get(key) {
                    Some(actual_val) if actual_val == expected_val => {}
                    Some(actual_val) => {
                        mismatch = Some(format!("{key}: expected {expected_val:?}, got {actual_val:?}"));
                        test_passed = false;
                    }
                    None => {
                        mismatch = Some(format!("{key}: expected {expected_val:?}, not in output"));
                        test_passed = false;
                    }
                }
            }

            // Check expected iterations
            if let Some(expected_iters) = test.expect_iterations
                && loop_result.iterations != expected_iters {
                    mismatch = Some(format!(
                        "iterations: expected {expected_iters}, got {}", loop_result.iterations
                    ));
                    test_passed = false;
            }

            // Check expected halt reason type
            if let Some(ref expected_halt) = test.expect_halt {
                let halt_type = match &loop_result.halt_reason {
                    super::chain::LoopHalt::MaxIterations => "MaxIterations",
                    super::chain::LoopHalt::HaltCondition { .. } => "HaltCondition",
                    super::chain::LoopHalt::Convergence { .. } => "Convergence",
                    super::chain::LoopHalt::ChainFailure { .. } => "ChainFailure",
                };
                if halt_type != expected_halt.as_str() {
                    mismatch = Some(format!(
                        "halt_reason: expected {expected_halt}, got {halt_type}"
                    ));
                    test_passed = false;
                }
            }

            if test_passed { passed += 1; }

            results.push(SingleProcessTestResult {
                name: test.name.clone(),
                passed: test_passed,
                iterations: loop_result.iterations,
                halt_reason,
                expected: test.expect.clone(),
                actual: loop_result.final_state,
                mismatch,
            });
        }

        ProcessTestResult {
            process_name: self.name.clone(),
            total: self.tests.len(),
            passed,
            failed: self.tests.len() - passed,
            results,
        }
    }
}

/// Load all process definitions from a directory
pub fn load_processes(dir: &Path) -> Result<Vec<(ProcessDefinition, std::path::PathBuf)>, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read process dir: {e}"))?;

    let mut processes = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Dir entry error: {e}"))?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
            match ProcessDefinition::load(&path) {
                Ok(def) => processes.push((def, path)),
                Err(e) => eprintln!("Warning: skipping {}: {e}", path.display()),
            }
        }
    }

    processes.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    Ok(processes)
}

/// Test all process definitions in a directory
pub fn test_processes(processes_dir: &Path) -> Result<Vec<ProcessTestResult>, String> {
    let processes = load_processes(processes_dir)?;
    let mut results = Vec::with_capacity(processes.len());

    for (def, path) in &processes {
        let mcg_dir = def.resolve_mcg_dir(path);
        let micrograms = load_all(&mcg_dir)?;
        results.push(def.test(&micrograms));
    }

    Ok(results)
}

/// Test all chain definitions in a directory
pub fn test_chains(chains_dir: &Path) -> Result<Vec<ChainTestResult>, String> {
    let chains = load_chains(chains_dir)?;
    let mut results = Vec::with_capacity(chains.len());

    for (def, path) in &chains {
        let mcg_dir = def.resolve_mcg_dir(path);
        let micrograms = load_all(&mcg_dir)?;
        let mut result = def.test(&micrograms);

        // Primitive signature chain validation
        let chain_mgs: Vec<_> = def.steps.iter()
            .filter_map(|name| micrograms.iter().find(|m| m.name == *name))
            .cloned()
            .collect();
        if !chain_mgs.is_empty() {
            let sig_validation = super::signature_validator::validate_chain_signatures(
                &def.name, &chain_mgs
            );
            result.signature_validation = Some(sig_validation);
        }

        results.push(result);
    }

    Ok(results)
}
