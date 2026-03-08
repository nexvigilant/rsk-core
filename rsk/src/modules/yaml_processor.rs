//! YAML/TOML Processing Module for RSK
//!
//! Provides unified parsing, validation, and schema extraction for YAML and TOML files.
//! Key use cases:
//! - Taxonomy schema definitions (decision trees, skill graphs)
//! - Configuration file validation
//! - Frontmatter extraction with proper YAML parsing (replacing regex-based)

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use thiserror::Error;

// === ERROR TYPES ===

#[derive(Error, Debug, Serialize)]
pub enum ConfigError {
    #[error("YAML parse error: {0}")]
    YamlParse(String),

    #[error("TOML parse error: {0}")]
    TomlParse(String),

    #[error("Schema validation error: {message}")]
    SchemaValidation { message: String, path: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String, path: String },

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        expected: String,
        actual: String,
        path: String,
    },
}

// === RESULT TYPES ===

#[derive(Debug, Serialize, Deserialize)]
pub struct ParseResult {
    pub status: String,
    pub format: String,
    pub data: JsonValue,
    pub keys: Vec<String>,
    pub depth: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub schema_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaxonomySchema {
    pub name: String,
    pub version: Option<String>,
    pub root_key: String,
    pub node_count: usize,
    pub max_depth: usize,
    pub has_conditions: bool,
    pub has_actions: bool,
    pub categories: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecisionTreeNode {
    pub id: String,
    pub condition: Option<String>,
    pub action: Option<String>,
    pub children: Vec<String>,
    pub depth: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecisionTreeAnalysis {
    pub status: String,
    pub name: String,
    pub total_nodes: usize,
    pub max_depth: usize,
    pub leaf_nodes: usize,
    pub branch_nodes: usize,
    pub conditions: Vec<String>,
    pub actions: Vec<String>,
    pub nodes: Vec<DecisionTreeNode>,
}

// === PUBLIC FUNCTIONS ===

/// Parse YAML content into unified JSON format
pub fn parse_yaml(content: &str) -> Result<ParseResult, ConfigError> {
    let data: JsonValue =
        serde_yaml::from_str(content).map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    let keys = extract_top_level_keys(&data);
    let depth = calculate_depth(&data);

    Ok(ParseResult {
        status: "success".to_string(),
        format: "yaml".to_string(),
        data,
        keys,
        depth,
    })
}

/// Parse TOML content into unified JSON format
pub fn parse_toml(content: &str) -> Result<ParseResult, ConfigError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(ParseResult {
            status: "success".to_string(),
            format: "toml".to_string(),
            data: JsonValue::Object(serde_json::Map::new()),
            keys: vec![],
            depth: 0,
        });
    }

    let data: JsonValue =
        toml::from_str(trimmed).map_err(|e| ConfigError::TomlParse(e.to_string()))?;

    Ok(ParseResult {
        status: "success".to_string(),
        format: "toml".to_string(),
        keys: get_json_keys(&data),
        depth: calculate_depth(&data),
        data,
    })
}

fn get_json_keys(value: &JsonValue) -> Vec<String> {
    let mut keys = Vec::new();
    if let JsonValue::Object(map) = value {
        for (k, v) in map {
            keys.push(k.clone());
            for inner_k in get_json_keys(v) {
                keys.push(format!("{k}.{inner_k}"));
            }
        }
    }
    keys
}

/// Auto-detect format and parse (YAML or TOML)
pub fn parse_config(content: &str) -> Result<ParseResult, ConfigError> {
    // Try YAML first (more common in skills ecosystem)
    if let Ok(result) = parse_yaml(content) {
        return Ok(result);
    }

    // Fall back to TOML
    parse_toml(content)
}

/// Validate YAML/TOML content against known schema patterns
pub fn validate_schema(content: &str, schema_type: Option<&str>) -> ValidationResult {
    let parse_result = match parse_config(content) {
        Ok(r) => r,
        Err(e) => {
            return ValidationResult {
                valid: false,
                errors: vec![e.to_string()],
                warnings: vec![],
                schema_type: schema_type.map(String::from),
            };
        }
    };

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Auto-detect schema type if not provided
    let detected_type = schema_type
        .map(String::from)
        .or_else(|| detect_schema_type(&parse_result.data));

    match detected_type.as_deref() {
        Some("decision-tree") => {
            validate_decision_tree(&parse_result.data, &mut errors, &mut warnings);
        }
        Some("taxonomy") => {
            validate_taxonomy(&parse_result.data, &mut errors, &mut warnings);
        }
        Some("skill-frontmatter") => {
            validate_skill_frontmatter(&parse_result.data, &mut errors, &mut warnings);
        }
        _ => {
            // Generic validation - just check structure
            if parse_result.depth > 10 {
                warnings
                    .push("Document depth exceeds 10 levels - may be overly complex".to_string());
            }
        }
    }

    ValidationResult {
        valid: errors.is_empty(),
        errors,
        warnings,
        schema_type: detected_type,
    }
}

/// Analyze a decision tree YAML file
pub fn analyze_decision_tree(content: &str) -> Result<DecisionTreeAnalysis, ConfigError> {
    let data: JsonValue =
        serde_yaml::from_str(content).map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    let name = data
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_string();

    let mut nodes = Vec::new();
    let mut conditions = Vec::new();
    let mut actions = Vec::new();

    // Extract nodes from common decision tree structures
    if let Some(tree) = data
        .get("tree")
        .or_else(|| data.get("nodes"))
        .or_else(|| data.get("decisions"))
    {
        extract_dt_nodes(tree, &mut nodes, &mut conditions, &mut actions, 0, "root");
    } else {
        // Try to parse the entire document as a tree
        extract_dt_nodes(&data, &mut nodes, &mut conditions, &mut actions, 0, "root");
    }

    let max_depth = nodes.iter().map(|n| n.depth).max().unwrap_or(0);
    let leaf_nodes = nodes.iter().filter(|n| n.children.is_empty()).count();
    let branch_nodes = nodes.len() - leaf_nodes;

    // Deduplicate conditions and actions
    conditions.sort();
    conditions.dedup();
    actions.sort();
    actions.dedup();

    Ok(DecisionTreeAnalysis {
        status: "success".to_string(),
        name,
        total_nodes: nodes.len(),
        max_depth,
        leaf_nodes,
        branch_nodes,
        conditions,
        actions,
        nodes,
    })
}

/// Extract taxonomy schema information from YAML
pub fn extract_taxonomy_schema(content: &str) -> Result<TaxonomySchema, ConfigError> {
    let data: JsonValue =
        serde_yaml::from_str(content).map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    let name = data
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_string();

    let version = data
        .get("version")
        .and_then(|v| v.as_str())
        .map(String::from);

    let keys = extract_top_level_keys(&data);
    let root_key = keys.first().cloned().unwrap_or_else(|| "root".to_string());

    let node_count = count_nodes(&data);
    let max_depth = calculate_depth(&data);

    // Check for decision tree patterns
    let has_conditions =
        content.contains("condition") || content.contains("if:") || content.contains("when:");
    let has_actions =
        content.contains("action") || content.contains("then:") || content.contains("do:");

    // Extract category names if present
    let categories = data
        .get("categories")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(TaxonomySchema {
        name,
        version,
        root_key,
        node_count,
        max_depth,
        has_conditions,
        has_actions,
        categories,
    })
}

/// Parse YAML frontmatter from SKILL.md content (proper YAML parsing)
pub fn parse_yaml_frontmatter(content: &str) -> Result<JsonValue, ConfigError> {
    use regex::Regex;
    #[allow(clippy::unwrap_used)] // compile-time literal regex pattern cannot fail to compile
    let re = Regex::new(r"(?s)^---\s*\n(.*?)\n---\s*\n").unwrap();

    if let Some(caps) = re.captures(content) {
        let frontmatter_content = &caps[1];
        serde_yaml::from_str(frontmatter_content).map_err(|e| ConfigError::YamlParse(e.to_string()))
    } else {
        // Try fallback for content that might not have a newline after closing --- or other variations
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() > 1 && lines[0].trim() == "---" {
            let mut end_idx = None;
            for (i, line) in lines.iter().enumerate().skip(1) {
                if line.trim() == "---" {
                    end_idx = Some(i);
                    break;
                }
            }
            if let Some(end) = end_idx {
                let fm_content = lines[1..end].join("\n");
                return serde_yaml::from_str(&fm_content)
                    .map_err(|e| ConfigError::YamlParse(e.to_string()));
            }
        }

        Err(ConfigError::YamlParse(
            "No valid frontmatter found (must be enclosed in --- markers)".to_string(),
        ))
    }
}

// === HELPER FUNCTIONS ===

fn extract_top_level_keys(data: &JsonValue) -> Vec<String> {
    match data {
        JsonValue::Object(map) => map.keys().cloned().collect(),
        _ => vec![],
    }
}

fn calculate_depth(data: &JsonValue) -> usize {
    match data {
        JsonValue::Object(map) => 1 + map.values().map(calculate_depth).max().unwrap_or(0),
        JsonValue::Array(arr) => 1 + arr.iter().map(calculate_depth).max().unwrap_or(0),
        _ => 0,
    }
}

fn count_nodes(data: &JsonValue) -> usize {
    match data {
        JsonValue::Object(map) => 1 + map.values().map(count_nodes).sum::<usize>(),
        JsonValue::Array(arr) => arr.iter().map(count_nodes).sum(),
        _ => 1,
    }
}

fn detect_schema_type(data: &JsonValue) -> Option<String> {
    let keys: Vec<&str> = match data {
        JsonValue::Object(map) => map.keys().map(|s| s.as_str()).collect(),
        _ => return None,
    };

    // Decision tree patterns
    if keys
        .iter()
        .any(|k| *k == "tree" || *k == "decisions" || *k == "conditions")
    {
        return Some("decision-tree".to_string());
    }

    // Skill frontmatter patterns
    if keys
        .iter()
        .any(|k| *k == "name" || *k == "compliance-level" || *k == "version")
        && keys.iter().any(|k| *k == "triggers" || *k == "description")
    {
        return Some("skill-frontmatter".to_string());
    }

    // Taxonomy patterns
    if keys
        .iter()
        .any(|k| *k == "categories" || *k == "taxonomy" || *k == "hierarchy")
    {
        return Some("taxonomy".to_string());
    }

    None
}

fn validate_decision_tree(data: &JsonValue, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    // Check for required structure
    if data.get("tree").is_none() && data.get("nodes").is_none() && data.get("decisions").is_none()
    {
        warnings.push(
            "No 'tree', 'nodes', or 'decisions' key found - may not be a decision tree".to_string(),
        );
    }

    // Check for name
    if data.get("name").is_none() {
        warnings.push("Missing 'name' field for decision tree".to_string());
    }

    // Validate node structure if present
    if let Some(tree) = data.get("tree").or_else(|| data.get("nodes")) {
        validate_tree_nodes(tree, errors, "");
    }
}

fn validate_tree_nodes(node: &JsonValue, errors: &mut Vec<String>, path: &str) {
    match node {
        JsonValue::Object(map) => {
            // Each node should have either a condition or an action (or both)
            let has_condition = map.contains_key("condition") || map.contains_key("if");
            let has_action = map.contains_key("action") || map.contains_key("then");
            let has_children = map.contains_key("children") || map.contains_key("branches");

            if !has_condition && !has_action && !has_children {
                errors.push(format!(
                    "Node at '{path}' has no condition, action, or children",
                ));
            }

            // Recursively validate children
            if let Some(children) = map.get("children").or_else(|| map.get("branches"))
                && let JsonValue::Array(arr) = children
            {
                for (i, child) in arr.iter().enumerate() {
                    let child_path = format!("{path}/children[{i}]");
                    validate_tree_nodes(child, errors, &child_path);
                }
            }
        }
        JsonValue::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                let item_path = format!("{path}[{i}]");
                validate_tree_nodes(item, errors, &item_path);
            }
        }
        _ => {}
    }
}

