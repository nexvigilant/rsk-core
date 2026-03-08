//! JSON Processing Module for RSK
//!
//! Provides high-performance JSON parsing, serialization, and manipulation.
//! Replaces Python's json module with Rust's serde_json for 2-5x speedup.
//!
//! Key use cases:
//! - Config file parsing
//! - API response handling
//! - Data serialization for caching
//! - Schema validation

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use thiserror::Error;

// === ERROR TYPES ===

#[derive(Error, Debug, Serialize)]
pub enum JsonError {
    #[error("JSON parse error: {0}")]
    ParseError(String),

    #[error("JSON serialization error: {0}")]
    SerializeError(String),

    #[error("Invalid JSON path: {path}")]
    InvalidPath { path: String },

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Key not found: {key}")]
    KeyNotFound { key: String },
}

// === RESULT TYPES ===

#[derive(Debug, Serialize, Deserialize)]
pub struct ParseResult {
    pub status: String,
    pub data: Value,
    pub keys: Vec<String>,
    pub depth: usize,
    pub value_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializeResult {
    pub status: String,
    pub json: String,
    pub size_bytes: usize,
    pub pretty: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub status: String,
    pub found: bool,
    pub value: Option<Value>,
    pub path: String,
    pub value_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MergeResult {
    pub status: String,
    pub data: Value,
    pub keys_added: usize,
    pub keys_overwritten: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiffResult {
    pub status: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub modified: Vec<String>,
    pub unchanged: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlattenResult {
    pub status: String,
    pub data: HashMap<String, Value>,
    pub total_keys: usize,
}

// === PUBLIC FUNCTIONS ===

/// Parse JSON string into structured result
pub fn parse_json(content: &str) -> Result<ParseResult, JsonError> {
    let data: Value =
        serde_json::from_str(content).map_err(|e| JsonError::ParseError(e.to_string()))?;

    let keys = extract_keys(&data);
    let depth = calculate_depth(&data);
    let value_type = get_value_type(&data);

    Ok(ParseResult {
        status: "success".to_string(),
        data,
        keys,
        depth,
        value_type,
    })
}

/// Parse JSON from bytes
pub fn parse_json_bytes(bytes: &[u8]) -> Result<ParseResult, JsonError> {
    let data: Value =
        serde_json::from_slice(bytes).map_err(|e| JsonError::ParseError(e.to_string()))?;

    let keys = extract_keys(&data);
    let depth = calculate_depth(&data);
    let value_type = get_value_type(&data);

    Ok(ParseResult {
        status: "success".to_string(),
        data,
        keys,
        depth,
        value_type,
    })
}

/// Serialize value to JSON string
pub fn serialize_json(value: &Value, pretty: bool) -> Result<SerializeResult, JsonError> {
    let json = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .map_err(|e| JsonError::SerializeError(e.to_string()))?;

    let size_bytes = json.len();

    Ok(SerializeResult {
        status: "success".to_string(),
        json,
        size_bytes,
        pretty,
    })
}

/// Serialize value to JSON bytes (more efficient for large data)
pub fn serialize_json_bytes(value: &Value) -> Result<Vec<u8>, JsonError> {
    serde_json::to_vec(value).map_err(|e| JsonError::SerializeError(e.to_string()))
}

/// Query a value at a JSON path (dot notation: "a.b.c" or bracket: "a[0].b")
pub fn query_path(data: &Value, path: &str) -> QueryResult {
    let parts = parse_path(path);
    let mut current = data;

    for part in &parts {
        current = match part {
            PathPart::Key(key) => match current.get(key) {
                Some(v) => v,
                None => {
                    return QueryResult {
                        status: "success".to_string(),
                        found: false,
                        value: None,
                        path: path.to_string(),
                        value_type: None,
                    };
                }
            },
            PathPart::Index(idx) => match current.get(*idx) {
                Some(v) => v,
                None => {
                    return QueryResult {
                        status: "success".to_string(),
                        found: false,
                        value: None,
                        path: path.to_string(),
                        value_type: None,
                    };
                }
            },
        };
    }

    QueryResult {
        status: "success".to_string(),
        found: true,
        value: Some(current.clone()),
        path: path.to_string(),
        value_type: Some(get_value_type(current)),
    }
}

/// Set a value at a JSON path (creates intermediate objects/arrays as needed)
pub fn set_path(data: &mut Value, path: &str, value: Value) -> Result<(), JsonError> {
    let parts = parse_path(path);
    if parts.is_empty() {
        *data = value;
        return Ok(());
    }

    let mut current = data;

    for (i, part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;

        if is_last {
            match part {
                PathPart::Key(key) => {
                    if let Value::Object(map) = current {
                        map.insert(key.clone(), value);
                        return Ok(());
                    }
                    return Err(JsonError::TypeMismatch {
                        expected: "object".to_string(),
                        actual: get_value_type(current),
                    });
                }
                PathPart::Index(idx) => {
                    if let Value::Array(arr) = current {
                        while arr.len() <= *idx {
                            arr.push(Value::Null);
                        }
                        arr[*idx] = value;
                        return Ok(());
                    }
                    return Err(JsonError::TypeMismatch {
                        expected: "array".to_string(),
                        actual: get_value_type(current),
                    });
                }
            }
        }

        // Navigate to next level, creating if needed
        let next_is_index = matches!(parts.get(i + 1), Some(PathPart::Index(_)));

        current = match part {
            PathPart::Key(key) => {
                if !current.is_object() {
                    *current = Value::Object(Map::new());
                }
                #[allow(clippy::unwrap_used)] // guarded: *current was just set to Object above if it wasn't already
                let obj = current.as_object_mut().unwrap();
                obj.entry(key.clone()).or_insert_with(|| {
                    if next_is_index {
                        Value::Array(vec![])
                    } else {
                        Value::Object(Map::new())
                    }
                })
            }
            PathPart::Index(idx) => {
                if !current.is_array() {
                    *current = Value::Array(vec![]);
                }
                #[allow(clippy::unwrap_used)] // guarded: *current was just set to Array above if it wasn't already
                let arr = current.as_array_mut().unwrap();
                while arr.len() <= *idx {
                    arr.push(if next_is_index {
                        Value::Array(vec![])
                    } else {
                        Value::Object(Map::new())
                    });
                }
                &mut arr[*idx]
            }
        };
    }

    Ok(())
}

/// Deep merge two JSON objects (source into target)
pub fn merge_json(target: &Value, source: &Value) -> MergeResult {
    let mut result = target.clone();
    let (added, overwritten) = merge_recursive(&mut result, source);

    MergeResult {
        status: "success".to_string(),
        data: result,
        keys_added: added,
        keys_overwritten: overwritten,
    }
}

/// Compare two JSON values and return differences
pub fn diff_json(left: &Value, right: &Value) -> DiffResult {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();
    let mut unchanged = Vec::new();

    diff_recursive(
        left,
        right,
        "",
        &mut added,
        &mut removed,
        &mut modified,
        &mut unchanged,
    );

    DiffResult {
        status: "success".to_string(),
        added,
        removed,
        modified,
        unchanged,
    }
}

/// Flatten nested JSON into dot-notation keys
pub fn flatten_json(data: &Value) -> FlattenResult {
    let mut result = HashMap::new();
    flatten_recursive(data, "", &mut result);

    let total_keys = result.len();

    FlattenResult {
        status: "success".to_string(),
        data: result,
        total_keys,
    }
}

/// Unflatten dot-notation keys back into nested JSON
pub fn unflatten_json(data: &HashMap<String, Value>) -> Result<Value, JsonError> {
    let mut result = Value::Object(Map::new());

    for (path, value) in data {
        set_path(&mut result, path, value.clone())?;
    }

    Ok(result)
}

/// Validate JSON against a simple type schema
pub fn validate_type(data: &Value, expected_type: &str) -> bool {
    match expected_type {
        "object" => data.is_object(),
        "array" => data.is_array(),
        "string" => data.is_string(),
        "number" => data.is_number(),
        "integer" => data.is_i64() || data.is_u64(),
        "float" => data.is_f64(),
        "boolean" => data.is_boolean(),
        "null" => data.is_null(),
        _ => false,
    }
}

/// Get all keys from a JSON object (non-recursive)
pub fn get_keys(data: &Value) -> Vec<String> {
    match data {
        Value::Object(map) => map.keys().cloned().collect(),
        _ => vec![],
    }
}

/// Get all values from a JSON array
pub fn get_values(data: &Value) -> Vec<Value> {
    match data {
        Value::Array(arr) => arr.clone(),
        _ => vec![],
    }
}

// === HELPER FUNCTIONS ===

#[derive(Debug)]
enum PathPart {
    Key(String),
    Index(usize),
}

fn parse_path(path: &str) -> Vec<PathPart> {
    let mut parts = Vec::new();
    let mut current_key = String::new();
    let mut in_bracket = false;
    let mut bracket_content = String::new();

    for ch in path.chars() {
        match ch {
            '.' if !in_bracket => {
                if !current_key.is_empty() {
                    parts.push(PathPart::Key(current_key.clone()));
                    current_key.clear();
                }
            }
            '[' => {
                if !current_key.is_empty() {
                    parts.push(PathPart::Key(current_key.clone()));
                    current_key.clear();
                }
                in_bracket = true;
                bracket_content.clear();
            }
            ']' => {
                if in_bracket {
                    if let Ok(idx) = bracket_content.parse::<usize>() {
                        parts.push(PathPart::Index(idx));
                    } else {
                        parts.push(PathPart::Key(bracket_content.clone()));
                    }
                    in_bracket = false;
                }
            }
            _ => {
                if in_bracket {
                    bracket_content.push(ch);
                } else {
                    current_key.push(ch);
                }
            }
        }
    }

    if !current_key.is_empty() {
        parts.push(PathPart::Key(current_key));
    }

    parts
}

fn extract_keys(data: &Value) -> Vec<String> {
    match data {
        Value::Object(map) => map.keys().cloned().collect(),
        _ => vec![],
    }
}

fn calculate_depth(data: &Value) -> usize {
    match data {
        Value::Object(map) => 1 + map.values().map(calculate_depth).max().unwrap_or(0),
        Value::Array(arr) => 1 + arr.iter().map(calculate_depth).max().unwrap_or(0),
        _ => 0,
    }
}

fn get_value_type(data: &Value) -> String {
    match data {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "boolean".to_string(),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer".to_string()
            } else {
                "float".to_string()
            }
        }
        Value::String(_) => "string".to_string(),
        Value::Array(_) => "array".to_string(),
        Value::Object(_) => "object".to_string(),
    }
}

fn merge_recursive(target: &mut Value, source: &Value) -> (usize, usize) {
    let mut added = 0;
    let mut overwritten = 0;

    match (target, source) {
        (Value::Object(target_map), Value::Object(source_map)) => {
            for (key, source_value) in source_map {
                if let Some(target_value) = target_map.get_mut(key) {
                    if target_value.is_object() && source_value.is_object() {
                        let (a, o) = merge_recursive(target_value, source_value);
                        added += a;
                        overwritten += o;
                    } else {
                        *target_value = source_value.clone();
                        overwritten += 1;
                    }
                } else {
                    target_map.insert(key.clone(), source_value.clone());
                    added += 1;
                }
            }
        }
        (target, source) => {
            *target = source.clone();
            overwritten += 1;
        }
    }

    (added, overwritten)
}

fn diff_recursive(
    left: &Value,
    right: &Value,
    path: &str,
    added: &mut Vec<String>,
    removed: &mut Vec<String>,
    modified: &mut Vec<String>,
    unchanged: &mut Vec<String>,
) {
    match (left, right) {
        (Value::Object(left_map), Value::Object(right_map)) => {
            // Check for removed and modified keys
            for (key, left_value) in left_map {
                let full_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };

                if let Some(right_value) = right_map.get(key) {
                    diff_recursive(
                        left_value,
                        right_value,
                        &full_path,
                        added,
                        removed,
                        modified,
                        unchanged,
                    );
                } else {
                    removed.push(full_path);
                }
            }

            // Check for added keys
            for key in right_map.keys() {
                if !left_map.contains_key(key) {
                    let full_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    added.push(full_path);
                }
            }
        }
        (Value::Array(left_arr), Value::Array(right_arr)) => {
            let max_len = left_arr.len().max(right_arr.len());
            for i in 0..max_len {
                let full_path = if path.is_empty() {
                    format!("[{i}]")
                } else {
                    format!("{path}[{i}]")
                };

                match (left_arr.get(i), right_arr.get(i)) {
                    (Some(l), Some(r)) => {
                        diff_recursive(l, r, &full_path, added, removed, modified, unchanged);
                    }
                    (Some(_), None) => removed.push(full_path),
                    (None, Some(_)) => added.push(full_path),
                    (None, None) => {}
                }
            }
        }
        _ => {
            let full_path = if path.is_empty() {
                "$".to_string()
            } else {
                path.to_string()
            };

            if left == right {
                unchanged.push(full_path);
            } else {
                modified.push(full_path);
            }
        }
    }
}

fn flatten_recursive(data: &Value, prefix: &str, result: &mut HashMap<String, Value>) {
    match data {
        Value::Object(map) => {
            for (key, value) in map {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_recursive(value, &new_prefix, result);
            }
        }
        Value::Array(arr) => {
            for (i, value) in arr.iter().enumerate() {
                let new_prefix = if prefix.is_empty() {
                    format!("[{i}]")
                } else {
                    format!("{prefix}[{i}]")
                };
                flatten_recursive(value, &new_prefix, result);
            }
        }
        _ => {
            result.insert(prefix.to_string(), data.clone());
        }
    }
}

// === TESTS ===

#[cfg(test)]
mod tests {
    use super::*;

