//! # Microgram Module
//!
//! The smallest unit of executable program — a single decision tree
//! with built-in test cases, defined in one YAML file.
//!
//! ## Properties
//! - **Atomic**: one input → one decision → one output
//! - **Chainable**: output type matches next input type
//! - **Self-testing**: carries test cases inline
//! - **Sub-second**: executes in < 1ms

pub mod catalog;
pub mod chain;
pub mod clone;
pub mod compose;
pub mod contracts;
pub mod coverage;
pub mod diff;
pub mod evolve;
pub mod generate;
pub mod hygiene;
pub mod interface;
pub mod matrix;
pub mod merge;
pub mod pipe;
pub mod shrink;
pub mod snapshot;
pub mod stress;
pub mod patrol;
pub mod chain_registry;
pub mod signature_validator;

// Re-export all public items from submodules
pub use catalog::{
    AliasCheckResult, AliasConflict, AliasSuggestion,
    Catalog, CatalogEntry, alias_check, catalog,
};
pub use chain::{
    BoundaryError, BoundaryErrorSeverity, ChainEgressValidationResult, EgressFinding, EnginePrimitive,
    ChainResult, ChainStatus, ChainValidationResult, LoopHalt, LoopResult,
    PathMismatch, PathSnapshotResult, ResilientChainResult, StepValidationError,
    ValidatedChainResult,
    chain, chain_accumulate, chain_accumulate_by_names,
    chain_by_names, chain_loop, chain_loop_by_names,
    chain_resilient, chain_resilient_by_names,
    chain_validate_all, chain_validate_egress, chain_validated, chain_verify_paths,
};
pub use hygiene::{
    BoundaryReport, FieldGap, HygieneReport,
    check_chain_hygiene, check_chain_hygiene_by_names,
};
pub use clone::clone_mutated;
pub use compose::{
    AutoResult, BenchResult, CompositionGoal, CompositionPlan,
    auto_execute, bench_all, compose,
};
pub use contracts::{ContractValidation, ContractViolation, validate_contracts};
pub use coverage::{CoverageResult, coverage, coverage_all};
pub use diff::{DiffResult, diff};
pub use evolve::evolve_tests;
pub use generate::MicrogramSpec;
pub use matrix::{MatrixCell, MatrixResult, matrix};
pub use merge::merge;
pub use pipe::{PipeEntry, PipeResult, filter_results, map_field, pipe, pipe_chain, reduce_count};
pub use shrink::shrink;
pub use snapshot::{Snapshot, snapshot_restore, snapshot_save};
pub use stress::{
    BaselineEntry, RegressionEntry, RegressionResult, StressResult, ValidatedStressResult,
    check_regression, load_baseline, save_baseline,
    stress, stress_all, stress_typed, stress_all_typed, stress_validated,
};
pub use chain_registry::{
    ChainDefinition, ChainTestCase, ChainTestResult, SingleChainTestResult,
    ProcessDefinition, ProcessTestCase, ProcessTestResult, SingleProcessTestResult,
    load_chains, load_processes, test_chains, test_processes,
};
pub use patrol::{
    PatrolFinding, PatrolReport, PatrolVerdict, Ring, SymbolClass,
    run_patrol, run_patrol_default,
};

use crate::modules::decision_engine::{
    DecisionContext, DecisionEngine, DecisionTree, Value,
};
use interface::{infer_input_types, infer_output_types, input_variables_set, output_fields_set};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Typed field declaration for a microgram interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceField {
    #[serde(rename = "type")]
    pub field_type: String, // "bool", "int", "float", "string"
    #[serde(default)]
    pub required: bool,
}

/// Declared interface: typed inputs and outputs for a microgram
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MicrogramInterface {
    #[serde(default)]
    pub inputs: HashMap<String, InterfaceField>,
    #[serde(default)]
    pub outputs: HashMap<String, InterfaceField>,
    /// Field aliases: maps alternative names to canonical field names.
    /// Key = alias name, Value = canonical field name that exists in inputs or outputs.
    /// Used by catalog to discover connections across naming boundaries.
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

/// A microgram definition — one YAML file, one program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Microgram {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub tree: DecisionTree,
    #[serde(default)]
    pub tests: Vec<MicrogramTest>,
    #[serde(default)]
    pub interface: Option<MicrogramInterface>,
    /// T1 primitive signature — which Lex Primitiva this microgram embodies
    #[serde(default)]
    pub primitive_signature: Option<PrimitiveSignature>,
}

/// T1 Lex Primitiva signature for a microgram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveSignature {
    /// Dominant primitive (the one this microgram IS)
    pub dominant: String,
    /// Full primitive expression (e.g., "κ(ς(∅))")
    pub expression: String,
    /// Component primes (irreducible primitives present)
    #[serde(default)]
    pub primes: Vec<String>,
    /// Arguments the dominant operates on (typed Σ: what is being summed?)
    #[serde(default)]
    pub arguments: Vec<String>,
    /// Predicted signature when chained with another microgram
    #[serde(default)]
    pub chain_prediction: Option<String>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

/// A self-contained test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrogramTest {
    pub input: HashMap<String, Value>,
    pub expect: HashMap<String, Value>,
}

/// Result of running a microgram
#[derive(Debug, Clone, Serialize)]
pub struct MicrogramResult {
    pub name: String,
    pub success: bool,
    pub path: Vec<String>,
    pub output: HashMap<String, Value>,
    pub duration_us: u64,
}

/// Result of running a microgram with boundary validation.
/// Wraps a normal `MicrogramResult` with ingress/egress error lists.
/// When both error lists are empty and `result.success` is true,
/// the full intout pipeline passed: input schema → transform → output schema.
#[derive(Debug, Clone, Serialize)]
pub struct ValidatedResult {
    pub result: MicrogramResult,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ingress_errors: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub egress_errors: Vec<String>,
}

impl ValidatedResult {
    /// True when ingress passed, transform succeeded, and egress passed.
    pub fn is_valid(&self) -> bool {
        self.result.success && self.ingress_errors.is_empty() && self.egress_errors.is_empty()
    }
}

/// Result of running self-tests
#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    pub name: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<SingleTestResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SingleTestResult {
    pub index: usize,
    pub passed: bool,
    pub input: HashMap<String, Value>,
    pub expected: HashMap<String, Value>,
    pub actual: HashMap<String, Value>,
    pub path: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mismatch: Option<String>,
}

/// Canonicalize interface type names to the 5 recognized forms.
/// Handles common aliases: boolean→bool, integer/number→int (number→float
/// when the field clearly holds decimals, but at the type-compatibility
/// level number is treated as numeric — same as int/float coercion).
fn canonicalize_type(t: &str) -> &str {
    match t {
        "boolean" => "bool",
        "integer" => "int",
        "number" => "float",
        other => other,
    }
}

/// Check if an actual value type is compatible with a declared type.
/// Permits numeric coercion (int↔float) since the decision engine
/// routinely compares across numeric types. Canonicalizes type aliases
/// before comparison (boolean→bool, integer→int, number→float).
fn types_compatible(actual: &str, declared: &str) -> bool {
    let actual = canonicalize_type(actual);
    let declared = canonicalize_type(declared);
    if actual == declared {
        return true;
    }
    // Null is compatible with any declared type — a tree path may
    // legitimately return null for an optional field on no-match paths.
    if actual == "null" {
        return true;
    }
    // Numeric coercion: int and float are interchangeable
    if (actual == "int" || actual == "float") && (declared == "int" || declared == "float") {
        return true;
    }
    // "any" in the declared type means no constraint
    if declared == "any" {
        return true;
    }
    false
}

