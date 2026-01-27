//! Code Generation Module (C8)
//!
//! Generates Rust validation rules, test scaffolds, and documentation from SMST.
//! Part of Tier 2 (Acceleration) in the Rust Migration Strategy.

use crate::modules::decision_engine::{DecisionNode, DecisionTree, Operator, Value};
use crate::modules::text_processor::SmstResult;
#[cfg(test)]
use crate::modules::text_processor::{SkillFrontmatter, SkillMachineSpec};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Generated validation rule from SMST INVARIANTS section
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationRule {
    /// Rule identifier (derived from skill name + index)
    pub id: String,
    /// Human-readable description of the rule
    pub description: String,
    /// Severity level: error, warning, info
    pub severity: String,
    /// The condition expression (extracted from INVARIANTS)
    pub condition: String,
    /// Error message when rule fails
    pub error_message: String,
}

/// Complete validation ruleset for a skill
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationRuleset {
    /// Skill name
    pub skill_name: String,
    /// Generated rules from INVARIANTS
    pub invariant_rules: Vec<ValidationRule>,
    /// Generated rules from `FAILURE_MODES`
    pub failure_mode_rules: Vec<ValidationRule>,
    /// Input validation rules
    pub input_rules: Vec<ValidationRule>,
    /// Output validation rules
    pub output_rules: Vec<ValidationRule>,
    /// Total rule count
    pub total_rules: usize,
}

/// Test case generated from SMST
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneratedTestCase {
    /// Test function name
    pub name: String,
    /// Test category: positive, negative, edge, stress, adversarial
    pub category: String,
    /// Test description
    pub description: String,
    /// Input values (as JSON-like representation)
    pub inputs: String,
    /// Expected output or behavior
    pub expected: String,
}

/// Test scaffold for a skill
#[derive(Debug, Serialize, Deserialize)]
pub struct TestScaffold {
    /// Skill name
    pub skill_name: String,
    /// Module path for tests
    pub module_path: String,
    /// Generated test cases
    pub test_cases: Vec<GeneratedTestCase>,
    /// Rust test module code (ready to use)
    pub rust_code: String,
}

/// Rust code stub generated from SMST
#[derive(Debug, Serialize, Deserialize)]
pub struct RustStub {
    /// Skill name
    pub skill_name: String,
    /// Module name (`snake_case`)
    pub module_name: String,
    /// Struct definitions
    pub structs: String,
    /// Function signatures
    pub functions: String,
    /// Complete Rust code
    pub full_code: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompilationTarget {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructSchema {
    pub struct_name: String,
    pub fields: Vec<FieldSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    pub field_name: String,
    pub field_type: String,
}

/// Generate validation rules from SMST
///
/// Parses `INVARIANTS` and `FAILURE_MODES` sections to create validation rules.
pub fn generate_validation_rules(smst: &SmstResult) -> ValidationRuleset {
    let skill_name = &smst.frontmatter.name;
    let mut invariant_rules = Vec::new();
    let mut failure_mode_rules = Vec::new();
    let mut input_rules = Vec::new();
    let mut output_rules = Vec::new();

    // Parse INVARIANTS section for validation rules
    if let Some(invariants) = &smst.spec.invariants {
        invariant_rules = parse_invariants_to_rules(skill_name, invariants);
    }

    // Parse FAILURE_MODES section for error handling rules
    if let Some(failure_modes) = &smst.spec.failure_modes {
        failure_mode_rules = parse_failure_modes_to_rules(skill_name, failure_modes);
    }

    // Parse INPUTS section for input validation
    if let Some(inputs) = &smst.spec.inputs {
        input_rules = parse_inputs_to_rules(skill_name, inputs);
    }

    // Parse OUTPUTS section for output validation
    if let Some(outputs) = &smst.spec.outputs {
        output_rules = parse_outputs_to_rules(skill_name, outputs);
    }

    let total_rules =
        invariant_rules.len() + failure_mode_rules.len() + input_rules.len() + output_rules.len();

    ValidationRuleset {
        skill_name: skill_name.clone(),
        invariant_rules,
        failure_mode_rules,
        input_rules,
        output_rules,
        total_rules,
    }
}

/// Parse INVARIANTS section text into validation rules
fn parse_invariants_to_rules(skill_name: &str, invariants: &str) -> Vec<ValidationRule> {
    let mut rules = Vec::new();

    let mut current_column_map: HashMap<String, usize> = HashMap::new();

    // Parse bullet points and numbered items
    for (idx, line) in invariants.lines().enumerate() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Header detection
        if line.starts_with('#') {
            continue;
        }

        // Table logic
        if line.starts_with('|') {
            if line.contains("---") {
                continue;
            }

            let parts: Vec<String> = line
                .trim_matches('|')
                .split('|')
                .map(|s| s.trim().to_lowercase())
                .collect();

            // Is this a header row?
            if parts
                .iter()
                .any(|p| p == "condition" || p == "type" || p == "invariant")
                || line.contains("---")
            {
                if !line.contains("---") {
                    current_column_map.clear();
                    for (i, p) in parts.iter().enumerate() {
                        current_column_map.insert(p.clone(), i);
                    }
                }
                continue;
            }

            // Extract based on column name if map exists
            let content = if !current_column_map.is_empty() {
                let actual_parts: Vec<&str> = line.trim_matches('|').split('|').collect();
                let col_idx = current_column_map
                    .get("condition")
                    .or_else(|| current_column_map.get("invariant"))
                    .or_else(|| current_column_map.get("type"))
                    .cloned()
                    .unwrap_or(0);

                actual_parts.get(col_idx).map(|s| s.trim()).unwrap_or("")
            } else {
                line.trim_matches('|')
                    .split('|')
                    .next()
                    .unwrap_or("")
                    .trim()
            };

            if content.is_empty()
                || content.to_lowercase() == "condition"
                || content.to_lowercase() == "invariant"
            {
                continue;
            }

            let (condition, error_message) = extract_invariant_pattern(content);
            rules.push(ValidationRule {
                id: format!("{}_inv_{}", to_snake_case(skill_name), idx),
                description: content.to_string(),
                severity: "error".to_string(),
                condition,
                error_message,
            });
            continue;
        }

        // Extract bullet points (- or * or numbered)
        let content = if line.starts_with('-') || line.starts_with('*') {
            line.trim_start_matches(['-', '*']).trim()
        } else if line
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            // Remove leading number and dot
            line.split_once('.')
                .map(|(_, rest)| rest.trim())
                .unwrap_or(line)
        } else {
            continue;
        };

        if content.is_empty() {
            continue;
        }

        // Extract key patterns from invariant text
        let (condition, error_message) = extract_invariant_pattern(content);

        rules.push(ValidationRule {
            id: format!("{}_inv_{}", to_snake_case(skill_name), idx),
            description: content.to_string(),
            severity: "error".to_string(),
            condition,
            error_message,
        });
    }

