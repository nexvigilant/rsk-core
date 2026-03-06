use crate::modules::decision_engine::Value;
use super::{Microgram, MicrogramResult, load_all};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

/// Apply alias remapping: for each alias declared by the target microgram,
/// if the input contains the alias name but not the canonical name, copy the value.
/// This bridges the naming gap at runtime — catalog discovers the connection,
/// this function makes the data flow through it.
fn apply_aliases(input: &mut HashMap<String, Value>, target: &Microgram) {
    let Some(iface) = &target.interface else { return };
    for (alias, canonical) in &iface.aliases {
        if input.contains_key(alias) && !input.contains_key(canonical) {
            if let Some(val) = input.get(alias).cloned() {
                input.insert(canonical.clone(), val);
            }
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
pub fn chain(micrograms: &[Microgram], initial_input: HashMap<String, Value>) -> ChainResult {
    let mut current_input = initial_input;
    let mut steps = Vec::with_capacity(micrograms.len());
    let mut total_us = 0u64;

    for mg in micrograms {
        // Remap aliased fields before running this microgram
        apply_aliases(&mut current_input, mg);

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
pub fn chain_resilient(
    micrograms: &[Microgram],
    initial_input: HashMap<String, Value>,
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

        let result = mg.run(remapped_input);
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
pub fn chain_accumulate(micrograms: &[Microgram], initial_input: HashMap<String, Value>) -> ChainResult {
    let mut context = initial_input;
    let mut steps = Vec::with_capacity(micrograms.len());
    let mut total_us = 0u64;

    for mg in micrograms {
        let mut step_input = context.clone();
        apply_aliases(&mut step_input, mg);

        let result = mg.run(step_input);
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

    Ok(chain_accumulate(&ordered, initial_input))
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

    Ok(chain_resilient(&ordered, initial_input))
}
