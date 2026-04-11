use crate::modules::decision_engine::Value;
use super::{Microgram, MicrogramResult, load_all};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

/// Apply alias remapping: for each alias declared by the target microgram,
/// if the input contains the alias name but not the canonical name, copy the value.
/// This bridges the naming gap at runtime — catalog discovers the connection,
/// this function makes the data flow through it.
pub fn apply_aliases(input: &mut HashMap<String, Value>, target: &Microgram) {
    let Some(iface) = &target.interface else { return };
    for (alias, canonical) in &iface.aliases {
        if input.contains_key(alias) && !input.contains_key(canonical)
            && let Some(val) = input.get(alias).cloned()
        {
            input.insert(canonical.clone(), val);
        }
    }
}

/// Result of chaining micrograms
#[derive(Debug, Clone, Serialize)]
pub struct ChainResult {
    pub success: bool,
    pub steps: Vec<MicrogramResult>,
    pub final_output: HashMap<String, Value>,
    pub total_duration_us: u64,
}

/// Chain multiple micrograms: output of step N becomes input of step N+1.
/// Alias-aware: remaps field names between steps using declared aliases.
/// When `strict` is true, each step validates required inputs before execution.
pub fn chain(micrograms: &[Microgram], initial_input: HashMap<String, Value>, strict: bool) -> ChainResult {
    let mut current_input = initial_input;
    let mut steps = Vec::with_capacity(micrograms.len());
    let mut total_us = 0u64;

    for mg in micrograms {
        // Remap aliased fields before running this microgram
        apply_aliases(&mut current_input, mg);

        let result = if strict {
            mg.run_strict(current_input)
        } else {
            mg.run(current_input)
        };
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

    Ok(chain(&ordered, initial_input, false))
}

/// Chain execution status
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum ChainStatus {
    Complete,
    Partial,
    Failed,
}

/// Result of a resilient chain execution
#[derive(Debug, Clone, Serialize)]
pub struct ResilientChainResult {
    pub status: ChainStatus,
    pub steps: Vec<MicrogramResult>,
    pub final_output: HashMap<String, Value>,
    pub total_duration_us: u64,
    pub failed_steps: Vec<String>,
}

/// Chain micrograms resiliently: continue after failure, mark failed steps.
/// Alias-aware: remaps field names between steps using declared aliases.
/// When `strict` is true, each step validates required inputs before execution.
pub fn chain_resilient(
    micrograms: &[Microgram],
    initial_input: HashMap<String, Value>,
    strict: bool,
) -> ResilientChainResult {
    let mut current_input = initial_input;
    let mut steps = Vec::with_capacity(micrograms.len());
    let mut total_us = 0u64;
    let mut failed_steps = Vec::new();
    let mut last_good_output = current_input.clone();

    for mg in micrograms {
        // Remap aliased fields before running this microgram
        let mut remapped_input = current_input.clone();
        apply_aliases(&mut remapped_input, mg);

        let result = if strict {
            mg.run_strict(remapped_input)
        } else {
            mg.run(remapped_input)
        };
        total_us += result.duration_us;

        if result.success {
            last_good_output = result.output.clone();
            current_input = result.output.clone();
        } else {
            failed_steps.push(mg.name.clone());
            current_input = last_good_output.clone();
        }
        steps.push(result);
    }

    let status = if failed_steps.is_empty() {
        ChainStatus::Complete
    } else if failed_steps.len() == micrograms.len() {
        ChainStatus::Failed
    } else {
        ChainStatus::Partial
    };

    let final_output = steps
        .last()
        .map(|s| {
            if s.success {
                s.output.clone()
            } else {
                last_good_output.clone()
            }
        })
        .unwrap_or_default();

    ResilientChainResult {
        status,
        steps,
        final_output,
        total_duration_us: total_us,
        failed_steps,
    }
}

/// Chain with context accumulation: each step's output MERGES into the running
/// context rather than replacing it. Fields from earlier steps survive through
/// steps that don't reference them. This prevents data loss in long chains.
/// When `strict` is true, each step validates required inputs before execution.
pub fn chain_accumulate(micrograms: &[Microgram], initial_input: HashMap<String, Value>, strict: bool) -> ChainResult {
    let mut context = initial_input;
    let mut steps = Vec::with_capacity(micrograms.len());
    let mut total_us = 0u64;

    for mg in micrograms {
        let mut step_input = context.clone();
        apply_aliases(&mut step_input, mg);

        let result = if strict {
            mg.run_strict(step_input)
        } else {
            mg.run(step_input)
        };
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

        // ACCUMULATE: merge output into context — new keys added, existing keys updated
        for (k, v) in &result.output {
            context.insert(k.clone(), v.clone());
        }
        steps.push(result);
    }

    // Final output is the full accumulated context (minus internal fields)
    let final_output: HashMap<String, Value> = context
        .into_iter()
        .filter(|(k, _)| !k.starts_with('_'))
        .collect();

    ChainResult {
        success: true,
        steps,
        final_output,
        total_duration_us: total_us,
    }
}

/// Severity of a boundary error — determines whether the chain halts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BoundaryErrorSeverity {
    /// Missing required field — downstream WILL fail. Halt-worthy.
    Missing,
    /// Wrong type but field present — downstream MAY succeed. Warning.
    TypeMismatch,
}