    rules
}

/// Extract condition and error message from invariant text
fn extract_invariant_pattern(text: &str) -> (String, String) {
    let text_lower = text.to_lowercase();

    // Pattern matching for common invariant expressions
    if text_lower.contains("must") {
        let condition = text.replace("must", "should").replace("Must", "Should");
        let error = format!("Invariant violation: {}", text);
        (condition, error)
    } else if text_lower.contains("always") {
        let condition = text.to_string();
        let error = format!("Always condition failed: {}", text);
        (condition, error)
    } else if text_lower.contains("never") {
        let condition = text
            .replace("never", "should not")
            .replace("Never", "Should not");
        let error = format!("Never condition violated: {}", text);
        (condition, error)
    } else if text_lower.contains("required") {
        let condition = text.to_string();
        let error = format!("Required condition not met: {}", text);
        (condition, error)
    } else {
        (text.to_string(), format!("Validation failed: {}", text))
    }
}

/// Parse `FAILURE_MODES` section into validation rules
fn parse_failure_modes_to_rules(skill_name: &str, failure_modes: &str) -> Vec<ValidationRule> {
    let mut rules = Vec::new();

    let mut current_column_map: HashMap<String, usize> = HashMap::new();

    for (idx, line) in failure_modes.lines().enumerate() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        if line.starts_with('#') {
            continue;
        }

        // Table logic
        if line.starts_with('|') {
            if line.contains("---") {
                continue;
            }

            let parts: Vec<String> = line
                .trim_matches('|')
                .split('|')
                .map(|s| s.trim().to_lowercase())
                .collect();

            // Is this a header row?
            if parts
                .iter()
                .any(|p| p == "trigger" || p == "response" || p == "mode")
                || line.contains("---")
            {
                if !line.contains("---") {
                    current_column_map.clear();
                    for (i, p) in parts.iter().enumerate() {
                        current_column_map.insert(p.clone(), i);
                    }
                }
                continue;
            }

            // Extract based on column name if map exists
            let content = if !current_column_map.is_empty() {
                let actual_parts: Vec<&str> = line.trim_matches('|').split('|').collect();
                let col_idx = current_column_map
                    .get("response")
                    .or_else(|| current_column_map.get("trigger"))
                    .or_else(|| current_column_map.get("mode"))
                    .cloned()
                    .unwrap_or(0);

                actual_parts.get(col_idx).map(|s| s.trim()).unwrap_or("")
            } else {
                line.trim_matches('|')
                    .split('|')
                    .last()
                    .unwrap_or("")
                    .trim()
            };

            if content.is_empty()
                || content.to_lowercase() == "response"
                || content.to_lowercase() == "trigger"
                || content.to_lowercase() == "mode"
            {
                continue;
            }

            let (severity, error_message) = parse_failure_mode_severity(content);
            rules.push(ValidationRule {
                id: format!("{}_fm_{}", to_snake_case(skill_name), idx),
                description: content.to_string(),
                severity,
                condition: format!("check_{}_fm_{}", to_snake_case(skill_name), idx),
                error_message,
            });
            continue;
        }

        // Look for error code patterns like FM-001, ERR_*, etc.
        let content = if line.starts_with('-') || line.starts_with('*') {
            line.trim_start_matches(['-', '*']).trim()
        } else if line.contains(':') || line.contains("FM-") || line.contains("ERR_") {
            line
        } else {
            continue;
        };

        if content.is_empty() {
            continue;
        }

        // Parse failure mode format: CODE: Description or just description
        let (severity, error_message) = parse_failure_mode_severity(content);

        rules.push(ValidationRule {
            id: format!("{}_fm_{}", to_snake_case(skill_name), idx),
            description: content.to_string(),
            severity,
            condition: format!("check_{}_fm_{}", to_snake_case(skill_name), idx),
            error_message,
        });
    }

    rules
}

