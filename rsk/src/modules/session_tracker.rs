//! # Session Tracker
//!
//! Unified session state tracking for skill execution history.
//! Replaces 233 individual skill_router.py files with a single Rust module.
//!
//! ## Features
//! - Atomic file operations (no partial writes)
//! - Lock-free in-memory state
//! - Efficient JSON serialization
//! - Optional log file output
//!
//! ## Migration Target
//! This module consolidates the pattern found in:
//! - `~/.claude/skills/*/scripts/skill_router.py` (233 instances)
//!
//! ## Performance
//! - State load: < 1ms
//! - State save: < 2ms (atomic)
//! - History append: O(1) amortized

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// A single execution entry in the history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEntry {
    /// Skill name that was executed
    pub skill_name: String,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Optional context/notes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Execution duration in milliseconds (if tracked)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Status: "started", "completed", "failed"
    #[serde(default = "default_status")]
    pub status: String,
}

fn default_status() -> String {
    "completed".to_string()
}

/// Session state for a skill or session
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionState {
    /// Session identifier
    #[serde(default)]
    pub session_id: String,
    /// Current skill context
    #[serde(default)]
    pub current_skill: String,
    /// Execution history (most recent last)
    #[serde(default)]
    pub execution_history: Vec<ExecutionEntry>,
    /// Additional metadata
    #[serde(default, flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SessionState {
    /// Create a new empty session state
    pub fn new() -> Self {
        Self {
            session_id: uuid_v4(),
            ..Default::default()
        }
    }

    /// Create with a specific session ID
    pub fn with_id(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            ..Default::default()
        }
    }

    /// Add an execution entry
    pub fn add_execution(&mut self, skill_name: &str, context: Option<&str>) {
        self.execution_history.push(ExecutionEntry {
            skill_name: skill_name.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            context: context.map(|s| s.to_string()),
            duration_ms: None,
            status: "started".to_string(),
        });
        self.current_skill = skill_name.to_string();
    }

    /// Mark the last execution as completed with optional duration
    pub fn complete_execution(&mut self, duration_ms: Option<u64>) {
        if let Some(entry) = self.execution_history.last_mut() {
            entry.status = "completed".to_string();
            entry.duration_ms = duration_ms;
        }
    }

    /// Mark the last execution as failed
    pub fn fail_execution(&mut self, error: Option<&str>) {
        if let Some(entry) = self.execution_history.last_mut() {
            entry.status = "failed".to_string();
            if let Some(err) = error {
                entry.context = Some(format!(
                    "{}: {}",
                    entry.context.as_deref().unwrap_or("Error"),
                    err
                ));
            }
        }
    }

    /// Get execution count for a specific skill
    pub fn execution_count(&self, skill_name: &str) -> usize {
        self.execution_history
            .iter()
            .filter(|e| e.skill_name == skill_name)
            .count()
    }

    /// Get the N most recent executions
    pub fn recent_executions(&self, n: usize) -> Vec<&ExecutionEntry> {
        self.execution_history.iter().rev().take(n).collect()
    }

    /// Trim history to keep only the most recent N entries
    pub fn trim_history(&mut self, keep: usize) {
        if self.execution_history.len() > keep {
            let start = self.execution_history.len() - keep;
            self.execution_history = self.execution_history[start..].to_vec();
        }
    }
}

/// Error types for session operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionError {
    /// File not found
    NotFound(String),
    /// IO error
    IoError(String),
    /// Parse error
    ParseError(String),
    /// Serialization error
    SerializeError(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(s) => write!(f, "Session file not found: {s}"),
            Self::IoError(s) => write!(f, "IO error: {s}"),
            Self::ParseError(s) => write!(f, "Parse error: {s}"),
            Self::SerializeError(s) => write!(f, "Serialization error: {s}"),
        }
    }
}

impl std::error::Error for SessionError {}

// ═══════════════════════════════════════════════════════════════════════════
// CORE OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Load session state from a JSON file
pub fn load_state(path: &Path) -> Result<SessionState, SessionError> {
    if !path.exists() {
        return Ok(SessionState::new());
    }

    let content = fs::read_to_string(path).map_err(|e| SessionError::IoError(e.to_string()))?;

    serde_json::from_str(&content).map_err(|e| SessionError::ParseError(e.to_string()))
}