impl Microgram {
    /// Load from a YAML file
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        Self::parse(&content)
    }

    /// Parse from YAML string
    pub fn parse(yaml: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml).map_err(|e| format!("Parse error: {e}"))
    }

    /// Validate input against declared interface.
    /// Returns a list of validation errors (empty = valid).
    /// Checks both required-field presence and type compatibility.
    pub fn validate_input(&self, input: &HashMap<String, Value>) -> Vec<String> {
        let mut errors = Vec::new();
        let Some(ref iface) = self.interface else {
            return errors;
        };
        for (field_name, field_spec) in &iface.inputs {
            match input.get(field_name) {
                None | Some(Value::Null) => {
                    if field_spec.required {
                        errors.push(format!(
                            "Missing required input '{field_name}' (expected: {})",
                            field_spec.field_type
                        ));
                    }
                }
                Some(val) => {
                    let actual_type = interface::value_type_name(val);
                    if !types_compatible(actual_type, &field_spec.field_type) {
                        errors.push(format!(
                            "Input '{field_name}': expected type '{}', got '{actual_type}'",
                            field_spec.field_type
                        ));
                    }
                }
            }
        }
        errors
    }

    /// Validate output against declared interface.
    /// Returns a list of validation errors (empty = valid).
    /// Checks that declared output fields are present with correct types.
    pub fn validate_output(&self, output: &HashMap<String, Value>) -> Vec<String> {
        let mut errors = Vec::new();
        let Some(ref iface) = self.interface else {
            return errors;
        };
        for (field_name, field_spec) in &iface.outputs {
            match output.get(field_name) {
                None => {
                    if field_spec.required {
                        errors.push(format!(
                            "Missing required output '{field_name}' (expected: {})",
                            field_spec.field_type
                        ));
                    }
                }
                Some(val) => {
                    let actual_type = interface::value_type_name(val);
                    if !types_compatible(actual_type, &field_spec.field_type) {
                        errors.push(format!(
                            "Output '{field_name}': expected type '{}', got '{actual_type}'",
                            field_spec.field_type
                        ));
                    }
                }
            }
        }
        errors
    }

    /// Execute with given input variables.
    /// If `strict` validation is enabled and required fields are missing,
    /// returns a REJECTED result instead of silently processing.
    pub fn run_strict(&self, input: HashMap<String, Value>) -> MicrogramResult {
        let validation_errors = self.validate_input(&input);
        if !validation_errors.is_empty() {
            let mut output = HashMap::new();
            output.insert(
                "_error".to_string(),
                Value::String(format!("REJECTED: {}", validation_errors.join("; "))),
            );
            output.insert("_valid".to_string(), Value::Bool(false));
            return MicrogramResult {
                name: self.name.clone(),
                success: false,
                path: vec!["input_validation".to_string()],
                output,
                duration_us: 0,
            };
        }
        self.run(input)
    }

    /// Execute with full ingress and egress boundary validation.
    ///
    /// Validates input types and required fields against the declared interface
    /// before execution (ingress), then validates output types and required
    /// fields after execution (egress). If no interface is declared, behaves
    /// identically to `run()`.
    ///
    /// Returns a `ValidatedResult` containing the normal `MicrogramResult`
    /// plus any ingress/egress validation errors.
    pub fn run_validated(&self, input: HashMap<String, Value>) -> ValidatedResult {
        let ingress_errors = self.validate_input(&input);
        if !ingress_errors.is_empty() {
            let mut output = HashMap::new();
            output.insert(
                "_error".to_string(),
                Value::String(format!("Ingress: {}", ingress_errors.join("; "))),
            );
            output.insert("_valid".to_string(), Value::Bool(false));
            return ValidatedResult {
                result: MicrogramResult {
                    name: self.name.clone(),
                    success: false,
                    path: vec!["ingress_validation".to_string()],
                    output,
                    duration_us: 0,
                },
                ingress_errors,
                egress_errors: Vec::new(),
            };
        }

        let result = self.run(input);

        let egress_errors = if result.success {
            self.validate_output(&result.output)
        } else {
            // Don't validate output on execution errors — the tree failed,
            // not the boundary contract.
            Vec::new()
        };

        let success = result.success && egress_errors.is_empty();

        ValidatedResult {
            result: MicrogramResult {
                success,
                ..result
            },
            ingress_errors: Vec::new(),
            egress_errors,
        }
    }

    /// Execute with given input variables
    pub fn run(&self, input: HashMap<String, Value>) -> MicrogramResult {
        let start = std::time::Instant::now();
        let engine = DecisionEngine::new(self.tree.clone());
        let mut ctx = DecisionContext {
            variables: input,
            execution_path: Vec::new(),
        };

        let exec_result = engine.execute(&mut ctx);
        let duration_us = u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX);

        let output = match exec_result {
            crate::modules::decision_engine::ExecutionResult::Value(v) => {
                if let Value::Object(map) = v {
                    map
                } else {
                    let mut m = HashMap::new();
                    m.insert("_result".to_string(), v);
                    m
                }
            }
            crate::modules::decision_engine::ExecutionResult::Error(e) => {
                let mut m = HashMap::new();
                m.insert("_error".to_string(), Value::String(e));
                m
            }
            crate::modules::decision_engine::ExecutionResult::LlmRequest { prompt, .. } => {
                let mut m = HashMap::new();
                m.insert("_llm_fallback".to_string(), Value::String(prompt));
                m
            }
        };

        MicrogramResult {
            name: self.name.clone(),
            success: !output.contains_key("_error"),
            path: ctx.execution_path,
            output,
            duration_us,
        }
    }

    /// Run all self-tests
    pub fn test(&self) -> TestResult {
        let mut results = Vec::with_capacity(self.tests.len());
        let mut passed = 0;

        for (i, test) in self.tests.iter().enumerate() {
            let result = self.run(test.input.clone());

            // Check each expected field
            let mut mismatch = None;
            let mut test_passed = true;

            for (key, expected_val) in &test.expect {
                match result.output.get(key) {
                    Some(actual_val) if actual_val == expected_val => {}
                    Some(actual_val) => {
                        mismatch = Some(format!(
                            "{key}: expected {expected_val:?}, got {actual_val:?}"
                        ));
                        test_passed = false;
                    }
                    None => {
                        mismatch = Some(format!("{key}: expected {expected_val:?}, got nothing"));
                        test_passed = false;
                    }
                }
            }

            if test_passed {
                passed += 1;
            }

            results.push(SingleTestResult {
                index: i,
                passed: test_passed,
                input: test.input.clone(),
                expected: test.expect.clone(),
                actual: result.output,
                path: result.path,
                mismatch,
            });
        }

        TestResult {
            name: self.name.clone(),
            total: self.tests.len(),
            passed,
            failed: self.tests.len() - passed,
            results,
        }
    }

    /// Validate declared interface against actual tree structure.
    /// Returns a list of violations (empty = valid).
    pub fn validate_interface(&self) -> Vec<String> {
        let Some(iface) = &self.interface else {
            return vec![];
        };

        let mut violations = Vec::new();
        let actual_inputs = input_variables_set(self);
        let actual_outputs = output_fields_set(self);

        // Check: every declared input should be referenced by at least one condition
        for (name, field) in &iface.inputs {
            if field.required && !actual_inputs.contains(name.as_str()) {
                violations.push(format!(
                    "input '{name}' declared as required but not referenced in any condition node"
                ));
            }
        }

        // Check: every declared output should appear in at least one return node
        for name in iface.outputs.keys() {
            if !actual_outputs.contains(name.as_str()) {
                violations.push(format!(
                    "output '{name}' declared but not found in any return node"
                ));
            }
        }

        // Check: every actual output should be declared (if interface is present)
        for field in &actual_outputs {
            if !field.starts_with('_') && !iface.outputs.contains_key(*field) {
                violations.push(format!(
                    "return node produces '{field}' which is not declared in interface outputs"
                ));
            }
        }

        violations
    }

    /// Get the typed inputs — declared if available, inferred otherwise
    pub fn typed_inputs(&self) -> HashMap<String, String> {
        if let Some(iface) = &self.interface {
            iface
                .inputs
                .iter()
                .map(|(k, v)| (k.clone(), v.field_type.clone()))
                .collect()
        } else {
            // Fallback: infer from test cases
            infer_input_types(self)
        }
    }

    /// Get the typed outputs — declared if available, inferred otherwise
    pub fn typed_outputs(&self) -> HashMap<String, String> {
        if let Some(iface) = &self.interface {
            iface
                .outputs
                .iter()
                .map(|(k, v)| (k.clone(), v.field_type.clone()))
                .collect()
        } else {
            infer_output_types(self)
        }
    }
}

/// Load all micrograms from a directory, recursing into subdirectories.
pub fn load_all(dir: &Path) -> Result<Vec<Microgram>, String> {
    let mut micrograms = Vec::new();
    load_all_recursive(dir, &mut micrograms)?;
    micrograms.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(micrograms)
}

fn load_all_recursive(dir: &Path, out: &mut Vec<Microgram>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("Cannot read dir: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Dir entry error: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            load_all_recursive(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
            match Microgram::load(&path) {
                Ok(mg) => out.push(mg),
                Err(e) => eprintln!("Warning: skipping {}: {e}", path.display()),
            }
        }
    }

    Ok(())
}