/// Determine severity from failure mode text
fn parse_failure_mode_severity(text: &str) -> (String, String) {
    let text_lower = text.to_lowercase();

    let severity = if text_lower.contains("critical") || text_lower.contains("fatal") {
        "error"
    } else if text_lower.contains("warning") || text_lower.contains("recoverable") {
        "warning"
    } else if text_lower.contains("info") || text_lower.contains("note") {
        "info"
    } else {
        "error" // Default to error for safety
    };

    (severity.to_string(), text.to_string())
}

/// Parse INPUTS section into validation rules
fn parse_inputs_to_rules(skill_name: &str, inputs: &str) -> Vec<ValidationRule> {
    let mut rules = Vec::new();

    for (idx, line) in inputs.lines().enumerate() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Extract input definitions (usually bullet points)
        let content = if line.starts_with('-') || line.starts_with('*') {
            line.trim_start_matches(['-', '*']).trim()
        } else if line.starts_with('|') {
            // Table row - skip header separators
            if line.contains("---") {
                continue;
            }
            line.trim_matches('|').trim()
        } else {
            continue;
        };

        if content.is_empty() {
            continue;
        }

        // Look for type annotations like: name: String, path: Path
        let (param_name, param_type) = extract_param_info(content);

        if !param_name.is_empty() {
            rules.push(ValidationRule {
                id: format!("{}_input_{}", to_snake_case(skill_name), idx),
                description: format!("Validate input: {}", content),
                severity: "error".to_string(),
                condition: format!("validate_{}({})", param_name, param_type),
                error_message: format!("Invalid input {}: expected {}", param_name, param_type),
            });
        }
    }

    rules
}

/// Extract parameter name and type from input definition
fn extract_param_info(text: &str) -> (String, String) {
    // Pattern: `name` (type) or name: type or name (type: description)

    // Try backtick pattern first: `param_name`
    if text.contains('`')
        && let Some(start) = text.find('`')
        && let Some(end) = text[start + 1..].find('`')
    {
        let name = &text[start + 1..start + 1 + end];
        // Try to find type in parentheses
        let type_str = if let Some(paren_start) = text.find('(') {
            if let Some(paren_end) = text.find(')') {
                text[paren_start + 1..paren_end].to_string()
            } else {
                "any".to_string()
            }
        } else {
            "any".to_string()
        };
        return (name.to_string(), type_str);
    }

    // Try colon pattern: name: type
    if text.contains(':') {
        let parts: Vec<&str> = text.splitn(2, ':').collect();
        if parts.len() == 2 {
            let name = parts[0]
                .trim()
                .trim_matches(|c| c == '`' || c == '*' || c == '_');
            let type_part = parts[1].split_whitespace().next().unwrap_or("any");
            return (name.to_string(), type_part.to_string());
        }
    }

    ("".to_string(), "".to_string())
}

/// Parse OUTPUTS section into validation rules
fn parse_outputs_to_rules(skill_name: &str, outputs: &str) -> Vec<ValidationRule> {
    let mut rules = Vec::new();

    for (idx, line) in outputs.lines().enumerate() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let content = if line.starts_with('-') || line.starts_with('*') {
            line.trim_start_matches(['-', '*']).trim()
        } else {
            continue;
        };

        if content.is_empty() {
            continue;
        }

        let (output_name, output_type) = extract_param_info(content);

        if !output_name.is_empty() {
            rules.push(ValidationRule {
                id: format!("{}_output_{}", to_snake_case(skill_name), idx),
                description: format!("Validate output: {}", content),
                severity: "error".to_string(),
                condition: format!("validate_output_{}({})", output_name, output_type),
                error_message: format!("Invalid output {}: expected {}", output_name, output_type),
            });
        }
    }

    rules
}