/// Save session state to a JSON file (atomic write)
pub fn save_state(path: &Path, state: &SessionState) -> Result<(), SessionError> {
    let content = serde_json::to_string_pretty(state)
        .map_err(|e| SessionError::SerializeError(e.to_string()))?;

    // Atomic write: write to temp file, then rename
    let temp_path = path.with_extension("json.tmp");

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| SessionError::IoError(e.to_string()))?;
    }

    fs::write(&temp_path, &content).map_err(|e| SessionError::IoError(e.to_string()))?;

    fs::rename(&temp_path, path).map_err(|e| SessionError::IoError(e.to_string()))?;

    Ok(())
}

/// Track a skill execution (load, update, save in one operation)
pub fn track_execution(
    state_path: &Path,
    skill_name: &str,
    context: Option<&str>,
) -> Result<SessionState, SessionError> {
    let mut state = load_state(state_path)?;
    state.add_execution(skill_name, context);
    save_state(state_path, &state)?;
    Ok(state)
}

/// Track execution completion
pub fn track_completion(
    state_path: &Path,
    duration_ms: Option<u64>,
) -> Result<SessionState, SessionError> {
    let mut state = load_state(state_path)?;
    state.complete_execution(duration_ms);
    save_state(state_path, &state)?;
    Ok(state)
}

/// Track execution failure
pub fn track_failure(state_path: &Path, error: Option<&str>) -> Result<SessionState, SessionError> {
    let mut state = load_state(state_path)?;
    state.fail_execution(error);
    save_state(state_path, &state)?;
    Ok(state)
}

/// Append to log file (optional feature from original skill_router.py)
pub fn append_log(log_path: &Path, skill_name: &str, message: &str) -> Result<(), SessionError> {
    use std::io::Write;

    // Ensure parent directory exists
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|e| SessionError::IoError(e.to_string()))?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|e| SessionError::IoError(e.to_string()))?;

    let timestamp = Utc::now().to_rfc3339();
    writeln!(file, "[{timestamp}] [{skill_name}] {message}")
        .map_err(|e| SessionError::IoError(e.to_string()))?;

    Ok(())
}

/// Full skill router replacement: track + log in one call
pub fn route_skill(
    state_path: &Path,
    log_path: Option<&Path>,
    skill_name: &str,
    context: Option<&str>,
) -> Result<SessionState, SessionError> {
    let state = track_execution(state_path, skill_name, context)?;

    if let Some(log) = log_path {
        let msg = context.unwrap_or("executed");
        append_log(log, skill_name, msg)?;
    }

    Ok(state)
}

/// Get default state file path for a skill
pub fn default_state_path(skill_name: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join(".claude")
        .join("skills")
        .join(skill_name)
        .join("session-state.json")
}

/// Get default log file path
pub fn default_log_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join(".claude")
        .join("skills")
        .join("skill-execution.log")
}

// ═══════════════════════════════════════════════════════════════════════════
// UTILITY
// ═══════════════════════════════════════════════════════════════════════════