/// Engine primitive that failed — mirrors primitive-failure-classifier.yaml
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EnginePrimitive {
    /// Schema validation failure (ingress or egress missing/type)
    Seal,
    /// Decision tree evaluation failure (wrong output, incomplete conversion)
    Convert,
    /// Chain sequencing failure (phase error)
    Time,
    /// Schema impedance mismatch between steps
    Lubricate,
    /// Interface contract violation (default/unknown)
    Transfer,
}

/// A structured boundary error with severity and primitive classification.
#[derive(Debug, Clone, Serialize)]
pub struct BoundaryError {
    pub step_index: usize,
    pub step_name: String,
    pub direction: String,
    pub message: String,
    pub severity: BoundaryErrorSeverity,
    /// Which engine primitive failed
    pub primitive: EnginePrimitive,
    /// Actionable fix recommendation
    pub recommendation: String,
}

/// Result of a validated chain execution
#[derive(Debug, Clone, Serialize)]
pub struct ValidatedChainResult {
    pub success: bool,
    pub steps: Vec<super::ValidatedResult>,
    pub final_output: HashMap<String, Value>,
    pub total_duration_us: u64,
    /// Cumulative ingress/egress errors across all steps (legacy, flat strings)
    pub boundary_errors: Vec<String>,
    /// Structured boundary errors with severity classification
    pub boundary_findings: Vec<BoundaryError>,
}

/// Chain micrograms with ingress/egress validation at every step.
///
/// Each step runs through `run_validated()`: ingress checks types against
/// declared interface before execution, egress checks output types after.
/// Micrograms without interfaces pass through unchanged (backward compatible).
///
/// On ingress failure: halts the chain (invalid input = can't proceed).
/// On egress failure: records the violation but continues (output exists, just wrong type).
/// When `accumulate` is true, uses accumulate semantics (merge outputs into context).
/// Classify an error message as Missing (halt-worthy) or TypeMismatch (warning).
fn classify_error_severity(err: &str) -> BoundaryErrorSeverity {
    if err.contains("Missing required") {
        BoundaryErrorSeverity::Missing
    } else {
        BoundaryErrorSeverity::TypeMismatch
    }
}

/// Classify a boundary error into engine primitive and recommendation.
/// Mirrors the logic of primitive-failure-classifier.yaml without YAML I/O.
fn classify_primitive(direction: &str, message: &str) -> (EnginePrimitive, &'static str) {
    match direction {
        "ingress" => {
            if message.contains("Missing required") {
                (EnginePrimitive::Seal, "Add required field to upstream output or insert adapter microgram")
            } else if message.contains("expected type") {
                (EnginePrimitive::Seal, "Schema types disagree \u{2014} add type-converting adapter")
            } else {
                (EnginePrimitive::Seal, "Ingress contract violation \u{2014} inspect upstream output schema")
            }
        }
        "egress" => {
            if message.contains("Missing required") {
                (EnginePrimitive::Convert, "Decision tree path doesn't produce required output \u{2014} add missing return nodes")
            } else if message.contains("expected type") {
                (EnginePrimitive::Convert, "Tree produces wrong type \u{2014} check return node values")
            } else {
                (EnginePrimitive::Convert, "Egress contract violation \u{2014} inspect return node schema")
            }
        }
        _ => (EnginePrimitive::Transfer, "Interface contract violation \u{2014} inspect chain topology"),
    }
}

