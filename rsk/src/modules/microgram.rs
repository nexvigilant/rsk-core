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

use crate::modules::decision_engine::{
    DecisionContext, DecisionEngine, DecisionNode, DecisionTree, Operator, Value,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

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

    /// Execute with given input variables
    pub fn run(&self, input: HashMap<String, Value>) -> MicrogramResult {
        let start = std::time::Instant::now();
        let engine = DecisionEngine::new(self.tree.clone());
        let mut ctx = DecisionContext {
            variables: input,
            execution_path: Vec::new(),
        };

        let exec_result = engine.execute(&mut ctx);
        let duration_us = start.elapsed().as_micros() as u64;

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
                            "{}: expected {:?}, got {:?}",
                            key, expected_val, actual_val
                        ));
                        test_passed = false;
                    }
                    None => {
                        mismatch = Some(format!("{}: expected {:?}, got nothing", key, expected_val));
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
}

// ═══════════════════════════════════════════════════════════════════════════
// MICROGRAM REGISTRY — scan a directory, load all .yaml files
// ═══════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════
// MICROGRAM CHAIN — pipe output of N into input of N+1
// ═══════════════════════════════════════════════════════════════════════════

/// Result of chaining micrograms
#[derive(Debug, Clone, Serialize)]
pub struct ChainResult {
    pub success: bool,
    pub steps: Vec<MicrogramResult>,
    pub final_output: HashMap<String, Value>,
    pub total_duration_us: u64,
}

/// Chain multiple micrograms: output of step N becomes input of step N+1
pub fn chain(micrograms: &[Microgram], initial_input: HashMap<String, Value>) -> ChainResult {
    let mut current_input = initial_input;
    let mut steps = Vec::with_capacity(micrograms.len());
    let mut total_us = 0u64;

    for mg in micrograms {
        let result = mg.run(current_input);
        total_us += result.duration_us;

        if !result.success {
            let final_output = result.output.clone();
            steps.push(result);
            return ChainResult {
                success: false,
                steps,
                final_output,
                total_duration_us: total_us,
            };
        }

        // Flow: this step's output becomes next step's input
        current_input = result.output.clone();
        steps.push(result);
    }

    let final_output = steps
        .last()
        .map(|s| s.output.clone())
        .unwrap_or_default();

    ChainResult {
        success: true,
        steps,
        final_output,
        total_duration_us: total_us,
    }
}

/// Load micrograms by name from a directory and chain them
pub fn chain_by_names(
    dir: &Path,
    names: &[&str],
    initial_input: HashMap<String, Value>,
) -> Result<ChainResult, String> {
    let all = load_all(dir)?;
    let mut ordered = Vec::with_capacity(names.len());

    for name in names {
        match all.iter().find(|mg| mg.name == *name) {
            Some(mg) => ordered.push(mg.clone()),
            None => return Err(format!("Microgram '{}' not found in {}", name, dir.display())),
        }
    }

    Ok(chain(&ordered, initial_input))
}

/// Load all micrograms from a directory
pub fn load_all(dir: &Path) -> Result<Vec<Microgram>, String> {
    let mut micrograms = Vec::new();

    let entries = std::fs::read_dir(dir).map_err(|e| format!("Cannot read dir: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Dir entry error: {e}"))?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
            match Microgram::load(&path) {
                Ok(mg) => micrograms.push(mg),
                Err(e) => eprintln!("Warning: skipping {}: {e}", path.display()),
            }
        }
    }

    micrograms.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(micrograms)
}

/// Run all self-tests across all micrograms in a directory
pub fn test_all(dir: &Path) -> Result<Vec<TestResult>, String> {
    let micrograms = load_all(dir)?;
    Ok(micrograms.iter().map(|mg| mg.test()).collect())
}

// ═══════════════════════════════════════════════════════════════════════════
// GENERATE — synthesize a microgram from a spec
// ═══════════════════════════════════════════════════════════════════════════

/// Spec for generating a microgram
#[derive(Debug, Clone)]
pub struct MicrogramSpec {
    pub name: String,
    pub description: String,
    pub variable: String,
    pub operator: String,     // gt, gte, lt, lte, eq, is_null, is_not_null, matches
    pub threshold: Value,     // comparison value
    pub true_label: String,   // output key name when true
    pub true_value: Value,    // output value when true
    pub false_label: String,  // output key name when false
    pub false_value: Value,   // output value when false
}

impl MicrogramSpec {
    /// Generate a Microgram from this spec
    pub fn build(&self) -> Microgram {
        let operator = match self.operator.as_str() {
            "gt" => Operator::Gt,
            "gte" => Operator::Gte,
            "lt" => Operator::Lt,
            "lte" => Operator::Lte,
            "eq" => Operator::Eq,
            "is_null" => Operator::IsNull,
            "is_not_null" => Operator::IsNotNull,
            "matches" => Operator::Matches,
            _ => Operator::Eq, // fallback
        };

        let mut true_output = HashMap::new();
        true_output.insert(self.true_label.clone(), self.true_value.clone());

        let mut false_output = HashMap::new();
        false_output.insert(self.false_label.clone(), self.false_value.clone());

        let mut nodes = HashMap::new();
        nodes.insert(
            "check".to_string(),
            DecisionNode::Condition {
                variable: self.variable.clone(),
                operator,
                value: Some(self.threshold.clone()),
                true_next: "yes".to_string(),
                false_next: "no".to_string(),
            },
        );
        nodes.insert(
            "yes".to_string(),
            DecisionNode::Return {
                value: Value::Object(true_output.into_iter().collect()),
            },
        );
        nodes.insert(
            "no".to_string(),
            DecisionNode::Return {
                value: Value::Object(false_output.into_iter().collect()),
            },
        );

        // Auto-generate test cases from boundary analysis
        let tests = self.generate_tests();

        Microgram {
            name: self.name.clone(),
            description: self.description.clone(),
            version: "0.1.0".to_string(),
            tree: DecisionTree {
                start: "check".to_string(),
                nodes,
            },
            tests,
        }
    }

