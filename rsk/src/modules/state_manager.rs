//! # State Manager
//!
//! Execution state persistence and checkpoint management.
//!
//! ## Features
//! - Create execution contexts with step tracking
//! - Save/load checkpoints to JSON files
//! - Find resumable executions
//! - Mark steps complete/failed/skipped
//! - Cleanup old checkpoints
//!
//! ## Storage
//! - Location: `~/.claude/chain-state/`
//! - Format: JSON (one file per context)
//! - Naming: `{name}-{timestamp}.json`
//!
//! ## Performance Targets
//! - Save checkpoint: < 5ms
//! - Load checkpoint: < 2ms
//! - List checkpoints: < 50ms for 1000 files
//!
//! ## Example
//! ```rust,ignore
//! use rsk::modules::state_manager::{CheckpointManager, ExecutionContext};
//!
//! let manager = CheckpointManager::new("/home/user/.claude/chain-state")?;
//!
//! // Create a new execution context
//! let mut ctx = manager.create_context("my-pipeline", 5);
//!
//! // Mark steps as complete
//! manager.mark_step_complete(&ctx.id, 0, json!({"result": "success"}))?;
//!
//! // Save checkpoint
//! manager.save(&ctx)?;
//!
//! // Later: resume from checkpoint
//! if let Some(ctx) = manager.find_resumable("my-pipeline")? {
//!     println!("Resuming from step {}", ctx.completed_steps.len());
//! }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Execution status for a context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Created but not started
    Created,
    /// Currently running
    Running,
    /// Paused (can resume)
    Paused,
    /// All steps completed
    Completed,
    /// Failed with error
    Failed(String),
    /// Cancelled by user
    Cancelled,
}

impl Default for ExecutionStatus {
    fn default() -> Self {
        Self::Created
    }
}

/// Result of a single step execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step index
    pub step: usize,
    /// Whether the step succeeded
    pub success: bool,
    /// Result message
    pub message: String,
    /// Output data (arbitrary JSON)
    pub output: Value,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Timestamp when step completed
    pub completed_at: DateTime<Utc>,
}

/// An execution context representing a pipeline run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Unique identifier (UUID)
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Current status
    pub status: ExecutionStatus,
    /// Total number of steps
    pub total_steps: usize,
    /// Indices of completed steps
    pub completed_steps: Vec<usize>,
    /// Indices of failed steps
    pub failed_steps: Vec<usize>,
    /// Indices of skipped steps
    pub skipped_steps: Vec<usize>,
    /// Results for each step
    pub step_results: HashMap<String, StepResult>,
    /// When execution started
    pub started_at: DateTime<Utc>,
    /// When last updated
    pub updated_at: DateTime<Utc>,
    /// Arbitrary metadata/artifacts
    pub artifacts: HashMap<String, Value>,
    /// Parent context ID (for nested executions)
    pub parent_id: Option<String>,
    /// Tags for filtering
    pub tags: Vec<String>,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(name: &str, total_steps: usize) -> Self {
        let now = Utc::now();
        Self {
            id: generate_id(),
            name: name.to_string(),
            status: ExecutionStatus::Created,
            total_steps,
            completed_steps: Vec::new(),
            failed_steps: Vec::new(),
            skipped_steps: Vec::new(),
            step_results: HashMap::new(),
            started_at: now,
            updated_at: now,
            artifacts: HashMap::new(),
            parent_id: None,
            tags: Vec::new(),
        }
    }

    /// Get the next step to execute (first non-completed, non-failed, non-skipped)
    pub fn next_step(&self) -> Option<usize> {
        for i in 0..self.total_steps {
            if !self.completed_steps.contains(&i)
                && !self.failed_steps.contains(&i)
                && !self.skipped_steps.contains(&i)
            {
                return Some(i);
            }
        }
        None
    }

    /// Check if execution is complete (all steps processed)
    pub fn is_complete(&self) -> bool {
        self.completed_steps.len() + self.failed_steps.len() + self.skipped_steps.len()
            >= self.total_steps
    }

    /// Calculate progress percentage
    pub fn progress_percent(&self) -> f32 {
        if self.total_steps == 0 {
            return 100.0;
        }
        (self.completed_steps.len() as f32 / self.total_steps as f32) * 100.0
    }

    /// Mark a step as started
    pub fn start_step(&mut self, _step: usize) {
        self.status = ExecutionStatus::Running;
        self.updated_at = Utc::now();
    }

    /// Mark a step as completed
    pub fn complete_step(&mut self, step: usize, result: StepResult) {
        if !self.completed_steps.contains(&step) {
            self.completed_steps.push(step);
        }
        self.step_results.insert(step.to_string(), result);
        self.updated_at = Utc::now();

        if self.is_complete() {
            self.status = if self.failed_steps.is_empty() {
                ExecutionStatus::Completed
            } else {
                ExecutionStatus::Failed("One or more steps failed".to_string())
            };
        }
    }

    /// Mark a step as failed
    pub fn fail_step(&mut self, step: usize, _error: &str, result: StepResult) {
        if !self.failed_steps.contains(&step) {
            self.failed_steps.push(step);
        }
        self.step_results.insert(step.to_string(), result);
        self.updated_at = Utc::now();

        if self.is_complete() {
            self.status = ExecutionStatus::Failed("One or more steps failed".to_string());
        }
    }

    /// Mark a step as skipped
    pub fn skip_step(&mut self, step: usize, reason: &str) {
        if !self.skipped_steps.contains(&step) {
            self.skipped_steps.push(step);
        }
        self.step_results.insert(
            step.to_string(),
            StepResult {
                step,
                success: false,
                message: format!("Skipped: {}", reason),
                output: Value::Null,
                duration_ms: 0,
                completed_at: Utc::now(),
            },
        );
        self.updated_at = Utc::now();
    }

    /// Add an artifact
    pub fn add_artifact(&mut self, key: &str, value: Value) {
        self.artifacts.insert(key.to_string(), value);
        self.updated_at = Utc::now();
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: &str) {
        if !self.tags.contains(&tag.to_string()) {
            self.tags.push(tag.to_string());
        }
    }
}

