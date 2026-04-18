use super::chain::{ChainResult, chain_by_names};
use super::{Microgram, load_all};
use crate::modules::decision_engine::{DecisionNode, Value};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

/// (name, outputs, inputs, aliases) — used by compose() for alias-aware chain building
type ProducerEntry = (String, Vec<String>, Vec<String>, HashMap<String, String>);

/// Goal for composition: what output fields are required
#[derive(Debug, Clone)]
pub struct CompositionGoal {
    pub required_outputs: Vec<String>, // field names that must exist in final output
    pub initial_input: HashMap<String, Value>,
}

/// Result of attempting to compose a chain
#[derive(Debug, Clone, Serialize)]
pub struct CompositionPlan {
    pub feasible: bool,
    pub chain: Vec<String>,    // ordered microgram names
    pub coverage: Vec<String>, // which required outputs are covered
    pub missing: Vec<String>,  // which required outputs can't be produced
}

/// Analyze which output fields a microgram can produce
pub(crate) fn output_fields(mg: &Microgram) -> Vec<String> {
    let mut fields = Vec::new();
    for node in mg.tree.nodes.values() {
        if let DecisionNode::Return {
            value: Value::Object(map),
        } = node
        {
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
pub(crate) fn input_variables(mg: &Microgram) -> Vec<String> {
    let mut vars = Vec::new();
    for node in mg.tree.nodes.values() {
        if let DecisionNode::Condition { variable, .. } = node
            && !vars.contains(variable)
        {
            vars.push(variable.clone());
        }
    }
    vars
}

/// Compose a chain from available micrograms to meet a goal
pub fn compose(dir: &Path, goal: &CompositionGoal) -> Result<CompositionPlan, String> {
    let all = load_all(dir)?;

    // Index: which microgram produces which output fields (+ aliases)
    let producers: Vec<ProducerEntry> = all
        .iter()
        .map(|mg| {
            let aliases = mg
                .interface
                .as_ref()
                .map(|iface| iface.aliases.clone())
                .unwrap_or_default();
            (
                mg.name.clone(),
                output_fields(mg),
                input_variables(mg),
                aliases,
            )
        })
        .collect();

    // Greedy forward chain: start with available inputs, find micrograms that can run,
    // add their outputs, repeat until goal is met or no progress.
    // Alias-aware: a microgram can run if its inputs are satisfied directly or via aliases.
    let mut available: Vec<String> = goal.initial_input.keys().cloned().collect();
    let mut chain_names: Vec<String> = Vec::new();
    let mut used: Vec<bool> = vec![false; producers.len()];

    for _ in 0..producers.len() {
        let mut made_progress = false;
        for (i, (name, outputs, inputs, aliases)) in producers.iter().enumerate() {
            if used[i] {
                continue;
            }
            // Can this microgram run with currently available variables?
            // Check direct match OR alias resolution (alias in available → canonical input)
            let can_run = inputs.iter().all(|v| {
                if available.contains(v) {
                    return true;
                }
                // Check if any available name is aliased to this input
                aliases
                    .iter()
                    .any(|(alias, canonical)| canonical == v && available.contains(alias))
            });
            if !can_run {
                continue;
            }
            // Does it produce something we still need?
            let produces_needed = outputs
                .iter()
                .any(|o| goal.required_outputs.contains(o) && !available.contains(o));
            // Or does it produce something that unlocks another microgram?
            let produces_useful = outputs.iter().any(|o| !available.contains(o));

            if produces_needed || produces_useful {
                chain_names.push(name.clone());
                for o in outputs {
                    if !available.contains(o) {
                        available.push(o.clone());
                    }
                }
                // Alias expansion: for each new output, check if any mcg's
                // alias maps this output name to a canonical name. If so, that
                // canonical name is now effectively available (chain runtime
                // will remap it via apply_aliases).
                for o in outputs {
                    for (_, _, _, other_aliases) in &producers {
                        for (alias, canonical) in other_aliases {
                            if alias == o && !available.contains(canonical) {
                                available.push(canonical.clone());
                            }
                        }
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
        let input = mg
            .tests
            .first()
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
        #[allow(clippy::as_conversions)] // usize→f64 for ratio
        let timing_count = timings.len() as f64;
        #[allow(clippy::as_conversions)] // u64→f64 for ratio
        let timing_sum = timings.iter().sum::<u64>() as f64;
        let avg_us = timing_sum / timing_count;
        #[allow(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )] // f64→usize for index, value always non-negative
        let p95_idx = (timing_count * 0.95) as usize;
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
pub fn auto_execute(dir: &Path, goal: &CompositionGoal) -> Result<AutoResult, String> {
    let start = std::time::Instant::now();

    // Step 1: Compose
    let plan = compose(dir, goal)?;

    if !plan.feasible {
        return Ok(AutoResult {
            plan,
            execution: None,
            verified: false,
            duration_us: u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX),
        });
    }

    // Step 2: Execute the composed chain
    let names: Vec<&str> = plan.chain.iter().map(|s| s.as_str()).collect();
    let chain_result = chain_by_names(dir, &names, goal.initial_input.clone())?;

    // Step 3: Verify — all required outputs present in ANY step's output
    let verified = plan.coverage.iter().all(|field| {
        chain_result
            .steps
            .iter()
            .any(|step| step.output.contains_key(field))
    });

    let duration_us = u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX);

    Ok(AutoResult {
        plan,
        execution: Some(chain_result),
        verified,
        duration_us,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// ALIAS RESOLUTION — resolve field name aliases across microgram boundaries
// ═══════════════════════════════════════════════════════════════════════════

/// Check if output field `output_name` from A can feed into B, considering aliases.
/// Returns the canonical input field name in B if a match is found.
///
/// Resolution order:
/// 1. Direct match: output_name exists in B's inputs
/// 2. B declares alias: output_name is aliased to a canonical name in B's inputs
/// 3. A declares alias: A's canonical output has an alias that matches B's input
fn resolve_alias(
    output_name: &str,
    b_inputs: &[String],
    a_aliases: &HashMap<String, String>,
    b_aliases: &HashMap<String, String>,
) -> Option<String> {
    // 1. Direct match
    if b_inputs.iter().any(|i| i == output_name) {
        return Some(output_name.to_string());
    }
    // 2. B declares alias: output_name → canonical input in B
    if let Some(canonical) = b_aliases
        .get(output_name)
        .filter(|c| b_inputs.iter().any(|i| i == *c))
    {
        return Some(canonical.clone());
    }
    // 3. A declares alias: find if any alias of output_name matches a B input
    for (alias, canonical) in a_aliases {
        if canonical == output_name && b_inputs.iter().any(|i| i == alias) {
            return Some(alias.clone());
        }
    }
    None
}

/// Check if any output from A can feed any input of B (with alias resolution).
pub fn can_feed_with_aliases(
    a_outputs: &[String],
    b_inputs: &[String],
    a_aliases: &HashMap<String, String>,
    b_aliases: &HashMap<String, String>,
) -> bool {
    a_outputs
        .iter()
        .any(|o| resolve_alias(o, b_inputs, a_aliases, b_aliases).is_some())
}
