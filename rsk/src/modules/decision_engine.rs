//! # Decision Engine
//!
//! Deterministic execution engine for logic trees defined in YAML/JSON.

use dashmap::DashMap;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

/// Process-wide compiled regex cache for `Operator::Matches`.
///
/// Micrograms commonly reuse a handful of patterns across many evaluations. Compiling
/// them once and sharing across tree executions removes the per-call allocation that
/// would otherwise dominate regex-heavy chains. Compilation failures are not cached
/// (we simply fall through to `false`), so a bad pattern doesn't poison the slot.
fn regex_cache() -> &'static DashMap<String, Arc<Regex>> {
    static CACHE: OnceLock<DashMap<String, Arc<Regex>>> = OnceLock::new();
    CACHE.get_or_init(DashMap::new)
}

fn cached_regex(pattern: &str) -> Option<Arc<Regex>> {
    let cache = regex_cache();
    if let Some(r) = cache.get(pattern) {
        return Some(Arc::clone(r.value()));
    }
    let compiled = Regex::new(pattern).ok()?;
    let arc = Arc::new(compiled);
    cache.insert(pattern.to_string(), Arc::clone(&arc));
    Some(arc)
}

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
            Value::Array(arr) => format!("{arr:?}"),
            Value::Object(obj) => format!("{obj:?}"),
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            #[allow(clippy::as_conversions)]
            // i64→f64 precision loss acceptable for numeric conversion
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            Value::String(s) => s.parse::<f64>().ok(),
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

/// Executor over a [`DecisionTree`].
///
/// Holds the tree by `Cow<'a, _>` so callers can either:
/// * own the tree (`DecisionEngine::new(tree)`) — preserves the legacy API, or
/// * borrow it (`DecisionEngine::borrowed(&tree)`) — skips the clone in hot paths
///   like [`super::microgram::Microgram::run`], which invokes the engine once per
///   run across 1.5K+ micrograms.
///
/// Execution is read-only over the tree, so the borrow variant is always safe
/// as long as the caller keeps the tree alive for the duration of `execute`.
pub struct DecisionEngine<'a> {
    tree: std::borrow::Cow<'a, DecisionTree>,
}

pub fn load_tree(yaml: &str) -> anyhow::Result<DecisionTree> {
    serde_yaml::from_str(yaml).map_err(|e| anyhow::anyhow!("Invalid decision tree YAML: {e}"))
}

pub fn load_tree_strict(yaml: &str) -> anyhow::Result<DecisionTree> {
    let tree: DecisionTree = serde_yaml::from_str(yaml)
        .map_err(|e| anyhow::anyhow!("Invalid decision tree YAML: {e}"))?;

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
            "Strict mode violation: Node '{id}' uses forbidden LlmFallback. Deterministic logic required."
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

impl DecisionEngine<'static> {
    /// Construct an engine that owns its tree. Back-compat for existing callers;
    /// prefer [`DecisionEngine::borrowed`] inside hot loops.
    pub fn new(tree: DecisionTree) -> Self {
        Self {
            tree: std::borrow::Cow::Owned(tree),
        }
    }
}

