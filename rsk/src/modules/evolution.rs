//! # Evolution Module
//!
//! Autonomous code synthesis and kernel expansion.
//! Translates DecisionTrees into native Rust Intrinsics.

use crate::modules::code_generator::to_snake_case;
use crate::modules::decision_engine::{DecisionNode, DecisionTree, Operator, Value};

/// Synthesizes a native Rust function from a DecisionTree
pub fn synthesize_intrinsic(name: &str, tree: &DecisionTree) -> String {
    let fn_name = if name == "execute" {
        "execute".to_string()
    } else {
        "intrinsic_".to_string() + &to_snake_case(name)
    };
    let mut code = String::new();

    code.push_str("use rsk::Value;\n");
    code.push_str("use std::collections::HashMap;\n\n");
    code.push_str(&format!("pub fn {}(input: Value) -> Value {{ \n", fn_name));
    code.push_str("    let mut variables = HashMap::new();\n");
    code.push_str("    if let Value::Object(map) = input { variables = map; }\n\n");

    code.push_str(&generate_node_code(&tree.start, tree, 1));
    code.push_str("}\n");
    code
}

/// Alias for synthesize_intrinsic to match skill-engineer expectations
pub fn compile_logic_to_rust(tree: &DecisionTree) -> String {
    // Generate 'execute' function for internal skill use
    synthesize_intrinsic("execute", tree)
}

fn generate_value_code(val: &Value) -> String {
    match val {
        Value::Null => "Value::Null".to_string(),
        Value::Bool(b) => "Value::Bool(".to_string() + &b.to_string() + ")",
        Value::Int(i) => "Value::Int(".to_string() + &i.to_string() + ")",
        Value::Float(f) => "Value::Float(".to_string() + &f.to_string() + ")",
        Value::String(s) => {
            "Value::String(\"".to_string() + &s.replace('"', "\\\"") + "\".to_string())"
        }
        Value::Array(arr) => {
            let mut s = "Value::Array(vec![".to_string();
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&generate_value_code(v));
            }
            s.push_str("])");
            s
        }
        Value::Object(map) => {
            let mut s = "{\n        let mut m = HashMap::new();\n".to_string();
            for (k, v) in map {
                s.push_str("        m.insert(\"");
                s.push_str(k);
                s.push_str("\".to_string(), ");
                s.push_str(&generate_value_code(v));
                s.push_str(");\n");
            }
            s.push_str("        Value::Object(m)\n    }");
            s
        }
    }
}

fn generate_node_code(node_id: &str, tree: &DecisionTree, indent_level: usize) -> String {
    let mut code = String::new();
    let indent = "    ".repeat(indent_level);

    let node = match tree.nodes.get(node_id) {
        Some(n) => n,
        None => return indent + "return Value::Null;\n",
    };

    match node {
        DecisionNode::Condition {
            variable,
            operator,
            value,
            true_next,
            false_next,
        } => {
            let op = match operator {
                Operator::Gt => ">",
                Operator::Lt => "<",
                Operator::Gte => ">=",
                Operator::Lte => "<=",
                _ => "==",
            };
            let val = match value {
                Some(Value::Int(i)) => i.to_string() + ".0",
                Some(Value::Float(f)) => f.to_string(),
                _ => "0.0".to_string(),
            };

            code.push_str(&indent);
            code.push_str("let var_val = variables.get(\" ");
            code.push_str(variable);
            code.push_str("\").and_then(|v| v.as_f64()).unwrap_or(0.0);\n");

            code.push_str(&indent);
            code.push_str("if var_val ");
            code.push_str(op);
            code.push_str(" ");
            code.push_str(&val);
            code.push_str(" {\n");

            code.push_str(&generate_node_code(true_next, tree, indent_level + 1));
            code.push_str(&indent);
            code.push_str("} else {\n");
            code.push_str(&generate_node_code(false_next, tree, indent_level + 1));
            code.push_str(&indent);
            code.push_str("}\n");
        }
        DecisionNode::Return { value } => {
            code.push_str(&indent);
            code.push_str("return ");
            code.push_str(&generate_value_code(value));
            code.push_str(";\n");
        }
        _ => {
            code.push_str(&indent);
            code.push_str("return Value::Null;\n");
        }
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]

    fn test_synthesis_logic_flow() {
        let mut nodes = HashMap::new();

        nodes.insert(
            "start".to_string(),
            DecisionNode::Condition {
                variable: "x".to_string(),

                operator: Operator::Gt,

                value: Some(Value::Int(10)),

                true_next: "high".to_string(),

                false_next: "low".to_string(),
            },
        );

        nodes.insert(
            "high".to_string(),
            DecisionNode::Return {
                value: Value::String("high".to_string()),
            },
        );

        nodes.insert(
            "low".to_string(),
            DecisionNode::Return {
                value: Value::String("low".to_string()),
            },
        );

        let tree = DecisionTree {
            start: "start".to_string(),
            nodes,
        };

        let code = synthesize_intrinsic("test_flow", &tree);

        assert!(code.contains("if var_val > 10.0"));

        assert!(code.contains("return Value::String(\"high\".to_string())"));

        assert!(code.contains("return Value::String(\"low\".to_string())"));
    }
}
