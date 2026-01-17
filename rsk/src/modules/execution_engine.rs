//! # Execution Engine
//!
//! DAG-based autonomous execution engine with checkpointing.
//!
//! ## Capabilities
//! - Build execution plans from module lists
//! - Topologically sort modules respecting dependencies
//! - Execute modules level-by-level with parallelization
//! - Checkpoint state for resume capability
//! - Emit Andon signals for progress tracking
//!
//! ## Performance Targets
//! - Plan building: < 1ms for 100 modules
//! - State serialization: < 1ms
//! - Checkpoint I/O: < 5ms
//!
//! ## Example
//! ```rust,ignore
//! use rsk::modules::execution_engine::{ExecutionModule, build_execution_plan};
//!
//! let modules = vec![
//!     ExecutionModule::new("M1", "Create stubs", vec![]),
//!     ExecutionModule::new("M2", "Implement builder", vec!["M1".to_string()]),
//!     ExecutionModule::new("M3", "Add CLI", vec!["M1".to_string()]),
//!     ExecutionModule::new("M4", "Bridge script", vec!["M2".to_string(), "M3".to_string()]),
//! ];
//!
//! let plan = build_execution_plan(modules).unwrap();
//! assert_eq!(plan.levels.len(), 3); // [M1], [M2, M3], [M4]
//! ```

use crate::modules::graph::{SkillGraph, SkillNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Effort size estimation for modules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffortSize {
    /// Small: < 30 minutes
    S,
    /// Medium: 30 min - 2 hours
    M,
    /// Large: 2-8 hours
    L,
    /// Extra Large: > 8 hours
    XL,
}

impl Default for EffortSize {
    fn default() -> Self {
        Self::M
    }
}

impl EffortSize {
    /// Convert effort size to estimated minutes
    pub fn to_minutes(&self) -> u32 {
        match self {
            EffortSize::S => 15,
            EffortSize::M => 60,
            EffortSize::L => 240,
            EffortSize::XL => 480,
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "S" | "SMALL" => Some(Self::S),
            "M" | "MEDIUM" => Some(Self::M),
            "L" | "LARGE" => Some(Self::L),
            "XL" | "XLARGE" | "EXTRA_LARGE" => Some(Self::XL),
            _ => None,
        }
    }
}

/// Module execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleStatus {
    /// Not yet started
    Pending,
    /// Currently executing
    InProgress,
    /// Successfully completed
    Completed,
    /// Failed with error message
    Failed(String),
    /// Skipped (e.g., due to dependency failure)
    Skipped,
}

impl Default for ModuleStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Andon signal for progress tracking (from Toyota Production System)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AndonSignal {
    /// Success - module completed successfully
    Green,
    /// Warning - completed with warnings
    Yellow,
    /// Failure - module failed
    Red,
    /// Informational - processing
    White,
    /// Blocked - waiting on external dependency
    Blue,
}

impl AndonSignal {
    /// Get display string for the signal
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Green => "GREEN",
            Self::Yellow => "YELLOW",
            Self::Red => "RED",
            Self::White => "WHITE",
            Self::Blue => "BLUE",
        }
    }
}

/// A single execution module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionModule {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Purpose/description
    pub purpose: String,
    /// List of module IDs this depends on
    pub dependencies: Vec<String>,
    /// Estimated effort
    pub effort: EffortSize,
    /// Risk score (0.0 - 1.0)
    pub risk: f32,
    /// Current status
    pub status: ModuleStatus,
    /// Whether this module is on the critical path
    pub critical: bool,
    /// Files/resources this module touches (for parallel conflict detection)
    pub resources: Vec<String>,
    /// Concrete deliverables
    pub deliverables: Vec<String>,
}

