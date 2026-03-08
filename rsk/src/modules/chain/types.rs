//! # Chain Types
//!
//! Core data structures for skill chain execution.
//!
//! ## Design Principles
//! - **Atomic**: Only type definitions, no logic
//! - **Serializable**: All types derive Serialize/Deserialize for persistence
//! - **Composable**: Types are building blocks for parser, executor, etc.
//!
//! ## Usage
//! ```rust,ignore
//! use rsk::modules::chain::types::{Chain, ChainStep, CompositionType};
//!
//! let step = ChainStep::new("smart-goal");
//! let chain = Chain::new("my-pipeline").with_step(step);
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════
// COMPOSITION TYPE
// ═══════════════════════════════════════════════════════════════════════════

/// How steps in a chain are composed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CompositionType {
    /// Steps execute one after another (default)
    #[default]
    Sequential,
    /// All steps execute concurrently
    Parallel,
    /// Steps execute based on conditions
    Conditional,
    /// Steps repeat N times
    Loop,
}

impl CompositionType {
    /// Parse from operator string
    pub fn from_operator(op: &str) -> Option<Self> {
        match op {
            "->" => Some(Self::Sequential),
            "|" => Some(Self::Parallel),
            "?" => Some(Self::Conditional),
            "*" => Some(Self::Loop),
            _ => None,
        }
    }

    /// Get the operator string
    pub fn as_operator(&self) -> &'static str {
        match self {
            Self::Sequential => "->",
            Self::Parallel => "|",
            Self::Conditional => "?",
            Self::Loop => "*",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CHAIN STEP
// ═══════════════════════════════════════════════════════════════════════════

/// A single step in a chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStep {
    /// Skill name to execute
    pub skill: String,
    /// Optional arguments to pass
    pub args: Option<String>,
    /// Input variables this step consumes
    pub inputs: Vec<String>,
    /// Output variables this step produces
    pub outputs: Vec<String>,
    /// Whether failure stops the chain
    pub required: bool,
    /// Whether this step can run in parallel with adjacent parallel steps
    pub parallel: bool,
    /// Group ID for parallel execution (steps with same group run together)
    pub parallel_group: Option<u32>,
    /// Timeout in seconds (0 = no timeout)
    pub timeout_secs: u32,
    /// Retry count on failure
    pub retries: u32,
}

impl ChainStep {
    /// Create a new chain step
    pub fn new(skill: &str) -> Self {
        Self {
            skill: skill.to_string(),
            args: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            required: true,
            parallel: false,
            parallel_group: None,
            timeout_secs: 60,
            retries: 0,
        }
    }

    /// Builder: set arguments
    pub fn with_args(mut self, args: &str) -> Self {
        self.args = Some(args.to_string());
        self
    }

    /// Builder: set inputs
    pub fn with_inputs(mut self, inputs: Vec<String>) -> Self {
        self.inputs = inputs;
        self
    }

    /// Builder: set outputs
    pub fn with_outputs(mut self, outputs: Vec<String>) -> Self {
        self.outputs = outputs;
        self
    }

    /// Builder: mark as optional (not required)
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Builder: mark as parallel
    pub fn parallel(mut self) -> Self {
        self.parallel = true;
        self
    }

    /// Builder: set parallel group
    pub fn with_parallel_group(mut self, group: u32) -> Self {
        self.parallel_group = Some(group);
        self.parallel = true;
        self
    }

    /// Builder: set timeout
    pub fn with_timeout(mut self, secs: u32) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Builder: set retries
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }
}

impl Default for ChainStep {
    fn default() -> Self {
        Self::new("")
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONDITIONAL STEP
// ═══════════════════════════════════════════════════════════════════════════

/// A conditional step with if/then/else branching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalStep {
    /// Condition expression to evaluate
    pub condition: String,
    /// Step to execute if condition is true
    pub then_step: ChainStep,
    /// Optional step to execute if condition is false
    pub else_step: Option<ChainStep>,
}

impl ConditionalStep {
    /// Create a new conditional step
    pub fn new(condition: &str, then_step: ChainStep) -> Self {
        Self {
            condition: condition.to_string(),
            then_step,
            else_step: None,
        }
    }

