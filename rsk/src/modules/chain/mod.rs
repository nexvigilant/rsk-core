//! # Chain Module
//!
//! Skill chain execution with parsing, validation, and parallel execution support.
//!
//! ## Architecture
//!
//! This module is composed of atomic sub-modules:
//!
//! | Module | Responsibility |
//! |--------|---------------|
//! | `types` | Core data structures (Chain, ChainStep, etc.) |
//! | `parser` | Parse inline and YAML chain definitions |
//! | `condition` | Evaluate conditional expressions |
//! | `validator` | Validate chain correctness |
//! | `executor` | Execute chains with parallel support |
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rsk::modules::chain::{parse_inline, execute_chain_with_fn, ExecutorConfig};
//!
//! // Parse an inline chain
//! let chain = parse_inline("smart-goal -> decompose-task -> proceed")?;
//!
//! // Execute with a custom executor
//! let result = execute_chain_with_fn(&chain, |skill, args, ctx| {
//!     // Execute the skill here
//!     SkillExecutionResult { success: true, output: json!({}), error: None, duration_ms: 0 }
//! }, &ExecutorConfig::default());
//!
//! assert!(result.success);
//! ```
//!
//! ## Comparison to Python chain.py
//!
//! This Rust implementation replaces the 1,053-line `chain.py` with:
//!
//! | Python | Rust | Benefit |
//! |--------|------|---------|
//! | `class Chain` | `types::Chain` | Compile-time type safety |
//! | `parse_inline()` | `parser::parse_inline()` | Zero-copy parsing, regex precompiled |
//! | `parse_yaml()` | `parser::parse_yaml()` | Uses serde (5-10x faster) |
//! | `evaluate_condition()` | `condition::evaluate_condition()` | No eval(), safe |
//! | `validate_chain()` | `validator::validate_chain()` | Exhaustive checks |
//! | `execute_chain()` | `executor::execute_chain()` | Parallel via rayon (future) |

pub mod condition;
pub mod executor;
pub mod parser;
pub mod types;
pub mod validator;

// Re-export commonly used types and functions
pub use condition::{ConditionError, evaluate_condition, evaluate_condition_result, resolve_value};
pub use executor::{
    ExecutionContext, ExecutorConfig, FnExecutor, SkillExecutionResult, SkillExecutor,
    execute_chain, execute_chain_with_fn,
};
pub use parser::{ParseError, assign_parallel_groups, parse_inline, parse_yaml};
pub use types::{
    Chain, ChainResult, ChainStep, CompositionType, ConditionalStep, StepResult, StepStatus,
    StepType,
};
pub use validator::{Severity, ValidationIssue, ValidationResult, is_valid, validate_chain};

// ═══════════════════════════════════════════════════════════════════════════
// HIGH-LEVEL CONVENIENCE FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Parse and validate an inline chain definition.
///
/// Combines parsing and validation in one step.
pub fn parse_and_validate_inline(input: &str) -> Result<(Chain, ValidationResult), ParseError> {
    let chain = parse_inline(input)?;
    let validation = validate_chain(&chain);
    Ok((chain, validation))
}

/// Parse and validate a YAML chain definition.
pub fn parse_and_validate_yaml(content: &str) -> Result<(Chain, ValidationResult), ParseError> {
    let chain = parse_yaml(content)?;
    let validation = validate_chain(&chain);
    Ok((chain, validation))
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_end_to_end_inline() {
        // Parse
        let chain = parse_inline("analyze -> transform -> output").unwrap();
        assert_eq!(chain.len(), 3);

        // Validate
        let validation = validate_chain(&chain);
        assert!(validation.valid);

        // Execute (dry run)
        let config = ExecutorConfig {
            dry_run: true,
            ..Default::default()
        };
        let result = execute_chain_with_fn(
            &chain,
            |_, _, _| SkillExecutionResult {
                success: true,
                output: json!({}),
                error: None,
                duration_ms: 0,
            },
            &config,
        );
        assert!(result.success);
        assert!(result.dry_run);
    }

    #[test]
    fn test_end_to_end_yaml() {
        let yaml = r#"
name: test-pipeline
steps:
  - skill: analyze
  - skill: transform
    parallel: true
  - skill: validate
    parallel: true
  - skill: output
"#;
        // Parse
        let chain = parse_yaml(yaml).unwrap();
        assert_eq!(chain.name, "test-pipeline");
        assert_eq!(chain.len(), 4);

        // Validate
        let validation = validate_chain(&chain);
        assert!(validation.valid);

        // Check parallel grouping
        assert!(chain.steps[1].is_parallel());
        assert!(chain.steps[2].is_parallel());
    }

    #[test]
    fn test_condition_integration() {
        let chain = Chain::new("test")
            .with_context("enabled", json!(true))
            .with_step(StepType::Conditional(ConditionalStep::new(
                "context.enabled == true",
                ChainStep::new("run-task"),
            )));

        let config = ExecutorConfig::default();
        let result = execute_chain_with_fn(
            &chain,
            |_, _, _| SkillExecutionResult {
                success: true,
                output: json!({"done": true}),
                error: None,
                duration_ms: 10,
            },
            &config,
        );

        assert!(result.success);
        assert_eq!(result.steps[0].branch_taken, Some("then".to_string()));
    }

    #[test]
    fn test_parse_and_validate_inline_convenience() {
        let (chain, validation) = parse_and_validate_inline("step-one -> step-two").unwrap();

        assert_eq!(chain.len(), 2);
        assert!(validation.valid);
    }
}
