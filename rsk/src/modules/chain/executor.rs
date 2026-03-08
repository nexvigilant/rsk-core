//! # Chain Executor
//!
//! Executes skill chains with parallel support and checkpointing.
//!
//! ## Design Principles
//! - **Atomic**: Only execution orchestration, delegates actual skill calls
//! - **Parallel**: Uses available parallelism for independent steps
//! - **Resumable**: Integrates with state_manager for checkpointing
//! - **Observable**: Emits progress events for monitoring
//!
//! ## Execution Model
//!
//! The executor takes a `Chain` and a skill execution function, then:
//! 1. Groups steps by parallel compatibility
//! 2. Executes each group (sequentially between groups, parallel within)
//! 3. Evaluates conditions for conditional steps
//! 4. Handles failures based on step.required flag
//! 5. Checkpoints state periodically for resume capability
//!
//! ## Usage
//! ```rust,ignore
//! use rsk::modules::chain::{Chain, ChainStep, execute_chain};
//!
//! let chain = Chain::new("my-pipeline")
//!     .with_step(ChainStep::new("analyze"))
//!     .with_step(ChainStep::new("transform"))
//!     .with_step(ChainStep::new("output"));
//!
//! // Define how to execute a skill
//! let executor = |skill: &str, args: Option<&str>| -> Result<Value, String> {
//!     // Call the actual skill here
//!     Ok(json!({"status": "success"}))
//! };
//!
//! let result = execute_chain(&chain, executor)?;
//! ```

use super::condition::evaluate_condition;
use super::types::{
    Chain, ChainResult, ChainStep, ConditionalStep, StepResult, StepStatus, StepType,
};
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;

// ═══════════════════════════════════════════════════════════════════════════
// EXECUTOR CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════

/// Configuration for chain execution
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Enable parallel execution of parallel steps
    pub parallel_enabled: bool,
    /// Maximum concurrent parallel steps
    pub max_parallel: usize,
    /// Enable checkpointing
    pub checkpointing: bool,
    /// Checkpoint directory (if checkpointing enabled)
    pub checkpoint_dir: Option<String>,
    /// Dry run mode (don't actually execute, just plan)
    pub dry_run: bool,
    /// Stop on first error
    pub fail_fast: bool,
    /// Default timeout per step (seconds)
    pub default_timeout_secs: u32,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            parallel_enabled: true,
            max_parallel: 4,
            checkpointing: false,
            checkpoint_dir: None,
            dry_run: false,
            fail_fast: true,
            default_timeout_secs: 60,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// EXECUTION CONTEXT
// ═══════════════════════════════════════════════════════════════════════════

/// Runtime context during chain execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Variables shared between steps
    pub variables: HashMap<String, Value>,
    /// Results from completed steps (by index)
    pub step_outputs: HashMap<usize, Value>,
    /// Whether to abort on next step
    pub should_abort: bool,
    /// Current step index
    pub current_step: usize,
    /// Total steps
    pub total_steps: usize,
}

impl ExecutionContext {
    fn new(total_steps: usize) -> Self {
        Self {
            variables: HashMap::new(),
            step_outputs: HashMap::new(),
            should_abort: false,
            current_step: 0,
            total_steps,
        }
    }

    /// Get context as JSON for condition evaluation
    fn as_json(&self) -> Value {
        let mut obj = serde_json::Map::new();

        // Add variables
        for (k, v) in &self.variables {
            obj.insert(k.clone(), v.clone());
        }

        // Add step outputs with step_ prefix
        for (i, v) in &self.step_outputs {
            obj.insert(format!("step_{i}"), v.clone());
        }

        // Add metadata
        obj.insert(
            "current_step".to_string(),
            Value::Number(self.current_step.into()),
        );
        obj.insert(
            "total_steps".to_string(),
            Value::Number(self.total_steps.into()),
        );

        Value::Object(obj)
    }

    /// Set a variable
    pub fn set_var(&mut self, key: &str, value: Value) {
        self.variables.insert(key.to_string(), value);
    }

