//! # Chain Validator
//!
//! Validates chain definitions for correctness and executability.
//!
//! ## Design Principles
//! - **Atomic**: Only validation logic, no execution
//! - **Comprehensive**: Checks structure, dependencies, cycles, resources
//! - **Actionable**: Error messages explain how to fix issues
//!
//! ## Validation Checks
//! 1. **Structure**: Chain has name and at least one step
//! 2. **Skill names**: Valid format (lowercase, hyphens)
//! 3. **Dependencies**: Input/output contracts are satisfiable
//! 4. **Cycles**: No circular dependencies between steps
//! 5. **Parallelism**: Parallel steps don't conflict on resources

use super::types::{Chain, ChainStep, StepType};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

// ═══════════════════════════════════════════════════════════════════════════
// VALIDATION RESULT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Severity level for validation issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Severity {
    /// Informational, no action required
    Info,
    /// Warning, execution may succeed but not recommended
    Warning,
    /// Error, chain cannot execute
    Error,
}

/// A single validation issue
#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub step_index: Option<usize>,
    pub skill_name: Option<String>,
}

impl ValidationIssue {
    fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            step_index: None,
            skill_name: None,
        }
    }

    fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message: message.into(),
            step_index: None,
            skill_name: None,
        }
    }

    fn info(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            code,
            message: message.into(),
            step_index: None,
            skill_name: None,
        }
    }

    fn at_step(mut self, index: usize, skill: &str) -> Self {
        self.step_index = Some(index);
        self.skill_name = Some(skill.to_string());
        self
    }
}

/// Complete validation result
#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

