use super::compose::input_variables;
use super::{Microgram, load_all};
use crate::modules::decision_engine::Value;
use serde::{Deserialize, Serialize};
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
        rng_state = rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        // Map to range [-1000, 1000]
        #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
        // u64→i64 intentional for PRNG range mapping
        let val = (rng_state >> 33) as i64;
        val % 2001 - 1000
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
    let avg_us = if timings.is_empty() {
        0.0
    } else {
        #[allow(clippy::as_conversions)] // u64→f64 and usize→f64 for averaging
        let avg = timings.iter().sum::<u64>() as f64 / timings.len() as f64;
        avg
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

/// Stress test a microgram with type-aware random inputs.
///
/// When the microgram has a declared interface, generates values matching
/// declared types (float with edge cases, bool, string). Falls back to
/// random integers for micrograms without interfaces.
pub fn stress_typed(mg: &Microgram, iterations: usize, seed: u64) -> StressResult {
    // If no interface, fall back to integer-only stress
    let Some(ref iface) = mg.interface else {
        return stress(mg, iterations, seed);
    };

    let mut succeeded = 0;
    let mut errored = 0;
    let mut timings = Vec::with_capacity(iterations);
    let mut error_inputs = Vec::new();

    let mut rng_state = seed;
    let mut next_u64 = || -> u64 {
        rng_state = rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        rng_state >> 33
    };

    for _ in 0..iterations {
        let mut input = HashMap::new();
        for (field_name, field_spec) in &iface.inputs {
            let val = match field_spec.field_type.as_str() {
                "bool" => Value::Bool(next_u64() % 2 == 0),
                "float" => {
                    let choice = next_u64() % 100;
                    if choice < 3 {
                        Value::Float(f64::NAN)
                    } else if choice < 5 {
                        Value::Float(f64::INFINITY)
                    } else if choice < 7 {
                        Value::Float(f64::NEG_INFINITY)
                    } else if choice < 10 {
                        Value::Float(0.0)
                    } else {
                        #[allow(clippy::as_conversions)]
                        let v = (next_u64() % 200001) as f64 / 100.0 - 1000.0;
                        Value::Float(v)
                    }
                }
                "int" => {
                    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
                    let v = (next_u64() as i64) % 2001 - 1000;
                    Value::Int(v)
                }
                "string" => {
                    let len = (next_u64() % 20) + 1;
                    let s: String = (0..len)
                        .map(|_| {
                            #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
                            let c = (b'a' + (next_u64() % 26) as u8) as char;
                            c
                        })
                        .collect();
                    Value::String(s)
                }
                _ => {
                    // Unknown type: use integer fallback
                    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
                    let v = (next_u64() as i64) % 2001 - 1000;
                    Value::Int(v)
                }
            };
            input.insert(field_name.clone(), val);
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
    let avg_us = if timings.is_empty() {
        0.0
    } else {
        #[allow(clippy::as_conversions)]
        let avg = timings.iter().sum::<u64>() as f64 / timings.len() as f64;
        avg
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

/// Stress test result with boundary validation metrics
#[derive(Debug, Clone, Serialize)]
pub struct ValidatedStressResult {
    pub base: StressResult,
    /// How many iterations had ingress validation errors
    pub ingress_failures: usize,
    /// How many iterations had egress validation errors
    pub egress_failures: usize,
}

/// Stress test a microgram using `run_validated()` to exercise boundary checks.
///
/// Uses type-aware input generation and validates both ingress and egress at every
/// iteration. Reports boundary failures separately from execution failures.
pub fn stress_validated(mg: &Microgram, iterations: usize, seed: u64) -> ValidatedStressResult {
    let Some(ref iface) = mg.interface else {
        // No interface: run_validated behaves like run(), so metrics are trivially 0
        return ValidatedStressResult {
            base: stress(mg, iterations, seed),
            ingress_failures: 0,
            egress_failures: 0,
        };
    };

    let mut succeeded = 0;
    let mut errored = 0;
    let mut timings = Vec::with_capacity(iterations);
    let mut error_inputs = Vec::new();
    let mut ingress_failures = 0;
    let mut egress_failures = 0;

    let mut rng_state = seed;
    let mut next_u64 = || -> u64 {
        rng_state = rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        rng_state >> 33
    };

    for _ in 0..iterations {
        let mut input = HashMap::new();
        for (field_name, field_spec) in &iface.inputs {
            let val = match field_spec.field_type.as_str() {
                "bool" => Value::Bool(next_u64() % 2 == 0),
                "float" => {
                    #[allow(clippy::as_conversions)]
                    let v = (next_u64() % 200001) as f64 / 100.0 - 1000.0;
                    Value::Float(v)
                }
                "int" => {
                    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
                    let v = (next_u64() as i64) % 2001 - 1000;
                    Value::Int(v)
                }
                "string" => {
                    let len = (next_u64() % 10) + 1;
                    let s: String = (0..len)
                        .map(|_| {
                            #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
                            let c = (b'a' + (next_u64() % 26) as u8) as char;
                            c
                        })
                        .collect();
                    Value::String(s)
                }
                _ => {
                    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
                    let v = (next_u64() as i64) % 2001 - 1000;
                    Value::Int(v)
                }
            };
            input.insert(field_name.clone(), val);
        }

        let vr = mg.run_validated(input.clone());
        timings.push(vr.result.duration_us);

        if !vr.ingress_errors.is_empty() {
            ingress_failures += 1;
        }
        if !vr.egress_errors.is_empty() {
            egress_failures += 1;
        }

        if vr.is_valid() {
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
    let avg_us = if timings.is_empty() {
        0.0
    } else {
        #[allow(clippy::as_conversions)]
        let avg = timings.iter().sum::<u64>() as f64 / timings.len() as f64;
        avg
    };

    ValidatedStressResult {
        base: StressResult {
            name: mg.name.clone(),
            iterations,
            succeeded,
            errored,
            min_us,
            max_us,
            avg_us,
            error_inputs,
        },
        ingress_failures,
        egress_failures,
    }
}

/// Stress all micrograms in a directory with type-aware inputs
pub fn stress_all_typed(
    dir: &Path,
    iterations: usize,
    seed: u64,
) -> Result<Vec<StressResult>, String> {
    let all = load_all(dir)?;
    Ok(all
        .iter()
        .enumerate()
        .map(|(i, mg)| {
            #[allow(clippy::as_conversions)]
            let offset = i as u64;
            stress_typed(mg, iterations, seed.wrapping_add(offset))
        })
        .collect())
}

/// A performance baseline entry for one microgram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineEntry {
    pub name: String,
    pub avg_us: f64,
    pub max_us: u64,
    pub iterations: usize,
}

/// Result of comparing stress results against a baseline
#[derive(Debug, Clone, Serialize)]
pub struct RegressionResult {
    pub regressions: Vec<RegressionEntry>,
    pub total_checked: usize,
}

/// A single regression finding
#[derive(Debug, Clone, Serialize)]
pub struct RegressionEntry {
    pub name: String,
    pub baseline_avg_us: f64,
    pub current_avg_us: f64,
    pub regression_pct: f64,
}

/// Save stress results as a performance baseline JSON file
pub fn save_baseline(results: &[StressResult], path: &Path) -> Result<(), String> {
    let entries: Vec<BaselineEntry> = results
        .iter()
        .map(|r| BaselineEntry {
            name: r.name.clone(),
            avg_us: r.avg_us,
            max_us: r.max_us,
            iterations: r.iterations,
        })
        .collect();
    let json =
        serde_json::to_string_pretty(&entries).map_err(|e| format!("Serialize baseline: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("Write baseline {}: {e}", path.display()))
}

/// Load a performance baseline from JSON
pub fn load_baseline(path: &Path) -> Result<Vec<BaselineEntry>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Read baseline {}: {e}", path.display()))?;
    serde_json::from_str(&content).map_err(|e| format!("Parse baseline: {e}"))
}

/// Compare stress results against a baseline, flagging regressions > threshold_pct
pub fn check_regression(
    results: &[StressResult],
    baseline: &[BaselineEntry],
    threshold_pct: f64,
) -> RegressionResult {
    let mut regressions = Vec::new();
    let mut total_checked = 0;

    for result in results {
        if let Some(base) = baseline.iter().find(|b| b.name == result.name) {
            total_checked += 1;
            if base.avg_us > 0.0 {
                let pct = (result.avg_us - base.avg_us) / base.avg_us * 100.0;
                if pct > threshold_pct {
                    regressions.push(RegressionEntry {
                        name: result.name.clone(),
                        baseline_avg_us: base.avg_us,
                        current_avg_us: result.avg_us,
                        regression_pct: pct,
                    });
                }
            }
        }
    }

    RegressionResult {
        regressions,
        total_checked,
    }
}

/// Stress all micrograms in a directory
pub fn stress_all(dir: &Path, iterations: usize, seed: u64) -> Result<Vec<StressResult>, String> {
    let all = load_all(dir)?;
    Ok(all
        .iter()
        .enumerate()
        .map(|(i, mg)| {
            #[allow(clippy::as_conversions)] // usize→u64 widening, safe on 64-bit
            let offset = i as u64;
            stress(mg, iterations, seed.wrapping_add(offset))
        })
        .collect())
}