    // === POSITIVE TESTS ===

    #[test]
    fn test_parse_json_object() {
        let json = r#"{"name": "test", "value": 42}"#;
        let result = parse_json(json).unwrap();
        assert_eq!(result.status, "success");
        assert_eq!(result.value_type, "object");
        assert!(result.keys.contains(&"name".to_string()));
    }

    #[test]
    fn test_parse_json_array() {
        let json = r#"[1, 2, 3, 4, 5]"#;
        let result = parse_json(json).unwrap();
        assert_eq!(result.status, "success");
        assert_eq!(result.value_type, "array");
    }

    #[test]
    fn test_parse_json_nested() {
        let json = r#"{"a": {"b": {"c": {"d": 1}}}}"#;
        let result = parse_json(json).unwrap();
        assert_eq!(result.depth, 4);
    }

    #[test]
    fn test_serialize_json_compact() {
        let value = serde_json::json!({"name": "test", "value": 42});
        let result = serialize_json(&value, false).unwrap();
        assert!(!result.json.contains('\n'));
        assert!(result.size_bytes > 0);
    }

    #[test]
    fn test_serialize_json_pretty() {
        let value = serde_json::json!({"name": "test", "value": 42});
        let result = serialize_json(&value, true).unwrap();
        assert!(result.json.contains('\n'));
        assert!(result.pretty);
    }