impl<'a> DecisionEngine<'a> {
    /// Construct an engine that borrows its tree — no clone, no allocation.
    pub fn borrowed(tree: &'a DecisionTree) -> Self {
        Self {
            tree: std::borrow::Cow::Borrowed(tree),
        }
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
            let Some(node) = self.tree.nodes.get(&current_id) else {
                return ExecutionResult::Error(format!("Node not found: {current_id}"));
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
                    let resolved_value = value.as_ref().map(|v| self.interpolate_value(v, ctx));
                    let result =
                        self.evaluate_condition(var_val, operator, resolved_value.as_ref());
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
                // Single forward pass: scan from an advancing cursor, append
                // unresolved prefix + resolved replacement + continue after
                // the tag. Avoids:
                // 1. Quadratic cost of `result.replace(full_tag, ...)` re-scanning
                //    the whole string on each tag.
                // 2. Infinite loop when a replacement value itself contains `{{`
                //    (prior version would re-find the injected brace).
                let mut out = String::with_capacity(s.len());
                let bytes = s.as_str();
                let mut cursor = 0;
                while cursor < bytes.len() {
                    let tail = &bytes[cursor..];
                    if let Some(rel_start) = tail.find("{{") {
                        out.push_str(&tail[..rel_start]);
                        let after_open = cursor + rel_start + 2;
                        if let Some(rel_end) = bytes[after_open..].find("}}") {
                            let key_path = bytes[after_open..after_open + rel_end].trim();
                            let replacement = self.resolve_path(key_path, ctx).as_string();
                            out.push_str(&replacement);
                            cursor = after_open + rel_end + 2;
                        } else {
                            // Unterminated `{{` — emit rest verbatim and stop.
                            out.push_str(&tail[rel_start..]);
                            break;
                        }
                    } else {
                        out.push_str(tail);
                        break;
                    }
                }
                Value::String(out)
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
            } else if let Some(val) = current {
                current = self.resolve_part(part, ctx, Some(val));
            } else {
                return Value::Null;
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
            let base_key = part.split('[').next().unwrap_or("");
            let idx_str = part
                .split('[')
                .nth(1)
                .map(|s| s.trim_end_matches(']'))
                .unwrap_or("");

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
        } else if let Some(b) = base {
            match b {
                Value::Object(map) => map.get(part).cloned(),
                _ => None,
            }
        } else {
            ctx.variables.get(part).cloned()
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
                        serde_json::to_string(&fields_val)
                            .ok()
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or_default();
                    let tactics: Vec<crate::modules::strategy::WinTactic> =
                        serde_json::to_string(&tactics_val)
                            .ok()
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or_default();
                    let results = crate::modules::strategy::StrategyOptimizer::new(fields, tactics)
                        .optimize();
                    let res_val: Vec<Value> = serde_json::to_string(&results)
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default();
                    Value::Array(res_val)
                } else {
                    Value::Null
                }
            }
            _ => Value::String(format!("Unknown: {function}")),
        }
    }

    fn evaluate_condition(&self, actual: &Value, op: &Operator, target: Option<&Value>) -> bool {
        match op {
            Operator::Eq => {
                let t = target.unwrap_or(&Value::Null);
                if actual == t {
                    return true;
                }
                // Cross-type numeric equality: Float(5.0) == String("5") == Int(5)
                match (actual.as_f64(), t.as_f64()) {
                    (Some(a), Some(b)) => a == b,
                    _ => false,
                }
            }
            Operator::Neq => {
                let t = target.unwrap_or(&Value::Null);
                if actual == t {
                    return false;
                }
                match (actual.as_f64(), t.as_f64()) {
                    (Some(a), Some(b)) => a != b,
                    _ => true,
                }
            }
            Operator::IsNull => matches!(actual, Value::Null),
            Operator::IsNotNull => !matches!(actual, Value::Null),
            Operator::Gt => match (actual.as_f64(), target.and_then(|v| v.as_f64())) {
                (Some(a), Some(t)) => a > t,
                _ => false,
            },
            Operator::Gte => match (actual.as_f64(), target.and_then(|v| v.as_f64())) {
                (Some(a), Some(t)) => a >= t,
                _ => false,
            },
            Operator::Lt => match (actual.as_f64(), target.and_then(|v| v.as_f64())) {
                (Some(a), Some(t)) => a < t,
                _ => false,
            },
            Operator::Lte => match (actual.as_f64(), target.and_then(|v| v.as_f64())) {
                (Some(a), Some(t)) => a <= t,
                _ => false,
            },
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
                    cached_regex(t).is_some_and(|re| re.is_match(s))
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

    #[test]
    fn interpolation_does_not_reenter_replacements() {
        // Regression: the previous implementation re-scanned the whole result
        // after each `replace`, so a replacement value that itself contained
        // `{{` would be mistaken for a fresh tag and looked up again — worst
        // case an infinite loop, otherwise unintended double-resolution.
        let mut ctx = DecisionContext::new();
        ctx.set("a", Value::String("{{b}}".to_string()));
        ctx.set("b", Value::String("final".to_string()));

        let empty_tree = DecisionTree {
            start: String::new(),
            nodes: HashMap::new(),
        };
        let engine = DecisionEngine::borrowed(&empty_tree);

        let template = Value::String("value={{a}}".to_string());
        let out = engine.interpolate_value(&template, &ctx);
        // Single-pass substitution: `{{a}}` → `{{b}}` (literal), not re-resolved.
        assert_eq!(out, Value::String("value={{b}}".to_string()));
    }

    #[test]
    fn interpolation_handles_unterminated_brace() {
        let ctx = DecisionContext::new();
        let empty_tree = DecisionTree {
            start: String::new(),
            nodes: HashMap::new(),
        };
        let engine = DecisionEngine::borrowed(&empty_tree);
        let template = Value::String("prefix {{oops".to_string());
        let out = engine.interpolate_value(&template, &ctx);
        // Unterminated tag passes through verbatim instead of looping forever.
        assert_eq!(out, Value::String("prefix {{oops".to_string()));
    }

    #[test]
    fn borrowed_engine_does_not_clone_tree() {
        // The API-level guarantee: constructing a borrowed engine takes a
        // reference, so the caller keeps ownership and no clone occurs. Spot
        // check that execution produces the same result as the owned API.
        let tree = DecisionTree {
            start: "root".to_string(),
            nodes: {
                let mut m = HashMap::new();
                m.insert(
                    "root".to_string(),
                    DecisionNode::Return {
                        value: Value::Bool(true),
                    },
                );
                m
            },
        };
        let engine = DecisionEngine::borrowed(&tree);
        let mut ctx = DecisionContext::new();
        match engine.execute(&mut ctx) {
            ExecutionResult::Value(Value::Bool(true)) => {}
            other => panic!("unexpected execution result: {other:?}"),
        }
        // tree is still usable after execute — proof that we borrowed it.
        assert_eq!(tree.start, "root");
    }

    #[test]
    fn regex_cache_reuses_compiled_regex() {
        // First call compiles and stores; second call fetches the same Arc.
        let a = cached_regex("^foo$").unwrap();
        let b = cached_regex("^foo$").unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        assert!(a.is_match("foo"));
        assert!(!a.is_match("bar"));
    }

    #[test]
    fn regex_cache_invalid_pattern_returns_none() {
        // Invalid patterns must not be cached nor panic.
        assert!(cached_regex("(unclosed").is_none());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Property tests — exercise numeric-coercion semantics with random inputs
    // ═══════════════════════════════════════════════════════════════════════

    use proptest::prelude::*;

    fn mk_condition_engine() -> DecisionEngine<'static> {
        DecisionEngine::new(DecisionTree {
            start: String::new(),
            nodes: HashMap::new(),
        })
    }

    proptest! {
        /// `Eq` with numeric coercion must agree regardless of whether the
        /// numbers are wrapped in Int, Float, or numeric-parseable String.
        #[test]
        fn eq_numeric_coercion_is_reflexive(n in any::<i32>()) {
            let e = mk_condition_engine();
            let n_i64 = i64::from(n);
            let int_val = Value::Int(n_i64);
            let float_val = Value::Float(n_i64 as f64);
            let string_val = Value::String(n_i64.to_string());

            prop_assert!(e.evaluate_condition(&int_val, &Operator::Eq, Some(&float_val)));
            prop_assert!(e.evaluate_condition(&float_val, &Operator::Eq, Some(&int_val)));
            prop_assert!(e.evaluate_condition(&int_val, &Operator::Eq, Some(&string_val)));
        }

        /// `Gt` is antisymmetric over distinct int values.
        #[test]
        fn gt_antisymmetric(a in any::<i32>(), b in any::<i32>()) {
            prop_assume!(a != b);
            let e = mk_condition_engine();
            let av = Value::Int(i64::from(a));
            let bv = Value::Int(i64::from(b));
            let a_gt_b = e.evaluate_condition(&av, &Operator::Gt, Some(&bv));
            let b_gt_a = e.evaluate_condition(&bv, &Operator::Gt, Some(&av));
            prop_assert!(a_gt_b ^ b_gt_a, "exactly one of a>b, b>a must hold for distinct ints");
        }

        /// `Gte` over equal ints must be true regardless of wrapping type.
        #[test]
        fn gte_equal_values_int_vs_float(n in -10_000i32..10_000) {
            let e = mk_condition_engine();
            let int_val = Value::Int(i64::from(n));
            let float_val = Value::Float(f64::from(n));
            prop_assert!(e.evaluate_condition(&int_val, &Operator::Gte, Some(&float_val)));
            prop_assert!(e.evaluate_condition(&float_val, &Operator::Gte, Some(&int_val)));
        }

        /// `Neq` must disagree with `Eq` on every input pair.
        #[test]
        fn eq_and_neq_are_opposites(a in any::<i32>(), b in any::<i32>()) {
            let e = mk_condition_engine();
            let av = Value::Int(i64::from(a));
            let bv = Value::Int(i64::from(b));
            let eq = e.evaluate_condition(&av, &Operator::Eq, Some(&bv));
            let neq = e.evaluate_condition(&av, &Operator::Neq, Some(&bv));
            prop_assert_eq!(eq, !neq);
        }

        /// `IsNull` is true iff the value is `Value::Null`.
        #[test]
        fn is_null_correct(s in "\\PC*") {
            let e = mk_condition_engine();
            let null = Value::Null;
            let non_null = Value::String(s.clone());
            prop_assert!(e.evaluate_condition(&null, &Operator::IsNull, None));
            prop_assert!(!e.evaluate_condition(&non_null, &Operator::IsNull, None));
            prop_assert!(!e.evaluate_condition(&null, &Operator::IsNotNull, None));
            prop_assert!(e.evaluate_condition(&non_null, &Operator::IsNotNull, None));
        }
    }
}