/// Generate test scaffold from SMST
pub fn generate_test_scaffold(smst: &SmstResult) -> TestScaffold {
    let skill_name = &smst.frontmatter.name;
    let module_name = to_snake_case(skill_name);
    let mut test_cases = Vec::new();

    // Generate positive tests from INPUTS/OUTPUTS
    if let Some(inputs) = &smst.spec.inputs {
        test_cases.extend(generate_positive_tests(skill_name, inputs));
    }

    // Generate negative tests from FAILURE_MODES
    if let Some(failure_modes) = &smst.spec.failure_modes {
        test_cases.extend(generate_negative_tests(skill_name, failure_modes));
    }

    // Generate edge case tests from INVARIANTS
    if let Some(invariants) = &smst.spec.invariants {
        test_cases.extend(generate_edge_tests(skill_name, invariants));
    }

    // Generate Rust test code
    let rust_code = generate_rust_test_module(&module_name, &test_cases);

    TestScaffold {
        skill_name: skill_name.clone(),
        module_path: format!("tests::{}", module_name),
        test_cases,
        rust_code,
    }
}

/// Generate positive test cases from inputs
fn generate_positive_tests(skill_name: &str, inputs: &str) -> Vec<GeneratedTestCase> {
    let mut tests = Vec::new();
    let snake_name = to_snake_case(skill_name);

    // Generate a basic happy path test
    tests.push(GeneratedTestCase {
        name: format!("test_{}_happy_path", snake_name),
        category: "positive".to_string(),
        description: format!("Test {} with valid inputs", skill_name),
        inputs: extract_sample_inputs(inputs),
        expected: "Ok(...)".to_string(),
    });

    tests
}

/// Generate negative test cases from failure modes
fn generate_negative_tests(skill_name: &str, failure_modes: &str) -> Vec<GeneratedTestCase> {
    let mut tests = Vec::new();
    let snake_name = to_snake_case(skill_name);

    for (idx, line) in failure_modes.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let content = line.trim_start_matches(['-', '*']).trim();
        if content.is_empty() {
            continue;
        }

        tests.push(GeneratedTestCase {
            name: format!("test_{}_failure_{}", snake_name, idx),
            category: "negative".to_string(),
            description: format!("Test {} handles: {}", skill_name, truncate(content, 50)),
            inputs: "/* trigger failure condition */".to_string(),
            expected: format!("Err(...) // {}", truncate(content, 30)),
        });
    }

    tests
}

/// Generate edge case tests from invariants
fn generate_edge_tests(skill_name: &str, invariants: &str) -> Vec<GeneratedTestCase> {
    let mut tests = Vec::new();
    let snake_name = to_snake_case(skill_name);

    for (idx, line) in invariants.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let content = line.trim_start_matches(['-', '*']).trim();
        if content.is_empty() {
            continue;
        }

        tests.push(GeneratedTestCase {
            name: format!("test_{}_edge_{}", snake_name, idx),
            category: "edge".to_string(),
            description: format!("Edge case for invariant: {}", truncate(content, 50)),
            inputs: "/* boundary condition */".to_string(),
            expected: "/* invariant maintained */".to_string(),
        });
    }

    tests
}

/// Extract sample inputs from INPUTS section
fn extract_sample_inputs(inputs: &str) -> String {
    let mut params = Vec::new();

    for line in inputs.lines() {
        let line = line.trim();
        let (name, type_str) = extract_param_info(line);
        if !name.is_empty() {
            params.push(format!("{}: /* {} */", name, type_str));
        }
    }

    if params.is_empty() {
        "/* input */".to_string()
    } else {
        params.join(", ")
    }
}

/// Generate Rust test module code
fn generate_rust_test_module(module_name: &str, test_cases: &[GeneratedTestCase]) -> String {
    let mut code = String::new();

    code.push_str(&format!("//! Generated tests for {}\n", module_name));
    code.push_str("//! Auto-generated by rsk generate tests\n\n");
    code.push_str("#[cfg(test)]\n");
    code.push_str(&format!("mod {}_tests {{\n", module_name));
    code.push_str("    use super::*;\n\n");

    for test in test_cases {
        code.push_str(&format!("    /// {}\n", test.description));
        code.push_str(&format!("    /// Category: {}\n", test.category));
        code.push_str("    #[test]\n");
        code.push_str(&format!("    fn {}() {{\n", test.name));
        code.push_str(&format!("        // Inputs: {}\n", test.inputs));
        code.push_str(&format!("        // Expected: {}\n", test.expected));
        code.push_str("        todo!(\"Implement test\")\n");
        code.push_str("    }\n\n");
    }

    code.push_str("}\n");
    code
}