    #[test]
    fn test_query_path_dot_notation() {
        let json = r#"{"a": {"b": {"c": 42}}}"#;
        let data = parse_json(json).unwrap().data;
        let result = query_path(&data, "a.b.c");
        assert!(result.found);
        assert_eq!(result.value, Some(serde_json::json!(42)));
    }

    #[test]
    fn test_query_path_bracket_notation() {
        let json = r#"{"items": [1, 2, 3]}"#;
        let data = parse_json(json).unwrap().data;
        let result = query_path(&data, "items[1]");
        assert!(result.found);
        assert_eq!(result.value, Some(serde_json::json!(2)));
    }

    #[test]
    fn test_query_path_mixed_notation() {
        let json = r#"{"users": [{"name": "Alice"}, {"name": "Bob"}]}"#;
        let data = parse_json(json).unwrap().data;
        let result = query_path(&data, "users[0].name");
        assert!(result.found);
        assert_eq!(result.value, Some(serde_json::json!("Alice")));
    }

    #[test]
    fn test_set_path_simple() {
        let mut data = serde_json::json!({});
        set_path(&mut data, "name", serde_json::json!("test")).unwrap();
        assert_eq!(data["name"], "test");
    }

    #[test]
    fn test_set_path_nested() {
        let mut data = serde_json::json!({});
        set_path(&mut data, "a.b.c", serde_json::json!(42)).unwrap();
        assert_eq!(data["a"]["b"]["c"], 42);
    }

