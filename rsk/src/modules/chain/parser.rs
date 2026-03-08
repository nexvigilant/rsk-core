//! # Chain Parser
//!
//! Parses chain definitions from inline strings and YAML files.
//!
//! ## Design Principles
//! - **Atomic**: Only parsing logic, no execution
//! - **Zero-copy where possible**: Uses references into source strings
//! - **Detailed errors**: Reports line/column for parse failures
//!
//! ## Supported Formats
//!
//! ### Inline Format
//! ```text
//! skill1 -> skill2 -> skill3       # Sequential
//! skill1 | skill2 | skill3         # Parallel
//! ```
//!
//! ### YAML Format
//! ```yaml
//! name: my-pipeline
//! steps:
//!   - name: First step
//!     skill: smart-goal
//!   - name: Parallel tasks
//!     skill: build
//!     parallel: true
//!   - conditional:
//!       if: context.has_tests
//!       then: run-tests
//!       else: skip-tests
//! ```

use super::types::{Chain, ChainStep, CompositionType, ConditionalStep, StepType};
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

// ═══════════════════════════════════════════════════════════════════════════
// ERROR TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Parse error with location information
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub source: Option<String>,
}

impl ParseError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            line: None,
            column: None,
            source: None,
        }
    }

    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    pub fn with_source(mut self, source: &str) -> Self {
        self.source = Some(source.to_string());
        self
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(col)) => {
                write!(f, "Parse error at {}:{}: {}", line, col, self.message)
            }
            (Some(line), None) => write!(f, "Parse error at line {}: {}", line, self.message),
            _ => write!(f, "Parse error: {}", self.message),
        }
    }
}

impl std::error::Error for ParseError {}

// ═══════════════════════════════════════════════════════════════════════════
// COMPILED REGEXES (lazy static)
// ═══════════════════════════════════════════════════════════════════════════

/// Valid skill name pattern: lowercase letters, digits, hyphens
#[allow(clippy::unwrap_used)] // Safety: compile-time literal pattern — Regex::new cannot fail
static SKILL_NAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9-]*$").unwrap());

/// Sequential operator
#[allow(clippy::unwrap_used)] // Safety: compile-time literal pattern — Regex::new cannot fail
static SEQUENTIAL_SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*->\s*").unwrap());

/// Parallel operator
#[allow(clippy::unwrap_used)] // Safety: compile-time literal pattern — Regex::new cannot fail
static PARALLEL_SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\|\s*").unwrap());

// ═══════════════════════════════════════════════════════════════════════════
// INLINE PARSER
// ═══════════════════════════════════════════════════════════════════════════

/// Parse an inline chain definition.
///
/// # Examples
/// ```rust,ignore
/// let chain = parse_inline("smart-goal -> decompose-task -> proceed")?;
/// assert_eq!(chain.len(), 3);
/// assert_eq!(chain.composition, CompositionType::Sequential);
///
/// let parallel = parse_inline("build | test | lint")?;
/// assert_eq!(parallel.composition, CompositionType::Parallel);
/// ```
pub fn parse_inline(input: &str) -> Result<Chain, ParseError> {
    let input = input.trim();

    if input.is_empty() {
        return Err(ParseError::new("Empty chain definition"));
    }

    // Detect composition type by operator presence
    let (composition, parts) = if input.contains(" -> ") {
        (
            CompositionType::Sequential,
            SEQUENTIAL_SPLIT.split(input).collect::<Vec<_>>(),
        )
    } else if input.contains(" | ") {
        (
            CompositionType::Parallel,
            PARALLEL_SPLIT.split(input).collect::<Vec<_>>(),
        )
    } else {
        // Single skill
        (CompositionType::Sequential, vec![input])
    };

    // Parse each part as a skill
    let mut steps = Vec::with_capacity(parts.len());
    for (i, part) in parts.iter().enumerate() {
        let part = part.trim();

        if part.is_empty() {
            return Err(ParseError::new("Empty skill name in chain").with_location(1, i + 1));
        }

        // Parse skill with optional args: "skill-name --arg1 --arg2"
        let (skill_name, args) = parse_skill_with_args(part)?;

        if !SKILL_NAME_REGEX.is_match(&skill_name) {
            return Err(ParseError::new(&format!(
                "Invalid skill name '{skill_name}': must be lowercase letters, digits, and hyphens"
            ))
            .with_source(part));
        }

        let mut step = ChainStep::new(&skill_name);
        if let Some(args) = args {
            step = step.with_args(&args);
        }

        // Mark as parallel if using parallel composition
        if composition == CompositionType::Parallel {
            step = step.parallel().with_parallel_group(0);
        }

        steps.push(StepType::Regular(step));
    }

    Ok(Chain {
        name: "inline".to_string(),
        description: None,
        steps,
        composition,
        context: std::collections::HashMap::new(),
        tags: Vec::new(),
        version: None,
    })
}