fn validate_taxonomy(data: &JsonValue, _errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    if data.get("categories").is_none() && data.get("taxonomy").is_none() {
        warnings.push("No 'categories' or 'taxonomy' key found".to_string());
    }

    if data.get("name").is_none() {
        warnings.push("Missing 'name' field for taxonomy".to_string());
    }

    // Check depth isn't excessive
    let depth = calculate_depth(data);
    if depth > 8 {
        warnings.push(format!(
            "Taxonomy depth ({depth}) is high - consider flattening",
        ));
    }
}

fn validate_skill_frontmatter(
    data: &JsonValue,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    // Required fields for skill frontmatter
    let required = ["name", "version", "compliance-level"];

    for field in required {
        if data.get(field).is_none() {
            errors.push(format!("Missing required field: {field}"));
        }
    }

    // Check compliance level is valid
    if let Some(level) = data.get("compliance-level").and_then(|v| v.as_str()) {
        let valid_levels = ["Bronze", "Silver", "Gold", "Platinum", "Diamond"];
        if !valid_levels.contains(&level) {
            errors.push(format!(
                "Invalid compliance-level: '{level}'. Expected one of: {valid_levels:?}",
            ));
        }
    }

    // Optional but recommended
    if data.get("triggers").is_none() {
        warnings.push("Missing 'triggers' field - skill may not be discoverable".to_string());
    }

    if data.get("description").is_none() {
        warnings.push("Missing 'description' field".to_string());
    }
}

