use super::{Microgram, MicrogramTest};
use crate::modules::decision_engine::{DecisionNode, Value};

/// Clone a microgram with a mutated threshold
pub fn clone_mutated(mg: &Microgram, new_name: &str, threshold_delta: i64) -> Microgram {
    let mut cloned = mg.clone();
    cloned.name = new_name.to_string();

    // Mutate: shift threshold values in condition nodes
    for node in cloned.tree.nodes.values_mut() {
        if let DecisionNode::Condition {
            value: Some(Value::Int(n)),
            ..
        } = node
        {
            *n += threshold_delta;
        }
    }

    // Regenerate tests from the mutated tree by running the existing test inputs
    cloned.tests = mg
        .tests
        .iter()
        .map(|test| {
            let result = cloned.run(test.input.clone());
            MicrogramTest {
                input: test.input.clone(),
                expect: result.output,
            }
        })
        .collect();

    cloned
}