    #[test]
    fn test_set_path_array() {
        let mut data = serde_json::json!({"items": []});
        set_path(&mut data, "items[0]", serde_json::json!("first")).unwrap();
        set_path(&mut data, "items[2]", serde_json::json!("third")).unwrap();
        assert_eq!(data["items"][0], "first");
        assert_eq!(data["items"][2], "third");
    }

    #[test]
    fn test_merge_json() {
        let target = serde_json::json!({"a": 1, "b": 2});
        let source = serde_json::json!({"b": 3, "c": 4});
        let result = merge_json(&target, &source);
        assert_eq!(result.data["a"], 1);
        assert_eq!(result.data["b"], 3);
        assert_eq!(result.data["c"], 4);
        assert_eq!(result.keys_added, 1);
        assert_eq!(result.keys_overwritten, 1);
    }

    #[test]
    fn test_merge_json_deep() {
        let target = serde_json::json!({"config": {"a": 1, "b": 2}});
        let source = serde_json::json!({"config": {"b": 3, "c": 4}});
        let result = merge_json(&target, &source);
        assert_eq!(result.data["config"]["a"], 1);
        assert_eq!(result.data["config"]["b"], 3);
        assert_eq!(result.data["config"]["c"], 4);
    }

    #[test]
    fn test_diff_json() {
        let left = serde_json::json!({"a": 1, "b": 2, "c": 3});
        let right = serde_json::json!({"a": 1, "b": 5, "d": 4});
        let result = diff_json(&left, &right);
        assert!(result.added.contains(&"d".to_string()));
        assert!(result.removed.contains(&"c".to_string()));
        assert!(result.modified.contains(&"b".to_string()));
        assert!(result.unchanged.contains(&"a".to_string()));
    }