    /// Auto-generate boundary test cases
    fn generate_tests(&self) -> Vec<MicrogramTest> {
        let mut tests = Vec::new();

        match &self.threshold {
            Value::Int(n) => {
                let n = *n;
                // At threshold
                let mut at_input = HashMap::new();
                at_input.insert(self.variable.clone(), Value::Int(n));
                let at_expected = match self.operator.as_str() {
                    "gt" | "matches" => {
                        let mut m = HashMap::new();
                        m.insert(self.false_label.clone(), self.false_value.clone());
                        m
                    }
                    _ => {
                        let mut m = HashMap::new();
                        m.insert(self.true_label.clone(), self.true_value.clone());
                        m
                    }
                };
                tests.push(MicrogramTest { input: at_input, expect: at_expected });

                // Above threshold
                let mut above_input = HashMap::new();
                above_input.insert(self.variable.clone(), Value::Int(n + 1));
                let mut above_expected = HashMap::new();
                above_expected.insert(self.true_label.clone(), self.true_value.clone());
                tests.push(MicrogramTest { input: above_input, expect: above_expected });

                // Below threshold
                let mut below_input = HashMap::new();
                below_input.insert(self.variable.clone(), Value::Int(n - 1));
                let mut below_expected = HashMap::new();
                below_expected.insert(self.false_label.clone(), self.false_value.clone());
                tests.push(MicrogramTest { input: below_input, expect: below_expected });
            }
            Value::Float(f) => {
                let f = *f;
                let mut at_input = HashMap::new();
                at_input.insert(self.variable.clone(), Value::Float(f));
                let at_expected = match self.operator.as_str() {
                    "gt" => {
                        let mut m = HashMap::new();
                        m.insert(self.false_label.clone(), self.false_value.clone());
                        m
                    }
                    _ => {
                        let mut m = HashMap::new();
                        m.insert(self.true_label.clone(), self.true_value.clone());
                        m
                    }
                };
                tests.push(MicrogramTest { input: at_input, expect: at_expected });

                let mut above_input = HashMap::new();
                above_input.insert(self.variable.clone(), Value::Float(f + 1.0));
                let mut above_expected = HashMap::new();
                above_expected.insert(self.true_label.clone(), self.true_value.clone());
                tests.push(MicrogramTest { input: above_input, expect: above_expected });

                let mut below_input = HashMap::new();
                below_input.insert(self.variable.clone(), Value::Float(f - 1.0));
                let mut below_expected = HashMap::new();
                below_expected.insert(self.false_label.clone(), self.false_value.clone());
                tests.push(MicrogramTest { input: below_input, expect: below_expected });
            }
            _ => {
                // For string/bool comparisons, generate true/false pair
                let mut true_input = HashMap::new();
                true_input.insert(self.variable.clone(), self.threshold.clone());
                let mut true_expected = HashMap::new();
                true_expected.insert(self.true_label.clone(), self.true_value.clone());
                tests.push(MicrogramTest { input: true_input, expect: true_expected });
            }
        }

        tests
    }

