//! Telemetry Module - Unified tracing infrastructure
//!
//! Provides structured logging and observability for the Rust Skill Kernel.
//!
//! ## Features
//!
//! - Span-based execution tracing
//! - Structured JSON logging
//! - Performance timing utilities
//! - Event emission for metrics

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::Span;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

// ============================================================================
// Metrics for Autonomous Skill Runtime
// ============================================================================

/// Advanced metrics for Autonomous Skill Runtime performance
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AutonomyStats {
    pub total_nodes_executed: u64,
    pub deterministic_nodes: u64,
    pub llm_fallback_nodes: u64,
    pub avg_latency_ms: f64,
}

impl AutonomyStats {
    pub fn autonomy_ratio(&self) -> f64 {
        if self.total_nodes_executed == 0 {
            return 1.0;
        }
        self.deterministic_nodes as f64 / self.total_nodes_executed as f64
    }

    pub fn ips(&self, total_duration_secs: f64) -> f64 {
        if total_duration_secs == 0.0 {
            return 0.0;
        }
        self.total_nodes_executed as f64 / total_duration_secs
    }
}

// ============================================================================
// Configuration
// ============================================================================

/// Unified telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Output format (text, json, compact)
    pub format: String,
    /// Include timestamps
    pub timestamps: bool,
    /// Include file/line info
    pub file_line: bool,
    /// Include target module
    pub target: bool,
    /// Environment filter (e.g., "rsk=debug,warn")
    pub filter: Option<String>,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "text".to_string(),
            timestamps: true,
            file_line: false,
            target: true,
            filter: None,
        }
    }
}

impl TelemetryConfig {
    /// Create a JSON-focused configuration
    pub fn json() -> Self {
        Self {
            format: "json".to_string(),
            ..Default::default()
        }
    }

    /// Create a compact configuration for CI/CD
    pub fn compact() -> Self {
        Self {
            format: "compact".to_string(),
            timestamps: false,
            target: false,
            ..Default::default()
        }
    }

    /// Create a verbose debug configuration
    pub fn debug() -> Self {
        Self {
            level: "debug".to_string(),
            file_line: true,
            ..Default::default()
        }
    }
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize the telemetry system
pub fn init_telemetry(config: TelemetryConfig) -> anyhow::Result<()> {
    let filter = config
        .filter
        .unwrap_or_else(|| format!("rsk={}", config.level));

    let filter_layer = EnvFilter::try_new(&filter).unwrap_or_else(|_| EnvFilter::new("info"));

    match config.format.as_str() {
        "json" => {
            let fmt_layer = fmt::layer()
                .json()
                .with_file(config.file_line)
                .with_line_number(config.file_line)
                .with_target(config.target);

            tracing_subscriber::registry()
                .with(filter_layer)
                .with(fmt_layer)
                .try_init()
                .map_err(|e| anyhow::anyhow!("Failed to initialize JSON telemetry: {}", e))?;
        }
        "compact" => {
            let fmt_layer = fmt::layer()
                .compact()
                .with_file(config.file_line)
                .with_line_number(config.file_line)
                .with_target(config.target);

            tracing_subscriber::registry()
                .with(filter_layer)
                .with(fmt_layer)
                .try_init()
                .map_err(|e| anyhow::anyhow!("Failed to initialize compact telemetry: {}", e))?;
        }
        _ => {
            let fmt_layer = fmt::layer()
                .with_file(config.file_line)
                .with_line_number(config.file_line)
                .with_target(config.target);

            tracing_subscriber::registry()
                .with(filter_layer)
                .with(fmt_layer)
                .try_init()
                .map_err(|e| anyhow::anyhow!("Failed to initialize default telemetry: {}", e))?;
        }
    }

    Ok(())
}

// ============================================================================
// Span Helpers
// ============================================================================

/// Create a named span with optional attributes
pub fn create_span(name: &'static str, attrs: &[(&'static str, &str)]) -> Span {
    let span = tracing::info_span!(target: "rsk", "operation", name = name);

    for (key, value) in attrs {
        span.record(*key, *value);
    }

    span
}

/// Create a span for a skill operation
pub fn skill_span(skill_name: &str, operation: &str) -> Span {
    tracing::info_span!(
        target: "rsk",
        "skill_operation",
        skill = %skill_name,
        operation = %operation
    )
}

// ============================================================================
// Timing Utilities
// ============================================================================

/// Timer for measuring operation duration
pub struct OperationTimer {
    name: String,
    start: Instant,
    threshold_warn_ms: Option<u64>,
}

impl OperationTimer {
    /// Create a new timer for an operation
    pub fn start(name: &str) -> Self {
        tracing::debug!(target: "rsk", "Timer started: {}", name);
        Self {
            name: name.to_string(),
            start: Instant::now(),
            threshold_warn_ms: None,
        }
    }

    /// Stop the timer and return duration
    pub fn stop(self) -> Duration {
        let elapsed = self.start.elapsed();
        let ms = elapsed.as_millis();

        if let Some(threshold) = self.threshold_warn_ms
            && ms > threshold as u128
        {
            tracing::warn!(
                target: "rsk",
                "Operation '{}' exceeded threshold: {}ms > {}ms",
                self.name, ms, threshold
            );
        }

        tracing::debug!(target: "rsk", "Timer stopped: {} ({}ms)", self.name, ms);
        elapsed
    }

    /// Get elapsed time without stopping
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

// ============================================================================
// Metrics Collection
// ============================================================================

/// Metric value types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Duration(u64), // milliseconds
    Text(String),
}

/// A collected metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: MetricValue,
    pub labels: Vec<(String, String)>,
    pub timestamp_ms: u64,
}

impl Metric {
    pub fn counter(name: &str, value: u64) -> Self {
        Self {
            name: name.to_string(),
            value: MetricValue::Counter(value),
            labels: vec![],
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

// ============================================================================
// Telemetry Status
// ============================================================================

/// Summary of telemetry configuration and status
#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetryStatus {
    pub enabled: bool,
    pub level: String,
    pub format: String,
    pub filter: String,
}

pub fn get_telemetry_status() -> TelemetryStatus {
    TelemetryStatus {
        enabled: true,
        level: "info".to_string(),
        format: "text".to_string(),
        filter: "rsk=info".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert_eq!(config.level, "info");
        assert_eq!(config.format, "text");
    }

    #[test]
    fn test_operation_timer_basic() {
        let timer = OperationTimer::start("test_op");
        std::thread::sleep(std::time::Duration::from_millis(5));
        let elapsed = timer.stop();
        assert!(elapsed.as_millis() >= 5);
    }
}