/// Generate Rust code stub from SMST
pub fn generate_rust_stub(smst: &SmstResult) -> RustStub {
    let skill_name = &smst.frontmatter.name;
    let module_name = to_snake_case(skill_name);

    let (structs, _) = generate_struct_definitions(smst);
    let functions = generate_function_signatures(smst);
    let full_code = generate_full_rust_module(smst);

    RustStub {
        skill_name: skill_name.clone(),
        module_name,
        structs,
        functions,
        full_code,
    }
}

/// Generate struct definitions from INPUTS/OUTPUTS/STATE
fn generate_struct_definitions_string(smst: &SmstResult) -> String {
    let mut code = String::new();
    let skill_name = &smst.frontmatter.name;
    let type_name = to_pascal_case(skill_name);

    // Input struct
    code.push_str(&format!("/// Input for {}\n", skill_name));
    code.push_str("#[derive(Debug, Clone, Default, Serialize, Deserialize)]\n");
    code.push_str(&format!("pub struct {}Input {{\n", type_name));
    if let Some(inputs) = &smst.spec.inputs {
        for line in inputs.lines() {
            let (name, type_str) = extract_param_info(line.trim());
            if !name.is_empty() {
                code.push_str(&format!(
                    "    pub {}: {},\n",
                    to_snake_case(&name),
                    rust_type_from_str(&type_str)
                ));
            }
        }
    }
    code.push_str("}\n\n");

    // Output struct
    code.push_str(&format!("/// Output for {}\n", skill_name));
    code.push_str("#[derive(Debug, Clone, Default, Serialize, Deserialize)]\n");
    code.push_str(&format!("pub struct {}Output {{\n", type_name));
    if let Some(outputs) = &smst.spec.outputs {
        for line in outputs.lines() {
            let (name, type_str) = extract_param_info(line.trim());
            if !name.is_empty() {
                code.push_str(&format!(
                    "    pub {}: {},\n",
                    to_snake_case(&name),
                    rust_type_from_str(&type_str)
                ));
            }
        }
    }
    code.push_str("}\n\n");

    // State struct (if present)
    if let Some(state) = &smst.spec.state
        && !state.trim().is_empty()
    {
        code.push_str(&format!("/// State for {}\n", skill_name));
        code.push_str("#[derive(Debug, Clone, Default, Serialize, Deserialize)]\n");
        code.push_str(&format!("pub struct {}State {{\n", type_name));
        for line in state.lines() {
            let (name, type_str) = extract_param_info(line.trim());
            if !name.is_empty() {
                code.push_str(&format!(
                    "    pub {}: {},\n",
                    to_snake_case(&name),
                    rust_type_from_str(&type_str)
                ));
            }
        }
        code.push_str("}\n\n");
    }

    code
}

/// Generate function signatures from PERFORMANCE section
fn generate_function_signatures(smst: &SmstResult) -> String {
    let mut code = String::new();
    let skill_name = &smst.frontmatter.name;
    let type_name = to_pascal_case(skill_name);
    let fn_name = to_snake_case(skill_name);

    code.push_str(&format!("/// Execute {}\n", skill_name));
    if let Some(desc) = &smst.frontmatter.description {
        code.push_str(&format!("///\n/// {}\n", desc));
    }
    code.push_str(&format!(
        "pub fn {}(input: {}Input) -> Result<{}Output, {}Error> {{\n",
        fn_name, type_name, type_name, type_name
    ));
    code.push_str("    todo!(\"Implement skill logic\")\n");
    code.push_str("}\n");

    code
}

/// Generate complete Rust module
fn generate_full_rust_module(smst: &SmstResult) -> String {
    let mut code = String::new();
    let skill_name = &smst.frontmatter.name;
    let type_name = to_pascal_case(skill_name);

    // Module header
    code.push_str(&format!(
        "//! {} - Auto-generated Rust module\n",
        skill_name
    ));
    if let Some(desc) = &smst.frontmatter.description {
        code.push_str(&format!("//!\n//! {}\n", desc));
    }
    code.push_str("//!\n//! Generated by: rsk generate stub\n\n");

    code.push_str("use serde::{Deserialize, Serialize};\n");
    code.push_str("use thiserror::Error;\n\n");

    // Error type
    code.push_str(&format!("/// Errors for {}\n", skill_name));
    code.push_str("#[derive(Debug, Error)]\n");
    code.push_str(&format!("pub enum {}Error {{\n", type_name));
    if let Some(failure_modes) = &smst.spec.failure_modes {
        for (idx, line) in failure_modes.lines().enumerate() {
            let line = line.trim().trim_start_matches(['-', '*']).trim();
            if !line.is_empty() && !line.starts_with('#') {
                code.push_str(&format!("    #[error(\"{}\")]\n", truncate(line, 60)));
                code.push_str(&format!("    Failure{},\n", idx));
            }
        }
    }
    code.push_str("    #[error(\"Unknown error: {0}\")]\n");
    code.push_str("    Unknown(String),\n");
    code.push_str("}\n\n");

    // Structs
    let (structs_code, _) = generate_struct_definitions(smst);
    code.push_str(&structs_code);

    // Functions
    code.push_str(&generate_function_signatures(smst));

    code
}

