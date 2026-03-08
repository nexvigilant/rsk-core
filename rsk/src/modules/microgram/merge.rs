use crate::modules::decision_engine::{DecisionNode, DecisionTree, Operator};
use super::Microgram;
use super::compose::input_variables;
use std::collections::HashMap;

/// Merge two micrograms into one by creating a dispatcher that routes
/// based on which input variables are present
pub fn merge(a: &Microgram, b: &Microgram, name: &str, description: &str) -> Microgram {
    let a_inputs = input_variables(a);

    // Build merged tree: dispatch node checks for A's variable first
    let mut nodes = HashMap::new();

    // Prefix all A nodes with "a_" and B nodes with "b_"
    for (node_name, node) in &a.tree.nodes {
        let prefixed = format!("a_{node_name}");
        let remapped = remap_node(node, "a_");
        nodes.insert(prefixed, remapped);
    }
    for (node_name, node) in &b.tree.nodes {
        let prefixed = format!("b_{node_name}");
        let remapped = remap_node(node, "b_");
        nodes.insert(prefixed, remapped);
    }

    // Dispatch: check if A's primary input exists → route to A, else B
    let a_var = a_inputs.first().cloned().unwrap_or_default();
    nodes.insert(
        "dispatch".to_string(),
        DecisionNode::Condition {
            variable: a_var,
            operator: Operator::IsNotNull,
            value: None,
            true_next: format!("a_{}", a.tree.start),
            false_next: format!("b_{}", b.tree.start),
        },
    );

    // Merge tests from both
    let mut tests = a.tests.clone();
    tests.extend(b.tests.iter().cloned());

    Microgram {
        name: name.to_string(),
        description: description.to_string(),
        version: "0.1.0".to_string(),
        tree: DecisionTree {
            start: "dispatch".to_string(),
            nodes,
        },
        tests,
        interface: None,
        primitive_signature: None,
    }
}

/// Remap node references with a prefix
fn remap_node(node: &DecisionNode, prefix: &str) -> DecisionNode {
    match node {
        DecisionNode::Condition { variable, operator, value, true_next, false_next } => {
            DecisionNode::Condition {
                variable: variable.clone(),
                operator: operator.clone(),
                value: value.clone(),
                true_next: format!("{prefix}{true_next}"),
                false_next: format!("{prefix}{false_next}"),
            }
        }
        DecisionNode::Return { value } => DecisionNode::Return { value: value.clone() },
        DecisionNode::Action { action, target, value, next } => {
            DecisionNode::Action {
                action: action.clone(),
                target: target.clone(),
                value: value.clone(),
                next: next.as_ref().map(|n| format!("{prefix}{n}")),
            }
        }
        DecisionNode::LlmFallback { prompt, schema } => {
            DecisionNode::LlmFallback { prompt: prompt.clone(), schema: schema.clone() }
        }
        DecisionNode::Intrinsic { function, input_variable, output_variable, next } => {
            DecisionNode::Intrinsic {
                function: function.clone(),
                input_variable: input_variable.clone(),
                output_variable: output_variable.clone(),
                next: format!("{prefix}{next}"),
            }
        }
    }
}
