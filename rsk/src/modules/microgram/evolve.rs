use super::{Microgram, MicrogramTest};
use crate::modules::decision_engine::{DecisionNode, Operator, Value};
use std::collections::HashMap;

/// Analyze a microgram and suggest additional test cases
pub fn evolve_tests(mg: &Microgram) -> Vec<MicrogramTest> {
    let mut new_tests = Vec::new();
    let existing_inputs: Vec<&HashMap<String, Value>> = mg.tests.iter().map(|t| &t.input).collect();

    // Strategy 1: Find boundary variables from the decision tree
    for node in mg.tree.nodes.values() {
        if let DecisionNode::Condition {
            variable,
            value: Some(threshold),
            operator,
            ..
        } = node
        {
            let has_var =
                |inputs: &[&HashMap<String, Value>], pred: &dyn Fn(&Value) -> bool| -> bool {
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
                        new_tests.push(MicrogramTest {
                            input,
                            expect: result.output,
                        });
                    }
                    // Add negative if not tested
                    if !has_var(&existing_inputs, &|v| matches!(v, Value::Int(i) if *i < 0)) {
                        let mut input = HashMap::new();
                        input.insert(variable.clone(), Value::Int(-1));
                        let result = mg.run(input.clone());
                        new_tests.push(MicrogramTest {
                            input,
                            expect: result.output,
                        });
                    }
                    // Add exact boundary ±1 if not tested
                    if !has_var(
                        &existing_inputs,
                        &|v| matches!(v, Value::Int(i) if *i == n - 1),
                    ) {
                        let mut input = HashMap::new();
                        input.insert(variable.clone(), Value::Int(n - 1));
                        let result = mg.run(input.clone());
                        new_tests.push(MicrogramTest {
                            input,
                            expect: result.output,
                        });
                    }
                    if !has_var(
                        &existing_inputs,
                        &|v| matches!(v, Value::Int(i) if *i == n + 1),
                    ) {
                        let mut input = HashMap::new();
                        input.insert(variable.clone(), Value::Int(n + 1));
                        let result = mg.run(input.clone());
                        new_tests.push(MicrogramTest {
                            input,
                            expect: result.output,
                        });
                    }
                    // Large value
                    if !has_var(
                        &existing_inputs,
                        &|v| matches!(v, Value::Int(i) if *i > n * 10),
                    ) {
                        let mut input = HashMap::new();
                        input.insert(variable.clone(), Value::Int(n * 100));
                        let result = mg.run(input.clone());
                        new_tests.push(MicrogramTest {
                            input,
                            expect: result.output,
                        });
                    }
                }
                Value::Float(f) => {
                    let f = *f;
                    // Epsilon below
                    if !has_var(
                        &existing_inputs,
                        &|v| matches!(v, Value::Float(x) if (*x - f).abs() < 0.01 && *x < f),
                    ) {
                        let mut input = HashMap::new();
                        input.insert(variable.clone(), Value::Float(f - 0.001));
                        let result = mg.run(input.clone());
                        new_tests.push(MicrogramTest {
                            input,
                            expect: result.output,
                        });
                    }
                    // Epsilon above
                    if !has_var(
                        &existing_inputs,
                        &|v| matches!(v, Value::Float(x) if (*x - f).abs() < 0.01 && *x > f),
                    ) {
                        let mut input = HashMap::new();
                        input.insert(variable.clone(), Value::Float(f + 0.001));
                        let result = mg.run(input.clone());
                        new_tests.push(MicrogramTest {
                            input,
                            expect: result.output,
                        });
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
                    new_tests.push(MicrogramTest {
                        input,
                        expect: result.output,
                    });
                }
            }
        }
    }

    new_tests
}
