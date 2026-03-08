//! # Condition Evaluator
//!
//! Evaluates conditional expressions against a context.
//!
//! ## Design Principles
//! - **Atomic**: Only condition evaluation, no chain execution
//! - **Safe**: No arbitrary code execution, only predefined operations
//! - **Fast**: Minimal allocations, O(n) complexity for n tokens
//!
//! ## Supported Operations
//!
//! ### Comparison Operators
//! - `==` - Equality
//! - `!=` - Inequality
//! - `>`, `>=` - Greater than (or equal)
//! - `<`, `<=` - Less than (or equal)
//!
//! ### Boolean Literals
//! - `true`, `false`
//!
//! ### Value Resolution
//! - Dot notation: `context.foo.bar`
//! - Numeric literals: `42`, `3.14`, `-10`
//! - String literals: `"hello"`, `'world'`
//!
//! ## Examples
//! ```rust,ignore
//! use rsk::modules::chain::condition::evaluate_condition;
//! use serde_json::json;
//!
//! let context = json!({
//!     "has_tests": true,
//!     "count": 5
//! });
//!
//! assert!(evaluate_condition("context.has_tests == true", &context));
//! assert!(evaluate_condition("context.count > 0", &context));
//! assert!(!evaluate_condition("context.count < 0", &context));
//! ```

use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════
// ERROR TYPE
// ═══════════════════════════════════════════════════════════════════════════

/// Condition evaluation error
#[derive(Debug, Clone)]
pub struct ConditionError {
    pub message: String,
    pub condition: String,
}

impl ConditionError {
    pub fn new(message: &str, condition: &str) -> Self {
        Self {
            message: message.to_string(),
            condition: condition.to_string(),
        }
    }
}

impl std::fmt::Display for ConditionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Condition error in '{}': {}",
            self.condition, self.message
        )
    }
}

impl std::error::Error for ConditionError {}

// ═══════════════════════════════════════════════════════════════════════════
// COMPARISON OPERATORS
// ═══════════════════════════════════════════════════════════════════════════

/// Supported comparison operators (in order of precedence for parsing)
const COMPARISON_OPERATORS: [&str; 6] = ["==", "!=", ">=", "<=", ">", "<"];

/// Compare two JSON values
fn compare_values(left: &Value, op: &str, right: &Value) -> bool {
    match op {
        "==" => values_equal(left, right),
        "!=" => !values_equal(left, right),
        ">" => compare_numeric(left, right).map(|c| c > 0).unwrap_or(false),
        ">=" => compare_numeric(left, right)
            .map(|c| c >= 0)
            .unwrap_or(false),
        "<" => compare_numeric(left, right).map(|c| c < 0).unwrap_or(false),
        "<=" => compare_numeric(left, right)
            .map(|c| c <= 0)
            .unwrap_or(false),
        _ => false,
    }
}

/// Check equality between two JSON values
fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        // Null comparison
        (Value::Null, Value::Null) => true,

        // Boolean comparison
        (Value::Bool(a), Value::Bool(b)) => a == b,

        // Number comparison (handle int vs float)
        (Value::Number(a), Value::Number(b)) => {
            if let (Some(a_f), Some(b_f)) = (a.as_f64(), b.as_f64()) {
                (a_f - b_f).abs() < f64::EPSILON
            } else {
                a == b
            }
        }

        // String comparison
        (Value::String(a), Value::String(b)) => a == b,

        // Cross-type comparisons
        (Value::Bool(true), Value::String(s)) | (Value::String(s), Value::Bool(true)) => {
            s.to_lowercase() == "true"
        }
        (Value::Bool(false), Value::String(s)) | (Value::String(s), Value::Bool(false)) => {
            s.to_lowercase() == "false"
        }

        // Arrays and objects: deep equality
        (Value::Array(a), Value::Array(b)) => a == b,
        (Value::Object(a), Value::Object(b)) => a == b,

        _ => false,
    }
}

