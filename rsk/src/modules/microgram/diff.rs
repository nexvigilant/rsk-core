use super::Microgram;
use super::compose::{can_feed_with_aliases, input_variables, output_fields};
use serde::Serialize;

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
    pub test_overlap: usize,      // tests with identical inputs
    pub behavior_matches: usize,  // of overlapping tests, how many produce same output
    pub behavior_diverges: usize, // of overlapping tests, how many produce different output
    pub compatible: bool,         // can they be chained? (left outputs ∩ right inputs ≠ ∅)
}

/// Compare two micrograms structurally and behaviorally
pub fn diff(a: &Microgram, b: &Microgram) -> DiffResult {
    let a_inputs = input_variables(a);
    let a_outputs = output_fields(a);
    let b_inputs = input_variables(b);
    let b_outputs = output_fields(b);

    let shared_inputs: Vec<String> = a_inputs
        .iter()
        .filter(|v| b_inputs.contains(v))
        .cloned()
        .collect();
    let left_only_inputs: Vec<String> = a_inputs
        .iter()
        .filter(|v| !b_inputs.contains(v))
        .cloned()
        .collect();
    let right_only_inputs: Vec<String> = b_inputs
        .iter()
        .filter(|v| !a_inputs.contains(v))
        .cloned()
        .collect();

    let shared_outputs: Vec<String> = a_outputs
        .iter()
        .filter(|v| b_outputs.contains(v))
        .cloned()
        .collect();
    let left_only_outputs: Vec<String> = a_outputs
        .iter()
        .filter(|v| !b_outputs.contains(v))
        .cloned()
        .collect();
    let right_only_outputs: Vec<String> = b_outputs
        .iter()
        .filter(|v| !a_outputs.contains(v))
        .cloned()
        .collect();

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

    // Compatible: can A feed B? (with alias resolution)
    let a_aliases = a
        .interface
        .as_ref()
        .map(|iface| iface.aliases.clone())
        .unwrap_or_default();
    let b_aliases = b
        .interface
        .as_ref()
        .map(|iface| iface.aliases.clone())
        .unwrap_or_default();
    let compatible = can_feed_with_aliases(&a_outputs, &b_inputs, &a_aliases, &b_aliases);

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