pub fn chain_validated(
    micrograms: &[Microgram],
    initial_input: HashMap<String, Value>,
    accumulate: bool,
) -> ValidatedChainResult {
    let mut context = initial_input;
    let mut steps = Vec::with_capacity(micrograms.len());
    let mut total_us = 0u64;
    let mut boundary_errors = Vec::new();
    let mut boundary_findings = Vec::new();

    for (i, mg) in micrograms.iter().enumerate() {
        let mut step_input = context.clone();
        apply_aliases(&mut step_input, mg);

        let vr = mg.run_validated(step_input);
        total_us += vr.result.duration_us;

        // Collect boundary errors with step context, severity, and primitive classification
        for err in &vr.ingress_errors {
            boundary_errors.push(format!("Step {} ({}): ingress: {err}", i, mg.name));
            let (primitive, recommendation) = classify_primitive("ingress", err);
            boundary_findings.push(BoundaryError {
                step_index: i,
                step_name: mg.name.clone(),
                direction: "ingress".to_string(),
                message: err.clone(),
                severity: classify_error_severity(err),
                primitive,
                recommendation: recommendation.to_string(),
            });
        }
        for err in &vr.egress_errors {
            boundary_errors.push(format!("Step {} ({}): egress: {err}", i, mg.name));
            let (primitive, recommendation) = classify_primitive("egress", err);
            boundary_findings.push(BoundaryError {
                step_index: i,
                step_name: mg.name.clone(),
                direction: "egress".to_string(),
                message: err.clone(),
                severity: classify_error_severity(err),
                primitive,
                recommendation: recommendation.to_string(),
            });
        }

        // Ingress failure: halt the chain
        if !vr.ingress_errors.is_empty() {
            let final_output = vr.result.output.clone();
            steps.push(vr);
            return ValidatedChainResult {
                success: false,
                steps,
                final_output,
                total_duration_us: total_us,
                boundary_errors,
                boundary_findings,
            };
        }

        // Execution failure: halt the chain
        if !vr.result.success {
            let final_output = vr.result.output.clone();
            steps.push(vr);
            return ValidatedChainResult {
                success: false,
                steps,
                final_output,
                total_duration_us: total_us,
                boundary_errors,
                boundary_findings,
            };
        }

        // Egress failure with Missing severity: halt (downstream WILL fail)
        let has_missing_egress = vr.egress_errors.iter().any(|e|
            classify_error_severity(e) == BoundaryErrorSeverity::Missing
        );
        if has_missing_egress {
            let final_output = vr.result.output.clone();
            steps.push(vr);
            return ValidatedChainResult {
                success: false,
                steps,
                final_output,
                total_duration_us: total_us,
                boundary_errors,
                boundary_findings,
            };
        }
        // Egress TypeMismatch: record but continue (value exists, just wrong type)

        // Update context for next step
        if accumulate {
            for (k, v) in &vr.result.output {
                context.insert(k.clone(), v.clone());
            }
        } else {
            context = vr.result.output.clone();
        }

        steps.push(vr);
    }

    let final_output: HashMap<String, Value> = if accumulate {
        context.into_iter().filter(|(k, _)| !k.starts_with('_')).collect()
    } else {
        steps.last().map(|s| s.result.output.clone()).unwrap_or_default()
    };

    let success = boundary_errors.is_empty();
    ValidatedChainResult {
        success,
        steps,
        final_output,
        total_duration_us: total_us,
        boundary_errors,
        boundary_findings,
    }
}

/// Load micrograms by name and chain them with context accumulation
pub fn chain_accumulate_by_names(
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

    Ok(chain_accumulate(&ordered, initial_input, false))
}

/// Load micrograms by name and chain them resiliently
pub fn chain_resilient_by_names(
    dir: &Path,
    names: &[&str],
    initial_input: HashMap<String, Value>,
) -> Result<ResilientChainResult, String> {
    let all = load_all(dir)?;
    let mut ordered = Vec::with_capacity(names.len());

    for name in names {
        match all.iter().find(|mg| mg.name == *name) {
            Some(mg) => ordered.push(mg.clone()),
            None => return Err(format!("Microgram '{}' not found in {}", name, dir.display())),
        }
    }

    Ok(chain_resilient(&ordered, initial_input, false))
}