/// Compare two values numerically, returns -1, 0, or 1
fn compare_numeric(left: &Value, right: &Value) -> Option<i8> {
    let left_num = value_to_f64(left)?;
    let right_num = value_to_f64(right)?;

    if (left_num - right_num).abs() < f64::EPSILON {
        Some(0)
    } else if left_num > right_num {
        Some(1)
    } else {
        Some(-1)
    }
}

/// Convert a JSON value to f64 for numeric comparison
fn value_to_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        Value::Bool(true) => Some(1.0),
        Value::Bool(false) => Some(0.0),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// VALUE RESOLUTION
// ═══════════════════════════════════════════════════════════════════════════

/// Resolve a value expression from context.
///
/// Supports:
/// - Dot notation: `context.foo.bar` or just `foo.bar`
/// - Numeric literals: `42`, `3.14`, `-10`
/// - String literals: `"hello"`, `'world'`
/// - Boolean literals: `true`, `false`
/// - Null literal: `null`
pub fn resolve_value(expr: &str, context: &Value) -> Value {
    let expr = expr.trim();

    // Empty expression
    if expr.is_empty() {
        return Value::Null;
    }

    // Boolean literals
    if expr.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }
    if expr.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }

    // Null literal
    if expr.eq_ignore_ascii_case("null") || expr.eq_ignore_ascii_case("none") {
        return Value::Null;
    }

    // String literals (quoted)
    if (expr.starts_with('"') && expr.ends_with('"'))
        || (expr.starts_with('\'') && expr.ends_with('\''))
    {
        return Value::String(expr[1..expr.len() - 1].to_string());
    }

    // Numeric literals
    if let Ok(n) = expr.parse::<i64>() {
        return Value::Number(n.into());
    }
    if let Ok(n) = expr.parse::<f64>()
        && let Some(num) = serde_json::Number::from_f64(n)
    {
        return Value::Number(num);
    }

    // Dot notation path resolution
    resolve_path(expr, context)
}

/// Resolve a dot-notation path from context.
///
/// Examples:
/// - `context.foo` - looks up `foo` in context
/// - `foo.bar` - looks up `foo.bar` in context
/// - `context.foo.bar.baz` - nested lookup
fn resolve_path(path: &str, context: &Value) -> Value {
    let mut path = path;

    // Strip optional "context." prefix
    if let Some(stripped) = path.strip_prefix("context.") {
        path = stripped;
    }

    // Split by dots and traverse
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = context;

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        match current {
            Value::Object(map) => {
                if let Some(value) = map.get(part) {
                    current = value;
                } else {
                    return Value::Null;
                }
            }
            Value::Array(arr) => {
                // Try to parse as array index
                if let Ok(index) = part.parse::<usize>() {
                    if let Some(value) = arr.get(index) {
                        current = value;
                    } else {
                        return Value::Null;
                    }
                } else {
                    return Value::Null;
                }
            }
            _ => return Value::Null,
        }
    }

    current.clone()
}

// ═══════════════════════════════════════════════════════════════════════════
// CONDITION EVALUATION
// ═══════════════════════════════════════════════════════════════════════════

/// Evaluate a condition expression against a context.
///
/// # Arguments
/// * `condition` - The condition string to evaluate
/// * `context` - JSON context object for value resolution
///
/// # Returns
/// * `true` if the condition evaluates to true
/// * `false` otherwise (including on parse errors for safety)
///
/// # Examples
/// ```rust,ignore
/// let ctx = json!({"count": 5, "enabled": true});
///
/// assert!(evaluate_condition("context.count > 0", &ctx));
/// assert!(evaluate_condition("context.enabled == true", &ctx));
/// assert!(evaluate_condition("true", &ctx));
/// assert!(!evaluate_condition("false", &ctx));
/// ```
pub fn evaluate_condition(condition: &str, context: &Value) -> bool {
    evaluate_condition_result(condition, context).unwrap_or(false)
}