/// Generate a DecisionTree from SMST
///
/// This is a heuristic-based generator that attempts to convert
/// plain-text logic from the SMST into a deterministic decision tree.
pub fn generate_decision_tree(smst: &SmstResult) -> DecisionTree {
    let mut nodes = HashMap::new();
    let start_node = "start_node".to_string();

    // 1. Extract real input parameters to use as variables
    let mut input_vars = Vec::new();
    if let Some(inputs) = &smst.spec.inputs {
        for line in inputs.lines() {
            let (name, _) = extract_param_info(line.trim());
            if !name.is_empty() {
                input_vars.push(name);
            }
        }
    }

    let default_var = input_vars
        .first()
        .cloned()
        .unwrap_or_else(|| "input".to_string());

    // 2. Map invariants to deterministic condition nodes
    let rules = generate_validation_rules(smst);

    // Initial action node
    nodes.insert(
        start_node.clone(),
        DecisionNode::Action {
            action: "log".to_string(),
            target: None,
            value: Some(Value::String(format!(
                "Starting validation for {}",
                smst.frontmatter.name
            ))),
            next: Some("check_inv_0".to_string()),
        },
    );

    for (i, rule) in rules.invariant_rules.iter().enumerate() {
        let node_id = format!("check_inv_{}", i);
        let next_id = if i < rules.invariant_rules.len() - 1 {
            format!("check_inv_{}", i + 1)
        } else {
            "execute_core_logic".to_string()
        };

        let variable = input_vars
            .iter()
            .find(|v| rule.description.to_lowercase().contains(&v.to_lowercase()))
            .cloned()
            .unwrap_or_else(|| default_var.clone());

        nodes.insert(
            node_id,
            DecisionNode::Condition {
                variable,
                operator: Operator::IsNotNull,
                value: None,
                true_next: next_id,
                false_next: format!("fail_inv_{}", i),
            },
        );

        let mut error_obj = HashMap::new();
        error_obj.insert("status".to_string(), Value::String("error".to_string()));
        error_obj.insert(
            "message".to_string(),
            Value::String(rule.error_message.clone()),
        );

        nodes.insert(
            format!("fail_inv_{}", i),
            DecisionNode::Return {
                value: Value::Object(error_obj),
            },
        );
    }

    // 3. Main execution branch (LLM Fallback for complex logic)
    nodes.insert("execute_core_logic".to_string(), DecisionNode::LlmFallback {
        prompt: format!("Execute the core algorithm for {} based on provided inputs and SKILL.md specification.", smst.frontmatter.name),
        schema: None,
    });

    DecisionTree {
        start: start_node,
        nodes,
    }
}

/// Convert string to `snake_case`
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        if c == '-' {
            result.push('_');
        } else {
            result.push(c.to_lowercase().next().unwrap_or(c));
        }
    }
    result
}

/// Convert string to `PascalCase`
fn to_pascal_case(s: &str) -> String {
    s.split(['-', '_'])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect()
}

/// Map common type strings to Rust types
pub fn compile_rules(rules: &[ValidationRule], _target: CompilationTarget) -> String {
    let mut code = String::new();
    for rule in rules {
        code.push_str(&format!("    // {}\n", rule.description));
        code.push_str(&format!(
            "    if !({}) {{ return Err(SkillError::Validation(\"{}\".to_string())); }}\n",
            rule.condition, rule.error_message
        ));
    }
    code
}

pub fn compile_rules_with_schema(
    rules: &[ValidationRule],
    target: CompilationTarget,
    _schema: Option<&StructSchema>,
) -> String {
    // For now, schema-aware is same as basic, but can be improved
    compile_rules(rules, target)
}

pub fn generate_extensive_tests(_smst: &SmstResult) -> Vec<GeneratedTestCase> {
    // Placeholder for more complex test generation
    Vec::new()
}

pub fn generate_schema_aware_tests(
    _smst: &SmstResult,
    input: &StructSchema,
    _output: &StructSchema,
) -> Vec<GeneratedTestCase> {
    let mut tests = Vec::new();

    for field in &input.fields {
        tests.push(GeneratedTestCase {
            name: format!("test_boundary_{}", to_snake_case(&field.field_name)),
            category: "edge".to_string(),
            description: format!("Boundary test for field {}", field.field_name),
            inputs: format!(
                "{{ \"{}\": /* boundary value for {} */ }}",
                field.field_name, field.field_type
            ),
            expected: "Ok(_) or Err(ValidationError)".to_string(),
        });
    }

    tests
}

