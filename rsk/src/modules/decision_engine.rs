//! # Decision Engine
//!
//! Deterministic execution engine for logic trees defined in YAML/JSON.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    NotContains,
    Matches,
    IsNull,
    IsNotNull,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Value {
    pub fn as_string(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Array(arr) => format!("{:?}", arr),
            Value::Object(obj) => format!("{:?}", obj),
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DecisionNode {
    Condition {
        variable: String,
        operator: Operator,
        #[serde(default)]
        value: Option<Value>,
        true_next: String,
        false_next: String,
    },
    Action {
        action: String,
        #[serde(default)]
        target: Option<String>,
        #[serde(default)]
        value: Option<Value>,
        #[serde(default)]
        next: Option<String>,
    },
    Return {
        value: Value,
    },
    LlmFallback {
        prompt: String,
        schema: Option<Value>,
    },
    Intrinsic {
        function: String,
        input_variable: String,
        output_variable: String,
        next: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTree {
    pub start: String,
    pub nodes: HashMap<String, DecisionNode>,
}

#[derive(Debug, Clone, Default)]
pub struct DecisionContext {
    pub variables: HashMap<String, Value>,
    pub execution_path: Vec<String>,
}

impl DecisionContext {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set(&mut self, key: &str, value: Value) {
        self.variables.insert(key.to_string(), value);
    }
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.variables.get(key)
    }
}

pub struct DecisionEngine {
    tree: DecisionTree,
}

pub fn load_tree(yaml: &str) -> anyhow::Result<DecisionTree> {
    serde_yaml::from_str(yaml).map_err(|e| anyhow::anyhow!("Invalid decision tree YAML: {}", e))
}

pub fn load_tree_strict(yaml: &str) -> anyhow::Result<DecisionTree> {
    let tree: DecisionTree = serde_yaml::from_str(yaml)
        .map_err(|e| anyhow::anyhow!("Invalid decision tree YAML: {}", e))?;

    // Phase 2 mandate: No LlmFallback allowed in Diamond skills
    // We walk all nodes to ensure zero-tolerance enforcement
    for (id, node) in &tree.nodes {
        validate_node_recursive(id, node, &tree.nodes)?;
    }

    Ok(tree)
}

fn validate_node_recursive(
    id: &str,
    node: &DecisionNode,
    _nodes: &HashMap<String, DecisionNode>,
) -> anyhow::Result<()> {
    match node {
        DecisionNode::LlmFallback { .. } => Err(anyhow::anyhow!(
            "Strict mode violation: Node '{}' uses forbidden LlmFallback. Deterministic logic required.",
            id
        )),
        // In the future, we could add cycle detection here if needed
        _ => Ok(()),
    }
}

#[derive(Debug, Clone)]
pub enum ExecutionResult {
    Value(Value),
    LlmRequest {
        prompt: String,
        context: HashMap<String, Value>,
    },
    Error(String),
}

impl DecisionEngine {
    pub fn new(tree: DecisionTree) -> Self {
        Self { tree }
    }

    pub fn execute(&self, ctx: &mut DecisionContext) -> ExecutionResult {
        let mut current_id = self.tree.start.clone();
        let max_steps = 1000;
        let mut steps = 0;

        loop {
            if steps >= max_steps {
                return ExecutionResult::Error("Max depth".to_string());
            }
            steps += 1;
            ctx.execution_path.push(current_id.clone());
            let node = match self.tree.nodes.get(&current_id) {
                Some(n) => n,
                None => return ExecutionResult::Error(format!("Node not found: {}", current_id)),
            };

            match node {
                DecisionNode::Condition {
                    variable,
                    operator,
                    value,
                    true_next,
                    false_next,
                } => {
                    let var_val = ctx.get(variable).unwrap_or(&Value::Null);
                    let result = self.evaluate_condition(var_val, operator, value.as_ref());
                    current_id = if result {
                        true_next.clone()
                    } else {
                        false_next.clone()
                    };
                }
                DecisionNode::Action {
                    action,
                    target,
                    value,
                    next,
                } => {
                    let processed_val = value
                        .as_ref()
                        .map(|v| self.interpolate_value(v, ctx))
                        .unwrap_or(Value::Null);
                    match action.as_str() {
                        "set_variable" => {
                            if let Some(t) = target {
                                ctx.set(t, processed_val);
                            }
                        }
                        "log" => eprintln!("[logic] {}", processed_val.as_string()),
                        _ => {}
                    }
                    match next {
                        Some(n) => current_id = n.clone(),
                        None => return ExecutionResult::Error("Action no next".to_string()),
                    }
                }
                DecisionNode::Return { value } => {
                    return ExecutionResult::Value(self.interpolate_value(value, ctx));
                }
                DecisionNode::LlmFallback { prompt, .. } => {
                    return ExecutionResult::LlmRequest {
                        prompt: prompt.clone(),
                        context: ctx.variables.clone(),
                    };
                }
                DecisionNode::Intrinsic {
                    function,
                    input_variable,
                    output_variable,
                    next,
                } => {
                    let input = ctx.get(input_variable).cloned().unwrap_or(Value::Null);
                    let result = self.execute_intrinsic(function, input);
                    ctx.set(output_variable, result);
                    current_id = next.clone();
                }
            }
        }
    }

    pub fn interpolate_value(&self, val: &Value, ctx: &DecisionContext) -> Value {
        match val {
            Value::String(s) if s.contains("{{") => {
                let mut result = s.clone();
                while let Some(start) = result.find("{{") {
                    if let Some(end) = result[start..].find("}}") {
                        let full_tag = &result[start..start + end + 2];
                        let key_path = result[start + 2..start + end].trim();
                        let replacement = self.resolve_path(key_path, ctx).as_string();
                        result = result.replace(full_tag, &replacement);
                    } else {
                        break;
                    }
                }
                Value::String(result)
            }
            Value::Object(map) => {
                let mut new_map = HashMap::new();
                for (k, v) in map {
                    new_map.insert(k.clone(), self.interpolate_value(v, ctx));
                }
                Value::Object(new_map)
            }
            Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.interpolate_value(v, ctx)).collect())
            }
            _ => val.clone(),
        }
    }

    fn resolve_path(&self, path: &str, ctx: &DecisionContext) -> Value {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = None;

        for (i, part) in parts.iter().enumerate() {
            if i == 0 {
                current = self.resolve_part(part, ctx, None);
            } else {
                if let Some(val) = current {
                    current = self.resolve_part(part, ctx, Some(val));
                } else {
                    return Value::Null;
                }
            }
        }
        current.unwrap_or(Value::Null)
    }

    fn resolve_part(
        &self,
        part: &str,
        ctx: &DecisionContext,
        base: Option<Value>,
    ) -> Option<Value> {
        if part.contains('[') && part.contains(']') {
            let base_key = part.split('[').next().unwrap();
            let idx_str = part.split('[').nth(1).unwrap().trim_end_matches(']');

            let array_val = if let Some(b) = base {
                if let Value::Object(map) = b {
                    map.get(base_key).cloned()
                } else {
                    None
                }
            } else {
                ctx.variables.get(base_key).cloned()
            };

            if let (Some(Value::Array(arr)), Ok(idx)) = (array_val, idx_str.parse::<usize>()) {
                arr.get(idx).cloned()
            } else {
                None
            }
        } else {
            if let Some(b) = base {
                match b {
                    Value::Object(map) => map.get(part).cloned(),
                    _ => None,
                }
            } else {
                ctx.variables.get(part).cloned()
            }
        }
    }

    fn execute_intrinsic(&self, function: &str, input: Value) -> Value {
        match function {
            "is_prime" => {
                if let Value::Int(n) = input {
                    let res = crate::modules::math::is_prime(n);
                    let mut map = HashMap::new();
                    map.insert("is_prime".to_string(), Value::Bool(res.is_prime));
                    map.insert("number".to_string(), Value::Int(res.number));
                    map.insert("reason".to_string(), Value::String(res.reason));
                    Value::Object(map)
                } else {
                    Value::String("Error: int required".to_string())
                }
            }
            "sha256" => {
                let res = crate::modules::crypto::sha256_hash(&input.as_string());
                let mut map = HashMap::new();
                map.insert("hex".to_string(), Value::String(res.hex));
                Value::Object(map)
            }
            "optimize_strategy" => {
                if let Value::Object(args) = input {
                    let fields_val = args
                        .get("fields")
                        .cloned()
                        .unwrap_or(Value::Array(Vec::new()));
                    let tactics_val = args
                        .get("tactics")
                        .cloned()
                        .unwrap_or(Value::Array(Vec::new()));
                    let fields: Vec<crate::modules::strategy::StrategicField> =
                        serde_json::from_str(&serde_json::to_string(&fields_val).unwrap())
                            .unwrap_or_default();
                    let tactics: Vec<crate::modules::strategy::WinTactic> =
                        serde_json::from_str(&serde_json::to_string(&tactics_val).unwrap())
                            .unwrap_or_default();
                    let results = crate::modules::strategy::StrategyOptimizer::new(fields, tactics)
                        .optimize();
                    let res_val: Vec<Value> =
                        serde_json::from_str(&serde_json::to_string(&results).unwrap()).unwrap();
                    Value::Array(res_val)
                } else {
                    Value::Null
                }
            }
            _ => Value::String(format!("Unknown: {}", function)),
        }
    }

    fn evaluate_condition(&self, actual: &Value, op: &Operator, target: Option<&Value>) -> bool {
        match op {
            Operator::Eq => actual == target.unwrap_or(&Value::Null),
            Operator::Neq => actual != target.unwrap_or(&Value::Null),
            Operator::IsNull => matches!(actual, Value::Null),
            Operator::IsNotNull => !matches!(actual, Value::Null),
            Operator::Gt => {
                actual.as_f64().unwrap_or(0.0) > target.and_then(|v| v.as_f64()).unwrap_or(0.0)
            }
            Operator::Gte => {
                actual.as_f64().unwrap_or(0.0) >= target.and_then(|v| v.as_f64()).unwrap_or(0.0)
            }
            Operator::Lt => {
                actual.as_f64().unwrap_or(0.0) < target.and_then(|v| v.as_f64()).unwrap_or(0.0)
            }
            Operator::Lte => {
                actual.as_f64().unwrap_or(0.0) <= target.and_then(|v| v.as_f64()).unwrap_or(0.0)
            }
            Operator::Contains => {
                if let (Value::String(s), Some(Value::String(t))) = (actual, target) {
                    s.contains(t)
                } else if let (Value::Array(a), Some(t)) = (actual, target) {
                    a.contains(t)
                } else {
                    false
                }
            }
            Operator::NotContains => {
                if let (Value::String(s), Some(Value::String(t))) = (actual, target) {
                    !s.contains(t)
                } else if let (Value::Array(a), Some(t)) = (actual, target) {
                    !a.contains(t)
                } else {
                    true
                }
            }
            Operator::Matches => {
                if let (Value::String(s), Some(Value::String(t))) = (actual, target) {
                    if let Ok(re) = Regex::new(t) {
                        re.is_match(s)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolation_strategic() {
        let mut ctx = DecisionContext::new();
        let mut strategy = HashMap::new();
        strategy.insert("field_id".to_string(), Value::String("F1".to_string()));
        ctx.set("optimal_paths", Value::Array(vec![Value::Object(strategy)]));

        let engine = DecisionEngine::new(DecisionTree {
            start: "".to_string(),
            nodes: HashMap::new(),
        });

        let template = Value::String("Field is {{optimal_paths[0].field_id}}".to_string());
        let result = engine.interpolate_value(&template, &ctx);
        assert_eq!(result, Value::String("Field is F1".to_string()));
    }

    #[test]
    fn test_strict_logic_no_llm() {
        let yaml = r#"
start: node1
nodes:
  node1:
    type: llm_fallback
    prompt: "Lazy logic"
"#;
        let result = load_tree_strict(yaml);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("uses forbidden LlmFallback")
        );
    }
}