    /// Builder: add else branch
    pub fn with_else(mut self, else_step: ChainStep) -> Self {
        self.else_step = Some(else_step);
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// STEP TYPE (ENUM)
// ═══════════════════════════════════════════════════════════════════════════

/// Union type for regular and conditional steps
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepType {
    /// Regular skill execution
    Regular(ChainStep),
    /// Conditional branching
    Conditional(ConditionalStep),
}

impl StepType {
    /// Get the skill name (for regular steps or then_step for conditional)
    pub fn skill_name(&self) -> &str {
        match self {
            Self::Regular(step) => &step.skill,
            Self::Conditional(cond) => &cond.then_step.skill,
        }
    }

    /// Check if this is a parallel step
    pub fn is_parallel(&self) -> bool {
        match self {
            Self::Regular(step) => step.parallel,
            Self::Conditional(_) => false, // Conditionals are never parallel
        }
    }

    /// Get parallel group if any
    pub fn parallel_group(&self) -> Option<u32> {
        match self {
            Self::Regular(step) => step.parallel_group,
            Self::Conditional(_) => None,
        }
    }
}

impl From<ChainStep> for StepType {
    fn from(step: ChainStep) -> Self {
        Self::Regular(step)
    }
}

impl From<ConditionalStep> for StepType {
    fn from(step: ConditionalStep) -> Self {
        Self::Conditional(step)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CHAIN
// ═══════════════════════════════════════════════════════════════════════════

/// A complete skill chain definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    /// Human-readable name
    pub name: String,
    /// Description of what this chain does
    pub description: Option<String>,
    /// Steps in the chain
    pub steps: Vec<StepType>,
    /// Primary composition mode
    pub composition: CompositionType,
    /// Shared execution context (variables accessible to all steps)
    pub context: HashMap<String, Value>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Version string
    pub version: Option<String>,
}

impl Chain {
    /// Create a new chain
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            steps: Vec::new(),
            composition: CompositionType::Sequential,
            context: HashMap::new(),
            tags: Vec::new(),
            version: None,
        }
    }

    /// Builder: add description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Builder: add a step
    pub fn with_step<S: Into<StepType>>(mut self, step: S) -> Self {
        self.steps.push(step.into());
        self
    }

    /// Builder: add multiple steps
    pub fn with_steps<S: Into<StepType>>(mut self, steps: impl IntoIterator<Item = S>) -> Self {
        for step in steps {
            self.steps.push(step.into());
        }
        self
    }

    /// Builder: set composition type
    pub fn with_composition(mut self, composition: CompositionType) -> Self {
        self.composition = composition;
        self
    }

    /// Builder: add context variable
    pub fn with_context(mut self, key: &str, value: Value) -> Self {
        self.context.insert(key.to_string(), value);
        self
    }

    /// Builder: add tag
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Builder: set version
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }

    /// Get step count
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Get step by index
    pub fn get_step(&self, index: usize) -> Option<&StepType> {
        self.steps.get(index)
    }

    /// Check if chain has any parallel steps
    pub fn has_parallel_steps(&self) -> bool {
        self.steps.iter().any(|s| s.is_parallel())
    }

    /// Get all skill names in the chain
    pub fn skill_names(&self) -> Vec<&str> {
        self.steps.iter().map(|s| s.skill_name()).collect()
    }
}

impl Default for Chain {
    fn default() -> Self {
        Self::new("unnamed")
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// EXECUTION RESULT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Status of a step execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Not yet executed
    Pending,
    /// Currently executing
    Running,
    /// Completed successfully
    Success,
    /// Failed with error
    Failed,
    /// Skipped (dependency failed or condition false)
    Skipped,
    /// Timed out
    TimedOut,
    /// Would execute (dry run)
    WouldExecute,
}

/// Result of executing a single step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step index in the chain
    pub index: usize,
    /// Skill name
    pub skill: String,
    /// Execution status
    pub status: StepStatus,
    /// Output data (if any)
    pub output: Option<Value>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Start timestamp (Unix millis)
    pub start_time: u64,
    /// End timestamp (Unix millis)
    pub end_time: u64,
    /// Which branch was taken (for conditional steps)
    pub branch_taken: Option<String>,
}

impl StepResult {
    /// Create a pending result
    pub fn pending(index: usize, skill: &str) -> Self {
        Self {
            index,
            skill: skill.to_string(),
            status: StepStatus::Pending,
            output: None,
            error: None,
            duration_ms: 0,
            start_time: 0,
            end_time: 0,
            branch_taken: None,
        }
    }

    /// Check if step succeeded
    pub fn is_success(&self) -> bool {
        self.status == StepStatus::Success
    }

