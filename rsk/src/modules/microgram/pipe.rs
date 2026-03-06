use crate::modules::decision_engine::Value;
use super::{Microgram, load_all};
use super::chain::chain;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

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