/// Result of a looped chain execution
#[derive(Debug, Clone, Serialize)]
pub struct LoopResult {
    /// Whether the loop completed (hit max_iterations or halt condition)
    pub success: bool,
    /// How many iterations executed
    pub iterations: usize,
    /// Why the loop stopped
    pub halt_reason: LoopHalt,
    /// Results from each iteration
    pub iteration_results: Vec<ChainResult>,
    /// Final accumulated state
    pub final_state: HashMap<String, Value>,
    /// Total duration across all iterations
    pub total_duration_us: u64,
    /// Per-iteration outputs (the trajectory)
    pub trajectory: Vec<HashMap<String, Value>>,
}

/// Why a loop halted
#[derive(Debug, Clone, Serialize)]
pub enum LoopHalt {
    /// Hit the maximum iteration count
    MaxIterations,
    /// A halt field matched the halt value
    HaltCondition { field: String, value: Value },
    /// A chain step failed
    ChainFailure { iteration: usize, step: String },
    /// Output converged (no change between iterations)
    Convergence { iteration: usize },
}

/// Loop a chain: run the same chain repeatedly, feeding each iteration's output
/// as the next iteration's input. Accumulates state across iterations.
///
/// Halt conditions (checked in order, first match stops):
/// 1. `halt_field` + `halt_value`: stop when output[field] == value
/// 2. Convergence: stop when output == previous output (ρ-fixpoint)
/// 3. `max_iterations`: hard ceiling (default: 100)
/// 4. Chain failure: any step fails → halt with ChainFailure
pub fn chain_loop(
    micrograms: &[Microgram],
    initial_input: HashMap<String, Value>,
    max_iterations: usize,
    halt_field: Option<&str>,
    halt_value: Option<&Value>,
    strict: bool,
) -> LoopResult {
    let max = if max_iterations == 0 { 100 } else { max_iterations };
    let mut current_input = initial_input;
    let mut iteration_results = Vec::new();
    let mut trajectory = Vec::new();
    let mut total_us = 0u64;
    let mut prev_output: Option<HashMap<String, Value>> = None;

    for i in 0..max {
        let result = chain_accumulate(micrograms, current_input.clone(), strict);
        total_us += result.total_duration_us;

        if !result.success {
            let failed_step = result.steps.iter()
                .find(|s| !s.success)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "unknown".to_string());
            trajectory.push(result.final_output.clone());
            let final_state = result.final_output.clone();
            iteration_results.push(result);
            return LoopResult {
                success: false,
                iterations: i + 1,
                halt_reason: LoopHalt::ChainFailure { iteration: i, step: failed_step },
                iteration_results,
                final_state,
                total_duration_us: total_us,
                trajectory,
            };
        }

        let output = result.final_output.clone();
        trajectory.push(output.clone());

        // Check halt condition
        if let (Some(field), Some(value)) = (halt_field, halt_value)
            && output.get(field).is_some_and(|actual| actual == value)
        {
            let final_state = output;
            iteration_results.push(result);
            return LoopResult {
                success: true,
                iterations: i + 1,
                halt_reason: LoopHalt::HaltCondition {
                    field: field.to_string(),
                    value: value.clone(),
                },
                iteration_results,
                final_state,
                total_duration_us: total_us,
                trajectory,
            };
        }

        // Check convergence (ρ-fixpoint)
        if prev_output.as_ref().is_some_and(|prev| &output == prev) {
            let final_state = output;
            iteration_results.push(result);
            return LoopResult {
                success: true,
                iterations: i + 1,
                halt_reason: LoopHalt::Convergence { iteration: i },
                iteration_results,
                final_state,
                total_duration_us: total_us,
                trajectory,
            };
        }

        prev_output = Some(output.clone());

        // Feed output back as input for next iteration
        // Merge: keep initial input fields that output didn't overwrite
        for (k, v) in &output {
            current_input.insert(k.clone(), v.clone());
        }

        iteration_results.push(result);
    }

    let final_state = trajectory.last().cloned().unwrap_or_default();
    LoopResult {
        success: true,
        iterations: max,
        halt_reason: LoopHalt::MaxIterations,
        iteration_results,
        final_state,
        total_duration_us: total_us,
        trajectory,
    }
}