pub fn generate_test_module_code_schema_aware(name: &str, tests: &[GeneratedTestCase]) -> String {
    generate_rust_test_module(&to_snake_case(name), tests)
}

#[deprecated]
pub fn generate_test_module_code(name: &str, tests: &[GeneratedTestCase]) -> String {
    generate_rust_test_module(&to_snake_case(name), tests)
}

pub fn generate_attestation_code(intent: &crate::modules::intent::StructuredIntent) -> String {
    let mut code = String::new();
    code.push_str("#[test]\n");
    code.push_str("fn test_intent_attestation() {\n");
    code.push_str("    // PROOF: Verify implementation pattern matches claimed intent\n");
    code.push_str(&format!(
        "    let claimed_pattern = \"{:?}\";\n",
        intent.pattern
    ));
    code.push_str("    let actual_pattern = env!(\"SKILL_PATTERN\");\n");
    code.push_str("    assert_eq!(actual_pattern, claimed_pattern, \"Intent Breach: implementation pattern does not match claim\");\n");
    code.push_str("\n");
    code.push_str("    // PROOF: Verify required kernel modules are linked\n");
    for module in &intent.rsk_modules {
        code.push_str(&format!("    assert!(cfg!(feature = \"{}\"), \"Capability Gap: required module '{}' not available\");\n", module, module));
    }
    code.push_str("}\n");
    code
}

pub fn generate_struct_definitions(smst: &SmstResult) -> (String, Vec<StructSchema>) {
    let code = generate_struct_definitions_string(smst);
    let mut schemas = Vec::new();
    let skill_name = &smst.frontmatter.name;
    let type_name = to_pascal_case(skill_name);

    // Input schema
    let mut input_fields = Vec::new();
    if let Some(inputs) = &smst.spec.inputs {
        for line in inputs.lines() {
            let (name, type_str) = extract_param_info(line.trim());
            if !name.is_empty() {
                input_fields.push(FieldSchema {
                    field_name: to_snake_case(&name),
                    field_type: rust_type_from_str(&type_str).to_string(),
                });
            }
        }
    }
    schemas.push(StructSchema {
        struct_name: format!("{}Input", type_name),
        fields: input_fields,
    });

    // Output schema
    let mut output_fields = Vec::new();
    if let Some(outputs) = &smst.spec.outputs {
        for line in outputs.lines() {
            let (name, type_str) = extract_param_info(line.trim());
            if !name.is_empty() {
                output_fields.push(FieldSchema {
                    field_name: to_snake_case(&name),
                    field_type: rust_type_from_str(&type_str).to_string(),
                });
            }
        }
    }
    schemas.push(StructSchema {
        struct_name: format!("{}Output", type_name),
        fields: output_fields,
    });

    // State schema
    if let Some(state) = &smst.spec.state
        && !state.trim().is_empty()
    {
        let mut state_fields = Vec::new();
        for line in state.lines() {
            let (name, type_str) = extract_param_info(line.trim());
            if !name.is_empty() {
                state_fields.push(FieldSchema {
                    field_name: to_snake_case(&name),
                    field_type: rust_type_from_str(&type_str).to_string(),
                });
            }
        }
        schemas.push(StructSchema {
            struct_name: format!("{}State", type_name),
            fields: state_fields,
        });
    }

    (code, schemas)
}

fn rust_type_from_str(type_str: &str) -> &'static str {
    let lower = type_str.to_lowercase();
    match lower.as_str() {
        "string" | "str" | "text" => "String",
        "int" | "integer" | "i32" => "i32",
        "i64" | "long" => "i64",
        "float" | "f32" => "f32",
        "f64" | "double" | "number" => "f64",
        "bool" | "boolean" => "bool",
        "path" | "filepath" => "std::path::PathBuf",
        "json" | "object" => "serde_json::Value",
        "array" | "list" | "vec" => "Vec<serde_json::Value>",
        "optional" | "option" => "Option<String>",
        _ => "String", // Default to String
    }
}

/// Truncate string to max length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_smst;

    const SAMPLE_SKILL: &str = r#"---
name: test-skill
description: A test skill for code generation
version: 1.0.0
compliance-level: diamond
categories:
  - testing
  - validation
---

# test-skill

Test skill for validating code generation.

## Machine Specification

### 1. INPUTS

- `path` (String): Path to input file
- `options` (Object): Configuration options
- `threshold` (i32): Score threshold

### 2. OUTPUTS

- `result` (String): Processing result
- `score` (f64): Calculated score
- `passed` (bool): Whether validation passed

### 3. STATE

- `cache` (Object): Internal cache
- `counter` (i64): Execution counter

### 4. OPERATOR MODE

| Mode | Behavior |
|------|----------|
| validate | Run validation only |
| execute | Full execution |