    /// Get a variable
    pub fn get_var(&self, key: &str) -> Option<&Value> {
        self.variables.get(key)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SKILL EXECUTOR TRAIT
// ═══════════════════════════════════════════════════════════════════════════

/// Result from executing a single skill
pub struct SkillExecutionResult {
    pub success: bool,
    pub output: Value,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Trait for skill execution implementations.
///
/// Implement this to provide actual skill execution logic.
pub trait SkillExecutor: Send + Sync {
    /// Execute a skill with given arguments.
    ///
    /// # Arguments
    /// * `skill` - Skill name to execute
    /// * `args` - Optional arguments string
    /// * `context` - Execution context with variables
    ///
    /// # Returns
    /// Result with output value or error
    fn execute(
        &self,
        skill: &str,
        args: Option<&str>,
        context: &ExecutionContext,
    ) -> SkillExecutionResult;
}

/// Simple function-based executor for testing and simple use cases
pub struct FnExecutor<F>
where
    F: Fn(&str, Option<&str>, &ExecutionContext) -> SkillExecutionResult + Send + Sync,
{
    func: F,
}

impl<F> FnExecutor<F>
where
    F: Fn(&str, Option<&str>, &ExecutionContext) -> SkillExecutionResult + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F> SkillExecutor for FnExecutor<F>
where
    F: Fn(&str, Option<&str>, &ExecutionContext) -> SkillExecutionResult + Send + Sync,
{
    fn execute(
        &self,
        skill: &str,
        args: Option<&str>,
        context: &ExecutionContext,
    ) -> SkillExecutionResult {
        (self.func)(skill, args, context)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// STEP EXECUTION
// ═══════════════════════════════════════════════════════════════════════════

/// Execute a single regular step
fn execute_step(
    index: usize,
    step: &ChainStep,
    executor: &dyn SkillExecutor,
    context: &mut ExecutionContext,
    config: &ExecutorConfig,
) -> StepResult {
    let start = Instant::now();
    let start_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0);

    // Dry run mode
    if config.dry_run {
        return StepResult {
            index,
            skill: step.skill.clone(),
            status: StepStatus::WouldExecute,
            output: None,
            error: None,
            duration_ms: 0,
            start_time,
            end_time: start_time,
            branch_taken: None,
        };
    }

    // Execute the skill
    let result = executor.execute(&step.skill, step.args.as_deref(), context);
    let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    let end_time = start_time.saturating_add(duration_ms);

    // Store output in context
    if result.success {
        context.step_outputs.insert(index, result.output.clone());

        // Auto-flow: if output has a "result" object, unpack its fields into context variables
        if let Some(obj) = result.output.as_object()
            && let Some(Value::Object(result_fields)) = obj.get("result")
        {
            for (key, val) in result_fields {
                context.set_var(key, val.clone());
            }
        }

        // If step has named outputs, store them as variables (explicit override)
        if !step.outputs.is_empty() && result.output.is_object()
            && let Some(obj) = result.output.as_object()
        {
            for output_name in &step.outputs {
                if let Some(val) = obj.get(output_name) {
                    context.set_var(output_name, val.clone());
                }
            }
        }
    }

    StepResult {
        index,
        skill: step.skill.clone(),
        status: if result.success {
            StepStatus::Success
        } else {
            StepStatus::Failed
        },
        output: if result.success {
            Some(result.output)
        } else {
            None
        },
        error: result.error,
        duration_ms,
        start_time,
        end_time,
        branch_taken: None,
    }
}

/// Execute a conditional step
fn execute_conditional_step(
    index: usize,
    cond: &ConditionalStep,
    executor: &dyn SkillExecutor,
    context: &mut ExecutionContext,
    config: &ExecutorConfig,
) -> StepResult {
    let ctx_json = context.as_json();

    // Evaluate condition
    let condition_result = evaluate_condition(&cond.condition, &ctx_json);

    let (step_to_execute, branch_taken) = if condition_result {
        (&cond.then_step, "then")
    } else if let Some(else_step) = &cond.else_step {
        (else_step, "else")
    } else {
        // No else branch, return skipped
        return StepResult {
            index,
            skill: cond.then_step.skill.clone(),
            status: StepStatus::Skipped,
            output: None,
            error: None,
            duration_ms: 0,
            start_time: 0,
            end_time: 0,
            branch_taken: Some("skipped".to_string()),
        };
    };

    // Execute the chosen branch
    let mut result = execute_step(index, step_to_execute, executor, context, config);
    result.branch_taken = Some(branch_taken.to_string());
    result
}

// ═══════════════════════════════════════════════════════════════════════════
// GROUP EXECUTION
// ═══════════════════════════════════════════════════════════════════════════

/// Group steps by parallel compatibility for execution.
///
/// Returns groups where steps within each group can run in parallel.
fn group_steps(chain: &Chain) -> Vec<Vec<(usize, &StepType)>> {
    let mut groups: Vec<Vec<(usize, &StepType)>> = Vec::new();
    let mut current_group: Vec<(usize, &StepType)> = Vec::new();
    let mut current_parallel_group: Option<u32> = None;

    for (i, step) in chain.steps.iter().enumerate() {
        let step_parallel_group = step.parallel_group();

        if step_parallel_group.is_some() && step_parallel_group == current_parallel_group {
            // Same parallel group, add to current
            current_group.push((i, step));
        } else {
            // Different group or not parallel
            if !current_group.is_empty() {
                groups.push(std::mem::take(&mut current_group));
            }

            current_group.push((i, step));
            current_parallel_group = step_parallel_group;
        }
    }

    // Don't forget last group
    if !current_group.is_empty() {
        groups.push(current_group);
    }

    groups
}

/// Execute a group of steps (potentially in parallel)
fn execute_group(
    group: &[(usize, &StepType)],
    executor: &dyn SkillExecutor,
    context: &mut ExecutionContext,
    config: &ExecutorConfig,
) -> Vec<StepResult> {
    let is_parallel = group.len() > 1 && group.iter().all(|(_, s)| s.is_parallel());

    if is_parallel && config.parallel_enabled && !config.dry_run {
        execute_group_parallel(group, executor, context, config)
    } else {
        execute_group_sequential(group, executor, context, config)
    }
}

/// Execute a group in parallel using rayon
fn execute_group_parallel(
    group: &[(usize, &StepType)],
    executor: &dyn SkillExecutor,
    context: &mut ExecutionContext,
    config: &ExecutorConfig,
) -> Vec<StepResult> {
    // Snapshot the context for parallel read access
    let context_snapshot = context.clone();

    // Execute steps in parallel, collecting results
    let results: Vec<StepResult> = group
        .par_iter()
        .map(|(index, step)| match step {
            StepType::Regular(chain_step) => {
                execute_step_parallel(*index, chain_step, executor, &context_snapshot, config)
            }
            StepType::Conditional(cond_step) => execute_conditional_step_parallel(
                *index,
                cond_step,
                executor,
                &context_snapshot,
                config,
            ),
        })
        .collect();

    // Merge results back into the main context
    for result in &results {
        if let Some(output) = &result.output {
            context.step_outputs.insert(result.index, output.clone());
        }

        // Check for failures
        let is_failure =
            result.status == StepStatus::Failed || result.status == StepStatus::TimedOut;
        if is_failure && config.fail_fast {
            // Find if this step is required
            if let Some((_, step)) = group.iter().find(|(i, _)| *i == result.index) {
                let is_required = match step {
                    StepType::Regular(s) => s.required,
                    StepType::Conditional(_) => true,
                };
                if is_required {
                    context.should_abort = true;
                }
            }
        }
    }

    results
}

/// Execute a single step in parallel context (immutable context)
fn execute_step_parallel(
    index: usize,
    step: &ChainStep,
    executor: &dyn SkillExecutor,
    context: &ExecutionContext,
    config: &ExecutorConfig,
) -> StepResult {
    use std::time::{SystemTime, UNIX_EPOCH};

    let start = Instant::now();
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0);

    if config.dry_run {
        return StepResult {
            index,
            skill: step.skill.clone(),
            status: StepStatus::WouldExecute,
            output: None,
            error: None,
            duration_ms: 0,
            start_time,
            end_time: start_time,
            branch_taken: None,
        };
    }

    let exec_result = executor.execute(&step.skill, step.args.as_deref(), context);
    let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    let end_time = start_time.saturating_add(duration_ms);

    if exec_result.success {
        StepResult {
            index,
            skill: step.skill.clone(),
            status: StepStatus::Success,
            output: Some(exec_result.output),
            error: None,
            duration_ms,
            start_time,
            end_time,
            branch_taken: None,
        }
    } else {
        StepResult {
            index,
            skill: step.skill.clone(),
            status: StepStatus::Failed,
            output: None,
            error: exec_result.error,
            duration_ms,
            start_time,
            end_time,
            branch_taken: None,
        }
    }
}

/// Execute a conditional step in parallel context (immutable context)
fn execute_conditional_step_parallel(
    index: usize,
    cond_step: &ConditionalStep,
    executor: &dyn SkillExecutor,
    context: &ExecutionContext,
    config: &ExecutorConfig,
) -> StepResult {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Evaluate the condition
    let condition_met = evaluate_condition(&cond_step.condition, &context.as_json());
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0);

    if condition_met {
        let mut result =
            execute_step_parallel(index, &cond_step.then_step, executor, context, config);
        result.branch_taken = Some("then".to_string());
        result
    } else if let Some(else_step) = &cond_step.else_step {
        let mut result = execute_step_parallel(index, else_step, executor, context, config);
        result.branch_taken = Some("else".to_string());
        result
    } else {
        StepResult {
            index,
            skill: cond_step.then_step.skill.clone(),
            status: StepStatus::Skipped,
            output: None,
            error: None,
            duration_ms: 0,
            start_time,
            end_time: start_time,
            branch_taken: Some("skipped".to_string()),
        }
    }
}

/// Execute a group sequentially
fn execute_group_sequential(
    group: &[(usize, &StepType)],
    executor: &dyn SkillExecutor,
    context: &mut ExecutionContext,
    config: &ExecutorConfig,
) -> Vec<StepResult> {
    let mut results = Vec::with_capacity(group.len());

    for (index, step) in group {
        context.current_step = *index;

        let result = match step {
            StepType::Regular(chain_step) => {
                execute_step(*index, chain_step, executor, context, config)
            }
            StepType::Conditional(cond_step) => {
                execute_conditional_step(*index, cond_step, executor, context, config)
            }
        };

        let is_failure =
            result.status == StepStatus::Failed || result.status == StepStatus::TimedOut;

        // Check if we should abort
        if is_failure {
            let is_required = match step {
                StepType::Regular(s) => s.required,
                StepType::Conditional(_) => true, // Conditionals are always required
            };

            if is_required && config.fail_fast {
                context.should_abort = true;
            }
        }

        results.push(result);

        if context.should_abort {
            break;
        }
    }

    results
}

// ═══════════════════════════════════════════════════════════════════════════
// MAIN EXECUTION FUNCTION
// ═══════════════════════════════════════════════════════════════════════════

/// Execute a chain with the given skill executor.
///
/// # Arguments
/// * `chain` - The chain to execute
/// * `executor` - Implementation of SkillExecutor trait
/// * `config` - Execution configuration
///
/// # Returns
/// * `ChainResult` with all step results and overall success status
pub fn execute_chain(
    chain: &Chain,
    executor: &dyn SkillExecutor,
    config: &ExecutorConfig,
) -> ChainResult {
    let start = Instant::now();
    let mut context = ExecutionContext::new(chain.steps.len());

    // Initialize context with chain's context variables
    for (k, v) in &chain.context {
        context.set_var(k, v.clone());
    }

    // Group steps for execution
    let groups = group_steps(chain);

    // Execute each group
    let mut all_results: Vec<StepResult> = Vec::with_capacity(chain.steps.len());

    for group in groups {
        if context.should_abort {
            // Mark remaining steps as skipped
            for (index, step) in group {
                all_results.push(StepResult {
                    index,
                    skill: step.skill_name().to_string(),
                    status: StepStatus::Skipped,
                    output: None,
                    error: Some("Aborted due to previous failure".to_string()),
                    duration_ms: 0,
                    start_time: 0,
                    end_time: 0,
                    branch_taken: None,
                });
            }
            continue;
        }

        let group_results = execute_group(&group, executor, &mut context, config);
        all_results.extend(group_results);
    }

    // Calculate overall success - failures from optional steps don't count
    let success = all_results.iter().enumerate().all(|(i, r)| {
        // Success, Skipped, or WouldExecute are always OK
        if matches!(
            r.status,
            StepStatus::Success | StepStatus::Skipped | StepStatus::WouldExecute
        ) {
            return true;
        }
        // For Failed/TimedOut, check if the step was optional
        if let Some(step) = chain.steps.get(i) {
            match step {
                StepType::Regular(s) => !s.required, // Optional step failure is OK
                StepType::Conditional(_) => false,   // Conditional steps are always required
            }
        } else {
            false
        }
    });

    let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

    ChainResult {
        chain_name: chain.name.clone(),
        success,
        steps: all_results,
        duration_ms,
        dry_run: config.dry_run,
        checkpoint_id: None,
        parallel_efficiency: None,
        error: if success {
            None
        } else {
            Some("One or more steps failed".to_string())
        },
    }
}

/// Convenience function to execute with a closure
pub fn execute_chain_with_fn<F>(
    chain: &Chain,
    executor_fn: F,
    config: &ExecutorConfig,
) -> ChainResult
where
    F: Fn(&str, Option<&str>, &ExecutionContext) -> SkillExecutionResult + Send + Sync,
{
    let executor = FnExecutor::new(executor_fn);
    execute_chain(chain, &executor, config)
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn success_executor(
        _skill: &str,
        _args: Option<&str>,
        _ctx: &ExecutionContext,
    ) -> SkillExecutionResult {
        SkillExecutionResult {
            success: true,
            output: json!({"status": "ok"}),
            error: None,
            duration_ms: 10,
        }
    }

    fn failing_executor(
        skill: &str,
        _args: Option<&str>,
        _ctx: &ExecutionContext,
    ) -> SkillExecutionResult {
        if skill == "fail" {
            SkillExecutionResult {
                success: false,
                output: Value::Null,
                error: Some("Intentional failure".to_string()),
                duration_ms: 5,
            }
        } else {
            success_executor(skill, _args, _ctx)
        }
    }

    #[test]
    fn test_execute_simple_chain() {
        let chain = Chain::new("test")
            .with_step(ChainStep::new("step-one"))
            .with_step(ChainStep::new("step-two"));

        let config = ExecutorConfig::default();
        let result = execute_chain_with_fn(&chain, success_executor, &config);

        assert!(result.success);
        assert_eq!(result.steps.len(), 2);
        assert!(result.steps.iter().all(|s| s.status == StepStatus::Success));
    }

    #[test]
    fn test_execute_dry_run() {
        let chain = Chain::new("test").with_step(ChainStep::new("step-one"));

        let config = ExecutorConfig {
            dry_run: true,
            ..Default::default()
        };
        let result = execute_chain_with_fn(&chain, success_executor, &config);

        assert!(result.success);
        assert!(result.dry_run);
        assert_eq!(result.steps[0].status, StepStatus::WouldExecute);
    }

    #[test]
    fn test_execute_fail_fast() {
        let chain = Chain::new("test")
            .with_step(ChainStep::new("step-one"))
            .with_step(ChainStep::new("fail"))
            .with_step(ChainStep::new("step-three"));

        let config = ExecutorConfig {
            fail_fast: true,
            ..Default::default()
        };
        let result = execute_chain_with_fn(&chain, failing_executor, &config);

        assert!(!result.success);
        assert_eq!(result.steps[0].status, StepStatus::Success);
        assert_eq!(result.steps[1].status, StepStatus::Failed);
        assert_eq!(result.steps[2].status, StepStatus::Skipped);
    }

    #[test]
    fn test_execute_optional_step_failure() {
        let chain = Chain::new("test")
            .with_step(ChainStep::new("step-one"))
            .with_step(ChainStep::new("fail").optional())
            .with_step(ChainStep::new("step-three"));

        let config = ExecutorConfig {
            fail_fast: true,
            ..Default::default()
        };
        let result = execute_chain_with_fn(&chain, failing_executor, &config);

        // Should continue after optional failure
        assert!(result.success); // Overall still succeeds because the failure was optional
        assert_eq!(result.steps[0].status, StepStatus::Success);
        assert_eq!(result.steps[1].status, StepStatus::Failed);
        assert_eq!(result.steps[2].status, StepStatus::Success);
    }

    #[test]
    fn test_execute_conditional_then() {
        let chain = Chain::new("test")
            .with_context("flag", json!(true))
            .with_step(StepType::Conditional(
                ConditionalStep::new("context.flag == true", ChainStep::new("then-step"))
                    .with_else(ChainStep::new("else-step")),
            ));

        let config = ExecutorConfig::default();
        let result = execute_chain_with_fn(&chain, success_executor, &config);

        assert!(result.success);
        assert_eq!(result.steps[0].skill, "then-step");
        assert_eq!(result.steps[0].branch_taken, Some("then".to_string()));
    }

    #[test]
    fn test_execute_conditional_else() {
        let chain = Chain::new("test")
            .with_context("flag", json!(false))
            .with_step(StepType::Conditional(
                ConditionalStep::new("context.flag == true", ChainStep::new("then-step"))
                    .with_else(ChainStep::new("else-step")),
            ));

        let config = ExecutorConfig::default();
        let result = execute_chain_with_fn(&chain, success_executor, &config);

        assert!(result.success);
        assert_eq!(result.steps[0].skill, "else-step");
        assert_eq!(result.steps[0].branch_taken, Some("else".to_string()));
    }

    #[test]
    fn test_execute_conditional_skipped() {
        let chain = Chain::new("test")
            .with_context("flag", json!(false))
            .with_step(StepType::Conditional(ConditionalStep::new(
                "context.flag == true",
                ChainStep::new("then-step"),
            ))); // No else branch

        let config = ExecutorConfig::default();
        let result = execute_chain_with_fn(&chain, success_executor, &config);

        assert!(result.success);
        assert_eq!(result.steps[0].status, StepStatus::Skipped);
        assert_eq!(result.steps[0].branch_taken, Some("skipped".to_string()));
    }

    #[test]
    fn test_group_steps_sequential() {
        let chain = Chain::new("test")
            .with_step(ChainStep::new("a"))
            .with_step(ChainStep::new("b"))
            .with_step(ChainStep::new("c"));

        let groups = group_steps(&chain);

        // Each step in its own group (no parallel)
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn test_group_steps_parallel() {
        let chain = Chain::new("test")
            .with_step(ChainStep::new("setup"))
            .with_step(ChainStep::new("build").parallel().with_parallel_group(1))
            .with_step(ChainStep::new("test").parallel().with_parallel_group(1))
            .with_step(ChainStep::new("deploy"));

        let groups = group_steps(&chain);

        // 3 groups: [setup], [build, test], [deploy]
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].len(), 1);
        assert_eq!(groups[1].len(), 2);
        assert_eq!(groups[2].len(), 1);
    }

    #[test]
    fn test_context_variable_passing() {
        let chain = Chain::new("test")
            .with_step(ChainStep::new("producer").with_outputs(vec!["result".to_string()]))
            .with_step(ChainStep::new("consumer").with_inputs(vec!["result".to_string()]));

        fn output_executor(
            skill: &str,
            _args: Option<&str>,
            _ctx: &ExecutionContext,
        ) -> SkillExecutionResult {
            if skill == "producer" {
                SkillExecutionResult {
                    success: true,
                    output: json!({"result": "produced_value"}),
                    error: None,
                    duration_ms: 10,
                }
            } else {
                SkillExecutionResult {
                    success: true,
                    output: json!({"consumed": true}),
                    error: None,
                    duration_ms: 10,
                }
            }
        }

        let config = ExecutorConfig::default();
        let result = execute_chain_with_fn(&chain, output_executor, &config);

        assert!(result.success);
    }
}
