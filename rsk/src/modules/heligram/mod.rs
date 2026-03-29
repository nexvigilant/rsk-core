//! # Heligram Module
//!
//! Helical microgram — two complementary decision strands (sense + antisense)
//! coupled by a pairing constraint. The pairing function IS the computation:
//! `∂(×(κ_sense, →_antisense))`.
//!
//! ## Properties
//! - **Dual-stranded**: sense evaluates, antisense falsifies
//! - **Paired**: output requires both strands to resolve through pairing matrix
//! - **Twist-aware**: periodic structural review in chains (every τ steps)
//! - **Backward-compatible**: minor groove output is microgram-compatible

pub mod dna;
pub mod forge;
pub mod promote;

use crate::modules::decision_engine::{
    DecisionContext, DecisionEngine, DecisionTree, Value,
};
use crate::modules::microgram::{MicrogramInterface, PrimitiveSignature};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A heligram — helical microgram with sense/antisense strands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heligram {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    /// Must be "heligram"
    #[serde(rename = "type", default = "default_type")]
    pub heligram_type: String,
    pub helix: HelixParams,
    pub sense: Strand,
    pub antisense: Strand,
    pub resolution: Resolution,
    #[serde(default)]
    pub interface: Option<HelixInterface>,
    #[serde(default)]
    pub tests: Vec<HeligramTest>,
    #[serde(default)]
    pub primitive_signature: Option<PrimitiveSignature>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_type() -> String {
    "heligram".to_string()
}

/// Helical parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelixParams {
    /// Structural review interval in chain steps
    #[serde(default = "default_twist_rate")]
    pub twist_rate: u32,
    /// Maps sense output field name → antisense output field name
    #[serde(default)]
    pub base_pairs: HashMap<String, String>,
}

fn default_twist_rate() -> u32 {
    3
}

/// A single strand — wraps a decision tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strand {
    pub tree: DecisionTree,
}

/// Resolution rules for reconciling sense + antisense outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    #[serde(default = "default_resolution_mode")]
    pub mode: String,
    #[serde(default)]
    pub rules: Vec<ResolutionRule>,
}

fn default_resolution_mode() -> String {
    "base_pair".to_string()
}

/// A single resolution rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionRule {
    /// Conditions to match (field_name → expected_value)
    #[serde(default)]
    pub when: Option<HashMap<String, Value>>,
    /// Default output (if no `when` — acts as fallback)
    #[serde(default)]
    pub default: Option<HashMap<String, Value>>,
    /// Output to emit when `when` matches
    #[serde(default)]
    pub emit: Option<HashMap<String, Value>>,
}

/// Two-groove interface: major (consumer) and minor (routing/audit)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelixInterface {
    #[serde(default)]
    pub major_groove: Option<MicrogramInterface>,
    #[serde(default)]
    pub minor_groove: Option<MicrogramInterface>,
}

/// A heligram test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeligramTest {
    #[serde(default)]
    pub name: Option<String>,
    pub input: HashMap<String, Value>,
    pub expect: HashMap<String, Value>,
}

/// Result of running a heligram
#[derive(Debug, Clone, Serialize)]
pub struct HeligramResult {
    pub name: String,
    pub success: bool,
    pub sense_output: HashMap<String, Value>,
    pub antisense_output: HashMap<String, Value>,
    pub resolved_output: HashMap<String, Value>,
    /// Did all base pairs agree?
    pub agreement: bool,
    pub duration_us: u64,
}

/// Result of running all self-tests
#[derive(Debug, Clone, Serialize)]
pub struct HeligramTestResult {
    pub name: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<SingleHeligramTestResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SingleHeligramTestResult {
    pub index: usize,
    pub name: Option<String>,
    pub passed: bool,
    pub input: HashMap<String, Value>,
    pub expected: HashMap<String, Value>,
    pub actual: HashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mismatch: Option<String>,
}

impl Heligram {
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