impl ExecutionModule {
    /// Create a new execution module with minimal required fields
    pub fn new(id: &str, name: &str, dependencies: Vec<String>) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            purpose: String::new(),
            dependencies,
            effort: EffortSize::default(),
            risk: 0.3,
            status: ModuleStatus::default(),
            critical: false,
            resources: Vec::new(),
            deliverables: Vec::new(),
        }
    }

    /// Builder pattern: set purpose
    pub fn with_purpose(mut self, purpose: &str) -> Self {
        self.purpose = purpose.to_string();
        self
    }

    /// Builder pattern: set effort
    pub fn with_effort(mut self, effort: EffortSize) -> Self {
        self.effort = effort;
        self
    }

    /// Builder pattern: set risk
    pub fn with_risk(mut self, risk: f32) -> Self {
        self.risk = risk.clamp(0.0, 1.0);
        self
    }

    /// Builder pattern: mark as critical
    pub fn critical(mut self) -> Self {
        self.critical = true;
        self
    }

    /// Builder pattern: add resources
    pub fn with_resources(mut self, resources: Vec<String>) -> Self {
        self.resources = resources;
        self
    }

    /// Builder pattern: add deliverables
    pub fn with_deliverables(mut self, deliverables: Vec<String>) -> Self {
        self.deliverables = deliverables;
        self
    }
}

/// Complete execution plan with DAG structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// All modules in the plan
    pub modules: HashMap<String, ExecutionModule>,
    /// Topologically sorted execution order
    pub execution_order: Vec<String>,
    /// Parallel execution levels (modules at same level can run concurrently)
    pub levels: Vec<Vec<String>>,
    /// Critical path through the DAG
    pub critical_path: Vec<String>,
    /// Total estimated duration in minutes
    pub estimated_duration_minutes: u32,
    /// Overall plan status
    pub status: PlanStatus,
}

/// Overall plan execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    /// Plan created but not started
    Created,
    /// Currently executing
    Running,
    /// Paused (can resume)
    Paused,
    /// All modules completed successfully
    Completed,
    /// One or more modules failed
    Failed,
    /// Plan was cancelled
    Cancelled,
}

impl Default for PlanStatus {
    fn default() -> Self {
        Self::Created
    }
}

/// Error types for execution engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionError {
    /// Circular dependency detected
    CycleDetected(Vec<String>),
    /// Module not found
    ModuleNotFound(String),
    /// Invalid module configuration
    InvalidModule(String),
    /// Resource conflict between modules
    ResourceConflict { module_a: String, module_b: String, resource: String },
    /// Checkpoint save/load failed
    CheckpointError(String),
    /// Generic execution error
    ExecutionFailed(String),
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CycleDetected(cycle) => {
                write!(f, "Circular dependency detected: {}", cycle.join(" -> "))
            }
            Self::ModuleNotFound(id) => write!(f, "Module not found: {}", id),
            Self::InvalidModule(msg) => write!(f, "Invalid module: {}", msg),
            Self::ResourceConflict { module_a, module_b, resource } => {
                write!(f, "Resource conflict: {} and {} both touch {}", module_a, module_b, resource)
            }
            Self::CheckpointError(msg) => write!(f, "Checkpoint error: {}", msg),
            Self::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
        }
    }
}

impl std::error::Error for ExecutionError {}

/// Result of executing a single module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResult {
    /// Module ID
    pub module_id: String,
    /// Andon signal (status)
    pub signal: AndonSignal,
    /// Result message
    pub message: String,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Any warnings generated
    pub warnings: Vec<String>,
    /// Artifacts produced
    pub artifacts: Vec<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// CORE FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Build an execution plan from a list of modules.