/// Load micrograms by name and loop them
pub fn chain_loop_by_names(
    dir: &Path,
    names: &[&str],
    initial_input: HashMap<String, Value>,
    max_iterations: usize,
    halt_field: Option<&str>,
    halt_value: Option<&Value>,
) -> Result<LoopResult, String> {
    let all = load_all(dir)?;
    let mut ordered = Vec::with_capacity(names.len());

    for name in names {
        match all.iter().find(|mg| mg.name == *name) {
            Some(mg) => ordered.push(mg.clone()),
            None => return Err(format!("Microgram '{}' not found in {}", name, dir.display())),
        }
    }

    Ok(chain_loop(&ordered, initial_input, max_iterations, halt_field, halt_value, false))
}

// ═══════════════════════════════════════════════════════════════════════════
// Application 1: Path Snapshot Testing (macrotest pattern)
//
// Checks that chain execution traverses expected decision tree paths,
// not just that outputs match. Structural regressions (tree reorganization
// that produces the same output via a different route) are caught here.
// ═══════════════════════════════════════════════════════════════════════════

/// A path expectation for one step in a chain
#[derive(Debug, Clone, Serialize)]
pub struct PathMismatch {
    pub step_index: usize,
    pub step_name: String,
    pub expected_path: Vec<String>,
    pub actual_path: Vec<String>,
}

/// Result of path snapshot verification
#[derive(Debug, Clone, Serialize)]
pub struct PathSnapshotResult {
    pub success: bool,
    pub steps_checked: usize,
    pub mismatches: Vec<PathMismatch>,
}

/// Run a chain and verify that each step's decision path matches expectations.
/// `expected_paths` is a per-step list of expected node traversal paths.
/// If `expected_paths` has fewer entries than chain steps, uncovered steps are skipped.
pub fn chain_verify_paths(
    micrograms: &[Microgram],
    initial_input: HashMap<String, Value>,
    expected_paths: &[Vec<String>],
    strict: bool,
) -> (ChainResult, PathSnapshotResult) {
    let result = if strict {
        chain_accumulate(micrograms, initial_input, true)
    } else {
        chain_accumulate(micrograms, initial_input, false)
    };

    let mut mismatches = Vec::new();
    let steps_checked = expected_paths.len().min(result.steps.len());

    for (i, expected) in expected_paths.iter().enumerate() {
        if i >= result.steps.len() {
            break;
        }
        let actual = &result.steps[i].path;
        if actual != expected {
            mismatches.push(PathMismatch {
                step_index: i,
                step_name: result.steps[i].name.clone(),
                expected_path: expected.clone(),
                actual_path: actual.clone(),
            });
        }
    }

    let snapshot = PathSnapshotResult {
        success: mismatches.is_empty(),
        steps_checked,
        mismatches,
    };

    (result, snapshot)
}

// ═══════════════════════════════════════════════════════════════════════════
// Application 2: Multi-Error Chain Validation (proc_macro_error pattern)
//
// Dry-run validation across ALL chain steps, collecting every boundary
// violation instead of halting at the first. Reports what WOULD fail
// without executing any decision trees.
// ═══════════════════════════════════════════════════════════════════════════

/// A single step's validation finding
#[derive(Debug, Clone, Serialize)]
pub struct StepValidationError {
    pub step_index: usize,
    pub step_name: String,
    pub errors: Vec<String>,
}

/// A field name collision in accumulate mode — two steps produce the same output field.
#[derive(Debug, Clone, Serialize)]
pub struct FieldCollision {
    pub field_name: String,
    pub first_step: usize,
    pub first_step_name: String,
    pub second_step: usize,
    pub second_step_name: String,
}

/// Result of validating an entire chain without executing it
#[derive(Debug, Clone, Serialize)]
pub struct ChainValidationResult {
    pub valid: bool,
    pub steps_checked: usize,
    pub step_errors: Vec<StepValidationError>,
    pub total_errors: usize,
    /// Fields produced by multiple steps in accumulate mode (later overwrites earlier)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub field_collisions: Vec<FieldCollision>,
}