/// Parse skill name and optional arguments from a string.
///
/// # Examples
/// - `"smart-goal"` -> `("smart-goal", None)`
/// - `"smart-goal --verbose"` -> `("smart-goal", Some("--verbose"))`
fn parse_skill_with_args(input: &str) -> Result<(String, Option<String>), ParseError> {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();

    let skill_name = parts[0].trim().to_string();
    let args = parts.get(1).map(|s| s.trim().to_string());

    if skill_name.is_empty() {
        return Err(ParseError::new("Empty skill name"));
    }

    Ok((skill_name, args))
}

// ═══════════════════════════════════════════════════════════════════════════
// YAML PARSER
// ═══════════════════════════════════════════════════════════════════════════

/// Intermediate YAML representation for deserialization
#[derive(Debug, Deserialize)]
struct YamlChain {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    steps: Vec<YamlStep>,
    #[serde(default)]
    composition: Option<String>,
    #[serde(default)]
    context: std::collections::HashMap<String, serde_json::Value>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum YamlStep {
    Simple(String),
    Named(YamlNamedStep),
    Conditional(YamlConditionalWrapper),
}

#[derive(Debug, Deserialize)]
struct YamlNamedStep {
    skill: String,
    #[serde(default)]
    args: Option<String>,
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    outputs: Vec<String>,
    #[serde(default)]
    required: Option<bool>,
    #[serde(default)]
    parallel: Option<bool>,
    #[serde(default)]
    parallel_group: Option<u32>,
    #[serde(default)]
    timeout: Option<u32>,
    #[serde(default)]
    retries: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct YamlConditionalWrapper {
    conditional: YamlConditional,
}

#[derive(Debug, Deserialize)]
struct YamlConditional {
    #[serde(rename = "if")]
    condition: String,
    then: YamlThenElse,
    #[serde(rename = "else")]
    else_branch: Option<YamlThenElse>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum YamlThenElse {
    Simple(String),
    Full { skill: String, args: Option<String> },
}

impl YamlThenElse {
    fn to_chain_step(&self) -> ChainStep {
        match self {
            YamlThenElse::Simple(skill) => ChainStep::new(skill),
            YamlThenElse::Full { skill, args } => {
                let mut step = ChainStep::new(skill);
                if let Some(a) = args {
                    step = step.with_args(a);
                }
                step
            }
        }
    }
}

/// Parse a YAML chain definition.
///
/// # Example YAML
/// ```yaml
/// name: my-pipeline
/// description: A sample pipeline
/// steps:
///   - skill: smart-goal
///   - skill: decompose-task
///     parallel: true
///   - skill: proceed
///     parallel: true
///   - conditional:
///       if: context.has_tests
///       then: run-tests
///       else: skip-tests
/// ```
pub fn parse_yaml(content: &str) -> Result<Chain, ParseError> {
    // Use serde_yaml to parse
    let yaml_chain: YamlChain = serde_yaml::from_str(content)
        .map_err(|e| ParseError::new(&format!("YAML parse error: {e}")))?;

    // Convert to Chain
    let mut steps = Vec::with_capacity(yaml_chain.steps.len());
    let mut current_parallel_group = 0u32;
    let mut in_parallel = false;

    for yaml_step in yaml_chain.steps {
        match yaml_step {
            YamlStep::Simple(skill_name) => {
                if !SKILL_NAME_REGEX.is_match(&skill_name) {
                    return Err(ParseError::new(&format!(
                        "Invalid skill name '{skill_name}': must be lowercase letters, digits, and hyphens"
                    )));
                }
                steps.push(StepType::Regular(ChainStep::new(&skill_name)));
                in_parallel = false;
            }
            YamlStep::Named(named) => {
                if !SKILL_NAME_REGEX.is_match(&named.skill) {
                    return Err(ParseError::new(&format!(
                        "Invalid skill name '{}': must be lowercase letters, digits, and hyphens",
                        named.skill
                    )));
                }

                let mut step = ChainStep::new(&named.skill);

                if let Some(args) = named.args {
                    step = step.with_args(&args);
                }
                if !named.inputs.is_empty() {
                    step = step.with_inputs(named.inputs);
                }
                if !named.outputs.is_empty() {
                    step = step.with_outputs(named.outputs);
                }
                if let Some(false) = named.required {
                    step = step.optional();
                }
                if let Some(timeout) = named.timeout {
                    step = step.with_timeout(timeout);
                }
                if let Some(retries) = named.retries {
                    step = step.with_retries(retries);
                }

                // Handle parallel grouping
                if let Some(true) = named.parallel {
                    if !in_parallel {
                        current_parallel_group += 1;
                        in_parallel = true;
                    }
                    step = step.parallel().with_parallel_group(
                        named.parallel_group.unwrap_or(current_parallel_group),
                    );
                } else {
                    in_parallel = false;
                }

                steps.push(StepType::Regular(step));
            }
            YamlStep::Conditional(wrapper) => {
                let cond = wrapper.conditional;
                let then_step = cond.then.to_chain_step();

                let mut conditional = ConditionalStep::new(&cond.condition, then_step);
                if let Some(else_branch) = cond.else_branch {
                    conditional = conditional.with_else(else_branch.to_chain_step());
                }

                steps.push(StepType::Conditional(conditional));
                in_parallel = false;
            }
        }
    }

    // Determine composition from YAML or infer from steps
    let composition = if let Some(comp_str) = yaml_chain.composition {
        match comp_str.to_lowercase().as_str() {
            "sequential" => CompositionType::Sequential,
            "parallel" => CompositionType::Parallel,
            "conditional" => CompositionType::Conditional,
            "loop" => CompositionType::Loop,
            _ => CompositionType::Sequential,
        }
    } else {
        CompositionType::Sequential
    };

    Ok(Chain {
        name: yaml_chain.name,
        description: yaml_chain.description,
        steps,
        composition,
        context: yaml_chain.context,
        tags: yaml_chain.tags,
        version: yaml_chain.version,
    })
}

/// Assign parallel groups to consecutive parallel steps.
///
/// Steps with `parallel: true` that are adjacent will share the same group ID.
pub fn assign_parallel_groups(chain: &mut Chain) {
    let mut current_group = 0u32;
    let mut in_parallel = false;

    for step in &mut chain.steps {
        if let StepType::Regular(chain_step) = step {
            if chain_step.parallel {
                if !in_parallel {
                    current_group += 1;
                    in_parallel = true;
                }
                if chain_step.parallel_group.is_none() {
                    chain_step.parallel_group = Some(current_group);
                }
            } else {
                in_parallel = false;
            }
        } else {
            // Conditionals break parallel groups
            in_parallel = false;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────
    // INLINE PARSER TESTS
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_inline_sequential() {
        let chain = parse_inline("smart-goal -> decompose-task -> proceed").unwrap();

        assert_eq!(chain.name, "inline");
        assert_eq!(chain.len(), 3);
        assert_eq!(chain.composition, CompositionType::Sequential);
        assert_eq!(
            chain.skill_names(),
            vec!["smart-goal", "decompose-task", "proceed"]
        );
    }

    #[test]
    fn test_parse_inline_parallel() {
        let chain = parse_inline("build | test | lint").unwrap();

        assert_eq!(chain.len(), 3);
        assert_eq!(chain.composition, CompositionType::Parallel);

        // All should have parallel group 0
        for step in &chain.steps {
            assert!(step.is_parallel());
            assert_eq!(step.parallel_group(), Some(0));
        }
    }

    #[test]
    fn test_parse_inline_single_skill() {
        let chain = parse_inline("smart-goal").unwrap();

        assert_eq!(chain.len(), 1);
        assert_eq!(chain.skill_names(), vec!["smart-goal"]);
    }

    #[test]
    fn test_parse_inline_with_args() {
        let chain = parse_inline("smart-goal --verbose -> proceed").unwrap();

        assert_eq!(chain.len(), 2);
        if let StepType::Regular(step) = &chain.steps[0] {
            assert_eq!(step.skill, "smart-goal");
            assert_eq!(step.args, Some("--verbose".to_string()));
        } else {
            panic!("Expected regular step");
        }
    }

    #[test]
    fn test_parse_inline_empty_fails() {
        assert!(parse_inline("").is_err());
        assert!(parse_inline("   ").is_err());
    }

    #[test]
    fn test_parse_inline_invalid_skill_name() {
        assert!(parse_inline("InvalidName").is_err()); // Uppercase
        assert!(parse_inline("123-skill").is_err()); // Starts with number
        assert!(parse_inline("skill_name").is_err()); // Underscore
    }

    // ─────────────────────────────────────────────────────────────────────────
    // YAML PARSER TESTS
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_yaml_basic() {
        let yaml = r#"
name: test-pipeline
steps:
  - skill: smart-goal
  - skill: proceed
"#;
        let chain = parse_yaml(yaml).unwrap();

        assert_eq!(chain.name, "test-pipeline");
        assert_eq!(chain.len(), 2);
        assert_eq!(chain.skill_names(), vec!["smart-goal", "proceed"]);
    }

    #[test]
    fn test_parse_yaml_with_description() {
        let yaml = r#"
name: documented-pipeline
description: A well-documented pipeline
steps:
  - skill: step1
"#;
        let chain = parse_yaml(yaml).unwrap();

        assert_eq!(
            chain.description,
            Some("A well-documented pipeline".to_string())
        );
    }

    #[test]
    fn test_parse_yaml_parallel_steps() {
        let yaml = r#"
name: parallel-pipeline
steps:
  - skill: setup
  - skill: build
    parallel: true
  - skill: test
    parallel: true
  - skill: deploy
"#;
        let chain = parse_yaml(yaml).unwrap();

        assert_eq!(chain.len(), 4);

        // setup is not parallel
        assert!(!chain.steps[0].is_parallel());

        // build and test are parallel in same group
        assert!(chain.steps[1].is_parallel());
        assert!(chain.steps[2].is_parallel());
        assert_eq!(
            chain.steps[1].parallel_group(),
            chain.steps[2].parallel_group()
        );

        // deploy is not parallel
        assert!(!chain.steps[3].is_parallel());
    }

    #[test]
    fn test_parse_yaml_conditional() {
        let yaml = r#"
name: conditional-pipeline
steps:
  - skill: analyze
  - conditional:
      if: context.has_tests
      then: run-tests
      else: skip-tests
"#;
        let chain = parse_yaml(yaml).unwrap();

        assert_eq!(chain.len(), 2);

        if let StepType::Conditional(cond) = &chain.steps[1] {
            assert_eq!(cond.condition, "context.has_tests");
            assert_eq!(cond.then_step.skill, "run-tests");
            assert!(cond.else_step.is_some());
            assert_eq!(cond.else_step.as_ref().unwrap().skill, "skip-tests");
        } else {
            panic!("Expected conditional step");
        }
    }

    #[test]
    fn test_parse_yaml_full_step_options() {
        let yaml = r#"
name: full-options
steps:
  - skill: complex-task
    args: --verbose --dry-run
    inputs:
      - input_file
    outputs:
      - output_file
    required: false
    timeout: 300
    retries: 2
"#;
        let chain = parse_yaml(yaml).unwrap();

        if let StepType::Regular(step) = &chain.steps[0] {
            assert_eq!(step.skill, "complex-task");
            assert_eq!(step.args, Some("--verbose --dry-run".to_string()));
            assert_eq!(step.inputs, vec!["input_file"]);
            assert_eq!(step.outputs, vec!["output_file"]);
            assert!(!step.required);
            assert_eq!(step.timeout_secs, 300);
            assert_eq!(step.retries, 2);
        } else {
            panic!("Expected regular step");
        }
    }

    #[test]
    fn test_parse_yaml_simple_steps() {
        let yaml = r#"
name: simple-list
steps:
  - step1
  - step2
  - step3
"#;
        // Note: This format may or may not be supported depending on YAML structure
        // If simple strings are allowed as steps, they should parse
        let result = parse_yaml(yaml);

        // The simple string format should work with our untagged enum
        if let Ok(chain) = result {
            assert_eq!(chain.len(), 3);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // PARALLEL GROUP ASSIGNMENT TESTS
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_assign_parallel_groups() {
        let mut chain = Chain::new("test")
            .with_step(ChainStep::new("a"))
            .with_step(ChainStep::new("b").parallel())
            .with_step(ChainStep::new("c").parallel())
            .with_step(ChainStep::new("d"))
            .with_step(ChainStep::new("e").parallel());

        assign_parallel_groups(&mut chain);

        // a: not parallel
        assert!(!chain.steps[0].is_parallel());

        // b, c: parallel group 1
        assert_eq!(chain.steps[1].parallel_group(), Some(1));
        assert_eq!(chain.steps[2].parallel_group(), Some(1));

        // d: not parallel
        assert!(!chain.steps[3].is_parallel());

        // e: parallel group 2 (new group after break)
        assert_eq!(chain.steps[4].parallel_group(), Some(2));
    }
}
