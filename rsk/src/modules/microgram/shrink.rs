use crate::modules::decision_engine::Value;
use super::Microgram;
use super::compose::input_variables;
use std::collections::HashMap;

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