    #[test]
    fn test_flatten_json() {
        let data = serde_json::json!({"a": {"b": 1}, "c": [2, 3]});
        let result = flatten_json(&data);
        assert_eq!(result.data.get("a.b"), Some(&serde_json::json!(1)));
        assert_eq!(result.data.get("c[0]"), Some(&serde_json::json!(2)));
        assert_eq!(result.data.get("c[1]"), Some(&serde_json::json!(3)));
    }

    #[test]
    fn test_unflatten_json() {
        let mut flat = HashMap::new();
        flat.insert("a.b".to_string(), serde_json::json!(1));
        flat.insert("a.c".to_string(), serde_json::json!(2));
        let result = unflatten_json(&flat).unwrap();
        assert_eq!(result["a"]["b"], 1);
        assert_eq!(result["a"]["c"], 2);
    }

    #[test]
    fn test_validate_type() {
        assert!(validate_type(&serde_json::json!({}), "object"));
        assert!(validate_type(&serde_json::json!([]), "array"));
        assert!(validate_type(&serde_json::json!("test"), "string"));
        assert!(validate_type(&serde_json::json!(42), "integer"));
        assert!(validate_type(&serde_json::json!(3.14), "float"));
        assert!(validate_type(&serde_json::json!(true), "boolean"));
        assert!(validate_type(&serde_json::json!(null), "null"));
    }