    /// Serialize the generated microgram to YAML string
    pub fn to_yaml(&self) -> Result<String, String> {
        let mg = self.build();
        serde_yaml::to_string(&mg).map_err(|e| format!("YAML serialization error: {e}"))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// EVOLVE — add edge-case tests to an existing microgram
// ═══════════════════════════════════════════════════════════════════════════

/// Analyze a microgram and suggest additional test cases
pub fn evolve_tests(mg: &Microgram) -> Vec<MicrogramTest> {
    let mut new_tests = Vec::new();
    let existing_inputs: Vec<&HashMap<String, Value>> = mg.tests.iter().map(|t| &t.input).collect();

    // Strategy 1: Find boundary variables from the decision tree
    for node in mg.tree.nodes.values() {
        if let DecisionNode::Condition { variable, value: Some(threshold), operator, .. } = node {
            let has_var = |inputs: &[&HashMap<String, Value>], pred: &dyn Fn(&Value) -> bool| -> bool {
                inputs.iter().any(|i| i.get(variable).is_some_and(pred))
            };

            match threshold {
                Value::Int(n) => {
                    let n = *n;
                    // Add zero if not tested
                    if !has_var(&existing_inputs, &|v| matches!(v, Value::Int(0))) {
                        let mut input = HashMap::new();
                        input.insert(variable.clone(), Value::Int(0));
                        let result = mg.run(input.clone());
                        new_tests.push(MicrogramTest { input, expect: result.output });
                    }
                    // Add negative if not tested
                    if !has_var(&existing_inputs, &|v| matches!(v, Value::Int(i) if *i < 0)) {
                        let mut input = HashMap::new();
                        input.insert(variable.clone(), Value::Int(-1));
                        let result = mg.run(input.clone());
                        new_tests.push(MicrogramTest { input, expect: result.output });
                    }
                    // Add exact boundary ±1 if not tested
                    if !has_var(&existing_inputs, &|v| matches!(v, Value::Int(i) if *i == n - 1)) {
                        let mut input = HashMap::new();
                        input.insert(variable.clone(), Value::Int(n - 1));
                        let result = mg.run(input.clone());
                        new_tests.push(MicrogramTest { input, expect: result.output });
                    }
                    if !has_var(&existing_inputs, &|v| matches!(v, Value::Int(i) if *i == n + 1)) {
                        let mut input = HashMap::new();
                        input.insert(variable.clone(), Value::Int(n + 1));
                        let result = mg.run(input.clone());
                        new_tests.push(MicrogramTest { input, expect: result.output });
                    }
                    // Large value
                    if !has_var(&existing_inputs, &|v| matches!(v, Value::Int(i) if *i > n * 10)) {
                        let mut input = HashMap::new();
                        input.insert(variable.clone(), Value::Int(n * 100));
                        let result = mg.run(input.clone());
                        new_tests.push(MicrogramTest { input, expect: result.output });
                    }
                }
                Value::Float(f) => {
                    let f = *f;
                    // Epsilon below
                    if !has_var(&existing_inputs, &|v| matches!(v, Value::Float(x) if (*x - f).abs() < 0.01 && *x < f)) {
                        let mut input = HashMap::new();
                        input.insert(variable.clone(), Value::Float(f - 0.001));
                        let result = mg.run(input.clone());
                        new_tests.push(MicrogramTest { input, expect: result.output });
                    }
                    // Epsilon above
                    if !has_var(&existing_inputs, &|v| matches!(v, Value::Float(x) if (*x - f).abs() < 0.01 && *x > f)) {
                        let mut input = HashMap::new();
                        input.insert(variable.clone(), Value::Float(f + 0.001));
                        let result = mg.run(input.clone());
                        new_tests.push(MicrogramTest { input, expect: result.output });
                    }
                }
                _ => {}
            }

            // Strategy 2: null test if operator isn't is_null/is_not_null
            if !matches!(operator, Operator::IsNull | Operator::IsNotNull) {
                let has_missing = existing_inputs.iter().any(|i| !i.contains_key(variable));
                if !has_missing {
                    // Run with missing variable to capture behavior
                    let input = HashMap::new();
                    let result = mg.run(input.clone());
                    new_tests.push(MicrogramTest { input, expect: result.output });
                }
            }
        }
    }

    new_tests
}

// ═══════════════════════════════════════════════════════════════════════════
// COMPOSE — auto-select and chain micrograms from a directory to meet a goal
// ═══════════════════════════════════════════════════════════════════════════

/// Goal for composition: what output fields are required
#[derive(Debug, Clone)]
pub struct CompositionGoal {
    pub required_outputs: Vec<String>,   // field names that must exist in final output
    pub initial_input: HashMap<String, Value>,
}

/// Result of attempting to compose a chain
#[derive(Debug, Clone, Serialize)]
pub struct CompositionPlan {
    pub feasible: bool,
    pub chain: Vec<String>,          // ordered microgram names
    pub coverage: Vec<String>,       // which required outputs are covered
    pub missing: Vec<String>,        // which required outputs can't be produced
}

/// Analyze which output fields a microgram can produce
fn output_fields(mg: &Microgram) -> Vec<String> {
    let mut fields = Vec::new();
    for node in mg.tree.nodes.values() {
        if let DecisionNode::Return { value: Value::Object(map) } = node {
            for key in map.keys() {
                if !fields.contains(key) {
                    fields.push(key.clone());
                }
            }
        }
    }
    fields
}

/// Analyze which input variables a microgram requires
fn input_variables(mg: &Microgram) -> Vec<String> {
    let mut vars = Vec::new();
    for node in mg.tree.nodes.values() {
        if let DecisionNode::Condition { variable, .. } = node {
            if !vars.contains(variable) {
                vars.push(variable.clone());
            }
        }
    }
    vars
}

/// Compose a chain from available micrograms to meet a goal
pub fn compose(
    dir: &Path,
    goal: &CompositionGoal,
) -> Result<CompositionPlan, String> {
    let all = load_all(dir)?;

    // Index: which microgram produces which output fields
    let producers: Vec<(String, Vec<String>, Vec<String>)> = all
        .iter()
        .map(|mg| (mg.name.clone(), output_fields(mg), input_variables(mg)))
        .collect();

    // Greedy forward chain: start with available inputs, find micrograms that can run,
    // add their outputs, repeat until goal is met or no progress
    let mut available: Vec<String> = goal.initial_input.keys().cloned().collect();
    let mut chain_names: Vec<String> = Vec::new();
    let mut used: Vec<bool> = vec![false; producers.len()];

    for _ in 0..producers.len() {
        let mut made_progress = false;
        for (i, (name, outputs, inputs)) in producers.iter().enumerate() {
            if used[i] {
                continue;
            }
            // Can this microgram run with currently available variables?
            let can_run = inputs.iter().all(|v| available.contains(v));
            if !can_run {
                continue;
            }
            // Does it produce something we still need?
            let produces_needed = outputs.iter().any(|o| {
                goal.required_outputs.contains(o) && !available.contains(o)
            });
            // Or does it produce something that unlocks another microgram?
            let produces_useful = outputs.iter().any(|o| !available.contains(o));

            if produces_needed || produces_useful {
                chain_names.push(name.clone());
                for o in outputs {
                    if !available.contains(o) {
                        available.push(o.clone());
                    }
                }
                used[i] = true;
                made_progress = true;
            }
        }
        if !made_progress {
            break;
        }
    }

    let coverage: Vec<String> = goal
        .required_outputs
        .iter()
        .filter(|r| available.contains(r))
        .cloned()
        .collect();
    let missing: Vec<String> = goal
        .required_outputs
        .iter()
        .filter(|r| !available.contains(r))
        .cloned()
        .collect();

    Ok(CompositionPlan {
        feasible: missing.is_empty(),
        chain: chain_names,
        coverage,
        missing,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// BENCH — measure execution performance across micrograms
// ═══════════════════════════════════════════════════════════════════════════

/// Performance benchmark result for a single microgram
#[derive(Debug, Clone, Serialize)]
pub struct BenchResult {
    pub name: String,
    pub iterations: usize,
    pub min_us: u64,
    pub max_us: u64,
    pub avg_us: f64,
    pub p95_us: u64,
    pub tests_pass: bool,
}

/// Benchmark all micrograms in a directory
pub fn bench_all(dir: &Path, iterations: usize) -> Result<Vec<BenchResult>, String> {
    let all = load_all(dir)?;
    let mut results = Vec::with_capacity(all.len());

    for mg in &all {
        // Use first test case as benchmark input, or empty
        let input = mg.tests.first()
            .map(|t| t.input.clone())
            .unwrap_or_default();

        let mut timings = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let result = mg.run(input.clone());
            timings.push(result.duration_us);
        }

        timings.sort();
        let min_us = timings[0];
        let max_us = timings[timings.len() - 1];
        let avg_us = timings.iter().sum::<u64>() as f64 / timings.len() as f64;
        let p95_idx = (timings.len() as f64 * 0.95) as usize;
        let p95_us = timings[p95_idx.min(timings.len() - 1)];

        let test_result = mg.test();

        results.push(BenchResult {
            name: mg.name.clone(),
            iterations,
            min_us,
            max_us,
            avg_us,
            p95_us,
            tests_pass: test_result.failed == 0,
        });
    }

    Ok(results)
}

// ═══════════════════════════════════════════════════════════════════════════
// AUTO — compose → chain → execute → verify in one shot
// ═══════════════════════════════════════════════════════════════════════════

/// Full auto-execution result
#[derive(Debug, Clone, Serialize)]
pub struct AutoResult {
    pub plan: CompositionPlan,
    pub execution: Option<ChainResult>,
    pub verified: bool,
    pub duration_us: u64,
}

/// Auto-compose and execute: goal → plan → chain → run → verify
pub fn auto_execute(
    dir: &Path,
    goal: &CompositionGoal,
) -> Result<AutoResult, String> {
    let start = std::time::Instant::now();

    // Step 1: Compose
    let plan = compose(dir, goal)?;

    if !plan.feasible {
        return Ok(AutoResult {
            plan,
            execution: None,
            verified: false,
            duration_us: start.elapsed().as_micros() as u64,
        });
    }

    // Step 2: Execute the composed chain
    let names: Vec<&str> = plan.chain.iter().map(|s| s.as_str()).collect();
    let chain_result = chain_by_names(dir, &names, goal.initial_input.clone())?;

    // Step 3: Verify — all required outputs present in ANY step's output
    let verified = plan.coverage.iter().all(|field| {
        chain_result.steps.iter().any(|step| step.output.contains_key(field))
    });

    let duration_us = start.elapsed().as_micros() as u64;

    Ok(AutoResult {
        plan,
        execution: Some(chain_result),
        verified,
        duration_us,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// CATALOG — full introspection of the microgram ecosystem
// ═══════════════════════════════════════════════════════════════════════════

/// Ecosystem catalog entry
#[derive(Debug, Clone, Serialize)]
pub struct CatalogEntry {
    pub name: String,
    pub description: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub test_count: usize,
    pub tests_pass: bool,
}

/// Full ecosystem catalog
#[derive(Debug, Clone, Serialize)]
pub struct Catalog {
    pub entries: Vec<CatalogEntry>,
    pub total_micrograms: usize,
    pub total_tests: usize,
    pub all_pass: bool,
    /// Reachability matrix: for each pair (A, B), can A's output feed B's input?
    pub connections: Vec<(String, String)>,
}

/// Build a full catalog of the microgram ecosystem
pub fn catalog(dir: &Path) -> Result<Catalog, String> {
    let all = load_all(dir)?;

    let mut entries = Vec::with_capacity(all.len());
    let mut total_tests = 0;
    let mut all_pass = true;

    for mg in &all {
        let inputs = input_variables(mg);
        let outputs = output_fields(mg);
        let test_result = mg.test();
        let tests_pass = test_result.failed == 0;
        if !tests_pass { all_pass = false; }
        total_tests += test_result.total;

        entries.push(CatalogEntry {
            name: mg.name.clone(),
            description: mg.description.clone(),
            inputs,
            outputs,
            test_count: test_result.total,
            tests_pass,
        });
    }

    // Build connection graph: A → B if any output of A matches any input of B
    let mut connections = Vec::new();
    for a in &entries {
        for b in &entries {
            if a.name == b.name { continue; }
            let can_feed = a.outputs.iter().any(|o| b.inputs.contains(o));
            if can_feed {
                connections.push((a.name.clone(), b.name.clone()));
            }
        }
    }

    Ok(Catalog {
        total_micrograms: entries.len(),
        total_tests,
        all_pass,
        entries,
        connections,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// DIFF — structural comparison of two micrograms
// ═══════════════════════════════════════════════════════════════════════════

/// Diff result between two micrograms
#[derive(Debug, Clone, Serialize)]
pub struct DiffResult {
    pub left: String,
    pub right: String,
    pub same_inputs: bool,
    pub same_outputs: bool,
    pub shared_inputs: Vec<String>,
    pub left_only_inputs: Vec<String>,
    pub right_only_inputs: Vec<String>,
    pub shared_outputs: Vec<String>,
    pub left_only_outputs: Vec<String>,
    pub right_only_outputs: Vec<String>,
    pub test_overlap: usize,         // tests with identical inputs
    pub behavior_matches: usize,     // of overlapping tests, how many produce same output
    pub behavior_diverges: usize,    // of overlapping tests, how many produce different output
    pub compatible: bool,            // can they be chained? (left outputs ∩ right inputs ≠ ∅)
}

/// Compare two micrograms structurally and behaviorally
pub fn diff(a: &Microgram, b: &Microgram) -> DiffResult {
    let a_inputs = input_variables(a);
    let a_outputs = output_fields(a);
    let b_inputs = input_variables(b);
    let b_outputs = output_fields(b);

    let shared_inputs: Vec<String> = a_inputs.iter().filter(|v| b_inputs.contains(v)).cloned().collect();
    let left_only_inputs: Vec<String> = a_inputs.iter().filter(|v| !b_inputs.contains(v)).cloned().collect();
    let right_only_inputs: Vec<String> = b_inputs.iter().filter(|v| !a_inputs.contains(v)).cloned().collect();

    let shared_outputs: Vec<String> = a_outputs.iter().filter(|v| b_outputs.contains(v)).cloned().collect();
    let left_only_outputs: Vec<String> = a_outputs.iter().filter(|v| !b_outputs.contains(v)).cloned().collect();
    let right_only_outputs: Vec<String> = b_outputs.iter().filter(|v| !a_outputs.contains(v)).cloned().collect();

    // Behavioral comparison: run shared test inputs through both
    let mut test_overlap = 0;
    let mut behavior_matches = 0;
    let mut behavior_diverges = 0;

    for a_test in &a.tests {
        for b_test in &b.tests {
            if a_test.input == b_test.input {
                test_overlap += 1;
                let a_result = a.run(a_test.input.clone());
                let b_result = b.run(b_test.input.clone());
                if a_result.output == b_result.output {
                    behavior_matches += 1;
                } else {
                    behavior_diverges += 1;
                }
            }
        }
    }

    // Compatible: can A feed B?
    let compatible = a_outputs.iter().any(|o| b_inputs.contains(o));

    DiffResult {
        left: a.name.clone(),
        right: b.name.clone(),
        same_inputs: left_only_inputs.is_empty() && right_only_inputs.is_empty(),
        same_outputs: left_only_outputs.is_empty() && right_only_outputs.is_empty(),
        shared_inputs,
        left_only_inputs,
        right_only_inputs,
        shared_outputs,
        left_only_outputs,
        right_only_outputs,
        test_overlap,
        behavior_matches,
        behavior_diverges,
        compatible,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MERGE — combine two micrograms into a multi-branch decision tree
// ═══════════════════════════════════════════════════════════════════════════

/// Merge two micrograms into one by creating a dispatcher that routes
/// based on which input variables are present
pub fn merge(a: &Microgram, b: &Microgram, name: &str, description: &str) -> Microgram {
    let a_inputs = input_variables(a);
    let b_inputs = input_variables(b);

    // Build merged tree: dispatch node checks for A's variable first
    let mut nodes = HashMap::new();

    // Prefix all A nodes with "a_" and B nodes with "b_"
    for (node_name, node) in &a.tree.nodes {
        let prefixed = format!("a_{}", node_name);
        let remapped = remap_node(node, "a_");
        nodes.insert(prefixed, remapped);
    }
    for (node_name, node) in &b.tree.nodes {
        let prefixed = format!("b_{}", node_name);
        let remapped = remap_node(node, "b_");
        nodes.insert(prefixed, remapped);
    }

    // Dispatch: check if A's primary input exists → route to A, else B
    let a_var = a_inputs.first().cloned().unwrap_or_default();
    nodes.insert(
        "dispatch".to_string(),
        DecisionNode::Condition {
            variable: a_var,
            operator: Operator::IsNotNull,
            value: None,
            true_next: format!("a_{}", a.tree.start),
            false_next: format!("b_{}", b.tree.start),
        },
    );

    // Merge tests from both
    let mut tests = a.tests.clone();
    tests.extend(b.tests.iter().cloned());

    Microgram {
        name: name.to_string(),
        description: description.to_string(),
        version: "0.1.0".to_string(),
        tree: DecisionTree {
            start: "dispatch".to_string(),
            nodes,
        },
        tests,
    }
}

/// Remap node references with a prefix
fn remap_node(node: &DecisionNode, prefix: &str) -> DecisionNode {
    match node {
        DecisionNode::Condition { variable, operator, value, true_next, false_next } => {
            DecisionNode::Condition {
                variable: variable.clone(),
                operator: operator.clone(),
                value: value.clone(),
                true_next: format!("{}{}", prefix, true_next),
                false_next: format!("{}{}", prefix, false_next),
            }
        }
        DecisionNode::Return { value } => DecisionNode::Return { value: value.clone() },
        DecisionNode::Action { action, target, value, next } => {
            DecisionNode::Action {
                action: action.clone(),
                target: target.clone(),
                value: value.clone(),
                next: next.as_ref().map(|n| format!("{}{}", prefix, n)),
            }
        }
        DecisionNode::LlmFallback { prompt, schema } => {
            DecisionNode::LlmFallback { prompt: prompt.clone(), schema: schema.clone() }
        }
        DecisionNode::Intrinsic { function, input_variable, output_variable, next } => {
            DecisionNode::Intrinsic {
                function: function.clone(),
                input_variable: input_variable.clone(),
                output_variable: output_variable.clone(),
                next: format!("{}{}", prefix, next),
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PIPE — stream multiple inputs through a chain, collect results
// ═══════════════════════════════════════════════════════════════════════════

/// Result of piping multiple inputs through a chain
#[derive(Debug, Clone, Serialize)]
pub struct PipeResult {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<PipeEntry>,
    pub total_duration_us: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PipeEntry {
    pub index: usize,
    pub input: HashMap<String, Value>,
    pub output: HashMap<String, Value>,
    pub success: bool,
    pub duration_us: u64,
}

/// Pipe multiple inputs through a microgram or chain
pub fn pipe(mg: &Microgram, inputs: &[HashMap<String, Value>]) -> PipeResult {
    let mut results = Vec::with_capacity(inputs.len());
    let mut succeeded = 0;
    let mut total_us = 0u64;

    for (i, input) in inputs.iter().enumerate() {
        let result = mg.run(input.clone());
        total_us += result.duration_us;
        if result.success { succeeded += 1; }

        results.push(PipeEntry {
            index: i,
            input: input.clone(),
            output: result.output,
            success: result.success,
            duration_us: result.duration_us,
        });
    }

    PipeResult {
        total: inputs.len(),
        succeeded,
        failed: inputs.len() - succeeded,
        results,
        total_duration_us: total_us,
    }
}

/// Pipe multiple inputs through a chain of micrograms
pub fn pipe_chain(
    dir: &Path,
    names: &[&str],
    inputs: &[HashMap<String, Value>],
) -> Result<PipeResult, String> {
    let all = load_all(dir)?;
    let mut ordered = Vec::with_capacity(names.len());
    for name in names {
        match all.iter().find(|mg| mg.name == *name) {
            Some(mg) => ordered.push(mg.clone()),
            None => return Err(format!("Microgram '{}' not found in {}", name, dir.display())),
        }
    }

    let mut results = Vec::with_capacity(inputs.len());
    let mut succeeded = 0;
    let mut total_us = 0u64;

    for (i, input) in inputs.iter().enumerate() {
        let chain_result = chain(&ordered, input.clone());
        total_us += chain_result.total_duration_us;
        if chain_result.success { succeeded += 1; }

        results.push(PipeEntry {
            index: i,
            input: input.clone(),
            output: chain_result.final_output,
            success: chain_result.success,
            duration_us: chain_result.total_duration_us,
        });
    }

    Ok(PipeResult {
        total: inputs.len(),
        succeeded,
        failed: inputs.len() - succeeded,
        results,
        total_duration_us: total_us,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// SNAPSHOT — save/restore ecosystem state as a single JSON file
// ═══════════════════════════════════════════════════════════════════════════

/// Ecosystem snapshot — serializable state of all micrograms + metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub timestamp: String,
    pub micrograms: Vec<Microgram>,
    pub total_tests: usize,
    pub all_pass: bool,
}

/// Save ecosystem state to a JSON snapshot file
pub fn snapshot_save(dir: &Path, out: &Path) -> Result<Snapshot, String> {
    let all = load_all(dir)?;
    let mut total_tests = 0;
    let mut all_pass = true;
    for mg in &all {
        let r = mg.test();
        total_tests += r.total;
        if r.failed > 0 { all_pass = false; }
    }

    let snap = Snapshot {
        timestamp: chrono::Utc::now().to_rfc3339(),
        micrograms: all,
        total_tests,
        all_pass,
    };

    let json = serde_json::to_string_pretty(&snap)
        .map_err(|e| format!("Serialize: {e}"))?;
    std::fs::write(out, json).map_err(|e| format!("Write: {e}"))?;
    Ok(snap)
}

/// Restore ecosystem from a snapshot file
pub fn snapshot_restore(snap_path: &Path, dir: &Path) -> Result<usize, String> {
    let content = std::fs::read_to_string(snap_path)
        .map_err(|e| format!("Read: {e}"))?;
    let snap: Snapshot = serde_json::from_str(&content)
        .map_err(|e| format!("Parse: {e}"))?;

    if !dir.exists() {
        std::fs::create_dir_all(dir).map_err(|e| format!("Mkdir: {e}"))?;
    }

    let mut count = 0;
    for mg in &snap.micrograms {
        let yaml = serde_yaml::to_string(mg).map_err(|e| format!("YAML: {e}"))?;
        let path = dir.join(format!("{}.yaml", mg.name));
        std::fs::write(&path, yaml).map_err(|e| format!("Write: {e}"))?;
        count += 1;
    }

    Ok(count)
}

// ═══════════════════════════════════════════════════════════════════════════
// STRESS — fuzz micrograms with random inputs
// ═══════════════════════════════════════════════════════════════════════════

/// Stress test result
#[derive(Debug, Clone, Serialize)]
pub struct StressResult {
    pub name: String,
    pub iterations: usize,
    pub succeeded: usize,
    pub errored: usize,
    pub min_us: u64,
    pub max_us: u64,
    pub avg_us: f64,
    pub error_inputs: Vec<HashMap<String, Value>>,
}

/// Stress test a microgram with random integer inputs
pub fn stress(mg: &Microgram, iterations: usize, seed: u64) -> StressResult {
    let vars = input_variables(mg);
    let mut succeeded = 0;
    let mut errored = 0;
    let mut timings = Vec::with_capacity(iterations);
    let mut error_inputs = Vec::new();

    // Simple LCG PRNG — no external dependency needed
    let mut rng_state = seed;
    let mut next_rand = || -> i64 {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        // Map to range [-1000, 1000]
        ((rng_state >> 33) as i64) % 2001 - 1000
    };

    for _ in 0..iterations {
        let mut input = HashMap::new();
        for var in &vars {
            input.insert(var.clone(), Value::Int(next_rand()));
        }

        let result = mg.run(input.clone());
        timings.push(result.duration_us);

        if result.success {
            succeeded += 1;
        } else {
            errored += 1;
            if error_inputs.len() < 10 {
                error_inputs.push(input);
            }
        }
    }

    timings.sort();
    let min_us = *timings.first().unwrap_or(&0);
    let max_us = *timings.last().unwrap_or(&0);
    let avg_us = if timings.is_empty() { 0.0 } else {
        timings.iter().sum::<u64>() as f64 / timings.len() as f64
    };

    StressResult {
        name: mg.name.clone(),
        iterations,
        succeeded,
        errored,
        min_us,
        max_us,
        avg_us,
        error_inputs,
    }
}

/// Stress all micrograms in a directory
pub fn stress_all(dir: &Path, iterations: usize, seed: u64) -> Result<Vec<StressResult>, String> {
    let all = load_all(dir)?;
    Ok(all.iter().enumerate().map(|(i, mg)| {
        stress(mg, iterations, seed.wrapping_add(i as u64))
    }).collect())
}

// ═══════════════════════════════════════════════════════════════════════════
// TRANSFORM — map/filter/reduce over pipe results
// ═══════════════════════════════════════════════════════════════════════════

/// Filter pipe results by a field condition
pub fn filter_results(results: &PipeResult, field: &str, op: &str, threshold: &Value) -> PipeResult {
    let filtered: Vec<PipeEntry> = results.results.iter().filter(|entry| {
        match entry.output.get(field) {
            Some(val) => match op {
                "eq" => val == threshold,
                "neq" => val != threshold,
                "gt" => cmp_values(val, threshold) == Some(std::cmp::Ordering::Greater),
                "gte" => matches!(cmp_values(val, threshold), Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)),
                "lt" => cmp_values(val, threshold) == Some(std::cmp::Ordering::Less),
                "lte" => matches!(cmp_values(val, threshold), Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)),
                _ => false,
            }
            None => false,
        }
    }).cloned().collect();

    let succeeded = filtered.iter().filter(|e| e.success).count();
    let total_us = filtered.iter().map(|e| e.duration_us).sum();

    PipeResult {
        total: filtered.len(),
        succeeded,
        failed: filtered.len() - succeeded,
        results: filtered,
        total_duration_us: total_us,
    }
}

/// Extract a single field from all pipe results
pub fn map_field(results: &PipeResult, field: &str) -> Vec<Option<Value>> {
    results.results.iter().map(|entry| {
        entry.output.get(field).cloned()
    }).collect()
}

/// Count occurrences of each unique value for a field
pub fn reduce_count(results: &PipeResult, field: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for entry in &results.results {
        let key = entry.output.get(field)
            .map(|v| format!("{:?}", v))
            .unwrap_or_else(|| "null".to_string());
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

/// Compare two Values for ordering (ints and floats only)
fn cmp_values(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Some(x.cmp(y)),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y),
        (Value::Int(x), Value::Float(y)) => (*x as f64).partial_cmp(y),
        (Value::Float(x), Value::Int(y)) => x.partial_cmp(&(*y as f64)),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MATRIX — cross-test every microgram against every other's test cases
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize)]
pub struct MatrixCell {
    pub runner: String,
    pub test_from: String,
    pub total: usize,
    pub executed: usize,  // ran without error
    pub matched: usize,   // output matched expected
}

#[derive(Debug, Clone, Serialize)]
pub struct MatrixResult {
    pub cells: Vec<MatrixCell>,
    pub total_runs: usize,
    pub cross_matches: usize,  // unexpected matches between different micrograms
}

/// Run every microgram against every other microgram's test inputs
pub fn matrix(dir: &Path) -> Result<MatrixResult, String> {
    let all = load_all(dir)?;
    let mut cells = Vec::new();
    let mut total_runs = 0;
    let mut cross_matches = 0;

    for runner in &all {
        for donor in &all {
            let mut executed = 0;
            let mut matched = 0;

            for test in &donor.tests {
                let result = runner.run(test.input.clone());
                total_runs += 1;
                if result.success { executed += 1; }

                // Check if output matches donor's expected output
                let matches = test.expect.iter().all(|(k, v)| {
                    result.output.get(k) == Some(v)
                });
                if matches { matched += 1; }
                if matches && runner.name != donor.name { cross_matches += 1; }
            }

            cells.push(MatrixCell {
                runner: runner.name.clone(),
                test_from: donor.name.clone(),
                total: donor.tests.len(),
                executed,
                matched,
            });
        }
    }

    Ok(MatrixResult { cells, total_runs, cross_matches })
}

// ═══════════════════════════════════════════════════════════════════════════
// COVERAGE — decision path coverage analysis
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize)]
pub struct CoverageResult {
    pub name: String,
    pub total_nodes: usize,
    pub covered_nodes: usize,
    pub coverage_pct: f64,
    pub uncovered: Vec<String>,
    pub paths_taken: Vec<Vec<String>>,
}

/// Analyze path coverage of a microgram's self-tests
pub fn coverage(mg: &Microgram) -> CoverageResult {
    let total_nodes = mg.tree.nodes.len();
    let mut visited: HashMap<String, bool> = mg.tree.nodes.keys()
        .map(|k| (k.clone(), false))
        .collect();
    let mut paths_taken = Vec::new();

    for test in &mg.tests {
        let result = mg.run(test.input.clone());
        for node_name in &result.path {
            visited.insert(node_name.clone(), true);
        }
        paths_taken.push(result.path);
    }

    let covered_nodes = visited.values().filter(|&&v| v).count();
    let uncovered: Vec<String> = visited.iter()
        .filter(|(_, v)| !*v)
        .map(|(k, _)| k.clone())
        .collect();

    let coverage_pct = if total_nodes == 0 { 100.0 } else {
        (covered_nodes as f64 / total_nodes as f64) * 100.0
    };

    CoverageResult {
        name: mg.name.clone(),
        total_nodes,
        covered_nodes,
        coverage_pct,
        uncovered,
        paths_taken,
    }
}

/// Coverage for all micrograms in a directory
pub fn coverage_all(dir: &Path) -> Result<Vec<CoverageResult>, String> {
    let all = load_all(dir)?;
    Ok(all.iter().map(|mg| coverage(mg)).collect())
}

// ═══════════════════════════════════════════════════════════════════════════
// CLONE — deep copy with mutations
// ═══════════════════════════════════════════════════════════════════════════

/// Clone a microgram with a mutated threshold
pub fn clone_mutated(
    mg: &Microgram,
    new_name: &str,
    threshold_delta: i64,
) -> Microgram {
    let mut cloned = mg.clone();
    cloned.name = new_name.to_string();

    // Mutate: shift threshold values in condition nodes
    for node in cloned.tree.nodes.values_mut() {
        if let DecisionNode::Condition { value: Some(Value::Int(n)), .. } = node {
            *n += threshold_delta;
        }
    }

    // Regenerate tests from the mutated tree by running the existing test inputs
    cloned.tests = mg.tests.iter().map(|test| {
        let result = cloned.run(test.input.clone());
        MicrogramTest {
            input: test.input.clone(),
            expect: result.output,
        }
    }).collect();

    cloned
}

// ═══════════════════════════════════════════════════════════════════════════
// SHRINK — minimize inputs that produce a target output
// ═══════════════════════════════════════════════════════════════════════════

/// Try to find the minimal input that still produces the same output
pub fn shrink(
    mg: &Microgram,
    input: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let target_result = mg.run(input.clone());
    let target_output = target_result.output;

    // Strategy: for each integer variable, binary search toward 0
    let mut minimal = input.clone();
    let required_vars = input_variables(mg);

    for var in &required_vars {
        if let Some(Value::Int(original)) = minimal.get(var).cloned() {
            // Binary search between 0 and original
            let (mut lo, mut hi) = if original >= 0 { (0, original) } else { (original, 0) };

            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                let mut test_input = minimal.clone();
                test_input.insert(var.clone(), Value::Int(mid));
                let result = mg.run(test_input);

                if result.output == target_output {
                    // Same output at mid — shrink toward 0
                    if original >= 0 { hi = mid; } else { lo = mid + 1; }
                } else {
                    // Different output — the boundary is between mid and original
                    if original >= 0 { lo = mid + 1; } else { hi = mid; }
                }
            }

            minimal.insert(var.clone(), Value::Int(lo));
        }
    }

    minimal
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

        let result = chain(&[gate, label], input);

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

        let result = chain(&[gate, label], input);

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
            assert!(r.min_us <= r.avg_us as u64 + 1);
            assert!(r.avg_us <= r.max_us as f64 + 1.0);
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
}