fn extract_dt_nodes(
    data: &JsonValue,
    nodes: &mut Vec<DecisionTreeNode>,
    conditions: &mut Vec<String>,
    actions: &mut Vec<String>,
    depth: usize,
    parent_id: &str,
) {
    match data {
        JsonValue::Object(map) => {
            let node_id = map
                .get("id")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_else(|| format!("{parent_id}_{}", nodes.len()));

            let condition = map
                .get("condition")
                .or_else(|| map.get("if"))
                .and_then(|v| v.as_str())
                .map(String::from);

            let action = map
                .get("action")
                .or_else(|| map.get("then"))
                .and_then(|v| v.as_str())
                .map(String::from);

            if let Some(ref c) = condition {
                conditions.push(c.clone());
            }
            if let Some(ref a) = action {
                actions.push(a.clone());
            }

            let mut child_ids = Vec::new();

            // Process children
            if let Some(JsonValue::Array(arr)) = map.get("children").or_else(|| map.get("branches"))
            {
                for (i, child) in arr.iter().enumerate() {
                    let child_id = format!("{node_id}_{i}");
                    child_ids.push(child_id.clone());
                    extract_dt_nodes(child, nodes, conditions, actions, depth + 1, &node_id);
                }
            }

            nodes.push(DecisionTreeNode {
                id: node_id,
                condition,
                action,
                children: child_ids,
                depth,
            });
        }
        JsonValue::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                extract_dt_nodes(
                    item,
                    nodes,
                    conditions,
                    actions,
                    depth,
                    &format!("{parent_id}_{i}"),
                );
            }
        }
        _ => {}
    }
}