///
/// This function:
/// 1. Validates all modules and their dependencies
/// 2. Builds a DAG from the module dependencies
/// 3. Detects cycles (returns error if found)
/// 4. Computes topological order
/// 5. Groups modules into parallel execution levels
/// 6. Identifies the critical path
///
/// # Arguments
/// * `modules` - List of execution modules
///
/// # Returns
/// * `Ok(ExecutionPlan)` - Valid execution plan
/// * `Err(ExecutionError)` - If validation fails or cycle detected
///
/// # Example
/// ```rust,ignore
/// let modules = vec![
///     ExecutionModule::new("M1", "Root task", vec![]),
///     ExecutionModule::new("M2", "Depends on M1", vec!["M1".to_string()]),
/// ];
/// let plan = build_execution_plan(modules)?;
/// ```
pub fn build_execution_plan(modules: Vec<ExecutionModule>) -> Result<ExecutionPlan, ExecutionError> {
    if modules.is_empty() {
        return Ok(ExecutionPlan {
            modules: HashMap::new(),
            execution_order: Vec::new(),
            levels: Vec::new(),
            critical_path: Vec::new(),
            estimated_duration_minutes: 0,
            status: PlanStatus::Created,
        });
    }

    // Build module map
    let mut module_map: HashMap<String, ExecutionModule> = HashMap::new();
    for module in modules {
        if module_map.contains_key(&module.id) {
            return Err(ExecutionError::InvalidModule(
                format!("Duplicate module ID: {}", module.id)
            ));
        }
        module_map.insert(module.id.clone(), module);
    }

    // Validate dependencies exist
    for module in module_map.values() {
        for dep in &module.dependencies {
            if !module_map.contains_key(dep) {
                return Err(ExecutionError::ModuleNotFound(dep.clone()));
            }
        }
    }

    // Build SkillGraph for DAG operations
    let mut graph = SkillGraph::new();
    for (id, module) in &module_map {
        graph.add_node(SkillNode {
            name: id.clone(),
            dependencies: module.dependencies.clone(),
            outputs: Vec::new(),
            adjacencies: Vec::new(),
        });
    }

    // Compute topological order (also detects cycles)
    let execution_order = match graph.topological_sort() {
        Ok(order) => order,
        Err(cycle) => return Err(ExecutionError::CycleDetected(cycle)),
    };

    // Compute parallel execution levels
    let levels = match graph.level_parallelization() {
        Ok(lvls) => lvls,
        Err(cycle) => return Err(ExecutionError::CycleDetected(cycle)),
    };

    // Compute critical path (longest path through DAG by effort)
    let critical_path = compute_critical_path(&module_map, &execution_order);

    // Mark modules on critical path
    let mut updated_modules = module_map.clone();
    for id in &critical_path {
        if let Some(module) = updated_modules.get_mut(id) {
            module.critical = true;
        }
    }

    // Calculate total estimated duration
    let estimated_duration_minutes = calculate_estimated_duration(&updated_modules, &levels);

    Ok(ExecutionPlan {
        modules: updated_modules,
        execution_order,
        levels,
        critical_path,
        estimated_duration_minutes,
        status: PlanStatus::Created,
    })
}

/// Compute the critical path through the DAG based on effort.
///
/// Uses forward/backward pass algorithm to find the longest path.
fn compute_critical_path(
    modules: &HashMap<String, ExecutionModule>,
    execution_order: &[String],
) -> Vec<String> {
    if execution_order.is_empty() {
        return Vec::new();
    }

    // Forward pass: compute earliest start time for each module
    let mut earliest_finish: HashMap<String, u32> = HashMap::new();
    let mut predecessors: HashMap<String, Option<String>> = HashMap::new();

    for id in execution_order {
        let module = &modules[id];
        let effort = module.effort.to_minutes();

        // Find max earliest finish of dependencies
        let mut max_dep_finish = 0u32;
        let mut best_pred: Option<String> = None;

        for dep in &module.dependencies {
            if let Some(&dep_finish) = earliest_finish.get(dep) {
                if dep_finish > max_dep_finish {
                    max_dep_finish = dep_finish;
                    best_pred = Some(dep.clone());
                }
            }
        }

        earliest_finish.insert(id.clone(), max_dep_finish + effort);
        predecessors.insert(id.clone(), best_pred);
    }

    // Find the module with the maximum earliest finish (end of critical path)
    let mut max_finish = 0u32;
    let mut end_module: Option<String> = None;

    for (id, &finish) in &earliest_finish {
        if finish > max_finish {
            max_finish = finish;
            end_module = Some(id.clone());
        }
    }

    // Backtrack to build critical path
    let mut critical_path = Vec::new();
    let mut current = end_module;

    while let Some(id) = current {
        critical_path.push(id.clone());
        current = predecessors.get(&id).and_then(|p| p.clone());
    }

    critical_path.reverse();
    critical_path
}

