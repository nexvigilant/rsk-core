//! Typed parameter structs for MCP tools.
//!
//! Each tool gets a dedicated struct — never raw `serde_json::Value`.

use schemars::JsonSchema;
use serde::Deserialize;

// ═══════════════════════════════════════════════════════════════════════════
// Microgram Tools
// ═══════════════════════════════════════════════════════════════════════════

/// Run a single microgram with JSON input
#[derive(Debug, Deserialize, JsonSchema)]
pub struct McgRunParams {
    /// Path to the microgram YAML file
    #[schemars(description = "Absolute or relative path to the microgram YAML file")]
    pub path: String,
    /// JSON input variables (key-value pairs)
    #[schemars(description = "JSON object of input variables for the microgram")]
    #[serde(default)]
    pub input: Option<serde_json::Value>,
}

/// Self-test a single microgram against its built-in test cases
#[derive(Debug, Deserialize, JsonSchema)]
pub struct McgTestParams {
    /// Path to the microgram YAML file
    #[schemars(description = "Absolute or relative path to the microgram YAML file")]
    pub path: String,
}

/// Self-test all micrograms in a directory (recursive)
#[derive(Debug, Deserialize, JsonSchema)]
pub struct McgTestAllParams {
    /// Directory containing microgram YAML files
    #[schemars(description = "Directory to scan for microgram YAML files (recursive)")]
    pub dir: String,
}

/// Execute a microgram chain (output of step N → input of step N+1)
#[derive(Debug, Deserialize, JsonSchema)]
pub struct McgChainParams {
    /// Chain expression: "prr-signal -> signal-to-causality -> naranjo-quick"
    #[schemars(description = "Chain expression: microgram names separated by ' -> '")]
    pub chain: String,
    /// Directory containing microgram YAML files
    #[schemars(description = "Directory to resolve microgram names from")]
    pub dir: String,
    /// Initial JSON input for the first step
    #[schemars(description = "JSON object of initial input variables")]
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    /// Accumulate all outputs across steps (default: false)
    #[schemars(description = "If true, all upstream outputs are preserved; if false, only last step output passes forward")]
    #[serde(default)]
    pub accumulate: bool,
}

/// Test all chain definitions in a directory
#[derive(Debug, Deserialize, JsonSchema)]
pub struct McgChainTestParams {
    /// Directory containing chain definition YAML files
    #[schemars(description = "Directory containing chain definition files")]
    pub dir: String,
    /// Directory containing microgram YAML files
    #[schemars(description = "Directory to resolve microgram names from")]
    pub micrograms_dir: String,
}

/// Load and list all micrograms in a directory
#[derive(Debug, Deserialize, JsonSchema)]
pub struct McgListParams {
    /// Directory containing microgram YAML files
    #[schemars(description = "Directory to scan for microgram YAML files (recursive)")]
    pub dir: String,
}

/// Get microgram info (interface, tests, primitive signature)
#[derive(Debug, Deserialize, JsonSchema)]
pub struct McgInfoParams {
    /// Path to the microgram YAML file
    #[schemars(description = "Absolute or relative path to the microgram YAML file")]
    pub path: String,
}

/// Run coverage analysis on a microgram
#[derive(Debug, Deserialize, JsonSchema)]
pub struct McgCoverageParams {
    /// Path to the microgram YAML file
    #[schemars(description = "Absolute or relative path to the microgram YAML file")]
    pub path: String,
}

/// Search micrograms by name, description, or primitive signature
#[derive(Debug, Deserialize, JsonSchema)]
pub struct McgSearchParams {
    /// Search query — matched case-insensitively against microgram name and description
    #[schemars(description = "Search term matched against name and description (case-insensitive)")]
    pub query: String,
    /// Optional directory override (defaults to ~/Projects/rsk-core/rsk/micrograms)
    #[schemars(description = "Directory to search in (default: ~/Projects/rsk-core/rsk/micrograms)")]
    #[serde(default)]
    pub dir: Option<String>,
    /// Maximum results to return (default: 20)
    #[schemars(description = "Maximum number of results (default: 20)")]
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Stress-test a microgram with random inputs
#[derive(Debug, Deserialize, JsonSchema)]
pub struct McgStressParams {
    /// Path to the microgram YAML file
    #[schemars(description = "Absolute or relative path to the microgram YAML file")]
    pub path: String,
    /// Number of iterations (default: 1000)
    #[schemars(description = "Number of random input iterations")]
    #[serde(default = "default_stress_iterations")]
    pub iterations: usize,
}

fn default_stress_iterations() -> usize {
    1000
}

// ═══════════════════════════════════════════════════════════════════════════
// Statistics Tools
// ═══════════════════════════════════════════════════════════════════════════

/// Chi-square independence test (2x2 contingency table)
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChiSquareParams {
    /// Cell a (top-left): drug+event count
    #[schemars(description = "Count: drug present AND event present")]
    pub a: i64,
    /// Cell b (top-right): drug+no-event count
    #[schemars(description = "Count: drug present AND event absent")]
    pub b: i64,
    /// Cell c (bottom-left): no-drug+event count
    #[schemars(description = "Count: drug absent AND event present")]
    pub c: i64,
    /// Cell d (bottom-right): no-drug+no-event count
    #[schemars(description = "Count: drug absent AND event absent")]
    pub d: i64,
}

/// Welch's t-test for two independent samples
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TTestParams {
    /// First sample group
    #[schemars(description = "Array of numeric values for group 1")]
    pub group1: Vec<f64>,
    /// Second sample group
    #[schemars(description = "Array of numeric values for group 2")]
    pub group2: Vec<f64>,
}

/// One-sample proportion test
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProportionTestParams {
    /// Number of successes
    #[schemars(description = "Number of observed successes")]
    pub successes: i64,
    /// Total sample size
    #[schemars(description = "Total number of observations")]
    pub n: i64,
    /// Null hypothesis proportion (0.0-1.0, default 0.5)
    #[schemars(description = "Expected proportion under null hypothesis (0.0-1.0, default 0.5)")]
    pub null_proportion: Option<f64>,
}

/// Pearson correlation test
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CorrelationParams {
    /// X values
    #[schemars(description = "Array of X values")]
    pub x: Vec<f64>,
    /// Y values
    #[schemars(description = "Array of Y values")]
    pub y: Vec<f64>,
}

// ═══════════════════════════════════════════════════════════════════════════
// Decision Engine Tools
// ═══════════════════════════════════════════════════════════════════════════

/// Evaluate a decision tree with input variables
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DecisionTreeParams {
    /// YAML content of the decision tree
    #[schemars(description = "Decision tree definition in YAML format")]
    pub yaml: String,
    /// Input variables as JSON object
    #[schemars(description = "JSON object of input variables for tree evaluation")]
    #[serde(default)]
    pub input: Option<serde_json::Value>,
}

// ═══════════════════════════════════════════════════════════════════════════
// Graph Tools
// ═══════════════════════════════════════════════════════════════════════════

/// Topological sort of a directed acyclic graph
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphTopsortParams {
    /// Edges as array of [from, to] pairs
    #[schemars(description = "Array of [from, to] string pairs defining graph edges")]
    pub edges: Vec<(String, String)>,
}

/// Compute parallel execution levels from a DAG
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphLevelsParams {
    /// Edges as array of [from, to] pairs
    #[schemars(description = "Array of [from, to] string pairs defining graph edges")]
    pub edges: Vec<(String, String)>,
}