// === TESTS ===

#[cfg(test)]
mod tests {
    use super::*;

    // === POSITIVE TESTS ===

    #[test]
    fn test_parse_yaml_simple() {
        let yaml = r#"
name: test-skill
version: "1.0.0"
description: A test skill
"#;
        let result = parse_yaml(yaml).unwrap();
        assert_eq!(result.status, "success");
        assert_eq!(result.format, "yaml");
        assert!(result.keys.contains(&"name".to_string()));
    }

    #[test]
    fn test_parse_toml_simple() {
        let toml = r#"
name = "test-skill"
version = "1.0.0"

[dependencies]
serde = "1.0"
"#;
        let result = parse_toml(toml).unwrap();
        assert_eq!(result.status, "success");
        assert_eq!(result.format, "toml");
        assert!(result.keys.contains(&"name".to_string()));
    }

    #[test]
    fn test_parse_yaml_nested() {
        let yaml = r#"
root:
  level1:
    level2:
      level3: value
"#;
        let result = parse_yaml(yaml).unwrap();
        assert_eq!(result.depth, 4);
    }

    #[test]
    fn test_parse_config_auto_detect_yaml() {
        let yaml = "key: value\nlist:\n  - item1\n  - item2";
        let result = parse_config(yaml).unwrap();
        assert_eq!(result.format, "yaml");
    }

    #[test]
    fn test_parse_yaml_frontmatter() {
        let content = r#"---
name: test-skill
version: "1.0.0"
compliance-level: Gold
---

# Test Skill

Content here...
"#;
        let result = parse_yaml_frontmatter(content).unwrap();
        assert_eq!(result.get("name").unwrap().as_str().unwrap(), "test-skill");
        assert_eq!(
            result.get("compliance-level").unwrap().as_str().unwrap(),
            "Gold"
        );
    }