/// Evaluate a condition with detailed error reporting.
pub fn evaluate_condition_result(condition: &str, context: &Value) -> Result<bool, ConditionError> {
    let condition = condition.trim();

    // Empty condition is false
    if condition.is_empty() {
        return Ok(false);
    }

    // Direct boolean literals
    if condition.eq_ignore_ascii_case("true") {
        return Ok(true);
    }
    if condition.eq_ignore_ascii_case("false") {
        return Ok(false);
    }

    // Try to find a comparison operator
    for op in COMPARISON_OPERATORS {
        if let Some(pos) = condition.find(op) {
            let left_str = condition[..pos].trim();
            let right_str = condition[pos + op.len()..].trim();

            if left_str.is_empty() || right_str.is_empty() {
                return Err(ConditionError::new(
                    &format!("Missing operand for operator '{op}'"),
                    condition,
                ));
            }

            let left_val = resolve_value(left_str, context);
            let right_val = resolve_value(right_str, context);

            return Ok(compare_values(&left_val, op, &right_val));
        }
    }

    // No operator found - treat as truthy check
    let value = resolve_value(condition, context);
    Ok(is_truthy(&value))
}

/// Check if a JSON value is "truthy"
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                f != 0.0
            } else {
                true
            }
        }
        Value::String(s) => !s.is_empty() && !s.eq_ignore_ascii_case("false"),
        Value::Array(arr) => !arr.is_empty(),
        Value::Object(obj) => !obj.is_empty(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_context() -> Value {
        json!({
            "has_tests": true,
            "count": 5,
            "name": "test",
            "nested": {
                "value": 42,
                "flag": false
            },
            "items": [1, 2, 3],
            "empty": "",
            "zero": 0
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // VALUE RESOLUTION TESTS
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_resolve_boolean_literals() {
        let ctx = json!({});
        assert_eq!(resolve_value("true", &ctx), Value::Bool(true));
        assert_eq!(resolve_value("false", &ctx), Value::Bool(false));
        assert_eq!(resolve_value("TRUE", &ctx), Value::Bool(true));
        assert_eq!(resolve_value("False", &ctx), Value::Bool(false));
    }

    #[test]
    fn test_resolve_numeric_literals() {
        let ctx = json!({});
        assert_eq!(resolve_value("42", &ctx), json!(42));
        assert_eq!(resolve_value("-10", &ctx), json!(-10));
        assert_eq!(resolve_value("3.14", &ctx), json!(3.14));
    }

    #[test]
    fn test_resolve_string_literals() {
        let ctx = json!({});
        assert_eq!(resolve_value("\"hello\"", &ctx), json!("hello"));
        assert_eq!(resolve_value("'world'", &ctx), json!("world"));
    }

    #[test]
    fn test_resolve_null() {
        let ctx = json!({});
        assert_eq!(resolve_value("null", &ctx), Value::Null);
        assert_eq!(resolve_value("none", &ctx), Value::Null);
    }

    #[test]
    fn test_resolve_context_path() {
        let ctx = test_context();

        assert_eq!(resolve_value("context.has_tests", &ctx), json!(true));
        assert_eq!(resolve_value("context.count", &ctx), json!(5));
        assert_eq!(resolve_value("context.name", &ctx), json!("test"));

        // Without context prefix
        assert_eq!(resolve_value("has_tests", &ctx), json!(true));
        assert_eq!(resolve_value("count", &ctx), json!(5));
    }

    #[test]
    fn test_resolve_nested_path() {
        let ctx = test_context();

        assert_eq!(resolve_value("context.nested.value", &ctx), json!(42));
        assert_eq!(resolve_value("context.nested.flag", &ctx), json!(false));
        assert_eq!(resolve_value("nested.value", &ctx), json!(42));
    }

    #[test]
    fn test_resolve_array_index() {
        let ctx = test_context();

        assert_eq!(resolve_value("context.items.0", &ctx), json!(1));
        assert_eq!(resolve_value("context.items.2", &ctx), json!(3));
        assert_eq!(resolve_value("context.items.10", &ctx), Value::Null);
    }

    #[test]
    fn test_resolve_missing_path() {
        let ctx = test_context();

        assert_eq!(resolve_value("context.nonexistent", &ctx), Value::Null);
        assert_eq!(resolve_value("context.nested.missing", &ctx), Value::Null);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // CONDITION EVALUATION TESTS
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_evaluate_boolean_literals() {
        let ctx = json!({});

        assert!(evaluate_condition("true", &ctx));
        assert!(!evaluate_condition("false", &ctx));
        assert!(evaluate_condition("TRUE", &ctx));
        assert!(!evaluate_condition("FALSE", &ctx));
    }

    #[test]
    fn test_evaluate_equality() {
        let ctx = test_context();

        assert!(evaluate_condition("context.has_tests == true", &ctx));
        assert!(evaluate_condition("context.count == 5", &ctx));
        assert!(evaluate_condition("context.name == \"test\"", &ctx));
        assert!(!evaluate_condition("context.count == 10", &ctx));
    }

    #[test]
    fn test_evaluate_inequality() {
        let ctx = test_context();

        assert!(evaluate_condition("context.count != 10", &ctx));
        assert!(!evaluate_condition("context.count != 5", &ctx));
    }

    #[test]
    fn test_evaluate_greater_than() {
        let ctx = test_context();

        assert!(evaluate_condition("context.count > 0", &ctx));
        assert!(evaluate_condition("context.count > 4", &ctx));
        assert!(!evaluate_condition("context.count > 5", &ctx));
        assert!(!evaluate_condition("context.count > 10", &ctx));
    }

    #[test]
    fn test_evaluate_greater_equal() {
        let ctx = test_context();

        assert!(evaluate_condition("context.count >= 5", &ctx));
        assert!(evaluate_condition("context.count >= 0", &ctx));
        assert!(!evaluate_condition("context.count >= 6", &ctx));
    }

    #[test]
    fn test_evaluate_less_than() {
        let ctx = test_context();

        assert!(evaluate_condition("context.count < 10", &ctx));
        assert!(!evaluate_condition("context.count < 5", &ctx));
        assert!(!evaluate_condition("context.count < 0", &ctx));
    }

    #[test]
    fn test_evaluate_less_equal() {
        let ctx = test_context();

        assert!(evaluate_condition("context.count <= 5", &ctx));
        assert!(evaluate_condition("context.count <= 10", &ctx));
        assert!(!evaluate_condition("context.count <= 4", &ctx));
    }

    #[test]
    fn test_evaluate_truthy_check() {
        let ctx = test_context();

        // Truthy values
        assert!(evaluate_condition("context.has_tests", &ctx));
        assert!(evaluate_condition("context.count", &ctx));
        assert!(evaluate_condition("context.name", &ctx));
        assert!(evaluate_condition("context.items", &ctx));
        assert!(evaluate_condition("context.nested", &ctx));

        // Falsy values
        assert!(!evaluate_condition("context.nested.flag", &ctx));
        assert!(!evaluate_condition("context.empty", &ctx));
        assert!(!evaluate_condition("context.zero", &ctx));
        assert!(!evaluate_condition("context.nonexistent", &ctx));
    }

    #[test]
    fn test_evaluate_empty_condition() {
        let ctx = json!({});
        assert!(!evaluate_condition("", &ctx));
        assert!(!evaluate_condition("   ", &ctx));
    }

    #[test]
    fn test_evaluate_nested_comparison() {
        let ctx = test_context();

        assert!(evaluate_condition("context.nested.value == 42", &ctx));
        assert!(evaluate_condition("context.nested.value > 40", &ctx));
        assert!(!evaluate_condition("context.nested.flag == true", &ctx));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // EDGE CASES
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_whitespace_handling() {
        let ctx = test_context();

        assert!(evaluate_condition("  context.count  ==  5  ", &ctx));
        assert!(evaluate_condition("context.count>4", &ctx));
        assert!(evaluate_condition("  true  ", &ctx));
    }

    #[test]
    fn test_string_boolean_comparison() {
        let ctx = json!({"str_true": "true", "str_false": "false"});

        assert!(evaluate_condition("context.str_true == true", &ctx));
        assert!(evaluate_condition("context.str_false == false", &ctx));
    }

    #[test]
    fn test_null_comparison() {
        let ctx = json!({"value": null, "present": 42});

        assert!(evaluate_condition("context.value == null", &ctx));
        assert!(evaluate_condition("context.missing == null", &ctx));
        assert!(!evaluate_condition("context.present == null", &ctx));
    }
}
