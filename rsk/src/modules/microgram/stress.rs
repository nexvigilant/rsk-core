use crate::modules::decision_engine::Value;
use super::{Microgram, load_all};
use super::compose::input_variables;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

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