    /// Check if step failed
    pub fn is_failed(&self) -> bool {
        matches!(self.status, StepStatus::Failed | StepStatus::TimedOut)
    }
}

/// Result of executing a complete chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainResult {
    /// Chain name
    pub chain_name: String,
    /// Overall success
    pub success: bool,
    /// Step results
    pub steps: Vec<StepResult>,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Whether this was a dry run
    pub dry_run: bool,
    /// Checkpoint ID (if checkpointing enabled)
    pub checkpoint_id: Option<String>,
    /// Parallel efficiency (total_step_time / wall_time)
    pub parallel_efficiency: Option<f32>,
    /// Error message (if chain failed)
    pub error: Option<String>,
}

impl ChainResult {
    /// Create a new chain result
    pub fn new(chain_name: &str) -> Self {
        Self {
            chain_name: chain_name.to_string(),
            success: true,
            steps: Vec::new(),
            duration_ms: 0,
            dry_run: false,
            checkpoint_id: None,
            parallel_efficiency: None,
            error: None,
        }
    }

    /// Count successful steps
    pub fn success_count(&self) -> usize {
        self.steps.iter().filter(|s| s.is_success()).count()
    }

    /// Count failed steps
    pub fn failed_count(&self) -> usize {
        self.steps.iter().filter(|s| s.is_failed()).count()
    }

    /// Get progress percentage
    pub fn progress_percent(&self) -> f32 {
        if self.steps.is_empty() {
            100.0
        } else {
            let completed = self
                .steps
                .iter()
                .filter(|s| s.status != StepStatus::Pending)
                .count();
            #[allow(clippy::as_conversions)] // usize→f32 for progress percentage
            let pct = completed as f32 / self.steps.len() as f32;
            pct * 100.0
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_step_builder() {
        let step = ChainStep::new("smart-goal")
            .with_args("--verbose")
            .with_timeout(120)
            .with_retries(3)
            .optional();

        assert_eq!(step.skill, "smart-goal");
        assert_eq!(step.args, Some("--verbose".to_string()));
        assert_eq!(step.timeout_secs, 120);
        assert_eq!(step.retries, 3);
        assert!(!step.required);
    }

    #[test]
    fn test_conditional_step() {
        let cond = ConditionalStep::new("context.has_tests == true", ChainStep::new("run-tests"))
            .with_else(ChainStep::new("skip-tests"));

        assert_eq!(cond.condition, "context.has_tests == true");
        assert_eq!(cond.then_step.skill, "run-tests");
        assert!(cond.else_step.is_some());
    }

    #[test]
    fn test_chain_builder() {
        let chain = Chain::new("my-pipeline")
            .with_description("Test pipeline")
            .with_step(ChainStep::new("step1"))
            .with_step(ChainStep::new("step2"))
            .with_composition(CompositionType::Sequential)
            .with_tag("test");

        assert_eq!(chain.name, "my-pipeline");
        assert_eq!(chain.len(), 2);
        assert!(chain.description.is_some());
        assert_eq!(chain.tags, vec!["test"]);
    }

    #[test]
    fn test_composition_type_operators() {
        assert_eq!(
            CompositionType::from_operator("->"),
            Some(CompositionType::Sequential)
        );
        assert_eq!(
            CompositionType::from_operator("|"),
            Some(CompositionType::Parallel)
        );
        assert_eq!(
            CompositionType::from_operator("?"),
            Some(CompositionType::Conditional)
        );
        assert_eq!(
            CompositionType::from_operator("*"),
            Some(CompositionType::Loop)
        );
        assert_eq!(CompositionType::from_operator("invalid"), None);
    }

    #[test]
    fn test_step_type_skill_name() {
        let regular = StepType::Regular(ChainStep::new("my-skill"));
        assert_eq!(regular.skill_name(), "my-skill");

        let conditional = StepType::Conditional(ConditionalStep::new(
            "true",
            ChainStep::new("conditional-skill"),
        ));
        assert_eq!(conditional.skill_name(), "conditional-skill");
    }

    #[test]
    fn test_chain_result() {
        let mut result = ChainResult::new("test-chain");
        result.steps.push(StepResult::pending(0, "step1"));
        result.steps.push(StepResult::pending(1, "step2"));

        assert_eq!(result.success_count(), 0);
        assert_eq!(result.progress_percent(), 0.0);

        result.steps[0].status = StepStatus::Success;
        assert_eq!(result.success_count(), 1);
        assert_eq!(result.progress_percent(), 50.0);
    }

    #[test]
    fn test_serialization() {
        let chain =
            Chain::new("serialize-test").with_step(ChainStep::new("step1").with_args("--flag"));

        let json = serde_json::to_string(&chain).unwrap();
        let deserialized: Chain = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "serialize-test");
        assert_eq!(deserialized.len(), 1);
    }
}