/// Checkpoint manager for persisting execution state
#[derive(Debug, Clone)]
pub struct CheckpointManager {
    /// Directory for checkpoint files
    state_dir: PathBuf,
    /// Cache of ID to filename for O(1) lookups
    id_map: HashMap<String, PathBuf>,
}

/// Error types for state management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateError {
    /// IO error
    IoError(String),
    /// Serialization error
    SerializeError(String),
    /// Deserialization error
    DeserializeError(String),
    /// Context not found
    NotFound(String),
    /// Invalid state
    InvalidState(String),
    /// Permission denied
    PermissionDenied(String),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(s) => write!(f, "IO error: {}", s),
            Self::SerializeError(s) => write!(f, "Serialization error: {}", s),
            Self::DeserializeError(s) => write!(f, "Deserialization error: {}", s),
            Self::NotFound(s) => write!(f, "Context not found: {}", s),
            Self::InvalidState(s) => write!(f, "Invalid state: {}", s),
            Self::PermissionDenied(s) => write!(f, "Permission denied: {}", s),
        }
    }
}

impl std::error::Error for StateError {}

// ═══════════════════════════════════════════════════════════════════════════
// CHECKPOINT MANAGER IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════════════════

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new(state_dir: &str) -> Result<Self, StateError> {
        let path = PathBuf::from(state_dir);

        // Verify path if it exists
        if path.exists() {
            if !path.is_dir() {
                return Err(StateError::InvalidState(format!(
                    "Path exists but is not a directory: {}",
                    state_dir
                )));
            }
            // Check write permissions by attempting to create a sentinel file if possible or just metadata check
            let metadata =
                std::fs::metadata(&path).map_err(|e| StateError::IoError(e.to_string()))?;
            if metadata.permissions().readonly() {
                return Err(StateError::PermissionDenied(format!(
                    "Directory is read-only: {}",
                    state_dir
                )));
            }
        } else {
            // Create directory if it doesn't exist
            std::fs::create_dir_all(&path).map_err(|e| StateError::IoError(e.to_string()))?;
        }

        let mut manager = Self {
            state_dir: path,
            id_map: HashMap::new(),
        };

        // Initial scan to build the ID map
        manager.refresh_id_map()?;

        Ok(manager)
    }

    /// Refresh the internal ID to path mapping
    fn refresh_id_map(&mut self) -> Result<(), StateError> {
        self.id_map.clear();
        let entries =
            std::fs::read_dir(&self.state_dir).map_err(|e| StateError::IoError(e.to_string()))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    // Our deterministic naming: [id].json
                    // If it follows the old naming [name]-[id].json, we try to extract the ID
                    if let Some(id) = stem.split('-').last() {
                        self.id_map.insert(id.to_string(), path.clone());
                    } else {
                        self.id_map.insert(stem.to_string(), path.clone());
                    }
                }
            }
        }
        Ok(())
    }

    /// Create a new execution context
    pub fn create_context(&self, name: &str, total_steps: usize) -> ExecutionContext {
        ExecutionContext::new(name, total_steps)
    }

    /// Save a context to disk
    pub fn save(&mut self, context: &ExecutionContext) -> Result<PathBuf, StateError> {
        // Use deterministic naming: [id].json for O(1) lookups
        let filename = format!("{}.json", context.id);
        let path = self.state_dir.join(&filename);

        let json = serde_json::to_string_pretty(context)
            .map_err(|e| StateError::SerializeError(e.to_string()))?;

        std::fs::write(&path, json).map_err(|e| StateError::IoError(e.to_string()))?;

        // Update cache
        self.id_map.insert(context.id.clone(), path.clone());

        Ok(path)
    }

    /// Load a context by ID (O(1) lookup via cache)
    pub fn load(&self, id: &str) -> Result<Option<ExecutionContext>, StateError> {
        // Use the cache for O(1) lookup
        if let Some(path) = self.id_map.get(id) {
            if path.exists() {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| StateError::IoError(e.to_string()))?;

                let context: ExecutionContext = serde_json::from_str(&content)
                    .map_err(|e| StateError::DeserializeError(e.to_string()))?;

                return Ok(Some(context));
            }
        }

        // Fallback for safety (though refresh_id_map should handle it)
        Ok(None)
    }

    /// Find a resumable context by name
    pub fn find_resumable(&self, name: &str) -> Result<Option<ExecutionContext>, StateError> {
        let contexts = self.list_by_name(name)?;

        // Find the most recent non-completed context
        let resumable = contexts
            .into_iter()
            .filter(|ctx| {
                matches!(
                    ctx.status,
                    ExecutionStatus::Created | ExecutionStatus::Running | ExecutionStatus::Paused
                )
            })
            .max_by_key(|ctx| ctx.updated_at);

        Ok(resumable)
    }

    /// List all contexts
    pub fn list(&self) -> Result<Vec<ExecutionContext>, StateError> {
        let mut contexts = Vec::new();

        for path in self.id_map.values() {
            if path.exists() {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| StateError::IoError(e.to_string()))?;

                if let Ok(context) = serde_json::from_str::<ExecutionContext>(&content) {
                    contexts.push(context);
                }
            }
        }

        // Sort by updated_at descending
        contexts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(contexts)
    }

    /// List contexts by name
    pub fn list_by_name(&self, name: &str) -> Result<Vec<ExecutionContext>, StateError> {
        let all = self.list()?;
        Ok(all.into_iter().filter(|ctx| ctx.name == name).collect())
    }

    /// List contexts by status
    pub fn list_by_status(
        &self,
        status: &ExecutionStatus,
    ) -> Result<Vec<ExecutionContext>, StateError> {
        let all = self.list()?;
        Ok(all
            .into_iter()
            .filter(|ctx| &ctx.status == status)
            .collect())
    }

    /// Delete a context by ID
    pub fn delete(&mut self, id: &str) -> Result<bool, StateError> {
        if let Some(path) = self.id_map.remove(id) {
            if path.exists() {
                std::fs::remove_file(&path).map_err(|e| StateError::IoError(e.to_string()))?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Cleanup old checkpoints
    ///
    /// Removes checkpoints older than `max_age_days` that are completed, cancelled, or failed.
    pub fn cleanup(&mut self, max_age_days: u32) -> Result<usize, StateError> {
        let mut removed = 0;
        let cutoff = Utc::now() - chrono::Duration::days(max_age_days as i64);

        let contexts = self.list()?;

        for ctx in contexts {
            if ctx.updated_at < cutoff {
                match ctx.status {
                    ExecutionStatus::Completed
                    | ExecutionStatus::Cancelled
                    | ExecutionStatus::Failed(_) => {
                        if self.delete(&ctx.id)? {
                            removed += 1;
                        }
                    }
                    _ => {} // Don't delete in-progress
                }
            }
        }

        Ok(removed)
    }

    /// Get summary statistics
    pub fn stats(&self) -> Result<CheckpointStats, StateError> {
        let contexts = self.list()?;

        let mut stats = CheckpointStats::default();
        stats.total = contexts.len();

        for ctx in contexts {
            match ctx.status {
                ExecutionStatus::Created => stats.created += 1,
                ExecutionStatus::Running => stats.running += 1,
                ExecutionStatus::Paused => stats.paused += 1,
                ExecutionStatus::Completed => stats.completed += 1,
                ExecutionStatus::Failed(_) => stats.failed += 1,
                ExecutionStatus::Cancelled => stats.cancelled += 1,
            }
        }

        Ok(stats)
    }
}

/// Statistics about checkpoints
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointStats {
    pub total: usize,
    pub created: usize,
    pub running: usize,
    pub paused: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Generate a unique ID (UUID v4)
fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Quick function to mark a step complete without the full manager
pub fn mark_step_complete(
    context: &mut ExecutionContext,
    step: usize,
    output: Value,
    duration_ms: u64,
) {
    let result = StepResult {
        step,
        success: true,
        message: "Completed".to_string(),
        output,
        duration_ms,
        completed_at: Utc::now(),
    };
    context.complete_step(step, result);
}

/// Quick function to mark a step failed
pub fn mark_step_failed(
    context: &mut ExecutionContext,
    step: usize,
    error: &str,
    duration_ms: u64,
) {
    let result = StepResult {
        step,
        success: false,
        message: error.to_string(),
        output: Value::Null,
        duration_ms,
        completed_at: Utc::now(),
    };
    context.fail_step(step, error, result);
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (CheckpointManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = CheckpointManager::new(temp_dir.path().to_str().unwrap()).unwrap();
        (manager, temp_dir)
    }

    // ───────────────────────────────────────────────────────────────────────
    // EXECUTION CONTEXT TESTS
    // ───────────────────────────────────────────────────────────────────────

    #[test]
    fn test_context_creation() {
        let ctx = ExecutionContext::new("test-pipeline", 5);

        assert_eq!(ctx.name, "test-pipeline");
        assert_eq!(ctx.total_steps, 5);
        assert_eq!(ctx.status, ExecutionStatus::Created);
        assert!(ctx.completed_steps.is_empty());
        assert!(ctx.failed_steps.is_empty());
    }

    #[test]
    fn test_context_progress() {
        let mut ctx = ExecutionContext::new("test", 4);

        assert_eq!(ctx.progress_percent(), 0.0);
        assert_eq!(ctx.next_step(), Some(0));

        mark_step_complete(&mut ctx, 0, Value::Null, 100);
        assert_eq!(ctx.progress_percent(), 25.0);
        assert_eq!(ctx.next_step(), Some(1));

        mark_step_complete(&mut ctx, 1, Value::Null, 100);
        assert_eq!(ctx.progress_percent(), 50.0);
    }

    #[test]
    fn test_context_completion() {
        let mut ctx = ExecutionContext::new("test", 2);

        assert!(!ctx.is_complete());

        mark_step_complete(&mut ctx, 0, Value::Null, 100);
        assert!(!ctx.is_complete());

        mark_step_complete(&mut ctx, 1, Value::Null, 100);
        assert!(ctx.is_complete());
        assert_eq!(ctx.status, ExecutionStatus::Completed);
    }

    #[test]
    fn test_context_with_failures() {
        let mut ctx = ExecutionContext::new("test", 2);

        mark_step_complete(&mut ctx, 0, Value::Null, 100);
        mark_step_failed(&mut ctx, 1, "Something went wrong", 50);

        assert!(ctx.is_complete());
        assert!(matches!(ctx.status, ExecutionStatus::Failed(_)));
    }

    #[test]
    fn test_skip_step() {
        let mut ctx = ExecutionContext::new("test", 3);

        mark_step_complete(&mut ctx, 0, Value::Null, 100);
        ctx.skip_step(1, "Dependency failed");
        mark_step_complete(&mut ctx, 2, Value::Null, 100);

        assert!(ctx.is_complete());
        assert!(ctx.skipped_steps.contains(&1));
    }

    // ───────────────────────────────────────────────────────────────────────
    // CHECKPOINT MANAGER TESTS
    // ───────────────────────────────────────────────────────────────────────

    #[test]
    fn test_save_and_load() {
        let (mut manager, _temp) = create_test_manager();

        let ctx = manager.create_context("test-pipeline", 3);
        let id = ctx.id.clone();

        manager.save(&ctx).unwrap();

        let loaded = manager.load(&id).unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.name, "test-pipeline");
        assert_eq!(loaded.total_steps, 3);
    }

    #[test]
    fn test_find_resumable() {
        let (mut manager, _temp) = create_test_manager();

        // Create a completed context
        let mut ctx1 = manager.create_context("my-pipeline", 2);
        mark_step_complete(&mut ctx1, 0, Value::Null, 100);
        mark_step_complete(&mut ctx1, 1, Value::Null, 100);
        manager.save(&ctx1).unwrap();

        // Create an in-progress context
        let mut ctx2 = manager.create_context("my-pipeline", 3);
        mark_step_complete(&mut ctx2, 0, Value::Null, 100);
        ctx2.status = ExecutionStatus::Paused;
        manager.save(&ctx2).unwrap();

        // Should find the paused one
        let resumable = manager.find_resumable("my-pipeline").unwrap();
        assert!(resumable.is_some());
        assert_eq!(resumable.unwrap().id, ctx2.id);
    }

    #[test]
    fn test_list_contexts() {
        let (mut manager, _temp) = create_test_manager();

        manager
            .save(&manager.create_context("pipeline-a", 2))
            .unwrap();
        manager
            .save(&manager.create_context("pipeline-b", 3))
            .unwrap();
        manager
            .save(&manager.create_context("pipeline-a", 4))
            .unwrap();

        let all = manager.list().unwrap();
        assert_eq!(all.len(), 3);

        let by_name = manager.list_by_name("pipeline-a").unwrap();
        assert_eq!(by_name.len(), 2);
    }

    #[test]
    fn test_delete_context() {
        let (mut manager, _temp) = create_test_manager();

        let ctx = manager.create_context("to-delete", 1);
        let id = ctx.id.clone();
        manager.save(&ctx).unwrap();

        assert!(manager.load(&id).unwrap().is_some());

        let deleted = manager.delete(&id).unwrap();
        assert!(deleted);

        assert!(manager.load(&id).unwrap().is_none());
    }

    #[test]
    fn test_stats() {
        let (mut manager, _temp) = create_test_manager();

        let mut ctx1 = manager.create_context("test", 1);
        mark_step_complete(&mut ctx1, 0, Value::Null, 100);
        manager.save(&ctx1).unwrap();

        let ctx2 = manager.create_context("test", 1);
        manager.save(&ctx2).unwrap();

        let stats = manager.stats().unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.created, 1);
    }

    // ───────────────────────────────────────────────────────────────────────
    // EDGE CASES
    // ───────────────────────────────────────────────────────────────────────

    #[test]
    fn test_empty_pipeline() {
        let ctx = ExecutionContext::new("empty", 0);
        assert!(ctx.is_complete());
        assert_eq!(ctx.progress_percent(), 100.0);
        assert_eq!(ctx.next_step(), None);
    }

    #[test]
    fn test_load_nonexistent() {
        let (mut manager, _temp) = create_test_manager();
        let result = manager.load("nonexistent-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_artifacts() {
        let mut ctx = ExecutionContext::new("test", 1);
        ctx.add_artifact("key1", serde_json::json!({"value": 42}));
        ctx.add_tag("important");

        assert!(ctx.artifacts.contains_key("key1"));
        assert!(ctx.tags.contains(&"important".to_string()));
    }
}