/// Calculate estimated duration considering parallelization.
///
/// The duration is the sum of the maximum effort at each level.
fn calculate_estimated_duration(
    modules: &HashMap<String, ExecutionModule>,
    levels: &[Vec<String>],
) -> u32 {
    levels.iter().map(|level| {
        level.iter()
            .filter_map(|id| modules.get(id))
            .map(|m| m.effort.to_minutes())
            .max()
            .unwrap_or(0)
    }).sum()
}

/// Get the next module to execute from a plan.
///
/// Returns the first pending module in execution order that has all
/// dependencies completed.
pub fn get_next_module(plan: &ExecutionPlan) -> Option<&ExecutionModule> {
    for id in &plan.execution_order {
        if let Some(module) = plan.modules.get(id) {
            if module.status == ModuleStatus::Pending {
                // Check if all dependencies are completed
                let deps_complete = module.dependencies.iter().all(|dep_id| {
                    plan.modules.get(dep_id)
                        .map(|dep| dep.status == ModuleStatus::Completed)
                        .unwrap_or(false)
                });
                if deps_complete {
                    return Some(module);
                }
            }
        }
    }
    None
}

/// Get all modules ready to execute in parallel.
///
/// Returns all pending modules whose dependencies are all completed.
pub fn get_ready_modules(plan: &ExecutionPlan) -> Vec<&ExecutionModule> {
    plan.execution_order.iter()
        .filter_map(|id| plan.modules.get(id))
        .filter(|module| {
            module.status == ModuleStatus::Pending &&
            module.dependencies.iter().all(|dep_id| {
                plan.modules.get(dep_id)
                    .map(|dep| dep.status == ModuleStatus::Completed)
                    .unwrap_or(false)
            })
        })
        .collect()
}

/// Mark a module as completed and return the result.
pub fn complete_module(
    plan: &mut ExecutionPlan,
    module_id: &str,
    signal: AndonSignal,
    message: &str,
    duration_ms: u64,
) -> Result<ModuleResult, ExecutionError> {
    let module = plan.modules.get_mut(module_id)
        .ok_or_else(|| ExecutionError::ModuleNotFound(module_id.to_string()))?;

    module.status = match signal {
        AndonSignal::Green | AndonSignal::Yellow => ModuleStatus::Completed,
        AndonSignal::Red => ModuleStatus::Failed(message.to_string()),
        _ => module.status.clone(),
    };

    Ok(ModuleResult {
        module_id: module_id.to_string(),
        signal,
        message: message.to_string(),
        duration_ms,
        warnings: Vec::new(),
        artifacts: Vec::new(),
    })
}

/// Check if the plan is complete (all modules completed or failed).
pub fn is_plan_complete(plan: &ExecutionPlan) -> bool {
    plan.modules.values().all(|m| {
        matches!(m.status, ModuleStatus::Completed | ModuleStatus::Failed(_) | ModuleStatus::Skipped)
    })
}