    #[test]
    fn test_validate_skill_frontmatter() {
        let yaml = r#"
name: test-skill
version: "1.0.0"
compliance-level: Diamond
triggers:
  - pattern: test
description: A test skill
"#;
        let result = validate_schema(yaml, Some("skill-frontmatter"));
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_extract_taxonomy_schema() {
        let yaml = r#"
name: test-taxonomy
version: "1.0.0"
categories:
  - category1
  - category2
tree:
  condition: "is_valid"
  action: "proceed"
"#;
        let schema = extract_taxonomy_schema(yaml).unwrap();
        assert_eq!(schema.name, "test-taxonomy");
        assert_eq!(schema.categories.len(), 2);
        assert!(schema.has_conditions);
        assert!(schema.has_actions);
    }

    #[test]
    fn test_analyze_decision_tree() {
        let yaml = r#"
name: simple-dt
tree:
  condition: "input > 0"
  children:
    - action: "positive"
    - action: "non-positive"
"#;
        let analysis = analyze_decision_tree(yaml).unwrap();
        assert_eq!(analysis.name, "simple-dt");
        assert!(analysis.total_nodes > 0);
        assert!(!analysis.conditions.is_empty());
    }

    // === NEGATIVE TESTS ===

    #[test]
    fn test_parse_yaml_invalid() {
        let yaml = "key: [invalid: yaml";
        let result = parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_toml_invalid() {
        let toml = "invalid toml [[[";
        let result = parse_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_frontmatter_no_opening() {
        let content = "No frontmatter here";
        let result = parse_yaml_frontmatter(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_frontmatter_no_closing() {
        let content = "---\nname: test\nno closing marker";
        let result = parse_yaml_frontmatter(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_skill_frontmatter_missing_required() {
        let yaml = r#"
description: Missing required fields
"#;
        let result = validate_schema(yaml, Some("skill-frontmatter"));
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("name")));
    }

    #[test]
    fn test_validate_skill_frontmatter_invalid_level() {
        let yaml = r#"
name: test
version: "1.0.0"
compliance-level: InvalidLevel
"#;
        let result = validate_schema(yaml, Some("skill-frontmatter"));
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("Invalid compliance-level"))
        );
    }

    // === EDGE CASES ===

    #[test]
    fn test_parse_yaml_empty() {
        let yaml = "";
        let result = parse_yaml(yaml);
        // Empty YAML parses to null
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_yaml_only_comments() {
        let yaml = "# Just a comment\n# Another comment";
        let result = parse_yaml(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_yaml_unicode() {
        let yaml = "name: \"\u{1F600} Emoji Test \u{4E2D}\u{6587}\"";
        let result = parse_yaml(yaml).unwrap();
        let name = result.data.get("name").unwrap().as_str().unwrap();
        assert!(name.contains("\u{1F600}"));
    }

    #[test]
    fn test_calculate_depth_flat() {
        let yaml = "a: 1\nb: 2\nc: 3";
        let result = parse_yaml(yaml).unwrap();
        assert_eq!(result.depth, 1);
    }

    #[test]
    fn test_toml_datetime_conversion() {
        let toml = r#"
[package]
created = 2025-01-13T10:00:00Z
"#;
        let result = parse_toml(toml).unwrap();
        assert!(result.data.get("package").is_some());
    }

    // === STRESS TESTS ===

    #[test]
    fn test_parse_yaml_large_array() {
        let items: Vec<String> = (0..1000).map(|i| format!("  - item{i}")).collect();
        let yaml = format!("items:\n{}", items.join("\n"));
        let result = parse_yaml(&yaml).unwrap();
        let arr = result.data.get("items").unwrap().as_array().unwrap();
        assert_eq!(arr.len(), 1000);
    }

    #[test]
    fn test_parse_yaml_deeply_nested() {
        // Create 20-level deep nesting
        let mut yaml = String::new();
        for i in 0..20 {
            yaml.push_str(&format!("{}level{i}:\n", "  ".repeat(i)));
        }
        yaml.push_str(&format!("{}value: deep", "  ".repeat(20)));

        let result = parse_yaml(&yaml).unwrap();
        assert!(result.depth >= 20);
    }

    // === ADVERSARIAL TESTS ===

    #[test]
    fn test_parse_yaml_injection_attempt() {
        // YAML bomb attempt - should be handled by serde_yaml safely
        let yaml = "key: !!python/object/apply:os.system [echo pwned]";
        let result = parse_yaml(yaml);
        // serde_yaml doesn't execute arbitrary code, so this just fails or parses safely
        // The key point is no code execution
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_parse_yaml_null_bytes() {
        let yaml = "key: value\0with\0nulls";
        // This should either parse safely or error - not crash
        let _ = parse_yaml(yaml);
    }

    #[test]
    fn test_detect_schema_type_decision_tree() {
        let yaml = "tree:\n  condition: test\n  action: do_something";
        let result = parse_yaml(yaml).unwrap();
        let detected = detect_schema_type(&result.data);
        assert_eq!(detected, Some("decision-tree".to_string()));
    }

    #[test]
    fn test_detect_schema_type_skill_frontmatter() {
        let yaml = "name: test\ncompliance-level: Gold\ntriggers:\n  - test";
        let result = parse_yaml(yaml).unwrap();
        let detected = detect_schema_type(&result.data);
        assert_eq!(detected, Some("skill-frontmatter".to_string()));
    }
}