    /// Execute with given input variables.
    /// Runs both strands, checks base pair agreement, resolves via rules.
    pub fn run(&self, input: HashMap<String, Value>) -> HeligramResult {
        let start = std::time::Instant::now();

        // Run sense strand
        let sense_output = run_strand(&self.sense, &input);

        // Run antisense strand (same input)
        let antisense_output = run_strand(&self.antisense, &input);

        // Check base pair agreement
        let agreement = self.check_agreement(&sense_output, &antisense_output);

        // Resolve: merge both outputs and match against resolution rules
        let resolved_output = self.resolve(&sense_output, &antisense_output);

        let duration_us = u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX);

        HeligramResult {
            name: self.name.clone(),
            success: true,
            sense_output,
            antisense_output,
            resolved_output,
            agreement,
            duration_us,
        }
    }

    /// Check if base pairs agree.
    /// Agreement: sense field is false/absent, OR antisense complement is false/absent.
    /// Disagreement: sense says true AND antisense says true (signal + falsified).
    fn check_agreement(
        &self,
        sense: &HashMap<String, Value>,
        antisense: &HashMap<String, Value>,
    ) -> bool {
        for (sense_key, antisense_key) in &self.helix.base_pairs {
            let s_val = sense.get(sense_key);
            let a_val = antisense.get(antisense_key);

            // Disagreement = sense is true AND antisense (falsification) is also true
            let sense_true = matches!(s_val, Some(Value::Bool(true)));
            let antisense_true = matches!(a_val, Some(Value::Bool(true)));

            if sense_true && antisense_true {
                return false;
            }
        }
        true
    }

    /// Resolve sense + antisense through the resolution rules.
    fn resolve(
        &self,
        sense: &HashMap<String, Value>,
        antisense: &HashMap<String, Value>,
    ) -> HashMap<String, Value> {
        // Merge both outputs into a combined map for matching
        let mut combined = sense.clone();
        for (k, v) in antisense {
            combined.insert(k.clone(), v.clone());
        }

        // Try each rule in order
        for rule in &self.resolution.rules {
            if let Some(ref when) = rule.when {
                let matches = when.iter().all(|(key, expected)| {
                    combined.get(key).map_or(false, |actual| values_match(actual, expected))
                });
                if matches {
                    if let Some(ref emit) = rule.emit {
                        let mut output = emit.clone();
                        // Template substitution: replace {{field}} with actual values
                        resolve_templates(&mut output, &combined);
                        // Inject agreement flag
                        let agreement = self.check_agreement(sense, antisense);
                        output.insert("agreement".to_string(), Value::Bool(agreement));
                        return output;
                    }
                }
            }
            // Default rule (no `when` clause)
            if rule.when.is_none() {
                if let Some(ref default) = rule.default {
                    let mut output = default.clone();
                    let agreement = self.check_agreement(sense, antisense);
                    output.insert("agreement".to_string(), Value::Bool(agreement));
                    return output;
                }
            }
        }

        // No rule matched — produce a minimal output
        let mut output = HashMap::new();
        output.insert("agreement".to_string(), Value::Bool(self.check_agreement(sense, antisense)));
        output.insert("_unresolved".to_string(), Value::Bool(true));
        output
    }

    /// Run all self-tests
    pub fn test(&self) -> HeligramTestResult {
        let mut results = Vec::with_capacity(self.tests.len());
        let mut passed = 0;

        for (i, test) in self.tests.iter().enumerate() {
            let run = self.run(test.input.clone());

            // Check each expected field against resolved output
            let mut all_match = true;
            let mut mismatch_details = Vec::new();

            for (key, expected) in &test.expect {
                match run.resolved_output.get(key) {
                    Some(actual) if values_match(actual, expected) => {}
                    Some(actual) => {
                        all_match = false;
                        mismatch_details.push(format!(
                            "{key}: expected {expected:?}, got {actual:?}"
                        ));
                    }
                    None => {
                        all_match = false;
                        mismatch_details.push(format!("{key}: missing from output"));
                    }
                }
            }

            if all_match {
                passed += 1;
            }

            results.push(SingleHeligramTestResult {
                index: i,
                name: test.name.clone(),
                passed: all_match,
                input: test.input.clone(),
                expected: test.expect.clone(),
                actual: run.resolved_output,
                mismatch: if mismatch_details.is_empty() {
                    None
                } else {
                    Some(mismatch_details.join("; "))
                },
            });
        }

        HeligramTestResult {
            name: self.name.clone(),
            total: self.tests.len(),
            passed,
            failed: self.tests.len() - passed,
            results,
        }
    }
}