/// Validate all steps in a chain against an initial input WITHOUT executing.
/// Simulates data flow: step N's declared outputs become step N+1's input,
/// applying alias remapping at each boundary. Reports every step that would
/// reject its input under strict mode.
pub fn chain_validate_all(
    micrograms: &[Microgram],
    initial_input: &HashMap<String, Value>,
) -> ChainValidationResult {
    let mut step_errors = Vec::new();
    let mut simulated_input = initial_input.clone();
    let mut field_collisions = Vec::new();
    // Track which step first produced each output field: field_name → (step_index, step_name)
    let mut output_owners: HashMap<String, (usize, String)> = HashMap::new();

    for (i, mg) in micrograms.iter().enumerate() {
        // Apply alias remapping (same as runtime)
        apply_aliases(&mut simulated_input, mg);

        // Validate this step's required inputs
        let errors = mg.validate_input(&simulated_input);
        if !errors.is_empty() {
            step_errors.push(StepValidationError {
                step_index: i,
                step_name: mg.name.clone(),
                errors,
            });
        }

        // Simulate output: use declared output fields as the next step's input.
        // Merge (accumulate mode) so upstream fields survive.
        if let Some(ref iface) = mg.interface {
            for field_name in iface.outputs.keys() {
                // Detect field collisions: if another step already produces this field,
                // the later step will silently overwrite it in accumulate mode.
                if let Some((first_idx, first_name)) = output_owners.get(field_name) {
                    field_collisions.push(FieldCollision {
                        field_name: field_name.clone(),
                        first_step: *first_idx,
                        first_step_name: first_name.clone(),
                        second_step: i,
                        second_step_name: mg.name.clone(),
                    });
                } else {
                    output_owners.insert(field_name.clone(), (i, mg.name.clone()));
                }

                if !simulated_input.contains_key(field_name) {
                    simulated_input.insert(field_name.clone(), Value::Null);
                }
            }
        }
    }

    let total_errors: usize = step_errors.iter().map(|s| s.errors.len()).sum();
    ChainValidationResult {
        valid: step_errors.is_empty() && field_collisions.is_empty(),
        steps_checked: micrograms.len(),
        step_errors,
        total_errors,
        field_collisions,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Application 3: Live Egress Validation via Test Cases
//
// Runs each microgram's self-tests and uses ACTUAL outputs (not declared
// interface placeholders) to simulate chain data flow. Catches runtime
// egress violations that static chain_validate_all misses because it
// uses Null placeholders instead of real computed values.
// ═══════════════════════════════════════════════════════════════════════════

/// A single egress finding from live test execution
#[derive(Debug, Clone, Serialize)]
pub struct EgressFinding {
    pub step_index: usize,
    pub step_name: String,
    pub test_index: usize,
    pub errors: Vec<String>,
    pub severity: BoundaryErrorSeverity,
    pub primitive: EnginePrimitive,
    pub recommendation: String,
}

/// Result of validating chain egress using actual test-case outputs
#[derive(Debug, Clone, Serialize)]
pub struct ChainEgressValidationResult {
    pub valid: bool,
    pub steps_checked: usize,
    pub findings: Vec<EgressFinding>,
    pub total_findings: usize,
}

/// Validate chain egress by running each microgram's self-tests and checking
/// whether actual outputs satisfy declared output interface contracts.
///
/// Unlike `chain_validate_all` (which simulates with Null placeholders),
/// this function executes real decision trees to detect runtime egress
/// violations: paths that don't produce required outputs, or produce
/// values with unexpected types.
pub fn chain_validate_egress(
    micrograms: &[Microgram],
) -> ChainEgressValidationResult {
    let mut findings = Vec::new();

    for (i, mg) in micrograms.iter().enumerate() {
        // Skip micrograms without interface declarations — nothing to validate
        if mg.interface.is_none() {
            continue;
        }

        // Run each self-test and validate the output against declared interface
        for (t_idx, test) in mg.tests.iter().enumerate() {
            let result = mg.run(test.input.clone());
            let egress_errors = mg.validate_output(&result.output);

            if !egress_errors.is_empty() {
                // Classify by worst severity in the batch
                let severity = if egress_errors.iter().any(|e| e.contains("Missing required")) {
                    BoundaryErrorSeverity::Missing
                } else {
                    BoundaryErrorSeverity::TypeMismatch
                };
                // Classify the worst error for primitive assignment
                let worst_err = egress_errors.first().map(|e| e.as_str()).unwrap_or("");
                let (primitive, recommendation) = classify_primitive("egress", worst_err);

                findings.push(EgressFinding {
                    step_index: i,
                    step_name: mg.name.clone(),
                    test_index: t_idx,
                    errors: egress_errors,
                    severity,
                    primitive,
                    recommendation: recommendation.to_string(),
                });
            }
        }
    }

    let total_findings = findings.len();
    ChainEgressValidationResult {
        valid: findings.is_empty(),
        steps_checked: micrograms.len(),
        findings,
        total_findings,
    }
}