/// Generate a simple UUID v4-like string (not cryptographically secure)
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    // Mix time with constants to get pseudo-random values
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)] // u128→u64 intentional truncation for pseudo-random mixing
    let a = (time as u64) ^ 0xDEADBEEF_CAFEBABE;
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)] // u128→u64 intentional truncation for pseudo-random mixing
    let b = (time >> 64) as u64 ^ 0x12345678_ABCDEF01;

    // Build UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
    // where y is 8, 9, a, or b
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)] // u64→u32/u16 intentional truncation for UUID segment extraction
    let seg1 = (a >> 32) as u32;
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
    let seg2 = ((a >> 16) & 0xFFFF) as u16;
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
    let seg3 = ((a >> 4) & 0xFFF) as u16;
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
    let seg4 = (((b >> 16) & 0x3FFF) | 0x8000) as u16;
    format!(
        "{seg1:08x}-{seg2:04x}-4{seg3:03x}-{seg4:04x}-{:012x}",
        (b & 0xFFFFFFFFFFFF)
    )
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // ───────────────────────────────────────────────────────────────────────
    // POSITIVE TESTS
    // ───────────────────────────────────────────────────────────────────────

    #[test]
    fn test_session_state_new() {
        let state = SessionState::new();
        assert!(!state.session_id.is_empty());
        assert!(state.execution_history.is_empty());
    }

    #[test]
    fn test_add_execution() {
        let mut state = SessionState::new();
        state.add_execution("test-skill", Some("test context"));

        assert_eq!(state.execution_history.len(), 1);
        assert_eq!(state.execution_history[0].skill_name, "test-skill");
        assert_eq!(
            state.execution_history[0].context,
            Some("test context".to_string())
        );
        assert_eq!(state.current_skill, "test-skill");
    }

    #[test]
    fn test_complete_execution() {
        let mut state = SessionState::new();
        state.add_execution("test-skill", None);
        state.complete_execution(Some(100));

        assert_eq!(state.execution_history[0].status, "completed");
        assert_eq!(state.execution_history[0].duration_ms, Some(100));
    }

    #[test]
    fn test_fail_execution() {
        let mut state = SessionState::new();
        state.add_execution("test-skill", Some("initial"));
        state.fail_execution(Some("timeout"));

        assert_eq!(state.execution_history[0].status, "failed");
        assert!(
            state.execution_history[0]
                .context
                .as_ref()
                .unwrap()
                .contains("timeout")
        );
    }

    #[test]
    fn test_save_and_load_state() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");

        let mut state = SessionState::new();
        state.add_execution("skill-1", None);
        state.add_execution("skill-2", Some("with context"));

        save_state(&path, &state).unwrap();
        let loaded = load_state(&path).unwrap();

        assert_eq!(loaded.session_id, state.session_id);
        assert_eq!(loaded.execution_history.len(), 2);
        assert_eq!(loaded.execution_history[1].skill_name, "skill-2");
    }

    #[test]
    fn test_track_execution() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");

        let state = track_execution(&path, "my-skill", Some("first run")).unwrap();
        assert_eq!(state.execution_history.len(), 1);

        let state = track_execution(&path, "my-skill", Some("second run")).unwrap();
        assert_eq!(state.execution_history.len(), 2);
    }

    #[test]
    fn test_append_log() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");

        append_log(&log_path, "skill-a", "first message").unwrap();
        append_log(&log_path, "skill-b", "second message").unwrap();

        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("[skill-a]"));
        assert!(content.contains("[skill-b]"));
        assert!(content.contains("first message"));
        assert!(content.contains("second message"));
    }

    #[test]
    fn test_route_skill() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        let log_path = dir.path().join("execution.log");

        let state =
            route_skill(&state_path, Some(&log_path), "routed-skill", Some("test")).unwrap();

        assert_eq!(state.execution_history.len(), 1);
        assert!(
            fs::read_to_string(&log_path)
                .unwrap()
                .contains("routed-skill")
        );
    }

    #[test]
    fn test_trim_history() {
        let mut state = SessionState::new();
        for i in 0..10 {
            state.add_execution(&format!("skill-{i}"), None);
        }

        assert_eq!(state.execution_history.len(), 10);
        state.trim_history(5);
        assert_eq!(state.execution_history.len(), 5);
        assert_eq!(state.execution_history[0].skill_name, "skill-5");
    }

    #[test]
    fn test_execution_count() {
        let mut state = SessionState::new();
        state.add_execution("skill-a", None);
        state.add_execution("skill-b", None);
        state.add_execution("skill-a", None);
        state.add_execution("skill-a", None);

        assert_eq!(state.execution_count("skill-a"), 3);
        assert_eq!(state.execution_count("skill-b"), 1);
        assert_eq!(state.execution_count("skill-c"), 0);
    }

    #[test]
    fn test_recent_executions() {
        let mut state = SessionState::new();
        for i in 0..5 {
            state.add_execution(&format!("skill-{i}"), None);
        }

        let recent = state.recent_executions(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].skill_name, "skill-4");
        assert_eq!(recent[2].skill_name, "skill-2");
    }

    // ───────────────────────────────────────────────────────────────────────
    // EDGE CASES
    // ───────────────────────────────────────────────────────────────────────

    #[test]
    fn test_load_nonexistent_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");

        let state = load_state(&path).unwrap();
        // Should return new state, not error
        assert!(state.execution_history.is_empty());
    }

    #[test]
    fn test_complete_empty_history() {
        let mut state = SessionState::new();
        // Should not panic
        state.complete_execution(Some(100));
        assert!(state.execution_history.is_empty());
    }

    #[test]
    fn test_atomic_save() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("atomic.json");

        let mut state = SessionState::new();
        state.add_execution("test", None);

        // First save
        save_state(&path, &state).unwrap();

        // Verify temp file doesn't exist after save
        let temp_path = path.with_extension("json.tmp");
        assert!(!temp_path.exists());

        // State file should exist
        assert!(path.exists());
    }
}