/// Run all self-tests across all micrograms in a directory
pub fn test_all(dir: &Path) -> Result<Vec<TestResult>, String> {
    let micrograms = load_all(dir)?;
    Ok(micrograms.iter().map(|mg| mg.test()).collect())
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    const IS_POSITIVE: &str = r#"
name: is-positive
description: "Check if a number is positive"
tree:
  start: check
  nodes:
    check:
      type: condition
      variable: n
      operator: gt
      value: 0
      true_next: yes
      false_next: no
    yes:
      type: return
      value:
        positive: true
    no:
      type: return
      value:
        positive: false
tests:
  - input: { n: 5 }
    expect: { positive: true }
  - input: { n: -3 }
    expect: { positive: false }
  - input: { n: 0 }
    expect: { positive: false }
"#;

    #[test]
    fn test_parse_microgram() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        assert_eq!(mg.name, "is-positive");
        assert_eq!(mg.tests.len(), 3);
    }

    #[test]
    fn test_run_microgram() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        let mut input = HashMap::new();
        input.insert("n".to_string(), Value::Int(42));

        let result = mg.run(input);
        assert!(result.success);
        assert_eq!(result.output.get("positive"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_self_test_pass() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        let result = mg.test();
        assert_eq!(result.total, 3);
        assert_eq!(result.passed, 3);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_self_test_reports_failure() {
        let yaml = r#"
name: broken-test
tree:
  start: always-true
  nodes:
    always-true:
      type: return
      value:
        answer: true
tests:
  - input: {}
    expect: { answer: false }
"#;
        let mg = Microgram::parse(yaml).unwrap();
        let result = mg.test();
        assert_eq!(result.failed, 1);
        assert!(result.results[0].mismatch.is_some());
    }

    #[test]
    fn test_sub_microsecond_execution() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        let mut input = HashMap::new();
        input.insert("n".to_string(), Value::Int(1));

        let result = mg.run(input);
        // Should be well under 1000 microseconds (1ms)
        assert!(result.duration_us < 1000, "Took {}us", result.duration_us);
    }

    #[test]
    fn test_load_from_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.yaml"), IS_POSITIVE).unwrap();

        let micrograms = load_all(dir.path()).unwrap();
        assert_eq!(micrograms.len(), 1);
        assert_eq!(micrograms[0].name, "is-positive");
    }

    #[test]
    fn test_test_all() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.yaml"), IS_POSITIVE).unwrap();

        let results = test_all(dir.path()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].passed, 3);
    }

    const THRESHOLD_GATE: &str = r#"
name: threshold-gate
description: "Pass/fail gate"
tree:
  start: check
  nodes:
    check:
      type: condition
      variable: value
      operator: gte
      value: 80
      true_next: pass
      false_next: fail
    pass:
      type: return
      value:
        gate: "PASS"
        score: 100
    fail:
      type: return
      value:
        gate: "FAIL"
        score: 0
tests:
  - input: { value: 90 }
    expect: { gate: "PASS" }
"#;

    const SCORE_LABEL: &str = r#"
name: score-label
description: "Label a score"
tree:
  start: check
  nodes:
    check:
      type: condition
      variable: score
      operator: gte
      value: 50
      true_next: high
      false_next: low
    high:
      type: return
      value:
        label: "HIGH"
    low:
      type: return
      value:
        label: "LOW"
tests:
  - input: { score: 100 }
    expect: { label: "HIGH" }
  - input: { score: 0 }
    expect: { label: "LOW" }