impl ValidationResult {
    fn new() -> Self {
        Self {
            valid: true,
            issues: Vec::new(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
        }
    }

    fn add(&mut self, issue: ValidationIssue) {
        match issue.severity {
            Severity::Error => {
                self.valid = false;
                self.error_count += 1;
            }
            Severity::Warning => self.warning_count += 1,
            Severity::Info => self.info_count += 1,
        }
        self.issues.push(issue);
    }

    /// Get all errors
    pub fn errors(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect()
    }

    /// Get all warnings
    pub fn warnings(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .collect()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// VALIDATION FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Validate a chain definition comprehensively.
///
/// Runs all validation checks and returns a detailed result.
pub fn validate_chain(chain: &Chain) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Run all validators
    validate_structure(chain, &mut result);
    validate_skill_names(chain, &mut result);
    validate_dependencies(chain, &mut result);
    validate_parallel_resources(chain, &mut result);
    validate_conditionals(chain, &mut result);

    result
}

/// Quick check if a chain is valid without detailed issues.
pub fn is_valid(chain: &Chain) -> bool {
    validate_chain(chain).valid
}

// ═══════════════════════════════════════════════════════════════════════════
// INDIVIDUAL VALIDATORS
// ═══════════════════════════════════════════════════════════════════════════

/// Validate basic structure requirements.
fn validate_structure(chain: &Chain, result: &mut ValidationResult) {
    // Chain must have a name
    if chain.name.is_empty() {
        result.add(ValidationIssue::error(
            "EMPTY_NAME",
            "Chain must have a non-empty name",
        ));
    }

    // Chain must have at least one step
    if chain.steps.is_empty() {
        result.add(ValidationIssue::error(
            "NO_STEPS",
            "Chain must have at least one step",
        ));
    }

    // Reasonable step count
    if chain.steps.len() > 100 {
        result.add(ValidationIssue::warning(
            "MANY_STEPS",
            format!(
                "Chain has {} steps; consider breaking into sub-chains",
                chain.steps.len()
            ),
        ));
    }
}

/// Validate skill name format.
fn validate_skill_names(chain: &Chain, result: &mut ValidationResult) {
    let name_regex = regex::Regex::new(r"^[a-z][a-z0-9-]*$").unwrap();

    for (i, step) in chain.steps.iter().enumerate() {
        let skill_name = step.skill_name();

        if skill_name.is_empty() {
            result.add(
                ValidationIssue::error("EMPTY_SKILL", "Skill name cannot be empty")
                    .at_step(i, skill_name),
            );
            continue;
        }

        if !name_regex.is_match(skill_name) {
            result.add(ValidationIssue::error(
                "INVALID_SKILL_NAME",
                format!(
                    "Skill name '{}' is invalid: must be lowercase letters, digits, and hyphens, starting with a letter",
                    skill_name
                ),
            ).at_step(i, skill_name));
        }

        // Check for reserved names
        if is_reserved_name(skill_name) {
            result.add(
                ValidationIssue::warning(
                    "RESERVED_NAME",
                    format!(
                        "Skill name '{}' is reserved and may conflict with built-in functionality",
                        skill_name
                    ),
                )
                .at_step(i, skill_name),
            );
        }
    }
}

/// Check if a name is reserved (built-in or system).
fn is_reserved_name(name: &str) -> bool {
    const RESERVED: &[&str] = &[
        "help", "version", "debug", "config", "init", "test", "build", "run", "exec", "eval",
        "true", "false", "null",
    ];
    RESERVED.contains(&name)
}

/// Validate input/output dependencies between steps.
fn validate_dependencies(chain: &Chain, result: &mut ValidationResult) {
    let mut available_outputs: HashSet<String> = HashSet::new();

    for (i, step) in chain.steps.iter().enumerate() {
        if let StepType::Regular(chain_step) = step {
            // Check if all inputs are available
            for input in &chain_step.inputs {
                if !available_outputs.contains(input) {
                    result.add(ValidationIssue::error(
                        "MISSING_INPUT",
                        format!(
                            "Step '{}' requires input '{}' which is not produced by any previous step",
                            chain_step.skill, input
                        ),
                    ).at_step(i, &chain_step.skill));
                }
            }

            // Add outputs to available set
            for output in &chain_step.outputs {
                if !available_outputs.insert(output.clone()) {
                    result.add(ValidationIssue::warning(
                        "DUPLICATE_OUTPUT",
                        format!(
                            "Output '{}' is produced by multiple steps; later value will overwrite",
                            output
                        ),
                    ).at_step(i, &chain_step.skill));
                }
            }
        }
    }
}

/// Validate that parallel steps don't conflict on resources.
fn validate_parallel_resources(chain: &Chain, result: &mut ValidationResult) {
    // Group steps by parallel group
    let mut groups: HashMap<u32, Vec<(usize, &ChainStep)>> = HashMap::new();

    for (i, step) in chain.steps.iter().enumerate() {
        if let StepType::Regular(chain_step) = step
            && let Some(group) = chain_step.parallel_group
        {
            groups.entry(group).or_default().push((i, chain_step));
        }
    }

    // Check each group for resource conflicts
    for (group_id, steps) in groups {
        let mut resources_used: HashMap<&String, (usize, &str)> = HashMap::new();

        for (i, step) in &steps {
            // Check both inputs and outputs as "resources"
            for resource in step.inputs.iter().chain(step.outputs.iter()) {
                if let Some((other_i, other_skill)) = resources_used.get(resource) {
                    if other_i != i {
                        result.add(ValidationIssue::error(
                            "PARALLEL_CONFLICT",
                            format!(
                                "Parallel group {} has resource conflict on '{}': steps '{}' and '{}' both access it",
                                group_id, resource, other_skill, step.skill
                            ),
                        ).at_step(*i, &step.skill));
                    }
                } else {
                    resources_used.insert(resource, (*i, &step.skill));
                }
            }
        }
    }
}

/// Validate conditional steps.
fn validate_conditionals(chain: &Chain, result: &mut ValidationResult) {
    let name_regex = regex::Regex::new(r"^[a-z][a-z0-9-]*$").unwrap();
    for (i, step) in chain.steps.iter().enumerate() {
        if let StepType::Conditional(cond) = step {
            // Condition must not be empty
            if cond.condition.trim().is_empty() {
                result.add(
                    ValidationIssue::error(
                        "EMPTY_CONDITION",
                        "Conditional step has empty condition",
                    )
                    .at_step(i, &cond.then_step.skill),
                );
            }

            // Validate then step skill name
            if !name_regex.is_match(&cond.then_step.skill) {
                result.add(
                    ValidationIssue::error(
                        "INVALID_THEN_SKILL",
                        format!(
                            "Then branch skill '{}' has invalid name",
                            cond.then_step.skill
                        ),
                    )
                    .at_step(i, &cond.then_step.skill),
                );
            }

            // Validate else step if present
            if let Some(else_step) = &cond.else_step
                && !name_regex.is_match(&else_step.skill)
            {
                result.add(
                    ValidationIssue::error(
                        "INVALID_ELSE_SKILL",
                        format!("Else branch skill '{}' has invalid name", else_step.skill),
                    )
                    .at_step(i, &else_step.skill),
                );
            }

            // Warn if no else branch
            if cond.else_step.is_none() {
                result.add(ValidationIssue::info(
                    "NO_ELSE_BRANCH",
                    "Conditional step has no else branch; nothing happens if condition is false",
                ).at_step(i, &cond.then_step.skill));
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::chain::types::ConditionalStep;

    #[test]
    fn test_valid_chain() {
        let chain = Chain::new("valid-chain")
            .with_step(ChainStep::new("step-one"))
            .with_step(ChainStep::new("step-two"));

        let result = validate_chain(&chain);
        assert!(result.valid);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_empty_name() {
        let chain = Chain::new("").with_step(ChainStep::new("step"));

        let result = validate_chain(&chain);
        assert!(!result.valid);
        assert!(result.errors().iter().any(|i| i.code == "EMPTY_NAME"));
    }

    #[test]
    fn test_no_steps() {
        let chain = Chain::new("empty-chain");

        let result = validate_chain(&chain);
        assert!(!result.valid);
        assert!(result.errors().iter().any(|i| i.code == "NO_STEPS"));
    }

    #[test]
    fn test_invalid_skill_name() {
        let chain = Chain::new("test").with_step(ChainStep::new("Invalid_Name"));

        let result = validate_chain(&chain);
        assert!(!result.valid);
        assert!(
            result
                .errors()
                .iter()
                .any(|i| i.code == "INVALID_SKILL_NAME")
        );
    }

    #[test]
    fn test_valid_skill_names() {
        let chain = Chain::new("test")
            .with_step(ChainStep::new("valid-name"))
            .with_step(ChainStep::new("name123"))
            .with_step(ChainStep::new("a-b-c-1-2-3"));

        let result = validate_chain(&chain);
        assert!(result.valid);
    }

    #[test]
    fn test_missing_input() {
        let chain = Chain::new("test")
            .with_step(ChainStep::new("step-one").with_outputs(vec!["out1".to_string()]))
            .with_step(ChainStep::new("step-two").with_inputs(vec!["missing".to_string()]));

        let result = validate_chain(&chain);
        assert!(!result.valid);
        assert!(result.errors().iter().any(|i| i.code == "MISSING_INPUT"));
    }

    #[test]
    fn test_valid_dependencies() {
        let chain = Chain::new("test")
            .with_step(ChainStep::new("step-one").with_outputs(vec!["data".to_string()]))
            .with_step(ChainStep::new("step-two").with_inputs(vec!["data".to_string()]));

        let result = validate_chain(&chain);
        assert!(result.valid);
    }

    #[test]
    fn test_parallel_resource_conflict() {
        let chain = Chain::new("test")
            .with_step(
                ChainStep::new("step-a")
                    .parallel()
                    .with_parallel_group(1)
                    .with_outputs(vec!["shared".to_string()]),
            )
            .with_step(
                ChainStep::new("step-b")
                    .parallel()
                    .with_parallel_group(1)
                    .with_outputs(vec!["shared".to_string()]),
            );

        let result = validate_chain(&chain);
        assert!(!result.valid);
        assert!(
            result
                .errors()
                .iter()
                .any(|i| i.code == "PARALLEL_CONFLICT")
        );
    }

    #[test]
    fn test_conditional_empty_condition() {
        let chain = Chain::new("test").with_step(StepType::Conditional(ConditionalStep::new(
            "",
            ChainStep::new("then-step"),
        )));

        let result = validate_chain(&chain);
        assert!(!result.valid);
        assert!(result.errors().iter().any(|i| i.code == "EMPTY_CONDITION"));
    }

    #[test]
    fn test_conditional_no_else_info() {
        let chain = Chain::new("test").with_step(StepType::Conditional(ConditionalStep::new(
            "context.flag == true",
            ChainStep::new("then-step"),
        )));

        let result = validate_chain(&chain);
        assert!(result.valid); // Info doesn't fail validation
        assert!(result.issues.iter().any(|i| i.code == "NO_ELSE_BRANCH"));
    }

    #[test]
    fn test_reserved_name_warning() {
        let chain = Chain::new("test").with_step(ChainStep::new("help"));

        let result = validate_chain(&chain);
        assert!(result.valid); // Warning doesn't fail validation
        assert!(result.warnings().iter().any(|i| i.code == "RESERVED_NAME"));
    }
}