/// Detect resource conflicts between modules at the same level.
///
/// Returns a list of conflicts if any modules in the same level touch the same resources.
pub fn detect_resource_conflicts(plan: &ExecutionPlan) -> Vec<ExecutionError> {
    let mut conflicts = Vec::new();

    for level in &plan.levels {
        let mut resource_owners: HashMap<&String, &String> = HashMap::new();
        
        for module_id in level {
            if let Some(module) = plan.modules.get(module_id) {
                for resource in &module.resources {
                    if let Some(other_module_id) = resource_owners.get(resource) {
                        conflicts.push(ExecutionError::ResourceConflict {
                            module_a: (*other_module_id).clone(),
                            module_b: module.id.clone(),
                            resource: resource.clone(),
                        });
                    } else {
                        resource_owners.insert(resource, &module.id);
                    }
                }
            }
        }
    }

    conflicts
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ───────────────────────────────────────────────────────────────────────
    // POSITIVE TESTS
    // ───────────────────────────────────────────────────────────────────────

    #[test]
    fn test_build_plan_empty() {
        let plan = build_execution_plan(vec![]).unwrap();
        assert!(plan.modules.is_empty());
        assert!(plan.levels.is_empty());
        assert_eq!(plan.estimated_duration_minutes, 0);
    }

    #[test]
    fn test_build_plan_single_module() {
        let modules = vec![
            ExecutionModule::new("M1", "Single task", vec![]),
        ];
        let plan = build_execution_plan(modules).unwrap();

        assert_eq!(plan.modules.len(), 1);
        assert_eq!(plan.execution_order, vec!["M1"]);
        assert_eq!(plan.levels.len(), 1);
        assert_eq!(plan.levels[0], vec!["M1"]);
    }

    #[test]
    fn test_build_plan_linear_chain() {
        let modules = vec![
            ExecutionModule::new("M1", "First", vec![]),
            ExecutionModule::new("M2", "Second", vec!["M1".to_string()]),
            ExecutionModule::new("M3", "Third", vec!["M2".to_string()]),
        ];
        let plan = build_execution_plan(modules).unwrap();

        assert_eq!(plan.levels.len(), 3);
        assert_eq!(plan.levels[0], vec!["M1"]);
        assert_eq!(plan.levels[1], vec!["M2"]);
        assert_eq!(plan.levels[2], vec!["M3"]);

        // Critical path should be all modules
        assert_eq!(plan.critical_path.len(), 3);
    }

    #[test]
    fn test_build_plan_diamond() {
        // M1 -> M2, M3 -> M4
        let modules = vec![
            ExecutionModule::new("M1", "Root", vec![]),
            ExecutionModule::new("M2", "Branch A", vec!["M1".to_string()]),
            ExecutionModule::new("M3", "Branch B", vec!["M1".to_string()]),
            ExecutionModule::new("M4", "Merge", vec!["M2".to_string(), "M3".to_string()]),
        ];
        let plan = build_execution_plan(modules).unwrap();

        assert_eq!(plan.levels.len(), 3);
        assert_eq!(plan.levels[0], vec!["M1"]);
        assert!(plan.levels[1].contains(&"M2".to_string()));
        assert!(plan.levels[1].contains(&"M3".to_string()));
        assert_eq!(plan.levels[2], vec!["M4"]);
    }

    #[test]
    fn test_build_plan_parallel_roots() {
        let modules = vec![
            ExecutionModule::new("M1", "Root A", vec![]),
            ExecutionModule::new("M2", "Root B", vec![]),
            ExecutionModule::new("M3", "Merge", vec!["M1".to_string(), "M2".to_string()]),
        ];
        let plan = build_execution_plan(modules).unwrap();

        assert_eq!(plan.levels.len(), 2);
        assert_eq!(plan.levels[0].len(), 2); // M1 and M2 parallel
        assert_eq!(plan.levels[1], vec!["M3"]);
    }

    #[test]
    fn test_get_next_module() {
        let modules = vec![
            ExecutionModule::new("M1", "First", vec![]),
            ExecutionModule::new("M2", "Second", vec!["M1".to_string()]),
        ];
        let plan = build_execution_plan(modules).unwrap();

        let next = get_next_module(&plan);
        assert!(next.is_some());
        assert_eq!(next.unwrap().id, "M1");
    }

    #[test]
    fn test_get_ready_modules_parallel() {
        let modules = vec![
            ExecutionModule::new("M1", "Root A", vec![]),
            ExecutionModule::new("M2", "Root B", vec![]),
            ExecutionModule::new("M3", "Depends on both", vec!["M1".to_string(), "M2".to_string()]),
        ];
        let plan = build_execution_plan(modules).unwrap();

        let ready = get_ready_modules(&plan);
        assert_eq!(ready.len(), 2); // M1 and M2 are both ready
    }

    #[test]
    fn test_effort_size_to_minutes() {
        assert_eq!(EffortSize::S.to_minutes(), 15);
        assert_eq!(EffortSize::M.to_minutes(), 60);
        assert_eq!(EffortSize::L.to_minutes(), 240);
        assert_eq!(EffortSize::XL.to_minutes(), 480);
    }

    // ───────────────────────────────────────────────────────────────────────
    // NEGATIVE TESTS
    // ───────────────────────────────────────────────────────────────────────

    #[test]
    fn test_build_plan_cycle_detected() {
        let modules = vec![
            ExecutionModule::new("M1", "A", vec!["M2".to_string()]),
            ExecutionModule::new("M2", "B", vec!["M1".to_string()]),
        ];
        let result = build_execution_plan(modules);

        assert!(result.is_err());
        match result.unwrap_err() {
            ExecutionError::CycleDetected(_) => {}
            _ => panic!("Expected CycleDetected error"),
        }
    }

    #[test]
    fn test_build_plan_missing_dependency() {
        let modules = vec![
            ExecutionModule::new("M1", "Depends on missing", vec!["MISSING".to_string()]),
        ];
        let result = build_execution_plan(modules);

        assert!(result.is_err());
        match result.unwrap_err() {
            ExecutionError::ModuleNotFound(id) => assert_eq!(id, "MISSING"),
            _ => panic!("Expected ModuleNotFound error"),
        }
    }

    #[test]
    fn test_build_plan_duplicate_id() {
        let modules = vec![
            ExecutionModule::new("M1", "First", vec![]),
            ExecutionModule::new("M1", "Duplicate", vec![]),
        ];
        let result = build_execution_plan(modules);

        assert!(result.is_err());
        match result.unwrap_err() {
            ExecutionError::InvalidModule(msg) => assert!(msg.contains("Duplicate")),
            _ => panic!("Expected InvalidModule error"),
        }
    }

    // ───────────────────────────────────────────────────────────────────────
    // EDGE CASES
    // ───────────────────────────────────────────────────────────────────────

    #[test]
    fn test_risk_clamping() {
        let module = ExecutionModule::new("M1", "Test", vec![])
            .with_risk(1.5); // Over max
        assert_eq!(module.risk, 1.0);

        let module2 = ExecutionModule::new("M2", "Test", vec![])
            .with_risk(-0.5); // Under min
        assert_eq!(module2.risk, 0.0);
    }

    #[test]
    fn test_resource_conflict_detection() {
        let modules = vec![
            ExecutionModule::new("M1", "A", vec![])
                .with_resources(vec!["file.rs".to_string()]),
            ExecutionModule::new("M2", "B", vec![])
                .with_resources(vec!["file.rs".to_string()]),
        ];
        // Both at level 0 (no deps) and touch same file

        let plan = build_execution_plan(modules).unwrap();
        let conflicts = detect_resource_conflicts(&plan);

        assert_eq!(conflicts.len(), 1);
    }

    // ───────────────────────────────────────────────────────────────────────
    // STRESS TESTS
    // ───────────────────────────────────────────────────────────────────────

    #[test]
    fn test_build_plan_100_modules_chain() {
        let modules: Vec<ExecutionModule> = (0..100)
            .map(|i| {
                let deps = if i > 0 {
                    vec![format!("M{}", i - 1)]
                } else {
                    vec![]
                };
                ExecutionModule::new(&format!("M{}", i), &format!("Module {}", i), deps)
            })
            .collect();

        let plan = build_execution_plan(modules).unwrap();
        assert_eq!(plan.levels.len(), 100);
        assert_eq!(plan.execution_order.len(), 100);
    }

    #[test]
    fn test_build_plan_100_modules_parallel() {
        let modules: Vec<ExecutionModule> = (0..100)
            .map(|i| {
                ExecutionModule::new(&format!("M{}", i), &format!("Module {}", i), vec![])
            })
            .collect();

        let plan = build_execution_plan(modules).unwrap();
        assert_eq!(plan.levels.len(), 1); // All parallel
        assert_eq!(plan.levels[0].len(), 100);
    }
}