"#;

    #[test]
    fn test_chain_two_micrograms() {
        let gate = Microgram::parse(THRESHOLD_GATE).unwrap();
        let label = Microgram::parse(SCORE_LABEL).unwrap();

        let mut input = HashMap::new();
        input.insert("value".to_string(), Value::Int(90));

        let result = chain(&[gate, label], input, false);

        assert!(result.success);
        assert_eq!(result.steps.len(), 2);
        // gate outputs score: 100 → label receives score: 100 → label: "HIGH"
        assert_eq!(
            result.final_output.get("label"),
            Some(&Value::String("HIGH".to_string()))
        );
    }

    #[test]
    fn test_chain_data_flows() {
        let gate = Microgram::parse(THRESHOLD_GATE).unwrap();
        let label = Microgram::parse(SCORE_LABEL).unwrap();

        // Below threshold → score: 0 → label: "LOW"
        let mut input = HashMap::new();
        input.insert("value".to_string(), Value::Int(50));

        let result = chain(&[gate, label], input, false);

        assert!(result.success);
        assert_eq!(
            result.final_output.get("label"),
            Some(&Value::String("LOW".to_string()))
        );
    }

    #[test]
    fn test_chain_by_names() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();
        std::fs::write(dir.path().join("label.yaml"), SCORE_LABEL).unwrap();

        let mut input = HashMap::new();
        input.insert("value".to_string(), Value::Int(95));

        let result =
            chain_by_names(dir.path(), &["threshold-gate", "score-label"], input).unwrap();

        assert!(result.success);
        assert_eq!(
            result.final_output.get("label"),
            Some(&Value::String("HIGH".to_string()))
        );
    }

    // ═════════════════════════════════════════════════════════════════════
    // T4: GENERATE tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_generate_microgram() {
        let spec = MicrogramSpec {
            name: "age-check".to_string(),
            description: "Check if age >= 18".to_string(),
            variable: "age".to_string(),
            operator: "gte".to_string(),
            threshold: Value::Int(18),
            true_label: "adult".to_string(),
            true_value: Value::Bool(true),
            false_label: "adult".to_string(),
            false_value: Value::Bool(false),
        };

        let mg = spec.build();
        assert_eq!(mg.name, "age-check");
        assert!(!mg.tests.is_empty());

        // Run the generated microgram
        let mut input = HashMap::new();
        input.insert("age".to_string(), Value::Int(21));
        let result = mg.run(input);
        assert!(result.success);
        assert_eq!(result.output.get("adult"), Some(&Value::Bool(true)));

        let mut input2 = HashMap::new();
        input2.insert("age".to_string(), Value::Int(15));
        let result2 = mg.run(input2);
        assert_eq!(result2.output.get("adult"), Some(&Value::Bool(false)));
    }

    #[test]
    fn test_generate_self_tests_pass() {
        let spec = MicrogramSpec {
            name: "temp-check".to_string(),
            description: "Check if temperature > 100".to_string(),
            variable: "temp".to_string(),
            operator: "gt".to_string(),
            threshold: Value::Int(100),
            true_label: "fever".to_string(),
            true_value: Value::Bool(true),
            false_label: "fever".to_string(),
            false_value: Value::Bool(false),
        };

        let mg = spec.build();
        let test_result = mg.test();
        // All auto-generated tests should pass (they're derived from the same logic)
        assert_eq!(test_result.failed, 0, "Generated tests should all pass: {:?}", test_result.results);
        assert!(test_result.total >= 3, "Should generate at least 3 boundary tests");
    }

    #[test]
    fn test_generate_to_yaml() {
        let spec = MicrogramSpec {
            name: "score-gate".to_string(),
            description: "Gate on score".to_string(),
            variable: "score".to_string(),
            operator: "gte".to_string(),
            threshold: Value::Int(80),
            true_label: "pass".to_string(),
            true_value: Value::Bool(true),
            false_label: "pass".to_string(),
            false_value: Value::Bool(false),
        };

        let yaml = spec.to_yaml().unwrap();
        // Roundtrip: parse the generated YAML back
        let mg = Microgram::parse(&yaml).unwrap();
        assert_eq!(mg.name, "score-gate");

        // Self-tests still pass after roundtrip
        let test_result = mg.test();
        assert_eq!(test_result.failed, 0);
    }

    // ═════════════════════════════════════════════════════════════════════
    // T4: EVOLVE tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_evolve_adds_boundary_tests() {
        let mg = Microgram::parse(THRESHOLD_GATE).unwrap();
        // threshold-gate has 1 test (value: 90). Evolve should add boundary cases.
        let new_tests = evolve_tests(&mg);
        assert!(!new_tests.is_empty(), "Should suggest new tests");

        // Verify all suggested tests produce valid output
        for test in &new_tests {
            let result = mg.run(test.input.clone());
            assert!(result.success, "Evolved test should run without error");
        }
    }

    #[test]
    fn test_evolve_suggests_zero_and_negative() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        // is-positive already tests 5, -3, 0 — evolve should add boundary around 0
        let new_tests = evolve_tests(&mg);

        // Should suggest at least the boundary ±1 around threshold (0)
        let has_minus_one = new_tests.iter().any(|t| {
            t.input.get("n") == Some(&Value::Int(-1))
        });
        let has_plus_one = new_tests.iter().any(|t| {
            t.input.get("n") == Some(&Value::Int(1))
        });
        // -1 is already covered by -3 test (same branch), but 1 might be new
        assert!(has_minus_one || has_plus_one, "Should suggest boundary values near threshold");
    }

    // ═════════════════════════════════════════════════════════════════════
    // T4: COMPOSE tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_compose_finds_chain() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();
        std::fs::write(dir.path().join("label.yaml"), SCORE_LABEL).unwrap();

        let goal = CompositionGoal {
            required_outputs: vec!["label".to_string()],
            initial_input: {
                let mut m = HashMap::new();
                m.insert("value".to_string(), Value::Int(90));
                m
            },
        };

        let plan = compose(dir.path(), &goal).unwrap();
        assert!(plan.feasible, "Should find a feasible chain");
        assert!(!plan.chain.is_empty());
        assert!(plan.missing.is_empty());
        assert!(plan.coverage.contains(&"label".to_string()));
    }

    #[test]
    fn test_compose_reports_infeasible() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();

        let goal = CompositionGoal {
            required_outputs: vec!["nonexistent_field".to_string()],
            initial_input: {
                let mut m = HashMap::new();
                m.insert("value".to_string(), Value::Int(90));
                m
            },
        };

        let plan = compose(dir.path(), &goal).unwrap();
        assert!(!plan.feasible, "Should report infeasible for unreachable output");
        assert!(plan.missing.contains(&"nonexistent_field".to_string()));
    }

    #[test]
    fn test_compose_multi_step() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();
        std::fs::write(dir.path().join("label.yaml"), SCORE_LABEL).unwrap();
        std::fs::write(dir.path().join("positive.yaml"), IS_POSITIVE).unwrap();

        // Goal: need both "label" and "gate" — requires threshold-gate (produces gate+score)
        // then score-label (consumes score, produces label)
        let goal = CompositionGoal {
            required_outputs: vec!["label".to_string(), "gate".to_string()],
            initial_input: {
                let mut m = HashMap::new();
                m.insert("value".to_string(), Value::Int(90));
                m
            },
        };

        let plan = compose(dir.path(), &goal).unwrap();
        assert!(plan.feasible);
        assert_eq!(plan.coverage.len(), 2);
    }

    // ═════════════════════════════════════════════════════════════════════
    // T5: BENCH tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_bench_all() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();
        std::fs::write(dir.path().join("positive.yaml"), IS_POSITIVE).unwrap();

        let results = bench_all(dir.path(), 100).unwrap();
        assert_eq!(results.len(), 2);

        for r in &results {
            assert_eq!(r.iterations, 100);
            #[allow(clippy::as_conversions, clippy::cast_possible_truncation, clippy::cast_sign_loss)] // f64→u64 for test assertion comparison
            let avg_as_u64 = r.avg_us as u64;
            assert!(r.min_us <= avg_as_u64 + 1);
            #[allow(clippy::as_conversions)] // u64→f64 for test assertion comparison
            let max_as_f64 = r.max_us as f64;
            assert!(r.avg_us <= max_as_f64 + 1.0);
            assert!(r.p95_us <= r.max_us);
            assert!(r.tests_pass);
            // Sub-millisecond: p95 should be under 1000us
            assert!(r.p95_us < 1000, "{} p95={}us", r.name, r.p95_us);
        }
    }

    // ═════════════════════════════════════════════════════════════════════
    // T5: AUTO tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_auto_execute_feasible() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();
        std::fs::write(dir.path().join("label.yaml"), SCORE_LABEL).unwrap();

        let goal = CompositionGoal {
            required_outputs: vec!["label".to_string()],
            initial_input: {
                let mut m = HashMap::new();
                m.insert("value".to_string(), Value::Int(90));
                m
            },
        };

        let result = auto_execute(dir.path(), &goal).unwrap();
        assert!(result.plan.feasible);
        assert!(result.execution.is_some());

        let exec = result.execution.unwrap();
        assert!(exec.success);
        assert!(exec.final_output.contains_key("label"));
        assert!(result.duration_us < 10_000, "Auto should complete in <10ms, took {}us", result.duration_us);
    }

    #[test]
    fn test_auto_execute_infeasible() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();

        let goal = CompositionGoal {
            required_outputs: vec!["impossible_field".to_string()],
            initial_input: HashMap::new(),
        };

        let result = auto_execute(dir.path(), &goal).unwrap();
        assert!(!result.plan.feasible);
        assert!(result.execution.is_none());
        assert!(!result.verified);
    }

    #[test]
    fn test_auto_full_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();
        std::fs::write(dir.path().join("label.yaml"), SCORE_LABEL).unwrap();
        std::fs::write(dir.path().join("positive.yaml"), IS_POSITIVE).unwrap();

        // Goal: produce "label" from value input
        // Expected: threshold-gate(value:95) → {gate:"PASS", score:100} → score-label(score:100) → {label:"HIGH"}
        let goal = CompositionGoal {
            required_outputs: vec!["label".to_string()],
            initial_input: {
                let mut m = HashMap::new();
                m.insert("value".to_string(), Value::Int(95));
                m
            },
        };

        let result = auto_execute(dir.path(), &goal).unwrap();
        assert!(result.plan.feasible);
        let exec = result.execution.unwrap();
        assert!(exec.success);
        assert_eq!(
            exec.final_output.get("label"),
            Some(&Value::String("HIGH".to_string()))
        );
    }

    // ═════════════════════════════════════════════════════════════════════
    // T5: CATALOG tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_catalog() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();
        std::fs::write(dir.path().join("label.yaml"), SCORE_LABEL).unwrap();
        std::fs::write(dir.path().join("positive.yaml"), IS_POSITIVE).unwrap();

        let cat = catalog(dir.path()).unwrap();
        assert_eq!(cat.total_micrograms, 3);
        assert!(cat.all_pass);
        assert!(cat.total_tests > 0);

        // threshold-gate outputs "score" → score-label inputs "score"
        let has_gate_to_label = cat.connections.iter().any(|(a, b)| {
            a == "threshold-gate" && b == "score-label"
        });
        assert!(has_gate_to_label, "Should detect gate→label connection via 'score'");
    }

    #[test]
    fn test_catalog_connection_graph() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();
        std::fs::write(dir.path().join("label.yaml"), SCORE_LABEL).unwrap();

        let cat = catalog(dir.path()).unwrap();

        // threshold-gate outputs: gate, score → score-label needs: score → connection exists
        // score-label outputs: label → threshold-gate needs: value → no connection
        assert!(!cat.connections.iter().any(|(a, _)| a == "score-label"),
            "score-label should not connect to anything (label doesn't feed value)");
    }

    // ═════════════════════════════════════════════════════════════════════
    // T6: DIFF tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_diff_same_inputs() {
        // is-positive and is-even both take "n"
        let a = Microgram::parse(IS_POSITIVE).unwrap();
        let b = Microgram::parse(r#"
name: is-even
description: "Check if even"
tree:
  start: check
  nodes:
    check:
      type: condition
      variable: n
      operator: eq
      value: 0
      true_next: yes
      false_next: no
    yes:
      type: return
      value:
        even: true
    no:
      type: return
      value:
        even: false
tests:
  - input: { n: 0 }
    expect: { even: true }
  - input: { n: 5 }
    expect: { even: false }
"#).unwrap();

        let d = diff(&a, &b);
        assert!(d.same_inputs, "Both take 'n'");
        assert!(!d.same_outputs, "Different output fields");
        assert_eq!(d.shared_inputs, vec!["n"]);
        assert!(!d.compatible, "positive doesn't feed into n");
    }

    #[test]
    fn test_diff_compatible_pair() {
        let gate = Microgram::parse(THRESHOLD_GATE).unwrap();
        let label = Microgram::parse(SCORE_LABEL).unwrap();

        let d = diff(&gate, &label);
        assert!(d.compatible, "gate outputs 'score', label inputs 'score'");
        assert!(!d.same_inputs);
        assert!(!d.same_outputs);
    }

    #[test]
    fn test_diff_behavioral_overlap() {
        // Two micrograms with identical test input {n: 5}
        let a = Microgram::parse(IS_POSITIVE).unwrap();
        let b = Microgram::parse(r#"
name: is-big
tree:
  start: check
  nodes:
    check:
      type: condition
      variable: n
      operator: gt
      value: 10
      true_next: yes
      false_next: no
    yes:
      type: return
      value: { big: true }
    no:
      type: return
      value: { big: false }
tests:
  - input: { n: 5 }
    expect: { big: false }
"#).unwrap();

        let d = diff(&a, &b);
        assert_eq!(d.test_overlap, 1, "Both test n=5");
        assert_eq!(d.behavior_diverges, 1, "Different outputs for n=5");
    }

    // ═════════════════════════════════════════════════════════════════════
    // T6: MERGE tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_merge_dispatch() {
        let pos = Microgram::parse(IS_POSITIVE).unwrap();
        let gate = Microgram::parse(THRESHOLD_GATE).unwrap();

        let merged = merge(&pos, &gate, "pos-or-gate", "Dispatch by input");

        // Should have dispatch + all nodes from both
        assert!(merged.tree.nodes.contains_key("dispatch"));
        assert!(merged.tree.nodes.contains_key("a_check"));
        assert!(merged.tree.nodes.contains_key("b_check"));

        // With n=5 → routes to is-positive path
        let mut input = HashMap::new();
        input.insert("n".to_string(), Value::Int(5));
        let result = merged.run(input);
        assert!(result.success);
        assert_eq!(result.output.get("positive"), Some(&Value::Bool(true)));

        // With value=90 (no n) → routes to threshold-gate path
        let mut input2 = HashMap::new();
        input2.insert("value".to_string(), Value::Int(90));
        let result2 = merged.run(input2);
        assert!(result2.success);
        assert_eq!(result2.output.get("gate"), Some(&Value::String("PASS".to_string())));
    }

    #[test]
    fn test_merge_preserves_tests() {
        let pos = Microgram::parse(IS_POSITIVE).unwrap();
        let gate = Microgram::parse(THRESHOLD_GATE).unwrap();
        let pos_tests = pos.tests.len();
        let gate_tests = gate.tests.len();

        let merged = merge(&pos, &gate, "merged", "test");
        assert_eq!(merged.tests.len(), pos_tests + gate_tests);
    }

    // ═════════════════════════════════════════════════════════════════════
    // T6: PIPE tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_pipe_single() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        let inputs = vec![
            { let mut m = HashMap::new(); m.insert("n".to_string(), Value::Int(5)); m },
            { let mut m = HashMap::new(); m.insert("n".to_string(), Value::Int(-3)); m },
            { let mut m = HashMap::new(); m.insert("n".to_string(), Value::Int(0)); m },
        ];

        let result = pipe(&mg, &inputs);
        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 3);
        assert_eq!(result.failed, 0);
        assert_eq!(result.results[0].output.get("positive"), Some(&Value::Bool(true)));
        assert_eq!(result.results[1].output.get("positive"), Some(&Value::Bool(false)));
    }

    #[test]
    fn test_pipe_chain() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();
        std::fs::write(dir.path().join("label.yaml"), SCORE_LABEL).unwrap();

        let inputs = vec![
            { let mut m = HashMap::new(); m.insert("value".to_string(), Value::Int(90)); m },
            { let mut m = HashMap::new(); m.insert("value".to_string(), Value::Int(50)); m },
            { let mut m = HashMap::new(); m.insert("value".to_string(), Value::Int(100)); m },
        ];

        let result = pipe_chain(dir.path(), &["threshold-gate", "score-label"], &inputs).unwrap();
        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 3);

        // value:90 → PASS(score:100) → HIGH
        assert_eq!(result.results[0].output.get("label"), Some(&Value::String("HIGH".to_string())));
        // value:50 → FAIL(score:0) → LOW
        assert_eq!(result.results[1].output.get("label"), Some(&Value::String("LOW".to_string())));
        // value:100 → PASS(score:100) → HIGH
        assert_eq!(result.results[2].output.get("label"), Some(&Value::String("HIGH".to_string())));
    }

    #[test]
    fn test_pipe_performance() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        let inputs: Vec<HashMap<String, Value>> = (0..1000)
            .map(|i| {
                let mut m = HashMap::new();
                m.insert("n".to_string(), Value::Int(i - 500));
                m
            })
            .collect();

        let result = pipe(&mg, &inputs);
        assert_eq!(result.total, 1000);
        assert_eq!(result.succeeded, 1000);
        // 1000 executions should complete in under 10ms
        assert!(result.total_duration_us < 10_000, "1000 pipes took {}us", result.total_duration_us);
    }

    // ═════════════════════════════════════════════════════════════════════
    // T7: SNAPSHOT tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_snapshot_save_restore() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();
        std::fs::write(dir.path().join("pos.yaml"), IS_POSITIVE).unwrap();

        let snap_file = dir.path().join("snapshot.json");
        let snap = snapshot_save(dir.path(), &snap_file).unwrap();
        assert_eq!(snap.micrograms.len(), 2);
        assert!(snap.all_pass);

        // Restore to a new directory
        let restore_dir = tempfile::tempdir().unwrap();
        let count = snapshot_restore(&snap_file, restore_dir.path()).unwrap();
        assert_eq!(count, 2);

        // Verify restored micrograms work
        let restored = load_all(restore_dir.path()).unwrap();
        assert_eq!(restored.len(), 2);
        for mg in &restored {
            let r = mg.test();
            assert_eq!(r.failed, 0);
        }
    }

    // ═════════════════════════════════════════════════════════════════════
    // T7: STRESS tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_stress_single() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        let result = stress(&mg, 10_000, 42);
        assert_eq!(result.iterations, 10_000);
        assert_eq!(result.succeeded, 10_000); // decision trees never error on valid types
        assert_eq!(result.errored, 0);
        assert!(result.avg_us < 100.0, "avg={}us", result.avg_us);
    }

    #[test]
    fn test_stress_all() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();
        std::fs::write(dir.path().join("pos.yaml"), IS_POSITIVE).unwrap();

        let results = stress_all(dir.path(), 1000, 99).unwrap();
        assert_eq!(results.len(), 2);
        for r in &results {
            assert_eq!(r.iterations, 1000);
            assert_eq!(r.errored, 0);
        }
    }

    // ═════════════════════════════════════════════════════════════════════
    // T7b: CTVP stress / chain validation tests
    // ═════════════════════════════════════════════════════════════════════

    const TYPED_MG: &str = r#"
