use super::Microgram;
use crate::modules::decision_engine::{DecisionNode, Value};
use std::collections::{HashMap, HashSet};

/// Infer the type string from a Value
pub(crate) fn value_type_name(v: &Value) -> &'static str {
    match v {
        Value::Bool(_) => "bool",
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Null => "null",
        Value::Object(_) => "object",
        Value::Array(_) => "array",
    }
}

/// Infer input types from test cases
pub(crate) fn infer_input_types(mg: &Microgram) -> HashMap<String, String> {
    let mut types: HashMap<String, String> = HashMap::new();
    for test in &mg.tests {
        for (k, v) in &test.input {
            types
                .entry(k.clone())
                .or_insert_with(|| value_type_name(v).to_string());
        }
    }
    types
}

/// Infer output types from test cases
pub(crate) fn infer_output_types(mg: &Microgram) -> HashMap<String, String> {
    let mut types: HashMap<String, String> = HashMap::new();
    for test in &mg.tests {
        for (k, v) in &test.expect {
            types
                .entry(k.clone())
                .or_insert_with(|| value_type_name(v).to_string());
        }
    }
    types
}

/// Set-based input variable extraction (for interface validation)
pub(crate) fn input_variables_set(mg: &Microgram) -> HashSet<&str> {
    let mut vars = HashSet::new();
    for node in mg.tree.nodes.values() {
        if let DecisionNode::Condition { variable, .. } = node {
            vars.insert(variable.as_str());
        }
    }
    vars
}

/// Set-based output field extraction (for interface validation)
pub(crate) fn output_fields_set(mg: &Microgram) -> HashSet<&str> {
    let mut fields = HashSet::new();
    for node in mg.tree.nodes.values() {
        if let DecisionNode::Return {
            value: Value::Object(map),
        } = node
        {
            for key in map.keys() {
                fields.insert(key.as_str());
            }
        }
    }
    fields
}