### 5. PERFORMANCE

- Latency: <50ms p95
- Delegated to: rsk kernel

### 6. INVARIANTS

- Score must be between 0 and 100
- Path must exist and be readable
- Threshold must be positive
- Cache must be cleared on error

### 7. FAILURE MODES

- FM-001: File not found (critical)
- FM-002: Invalid JSON format (recoverable)
- FM-003: Threshold exceeded (warning)

### 8. TELEMETRY

- execution_time_ms
- result_score
- failure_count
"#;

    #[test]
    fn test_generate_validation_rules() {
        let smst = extract_smst(SAMPLE_SKILL);
        let rules = generate_validation_rules(&smst);

        assert_eq!(rules.skill_name, "test-skill");
        assert!(
            !rules.invariant_rules.is_empty(),
            "Should have invariant rules"
        );
        assert!(
            !rules.failure_mode_rules.is_empty(),
            "Should have failure mode rules"
        );
        assert!(!rules.input_rules.is_empty(), "Should have input rules");
        assert!(rules.total_rules > 0, "Should have total rules > 0");
    }

    #[test]
    fn test_generate_test_scaffold() {
        let smst = extract_smst(SAMPLE_SKILL);
        let scaffold = generate_test_scaffold(&smst);

        assert_eq!(scaffold.skill_name, "test-skill");
        assert!(!scaffold.test_cases.is_empty(), "Should have test cases");
        assert!(
            scaffold.rust_code.contains("#[test]"),
            "Should have #[test] attribute"
        );
        assert!(
            scaffold.rust_code.contains("test_test_skill"),
            "Should have skill-named tests"
        );
    }

    #[test]
    fn test_generate_rust_stub() {
        let smst = extract_smst(SAMPLE_SKILL);
        let stub = generate_rust_stub(&smst);

        assert_eq!(stub.skill_name, "test-skill");
        assert_eq!(stub.module_name, "test_skill");
        assert!(
            stub.structs.contains("TestSkillInput"),
            "Should have input struct"
        );
        assert!(
            stub.structs.contains("TestSkillOutput"),
            "Should have output struct"
        );
        assert!(
            stub.functions.contains("pub fn test_skill"),
            "Should have main function"
        );
        assert!(
            stub.full_code.contains("TestSkillError"),
            "Should have error type"
        );
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("test-skill"), "test_skill");
        assert_eq!(to_snake_case("TestSkill"), "test_skill");
        assert_eq!(to_snake_case("my-cool-skill"), "my_cool_skill");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("test-skill"), "TestSkill");
        assert_eq!(to_pascal_case("my_cool_skill"), "MyCoolSkill");
    }

    #[test]
    fn test_rust_type_mapping() {
        assert_eq!(rust_type_from_str("String"), "String");
        assert_eq!(rust_type_from_str("i32"), "i32");
        assert_eq!(rust_type_from_str("boolean"), "bool");
        assert_eq!(rust_type_from_str("Path"), "std::path::PathBuf");
    }

    #[test]
    fn test_invariant_rules_parsing() {
        let smst = extract_smst(SAMPLE_SKILL);
        let rules = generate_validation_rules(&smst);

        // Should parse "must be" patterns
        let must_rule = rules
            .invariant_rules
            .iter()
            .find(|r| r.description.contains("must"));
        assert!(must_rule.is_some(), "Should find 'must' rule");
    }

    #[test]
    fn test_failure_mode_severity() {
        let (severity, _) = parse_failure_mode_severity("FM-001: File not found (critical)");
        assert_eq!(severity, "error");

        let (severity, _) = parse_failure_mode_severity("FM-003: Threshold exceeded (warning)");
        assert_eq!(severity, "warning");
    }

    #[test]
    fn test_extract_param_info() {
        let (name, type_str) = extract_param_info("`path` (String): Path to input file");
        assert_eq!(name, "path");
        assert_eq!(type_str, "String");

        let (name, type_str) = extract_param_info("score: f64 - the calculated score");
        assert_eq!(name, "score");
        assert_eq!(type_str, "f64");
    }

    #[test]
    fn test_empty_smst() {
        let empty_smst = SmstResult {
            frontmatter: SkillFrontmatter {
                name: "empty".to_string(),
                ..Default::default()
            },
            spec: SkillMachineSpec::default(),
            score: crate::SmstScore {
                total_score: 0.0,
                sections_present: 0,
                sections_required: 8,
                has_frontmatter: true,
                has_machine_spec: false,
                compliance_level: "bronze".to_string(),
                missing_sections: vec![],
            },
            is_diamond_compliant: false,
        };

        let rules = generate_validation_rules(&empty_smst);
        assert_eq!(rules.total_rules, 0);

        let scaffold = generate_test_scaffold(&empty_smst);
        assert!(scaffold.test_cases.is_empty());
    }
}