name: typed-gate
description: "Typed threshold gate for CTVP tests"
interface:
  inputs:
    score:
      type: float
  outputs:
    result:
      type: string
tree:
  start: check
  nodes:
    check:
      type: condition
      variable: score
      operator: gte
      value: 5.0
      true_next: pass
      false_next: fail
    pass:
      type: return
      value:
        result: "above"
    fail:
      type: return
      value:
        result: "below"
tests:
  - input: { score: 8.0 }
    expect: { result: "above" }
  - input: { score: 2.0 }
    expect: { result: "below" }
  - input: {}
    expect: { result: "below" }
"#;

    #[test]
    fn test_stress_typed() {
        let mg = Microgram::parse(TYPED_MG).unwrap();
        let result = stress_typed(&mg, 1000, 42);
        assert_eq!(result.iterations, 1000);
        assert_eq!(result.succeeded + result.errored, 1000);
        assert!(result.avg_us < 100.0, "avg={}us", result.avg_us);
    }

    #[test]
    fn test_stress_typed_falls_back_without_interface() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        assert!(mg.interface.is_none());
        let result = stress_typed(&mg, 500, 42);
        assert_eq!(result.iterations, 500);
        assert_eq!(result.errored, 0);
    }

    #[test]
    fn test_stress_validated() {
        let mg = Microgram::parse(TYPED_MG).unwrap();
        let result = stress_validated(&mg, 500, 42);
        assert_eq!(result.base.iterations, 500);
        assert_eq!(result.base.succeeded + result.base.errored, 500);
        // Boundary validation should catch type mismatches
        assert!(result.ingress_failures + result.egress_failures <= 500);
    }

    #[test]
    fn test_chain_validated() {
        let mg1 = Microgram::parse(TYPED_MG).unwrap();
        let mg2 = Microgram::parse(IS_POSITIVE).unwrap();
        let mut input = HashMap::new();
        input.insert("score".to_string(), Value::Float(8.0));
        input.insert("threshold".to_string(), Value::Float(5.0));

        let result = chain_validated(&[mg1, mg2], input, true);
        assert_eq!(result.steps.len(), 2);
        assert!(result.total_duration_us > 0);
    }

    #[test]
    fn test_baseline_save_load_regression() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        let results = vec![stress(&mg, 100, 42)];

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("baseline.json");

        save_baseline(&results, &path).unwrap();
        let loaded = load_baseline(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "is-positive");
        assert_eq!(loaded[0].iterations, 100);

        // Same results should show no regression
        let reg = check_regression(&results, &loaded, 50.0);
        assert_eq!(reg.total_checked, 1);
        assert!(reg.regressions.is_empty());
    }

    // ═════════════════════════════════════════════════════════════════════
    // T7: TRANSFORM tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_filter_results() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        let inputs = vec![
            { let mut m = HashMap::new(); m.insert("n".to_string(), Value::Int(5)); m },
            { let mut m = HashMap::new(); m.insert("n".to_string(), Value::Int(-3)); m },
            { let mut m = HashMap::new(); m.insert("n".to_string(), Value::Int(10)); m },
        ];
        let pipe_result = pipe(&mg, &inputs);

        // Filter: only positive=true
        let filtered = filter_results(&pipe_result, "positive", "eq", &Value::Bool(true));
        assert_eq!(filtered.total, 2); // n=5 and n=10
    }

    #[test]
    fn test_map_field() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        let inputs = vec![
            { let mut m = HashMap::new(); m.insert("n".to_string(), Value::Int(1)); m },
            { let mut m = HashMap::new(); m.insert("n".to_string(), Value::Int(-1)); m },
        ];
        let pipe_result = pipe(&mg, &inputs);

        let values = map_field(&pipe_result, "positive");
        assert_eq!(values, vec![Some(Value::Bool(true)), Some(Value::Bool(false))]);
    }

    #[test]
    fn test_reduce_count() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        let inputs: Vec<HashMap<String, Value>> = (-3..=3)
            .map(|i| { let mut m = HashMap::new(); m.insert("n".to_string(), Value::Int(i)); m })
            .collect();
        let pipe_result = pipe(&mg, &inputs);

        let counts = reduce_count(&pipe_result, "positive");
        // -3,-2,-1,0 → false (4), 1,2,3 → true (3)
        assert_eq!(counts.get("Bool(true)"), Some(&3));
        assert_eq!(counts.get("Bool(false)"), Some(&4));
    }

    // ═════════════════════════════════════════════════════════════════════
    // T8: MATRIX tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_matrix_self_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();
        std::fs::write(dir.path().join("pos.yaml"), IS_POSITIVE).unwrap();

        let result = matrix(dir.path()).unwrap();
        // 2 micrograms × 2 donors = 4 cells
        assert_eq!(result.cells.len(), 4);

        // Self-matches: each microgram should match its own tests
        let self_gate = result.cells.iter().find(|c| c.runner == "threshold-gate" && c.test_from == "threshold-gate").unwrap();
        assert_eq!(self_gate.matched, self_gate.total);

        let self_pos = result.cells.iter().find(|c| c.runner == "is-positive" && c.test_from == "is-positive").unwrap();
        assert_eq!(self_pos.matched, self_pos.total);
    }

    // ═════════════════════════════════════════════════════════════════════
    // T8: COVERAGE tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_coverage_full() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        let cov = coverage(&mg);
        // is-positive has 3 nodes (check, yes, no) and 3 tests covering all paths
        assert_eq!(cov.total_nodes, 3);
        assert_eq!(cov.covered_nodes, 3);
        assert!((cov.coverage_pct - 100.0).abs() < 0.01);
        assert!(cov.uncovered.is_empty());
    }

    #[test]
    fn test_coverage_partial() {
        // Microgram with only one test — won't cover all nodes
        let yaml = r#"
name: partial
tree:
  start: check
  nodes:
    check:
      type: condition
      variable: x
      operator: gt
      value: 0
      true_next: yes
      false_next: no
    yes:
      type: return
      value: { result: true }
    no:
      type: return
      value: { result: false }
tests:
  - input: { x: 5 }
    expect: { result: true }
"#;
        let mg = Microgram::parse(yaml).unwrap();
        let cov = coverage(&mg);
        assert_eq!(cov.total_nodes, 3);
        assert_eq!(cov.covered_nodes, 2); // check + yes, not no
        assert!(cov.coverage_pct < 100.0);
        assert!(cov.uncovered.contains(&"no".to_string()));
    }

    #[test]
    fn test_coverage_all_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate.yaml"), THRESHOLD_GATE).unwrap();
        std::fs::write(dir.path().join("pos.yaml"), IS_POSITIVE).unwrap();

        let results = coverage_all(dir.path()).unwrap();
        assert_eq!(results.len(), 2);
        // is-positive has full coverage, threshold-gate has 1 test → partial
        let pos = results.iter().find(|r| r.name == "is-positive").unwrap();
        assert_eq!(pos.coverage_pct, 100.0);
    }

    // ═════════════════════════════════════════════════════════════════════
    // T9: CLONE tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_clone_mutated() {
        let mg = Microgram::parse(THRESHOLD_GATE).unwrap();
        // Original threshold: 80. Clone with +20 → threshold becomes 100.
        let mutant = clone_mutated(&mg, "strict-gate", 20);

        assert_eq!(mutant.name, "strict-gate");

        // value=90 was PASS with threshold 80, should be FAIL with threshold 100
        let mut input = HashMap::new();
        input.insert("value".to_string(), Value::Int(90));
        let result = mutant.run(input);
        assert_eq!(result.output.get("gate"), Some(&Value::String("FAIL".to_string())));

        // value=100 should PASS with threshold 100
        let mut input2 = HashMap::new();
        input2.insert("value".to_string(), Value::Int(100));
        let result2 = mutant.run(input2);
        assert_eq!(result2.output.get("gate"), Some(&Value::String("PASS".to_string())));

        // Auto-generated tests should pass (regenerated from mutant logic)
        let test_result = mutant.test();
        assert_eq!(test_result.failed, 0);
    }

    // ═════════════════════════════════════════════════════════════════════
    // T9: SHRINK tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_shrink_finds_boundary() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();

        // Input n=1000 → positive:true. Shrink should find n=1 (minimal positive).
        let mut big_input = HashMap::new();
        big_input.insert("n".to_string(), Value::Int(1000));

        let minimal = shrink(&mg, &big_input);
        let min_n = match minimal.get("n") {
            Some(Value::Int(n)) => *n,
            _ => panic!("expected int"),
        };
        // Minimal n that produces positive:true with operator gt 0 is 1
        assert_eq!(min_n, 1, "Should shrink to boundary value 1");

        // Verify the shrunk input produces same output
        let original = mg.run(big_input);
        let shrunk = mg.run(minimal);
        assert_eq!(original.output, shrunk.output);
    }

    #[test]
    fn test_shrink_negative() {
        let mg = Microgram::parse(IS_POSITIVE).unwrap();

        // Input n=-500 → positive:false. Shrink toward 0.
        let mut neg_input = HashMap::new();
        neg_input.insert("n".to_string(), Value::Int(-500));

        let minimal = shrink(&mg, &neg_input);
        let min_n = match minimal.get("n") {
            Some(Value::Int(n)) => *n,
            _ => panic!("expected int"),
        };
        // Minimal n that still produces positive:false is 0 (gt 0 → false at 0)
        assert_eq!(min_n, 0, "Should shrink to boundary value 0");
    }

    // ═════════════════════════════════════════════════════════════════════
    // N3: END-TO-END ALIAS CHAIN ACCEPTANCE TEST
    // Proves the alias system works at RUNTIME, not just discovery time.
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_chain_alias_remapping() {
        // Source: outputs "valid_icsr" (bool)
        let source_yaml = r#"
name: e2a-source
description: "Outputs valid_icsr"
interface:
  inputs:
    reporter:
      type: bool
  outputs:
    valid_icsr:
      type: bool
tree:
  start: check
  nodes:
    check:
      type: condition
      variable: reporter
      operator: eq
      value: true
      true_next: valid
      false_next: invalid
    valid:
      type: return
      value:
        valid_icsr: true
    invalid:
      type: return
      value:
        valid_icsr: false
tests:
  - input: { reporter: true }
    expect: { valid_icsr: true }
"#;

        // Consumer: expects "valid" (bool), declares alias valid_icsr → valid
        let consumer_yaml = r#"
name: intake-consumer
description: "Consumes valid via alias"
interface:
  inputs:
    valid:
      type: bool
  aliases:
    valid_icsr: valid
  outputs:
    route:
      type: string
tree:
  start: check_valid
  nodes:
    check_valid:
      type: condition
      variable: valid
      operator: eq
      value: true
      true_next: triage
      false_next: reject
    triage:
      type: return
      value:
        route: TRIAGE
    reject:
      type: return
      value:
        route: REJECT
tests:
  - input: { valid: true }
    expect: { route: TRIAGE }
  - input: { valid: false }
    expect: { route: REJECT }
"#;

        let source = Microgram::parse(source_yaml).unwrap();
        let consumer = Microgram::parse(consumer_yaml).unwrap();

        // Chain: source → consumer. Source outputs {valid_icsr: true}.
        // Without alias remapping: consumer checks "valid", finds nothing, goes to REJECT.
        // With alias remapping: chain copies valid_icsr→valid, consumer finds valid=true → TRIAGE.
        let mut input = HashMap::new();
        input.insert("reporter".to_string(), Value::Bool(true));

        let result = chain::chain(&[source, consumer], input, false);
        assert!(result.success, "Chain should succeed");
        assert_eq!(
            result.final_output.get("route"),
            Some(&Value::String("TRIAGE".to_string())),
            "Alias remapping should make consumer see valid=true → TRIAGE, got: {:?}",
            result.final_output
        );
    }

    #[test]
    fn test_chain_alias_no_overwrite() {
        // Verify aliases don't overwrite existing values
        let consumer_yaml = r#"
name: no-overwrite
description: "Has alias but also direct input"
interface:
  inputs:
    score:
      type: int
  aliases:
    raw_score: score
  outputs:
    result:
      type: string
tree:
  start: check
  nodes:
    check:
      type: condition
      variable: score
      operator: gt
      value: 50
      true_next: high
      false_next: low
    high:
      type: return
      value:
        result: HIGH
    low:
      type: return
      value:
        result: LOW
tests:
  - input: { score: 75 }
    expect: { result: HIGH }
"#;

        let consumer = Microgram::parse(consumer_yaml).unwrap();

        // Input has BOTH raw_score and score. Alias should NOT overwrite score.
        let mut input = HashMap::new();
        input.insert("raw_score".to_string(), Value::Int(25));  // alias → low
        input.insert("score".to_string(), Value::Int(75));       // direct → high

        let result = chain::chain(&[consumer], input, false);
        assert!(result.success);
        assert_eq!(
            result.final_output.get("result"),
            Some(&Value::String("HIGH".to_string())),
            "Direct value should win over alias — score=75 not raw_score=25"
        );
    }

    #[test]
    fn test_compose_alias_expansion() {
        // Source produces valid_icsr. Consumer needs valid (via alias).
        // Compose should include consumer in the chain.
        let source_yaml = r#"
name: source-mg
description: "produces valid_icsr"
interface:
  outputs:
    valid_icsr:
      type: bool
tree:
  start: ret
  nodes:
    ret:
      type: return
      value:
        valid_icsr: true
tests:
  - input: {}
    expect: { valid_icsr: true }
"#;

        let consumer_yaml = r#"
name: consumer-mg
description: "consumes valid via alias, produces route"
interface:
  inputs:
    valid:
      type: bool
  aliases:
    valid_icsr: valid
  outputs:
    route:
      type: string
tree:
  start: check
  nodes:
    check:
      type: condition
      variable: valid
      operator: eq
      value: true
      true_next: yes
      false_next: no
    yes:
      type: return
      value:
        route: TRIAGE
    no:
      type: return
      value:
        route: REJECT
tests:
  - input: { valid: true }
    expect: { route: TRIAGE }
"#;

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("source.yaml"), source_yaml).unwrap();
        std::fs::write(dir.path().join("consumer.yaml"), consumer_yaml).unwrap();

        let goal = compose::CompositionGoal {
            required_outputs: vec!["route".to_string()],
            initial_input: HashMap::new(),
        };

        let plan = compose::compose(dir.path(), &goal).unwrap();
        assert!(plan.feasible, "Should compose a feasible chain via alias: {:?}", plan);
        assert!(plan.chain.contains(&"source-mg".to_string()), "Chain should include source");
        assert!(plan.chain.contains(&"consumer-mg".to_string()), "Chain should include consumer");
    }

    // ═══════════════════════════════════════════════════════════════════
    // Application 1: Path Snapshot Testing
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_path_snapshot_pass() {
        let gate = Microgram::parse(THRESHOLD_GATE).unwrap();
        let label = Microgram::parse(SCORE_LABEL).unwrap();

        let mut input = HashMap::new();
        input.insert("value".to_string(), Value::Int(90));

        let expected_paths = vec![
            vec!["check".to_string(), "pass".to_string()],        // gate: 90 >= 80 → pass
            vec!["check".to_string(), "high".to_string()],        // label: 100 >= 50 → high
        ];

        let (_result, snapshot) = chain::chain_verify_paths(
            &[gate, label], input, &expected_paths, false,
        );
        assert!(snapshot.success, "Path snapshot should match: {:?}", snapshot.mismatches);
        assert_eq!(snapshot.steps_checked, 2);
    }

    #[test]
    fn test_path_snapshot_detects_structural_change() {
        let gate = Microgram::parse(THRESHOLD_GATE).unwrap();
        let label = Microgram::parse(SCORE_LABEL).unwrap();

        let mut input = HashMap::new();
        input.insert("value".to_string(), Value::Int(90));

        // Deliberately wrong path expectation
        let expected_paths = vec![
            vec!["check".to_string(), "fail".to_string()],        // WRONG: actually passes
            vec!["check".to_string(), "high".to_string()],
        ];

        let (_result, snapshot) = chain::chain_verify_paths(
            &[gate, label], input, &expected_paths, false,
        );
        assert!(!snapshot.success, "Should detect path mismatch");
        assert_eq!(snapshot.mismatches.len(), 1);
        assert_eq!(snapshot.mismatches[0].step_index, 0);
        assert_eq!(snapshot.mismatches[0].step_name, "threshold-gate");
    }

    // ═══════════════════════════════════════════════════════════════════
    // Application 2: Multi-Error Chain Validation
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_validate_all_reports_every_step() {
        // Build a chain where multiple steps have required fields
        let step_a = Microgram::parse(r#"
name: step-a
description: "Requires x"
interface:
  inputs:
    x:
      type: int
      required: true
  outputs:
    y:
      type: int
tree:
  start: ret
  nodes:
    ret:
      type: return
      value:
        y: 1
tests:
  - input: { x: 1 }
    expect: { y: 1 }
"#).unwrap();

        let step_b = Microgram::parse(r#"
name: step-b
description: "Requires z"
interface:
  inputs:
    z:
      type: string
      required: true
  outputs:
    result:
      type: string
tree:
  start: ret
  nodes:
    ret:
      type: return
      value:
        result: done
tests:
  - input: { z: "hello" }
    expect: { result: done }
"#).unwrap();

        // Empty input — both steps should report errors
        let result = chain::chain_validate_all(&[step_a, step_b], &HashMap::new());
        assert!(!result.valid);
        assert_eq!(result.total_errors, 2);
        assert_eq!(result.step_errors.len(), 2);
        assert_eq!(result.step_errors[0].step_name, "step-a");
        assert_eq!(result.step_errors[1].step_name, "step-b");
    }

    #[test]
    fn test_validate_all_clean_when_inputs_provided() {
        let gate = Microgram::parse(THRESHOLD_GATE).unwrap();
        let label = Microgram::parse(SCORE_LABEL).unwrap();

        // These micrograms have no required fields, so any input is valid
        let result = chain::chain_validate_all(&[gate, label], &HashMap::new());
        assert!(result.valid);
        assert_eq!(result.total_errors, 0);
    }

    // ═══════════════════════════════════════════════════════════════════
    // Application 3: Vocabulary Hygiene
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_hygiene_detects_field_gaps() {
        let producer = Microgram::parse(r#"
name: producer
description: "Outputs alpha and beta"
interface:
  inputs: {}
  outputs:
    alpha:
      type: int
    beta:
      type: string
tree:
  start: ret
  nodes:
    ret:
      type: return
      value:
        alpha: 1
        beta: "x"
tests:
  - input: {}
    expect: { alpha: 1, beta: "x" }
"#).unwrap();

        let consumer = Microgram::parse(r#"
name: consumer
description: "Needs alpha, beta, and gamma"
interface:
  inputs:
    alpha:
      type: int
    beta:
      type: string
    gamma:
      type: bool
      required: true
  outputs:
    result:
      type: string
tree:
  start: ret
  nodes:
    ret:
      type: return
      value:
        result: done
tests:
  - input: { alpha: 1, beta: "x", gamma: true }
    expect: { result: done }
"#).unwrap();

        let report = hygiene::check_chain_hygiene(&[producer, consumer], &HashMap::new());
        assert!(!report.clean, "Should detect missing required field 'gamma'");
        assert_eq!(report.required_gaps, 1);
        assert_eq!(report.total_gaps, 1);
        assert_eq!(report.boundaries[0].coverage, 2.0 / 3.0);
        assert_eq!(report.boundaries[0].gaps[0].field, "gamma");
    }

    #[test]
    fn test_hygiene_clean_when_all_fields_satisfied() {
        let producer = Microgram::parse(r#"
name: producer
description: "Outputs score"
interface:
  outputs:
    score:
      type: int
tree:
  start: ret
  nodes:
    ret:
      type: return
      value:
        score: 100
tests:
  - input: {}
    expect: { score: 100 }
"#).unwrap();

        let consumer = Microgram::parse(r#"
name: consumer
description: "Needs score"
interface:
  inputs:
    score:
      type: int
      required: true
  outputs:
    label:
      type: string
tree:
  start: ret
  nodes:
    ret:
      type: return
      value:
        label: HIGH
tests:
  - input: { score: 100 }
    expect: { label: HIGH }
"#).unwrap();

        let report = hygiene::check_chain_hygiene(&[producer, consumer], &HashMap::new());
        assert!(report.clean, "All required fields satisfied");
        assert_eq!(report.required_gaps, 0);
        assert_eq!(report.overall_coverage, 1.0);
    }

    // ═══════════════════════════════════════════════════════════════════
    // Integration Patrol
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_patrol_runs_on_codebase() {
        // Run patrol against the actual microgram module
        let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let report = patrol::run_patrol_default(project_root).unwrap();

        // Patrol should find symbols and classify them
        assert!(report.total_symbols > 0, "Should find pub functions");
        assert_eq!(report.unclassified, 0, "All re-exported symbols should be classified");

        // Every finding should have a valid verdict
        for finding in &report.findings {
            match &finding.verdict {
                patrol::PatrolVerdict::Ok => {
                    // Ring meets or exceeds expected
                }
                patrol::PatrolVerdict::Unwired => {
                    // Feature at Ring < 2 or library at Ring 0
                    assert!(
                        finding.actual_ring < finding.expected_ring,
                        "Unwired finding {}: actual {:?} should be below expected {:?}",
                        finding.symbol, finding.actual_ring, finding.expected_ring
                    );
                }
                patrol::PatrolVerdict::Unclassified => {
                    assert_eq!(finding.classification, patrol::SymbolClass::Unclassified);
                }
                patrol::PatrolVerdict::StaleConfig => {
                    panic!(
                        "Stale config entry '{}': listed in patrol.yaml but not found in source",
                        finding.symbol
                    );
                }
            }
        }

        // No stale config entries should exist
        assert_eq!(report.stale, 0, "patrol.yaml should not contain stale entries");
    }

    // ───────────────────────────────────────────────────────────
    // run_validated / ingress / egress tests
    // ───────────────────────────────────────────────────────────

    const TYPED_MICROGRAM: &str = r#"
name: typed-gate
description: "Gate with full interface"
interface:
  inputs:
    score:
      type: float
      required: true
    label:
      type: string
      required: false
  outputs:
    passed:
      type: bool
      required: true
    grade:
      type: string
      required: true
tree:
  start: check
  nodes:
    check:
      type: condition
      variable: score
      operator: gte
      value: 70
      true_next: pass
      false_next: fail
    pass:
      type: return
      value:
        passed: true
        grade: "PASS"
    fail:
      type: return
      value:
        passed: false
        grade: "FAIL"
tests:
  - input: { score: 85 }
    expect: { passed: true, grade: "PASS" }
  - input: { score: 50 }
    expect: { passed: false, grade: "FAIL" }
"#;

    #[test]
    fn test_run_validated_happy_path() {
        let mg = Microgram::parse(TYPED_MICROGRAM).unwrap();
        let mut input = HashMap::new();
        input.insert("score".to_string(), Value::Float(90.0));

        let vr = mg.run_validated(input);
        assert!(vr.is_valid());
        assert!(vr.ingress_errors.is_empty());
        assert!(vr.egress_errors.is_empty());
        assert_eq!(vr.result.output.get("grade"), Some(&Value::String("PASS".to_string())));
    }

    #[test]
    fn test_run_validated_int_float_coercion() {
        // Interface declares float, input provides int — should pass via numeric coercion
        let mg = Microgram::parse(TYPED_MICROGRAM).unwrap();
        let mut input = HashMap::new();
        input.insert("score".to_string(), Value::Int(85));

        let vr = mg.run_validated(input);
        assert!(vr.is_valid(), "int→float coercion should be accepted");
    }

    #[test]
    fn test_run_validated_missing_required_input() {
        let mg = Microgram::parse(TYPED_MICROGRAM).unwrap();
        let input = HashMap::new(); // empty — missing required 'score'

        let vr = mg.run_validated(input);
        assert!(!vr.is_valid());
        assert_eq!(vr.ingress_errors.len(), 1);
        assert!(vr.ingress_errors[0].contains("Missing required input 'score'"));
        assert!(!vr.result.success);
    }

    #[test]
    fn test_run_validated_wrong_type_input() {
        let mg = Microgram::parse(TYPED_MICROGRAM).unwrap();
        let mut input = HashMap::new();
        input.insert("score".to_string(), Value::String("not_a_number".to_string()));

        let vr = mg.run_validated(input);
        assert!(!vr.is_valid());
        assert_eq!(vr.ingress_errors.len(), 1);
        assert!(vr.ingress_errors[0].contains("expected type 'float'"));
        assert!(vr.ingress_errors[0].contains("got 'string'"));
    }

    #[test]
    fn test_run_validated_optional_field_absent() {
        // 'label' is optional — omitting it should be fine
        let mg = Microgram::parse(TYPED_MICROGRAM).unwrap();
        let mut input = HashMap::new();
        input.insert("score".to_string(), Value::Float(75.0));

        let vr = mg.run_validated(input);
        assert!(vr.is_valid());
    }

    #[test]
    fn test_run_validated_optional_field_wrong_type() {
        // 'label' is optional but if provided must be a string
        let mg = Microgram::parse(TYPED_MICROGRAM).unwrap();
        let mut input = HashMap::new();
        input.insert("score".to_string(), Value::Float(75.0));
        input.insert("label".to_string(), Value::Int(42));

        let vr = mg.run_validated(input);
        assert!(!vr.is_valid());
        assert_eq!(vr.ingress_errors.len(), 1);
        assert!(vr.ingress_errors[0].contains("Input 'label'"));
    }

    #[test]
    fn test_run_validated_no_interface_passthrough() {
        // Microgram without interface should behave identically to run()
        let mg = Microgram::parse(IS_POSITIVE).unwrap();
        let mut input = HashMap::new();
        input.insert("n".to_string(), Value::Int(5));

        let vr = mg.run_validated(input);
        assert!(vr.is_valid());
        assert_eq!(vr.result.output.get("positive"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_validate_output_type_mismatch() {
        // Test egress validation directly: construct a microgram whose output
        // disagrees with its declared interface
        let yaml = r#"
name: bad-output
description: "Returns int where string is declared"
interface:
  inputs:
    x:
      type: int
      required: true
  outputs:
    result:
      type: string
      required: true
tree:
  start: go
  nodes:
    go:
      type: return
      value:
        result: 42
tests:
  - input: { x: 1 }
    expect: { result: 42 }
"#;
        let mg = Microgram::parse(yaml).unwrap();
        let mut input = HashMap::new();
        input.insert("x".to_string(), Value::Int(1));

        let vr = mg.run_validated(input);
        // Ingress passes, egress fails (output 'result' is int, declared string)
        assert!(vr.ingress_errors.is_empty());
        assert_eq!(vr.egress_errors.len(), 1);
        assert!(vr.egress_errors[0].contains("Output 'result'"));
        assert!(!vr.is_valid());
    }

    #[test]
    fn test_types_compatible() {
        assert!(types_compatible("int", "int"));
        assert!(types_compatible("int", "float"));
        assert!(types_compatible("float", "int"));
        assert!(types_compatible("string", "any"));
        assert!(!types_compatible("string", "int"));
        assert!(!types_compatible("bool", "string"));
    }

    #[test]
    fn test_type_canonicalization() {
        // boolean → bool
        assert!(types_compatible("bool", "boolean"));
        assert!(types_compatible("boolean", "bool"));
        // integer → int
        assert!(types_compatible("int", "integer"));
        assert!(types_compatible("integer", "int"));
        // number → float (numeric coercion)
        assert!(types_compatible("float", "number"));
        assert!(types_compatible("int", "number"));
        assert!(types_compatible("number", "int"));
        assert!(types_compatible("number", "float"));
        // Non-canonical still rejects cross-type
        assert!(!types_compatible("boolean", "integer"));
        assert!(!types_compatible("number", "string"));
    }

    #[test]
    fn test_canonicalize_type() {
        assert_eq!(canonicalize_type("boolean"), "bool");
        assert_eq!(canonicalize_type("integer"), "int");
        assert_eq!(canonicalize_type("number"), "float");
        assert_eq!(canonicalize_type("string"), "string");
        assert_eq!(canonicalize_type("float"), "float");
        assert_eq!(canonicalize_type("unknown"), "unknown");
    }
}