    // === NEGATIVE TESTS ===

    #[test]
    fn test_parse_json_invalid() {
        let json = "not valid json {";
        let result = parse_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_query_path_not_found() {
        let data = serde_json::json!({"a": 1});
        let result = query_path(&data, "b");
        assert!(!result.found);
        assert!(result.value.is_none());
    }

    #[test]
    fn test_query_path_deep_not_found() {
        let data = serde_json::json!({"a": {"b": 1}});
        let result = query_path(&data, "a.c.d");
        assert!(!result.found);
    }

    // === EDGE CASES ===

    #[test]
    fn test_parse_json_empty_object() {
        let json = "{}";
        let result = parse_json(json).unwrap();
        assert_eq!(result.value_type, "object");
        assert!(result.keys.is_empty());
    }

    #[test]
    fn test_parse_json_empty_array() {
        let json = "[]";
        let result = parse_json(json).unwrap();
        assert_eq!(result.value_type, "array");
    }

    #[test]
    fn test_parse_json_unicode() {
        let json = r#"{"emoji": "😀", "chinese": "中文"}"#;
        let result = parse_json(json).unwrap();
        assert!(result.data["emoji"].as_str().unwrap().contains("😀"));
    }

    #[test]
    fn test_query_path_empty() {
        let data = serde_json::json!({"a": 1});
        let result = query_path(&data, "");
        // Empty path returns the root
        assert!(result.found);
    }

    #[test]
    fn test_diff_identical() {
        let data = serde_json::json!({"a": 1, "b": 2});
        let result = diff_json(&data, &data);
        assert!(result.added.is_empty());
        assert!(result.removed.is_empty());
        assert!(result.modified.is_empty());
    }

    // === STRESS TESTS ===

    #[test]
    fn test_parse_large_array() {
        let items: Vec<i32> = (0..10000).collect();
        let json = serde_json::to_string(&items).unwrap();
        let result = parse_json(&json).unwrap();
        assert_eq!(result.value_type, "array");
    }

    #[test]
    fn test_deeply_nested() {
        // Create 50-level deep nesting
        let mut json = String::from("{\"l0\":");
        for i in 1..50 {
            json.push_str(&format!("{{\"l{i}\":"));
        }
        json.push_str("1");
        for _ in 0..50 {
            json.push('}');
        }

        let result = parse_json(&json).unwrap();
        assert!(result.depth >= 50);
    }

    #[test]
    fn test_flatten_large() {
        let mut data = serde_json::json!({});
        for i in 0..100 {
            data[format!("key{i}")] = serde_json::json!(i);
        }
        let result = flatten_json(&data);
        assert_eq!(result.total_keys, 100);
    }

    // === ADVERSARIAL TESTS ===

    #[test]
    fn test_parse_json_with_special_chars() {
        let json = r#"{"key": "value\nwith\nnewlines\tand\ttabs"}"#;
        let result = parse_json(json).unwrap();
        assert!(result.data["key"].as_str().unwrap().contains('\n'));
    }

    #[test]
    fn test_path_with_dots_in_key() {
        let json = r#"{"a.b": {"c": 1}}"#;
        let data = parse_json(json).unwrap().data;
        // Bracket notation should handle dots in keys
        let result = query_path(&data, "[a.b].c");
        assert!(result.found);
    }

    #[test]
    fn test_merge_conflicting_types() {
        let target = serde_json::json!({"a": [1, 2, 3]});
        let source = serde_json::json!({"a": {"nested": true}});
        let result = merge_json(&target, &source);
        // Source overwrites target completely when types differ
        assert!(result.data["a"].is_object());
    }
}