/// Run a single strand's decision tree
fn run_strand(strand: &Strand, input: &HashMap<String, Value>) -> HashMap<String, Value> {
    let engine = DecisionEngine::new(strand.tree.clone());
    let mut ctx = DecisionContext {
        variables: input.clone(),
        execution_path: Vec::new(),
    };

    match engine.execute(&mut ctx) {
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
    }
}

/// Compare two Values for test matching (loose: int/float coerce)
fn values_match(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
        (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => {
            (*a as f64 - b).abs() < f64::EPSILON
        }
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

/// Replace `{{field_name}}` in string values with actual values from combined output
fn resolve_templates(output: &mut HashMap<String, Value>, source: &HashMap<String, Value>) {
    let snapshot: Vec<(String, Value)> = output.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    for (key, val) in snapshot {
        if let Value::String(s) = &val {
            if s.contains("{{") {
                let mut resolved = s.clone();
                for (src_key, src_val) in source {
                    let placeholder = format!("{{{{{src_key}}}}}");
                    if resolved.contains(&placeholder) {
                        let replacement = match src_val {
                            Value::String(s) => s.clone(),
                            Value::Int(n) => n.to_string(),
                            Value::Float(f) => f.to_string(),
                            Value::Bool(b) => b.to_string(),
                            _ => String::new(),
                        };
                        resolved = resolved.replace(&placeholder, &replacement);
                    }
                }
                output.insert(key, Value::String(resolved));
            }
        }
    }
}

/// Result of running a heligram chain
#[derive(Debug, Clone, Serialize)]
pub struct HelixChainResult {
    pub success: bool,
    pub steps: Vec<HeligramResult>,
    /// Consensus: fraction of steps where agreement=true
    pub consensus_ratio: f64,
    pub final_output: HashMap<String, Value>,
    pub total_duration_us: u64,
}

/// Run a chain of heligrams, accumulating outputs.
/// Each step receives the merged output of all prior steps + original input.
pub fn chain(
    names: &[&str],
    dir: &Path,
    input: HashMap<String, Value>,
) -> Result<HelixChainResult, String> {
    let all = load_all(dir)?;

    let mut accumulated = input;
    let mut steps = Vec::new();
    let mut agree_count = 0usize;
    let mut total_us = 0u64;

    for name in names {
        let h = all.iter().find(|h| h.name == *name)
            .ok_or_else(|| format!("Heligram '{}' not found in {}", name, dir.display()))?;

        let result = h.run(accumulated.clone());
        total_us += result.duration_us;

        if result.agreement {
            agree_count += 1;
        }

        // Accumulate: merge resolved output into the running state
        for (k, v) in &result.resolved_output {
            accumulated.insert(k.clone(), v.clone());
        }
        // Also merge raw sense/antisense for downstream visibility
        for (k, v) in &result.sense_output {
            accumulated.insert(format!("{}_sense_{k}", name.replace('-', "_")), v.clone());
        }
        for (k, v) in &result.antisense_output {
            accumulated.insert(format!("{}_antisense_{k}", name.replace('-', "_")), v.clone());
        }

        steps.push(result);
    }

    let consensus = if steps.is_empty() { 1.0 } else { agree_count as f64 / steps.len() as f64 };

    Ok(HelixChainResult {
        success: true,
        steps,
        consensus_ratio: consensus,
        final_output: accumulated,
        total_duration_us: total_us,
    })
}

/// Load all heligrams from a directory
pub fn load_all(dir: &Path) -> Result<Vec<Heligram>, String> {
    let mut heligrams = Vec::new();
    load_all_recursive(dir, &mut heligrams)?;
    Ok(heligrams)
}

fn load_all_recursive(dir: &Path, out: &mut Vec<Heligram>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read directory {}: {e}", dir.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Directory entry error: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            load_all_recursive(&path, out)?;
        } else if path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
            match Heligram::load(&path) {
                Ok(h) => out.push(h),
                Err(e) => {
                    // Skip non-heligram YAML files silently
                    if !e.contains("missing field") {
                        eprintln!("Warning: {}: {e}", path.display());
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_prr_signal_helix() {
        let yaml = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("heligrams/prr-signal-helix.yaml")
        )
            .expect("heligram YAML should exist");
        let h = Heligram::parse(&yaml).expect("should parse");
        assert_eq!(h.name, "prr-signal-helix");
        assert_eq!(h.heligram_type, "heligram");
        assert_eq!(h.helix.twist_rate, 3);
        assert!(!h.helix.base_pairs.is_empty());
        assert!(!h.tests.is_empty());
    }

    #[test]
    fn test_run_confirmed_signal() {
        let yaml = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("heligrams/prr-signal-helix.yaml")
        )
            .expect("heligram YAML should exist");
        let h = Heligram::parse(&yaml).expect("should parse");

        let mut input = HashMap::new();
        input.insert("prr".to_string(), Value::Float(3.5));
        input.insert("total_reports".to_string(), Value::Int(50));
        input.insert("notoriety_bias".to_string(), Value::Bool(false));

        let result = h.run(input);
        assert!(result.success);
        assert!(result.agreement);
        assert_eq!(
            result.resolved_output.get("verdict"),
            Some(&Value::String("confirmed_signal".to_string()))
        );
        assert_eq!(
            result.resolved_output.get("confidence"),
            Some(&Value::String("high".to_string()))
        );
    }

    #[test]
    fn test_run_contested_signal() {
        let yaml = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("heligrams/prr-signal-helix.yaml")
        )
            .expect("heligram YAML should exist");
        let h = Heligram::parse(&yaml).expect("should parse");

        let mut input = HashMap::new();
        input.insert("prr".to_string(), Value::Float(3.5));
        input.insert("total_reports".to_string(), Value::Int(2));
        input.insert("notoriety_bias".to_string(), Value::Bool(false));

        let result = h.run(input);
        assert!(result.success);
        assert!(!result.agreement);
        assert_eq!(
            result.resolved_output.get("verdict"),
            Some(&Value::String("contested_signal".to_string()))
        );
    }

    #[test]
    fn test_run_no_signal() {
        let yaml = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("heligrams/prr-signal-helix.yaml")
        )
            .expect("heligram YAML should exist");
        let h = Heligram::parse(&yaml).expect("should parse");

        let mut input = HashMap::new();
        input.insert("prr".to_string(), Value::Float(1.2));
        input.insert("total_reports".to_string(), Value::Int(100));
        input.insert("notoriety_bias".to_string(), Value::Bool(false));

        let result = h.run(input);
        assert!(result.success);
        assert!(result.agreement);
        assert_eq!(
            result.resolved_output.get("verdict"),
            Some(&Value::String("no_signal".to_string()))
        );
    }

    #[test]
    fn test_null_safety() {
        let yaml = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("heligrams/prr-signal-helix.yaml")
        )
            .expect("heligram YAML should exist");
        let h = Heligram::parse(&yaml).expect("should parse");

        let result = h.run(HashMap::new());
        assert!(result.success);
        // Empty input should produce no_signal
        assert_eq!(
            result.resolved_output.get("verdict"),
            Some(&Value::String("no_signal".to_string()))
        );
    }

    #[test]
    fn test_self_tests() {
        let yaml = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("heligrams/prr-signal-helix.yaml")
        )
            .expect("heligram YAML should exist");
        let h = Heligram::parse(&yaml).expect("should parse");
        let result = h.test();
        assert_eq!(result.failed, 0, "All heligram self-tests should pass: {:?}",
            result.results.iter().filter(|r| !r.passed).collect::<Vec<_>>());
    }
}
