//! RSK MCP Server — typed tools wrapping the rsk computation kernel.

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{ErrorData as McpError, ServerHandler, tool, tool_handler, tool_router};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use rsk::modules::microgram::MicrogramIndex;

use crate::params;

/// RSK MCP Server
///
/// Exposes rsk-core's microgram runtime, statistical tests, decision engine,
/// and graph operations as typed MCP tools for AI agent consumption.
///
/// Holds a per-directory [`MicrogramIndex`] cache so repeated chain/list/info
/// calls against the same microgram tree share a single filesystem scan.
#[derive(Clone)]
pub struct RskMcpServer {
    tool_router: ToolRouter<Self>,
    /// Cache of loaded microgram directories. Shared across clones of the
    /// server so concurrent tool calls reuse the same index.
    index_cache: Arc<RwLock<HashMap<PathBuf, Arc<MicrogramIndex>>>>,
}

#[tool_router]
impl RskMcpServer {
    /// Create a new RSK MCP server
    #[must_use]
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            index_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Fetch (or build, on miss) a cached [`MicrogramIndex`] for `dir`.
    ///
    /// First call for a given directory pays the scan cost once; all subsequent
    /// calls return a cheap `Arc` clone. The cache never invalidates — MCP
    /// callers that mutate the fleet on disk should either restart the server
    /// or clear the cache out-of-band.
    fn index_for(&self, dir: &Path) -> Result<Arc<MicrogramIndex>, McpError> {
        // Canonicalize to make "./rsk/micrograms" and "rsk/micrograms" share a slot.
        let key = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());

        if let Ok(guard) = self.index_cache.read()
            && let Some(idx) = guard.get(&key)
        {
            return Ok(Arc::clone(idx));
        }

        let built = MicrogramIndex::load_lossy(dir).map_err(|e| {
            McpError::internal_error(format!("Failed to index {}: {e}", dir.display()), None)
        })?;
        let arc = Arc::new(built);
        if let Ok(mut guard) = self.index_cache.write() {
            guard.insert(key, Arc::clone(&arc));
        }
        Ok(arc)
    }

    // ════════════════════════════════════════════════════════════════════════
    // System Tools (1)
    // ════════════════════════════════════════════════════════════════════════

    #[tool(
        description = "Health check for RSK MCP server. Returns version, tool count, and status."
    )]
    fn rsk_health(&self) -> String {
        let tool_count = self.tool_router.list_all().len();
        serde_json::json!({
            "status": "healthy",
            "server": "rsk-mcp",
            "version": rsk::version(),
            "tool_count": tool_count,
        })
        .to_string()
    }

    // ════════════════════════════════════════════════════════════════════════
    // Microgram Tools (8)
    // ════════════════════════════════════════════════════════════════════════

    #[tool(
        description = "Run a microgram with JSON input. Returns decision path, output variables, and execution time (sub-millisecond)."
    )]
    fn mcg_run(&self, Parameters(p): Parameters<params::McgRunParams>) -> Result<String, McpError> {
        let path = std::path::Path::new(&p.path);
        let mg = rsk::modules::microgram::Microgram::load(path).map_err(|e| {
            McpError::internal_error(format!("Failed to load microgram: {e}"), None)
        })?;

        let input = parse_json_input(p.input)?;
        let result = mg.run(input);
        serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))
    }

    #[tool(
        description = "Self-test a microgram against its built-in test cases. Returns pass/fail count and individual test results."
    )]
    fn mcg_test(
        &self,
        Parameters(p): Parameters<params::McgTestParams>,
    ) -> Result<String, McpError> {
        let path = std::path::Path::new(&p.path);
        let mg = rsk::modules::microgram::Microgram::load(path).map_err(|e| {
            McpError::internal_error(format!("Failed to load microgram: {e}"), None)
        })?;

        let result = mg.test();
        serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))
    }

    #[tool(
        description = "Self-test ALL micrograms in a directory (recursive). Returns aggregate pass/fail counts across all programs."
    )]
    fn mcg_test_all(
        &self,
        Parameters(p): Parameters<params::McgTestAllParams>,
    ) -> Result<String, McpError> {
        let dir = std::path::Path::new(&p.dir);
        let results = rsk::modules::microgram::test_all(dir)
            .map_err(|e| McpError::internal_error(format!("test_all failed: {e}"), None))?;

        let total: usize = results.iter().map(|r| r.total).sum();
        let passed: usize = results.iter().map(|r| r.passed).sum();
        let failed: usize = results.iter().map(|r| r.failed).sum();

        let summary = serde_json::json!({
            "programs": results.len(),
            "total_tests": total,
            "passed": passed,
            "failed": failed,
            "failures": results.iter()
                .filter(|r| r.failed > 0)
                .map(|r| serde_json::json!({
                    "name": r.name,
                    "failed": r.failed,
                    "total": r.total,
                }))
                .collect::<Vec<_>>(),
        });

        serde_json::to_string_pretty(&summary)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))
    }

    #[tool(
        description = "Run a microgram chain: output of step N becomes input of step N+1. Alias-aware field remapping between steps."
    )]
    fn mcg_chain(
        &self,
        Parameters(p): Parameters<params::McgChainParams>,
    ) -> Result<String, McpError> {
        let dir = std::path::Path::new(&p.dir);
        let names: Vec<&str> = p.chain.split("->").map(str::trim).collect();

        let input = parse_json_input(p.input)?;
        let index = self.index_for(dir)?;

        let result = if p.accumulate {
            rsk::modules::microgram::chain_accumulate_with_index(&index, &names, input)
        } else {
            rsk::modules::microgram::chain_with_index(&index, &names, input)
        };

        match result {
            Ok(chain_result) => serde_json::to_string_pretty(&chain_result)
                .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None)),
            Err(e) => Err(McpError::internal_error(format!("Chain failed: {e}"), None)),
        }
    }

    #[tool(
        description = "Test all chain definitions in a directory against their declared test cases."
    )]
    fn mcg_chain_test(
        &self,
        Parameters(p): Parameters<params::McgChainTestParams>,
    ) -> Result<String, McpError> {
        let chain_dir = std::path::Path::new(&p.dir);

        let results = rsk::modules::microgram::test_chains(chain_dir)
            .map_err(|e| McpError::internal_error(format!("Failed to test chains: {e}"), None))?;
        serde_json::to_string_pretty(&results)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))
    }

    #[tool(
        description = "List all micrograms in a directory with name, description, and interface summary."
    )]
    fn mcg_list(
        &self,
        Parameters(p): Parameters<params::McgListParams>,
    ) -> Result<String, McpError> {
        let dir = std::path::Path::new(&p.dir);
        let index = self.index_for(dir)?;

        let listing: Vec<serde_json::Value> = index
            .all()
            .iter()
            .map(|mg| {
                serde_json::json!({
                    "name": mg.name,
                    "description": mg.description,
                    "version": mg.version,
                    "tests": mg.tests.len(),
                    "has_interface": mg.interface.is_some(),
                    "has_primitive_signature": mg.primitive_signature.is_some(),
                })
            })
            .collect();

        serde_json::to_string_pretty(&serde_json::json!({
            "count": listing.len(),
            "micrograms": listing,
        }))
        .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))
    }

    #[tool(
        description = "Get detailed info about a microgram: typed inputs/outputs, test cases, primitive signature."
    )]
    fn mcg_info(
        &self,
        Parameters(p): Parameters<params::McgInfoParams>,
    ) -> Result<String, McpError> {
        let path = std::path::Path::new(&p.path);
        let mg = rsk::modules::microgram::Microgram::load(path)
            .map_err(|e| McpError::internal_error(format!("Failed to load: {e}"), None))?;

        let info = serde_json::json!({
            "name": mg.name,
            "description": mg.description,
            "version": mg.version,
            "interface": mg.interface,
            "primitive_signature": mg.primitive_signature,
            "typed_inputs": mg.typed_inputs(),
            "typed_outputs": mg.typed_outputs(),
            "test_count": mg.tests.len(),
        });

        serde_json::to_string_pretty(&info)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))
    }

    #[tool(
        description = "Search micrograms by name or description keyword. Returns matching names with paths for use in other mcg_* tools. Default directory: ~/Projects/rsk-core/rsk/micrograms."
    )]
    fn mcg_search(
        &self,
        Parameters(p): Parameters<params::McgSearchParams>,
    ) -> Result<String, McpError> {
        let default_dir = expand_home("~/Projects/rsk-core/rsk/micrograms");
        let dir_str = p.dir.unwrap_or(default_dir);
        let dir = std::path::Path::new(&dir_str);
        let limit = p.limit.unwrap_or(20);
        let query_lower = p.query.to_lowercase();

        let mut results = Vec::new();
        search_recursive(dir, &query_lower, &mut results)
            .map_err(|e| McpError::internal_error(format!("Search failed: {e}"), None))?;

        results.truncate(limit);

        serde_json::to_string_pretty(&serde_json::json!({
            "query": p.query,
            "matches": results.len(),
            "results": results,
        }))
        .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))
    }

    #[tool(
        description = "Run coverage analysis on a microgram. Reports which decision paths are exercised by test cases."
    )]
    fn mcg_coverage(
        &self,
        Parameters(p): Parameters<params::McgCoverageParams>,
    ) -> Result<String, McpError> {
        let path = std::path::Path::new(&p.path);
        let mg = rsk::modules::microgram::Microgram::load(path)
            .map_err(|e| McpError::internal_error(format!("Failed to load: {e}"), None))?;

        let result = rsk::modules::microgram::coverage(&mg);
        serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))
    }

    // ════════════════════════════════════════════════════════════════════════
    // Statistics Tools (4)
    // ════════════════════════════════════════════════════════════════════════

    #[tool(
        description = "Chi-square independence test on a 2x2 contingency table. Returns chi-square statistic, p-value, and epistemic interpretation."
    )]
    fn stats_chi_square(&self, Parameters(p): Parameters<params::ChiSquareParams>) -> String {
        let input = rsk::modules::stats::ChiSquareInput {
            a: p.a,
            b: p.b,
            c: p.c,
            d: p.d,
        };
        let result = rsk::modules::stats::chi_square_test(&input);
        serde_json::to_string_pretty(&result).unwrap_or_default()
    }

    #[tool(
        description = "Welch's t-test for two independent samples. Returns t-statistic, degrees of freedom, p-value, and interpretation."
    )]
    fn stats_t_test(
        &self,
        Parameters(p): Parameters<params::TTestParams>,
    ) -> Result<String, McpError> {
        let input = rsk::modules::stats::TTestInput {
            group1: p.group1,
            group2: p.group2,
        };
        let result = rsk::modules::stats::t_test_independent(&input)
            .map_err(|e| McpError::internal_error(format!("t-test failed: {e}"), None))?;
        serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))
    }

    #[tool(
        description = "One-sample proportion test. Tests if observed proportion differs from null hypothesis."
    )]
    fn stats_proportion_test(
        &self,
        Parameters(p): Parameters<params::ProportionTestParams>,
    ) -> Result<String, McpError> {
        let input = rsk::modules::stats::ProportionInput {
            successes: p.successes,
            n: p.n,
            null: p.null_proportion,
        };
        let result = rsk::modules::stats::proportion_test(&input)
            .map_err(|e| McpError::internal_error(format!("Proportion test failed: {e}"), None))?;
        serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))
    }

    #[tool(
        description = "Pearson correlation test. Returns correlation coefficient r, p-value, and strength interpretation."
    )]
    fn stats_correlation(
        &self,
        Parameters(p): Parameters<params::CorrelationParams>,
    ) -> Result<String, McpError> {
        let input = rsk::modules::stats::CorrelationInput { x: p.x, y: p.y };
        let result = rsk::modules::stats::correlation_test(&input)
            .map_err(|e| McpError::internal_error(format!("Correlation failed: {e}"), None))?;
        serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))
    }

    // ════════════════════════════════════════════════════════════════════════
    // Decision Engine Tools (1)
    // ════════════════════════════════════════════════════════════════════════

    #[tool(
        description = "Process a decision tree defined in YAML with input variables. Returns the decision path and output variables."
    )]
    fn decision_tree_run(
        &self,
        Parameters(p): Parameters<params::DecisionTreeParams>,
    ) -> Result<String, McpError> {
        let tree = rsk::load_tree(&p.yaml)
            .map_err(|e| McpError::internal_error(format!("Failed to parse tree: {e}"), None))?;

        let input = parse_json_input(p.input)?;

        let mut ctx = rsk::DecisionContext::new();
        for (k, v) in input {
            ctx.set(&k, v);
        }
        let engine = rsk::DecisionEngine::new(tree);
        let result = engine.execute(&mut ctx);

        serde_json::to_string_pretty(&serde_json::json!({
            "result": format!("{result:?}"),
            "outputs": ctx.variables,
        }))
        .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))
    }

    // ════════════════════════════════════════════════════════════════════════
    // Graph Tools (2)
    // ════════════════════════════════════════════════════════════════════════

    #[tool(
        description = "Topological sort of a directed acyclic graph. Returns nodes in dependency order or error if cycle detected."
    )]
    fn graph_topsort(
        &self,
        Parameters(p): Parameters<params::GraphTopsortParams>,
    ) -> Result<String, McpError> {
        let graph = edges_to_skill_graph(&p.edges);
        match graph.topological_sort() {
            Ok(order) => serde_json::to_string_pretty(&serde_json::json!({ "order": order }))
                .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None)),
            Err(cycle) => Err(McpError::internal_error(
                format!("Cycle detected: {cycle:?}"),
                None,
            )),
        }
    }

    #[tool(
        description = "Compute parallel execution levels from a DAG. Groups nodes that can run concurrently."
    )]
    fn graph_parallel_levels(
        &self,
        Parameters(p): Parameters<params::GraphLevelsParams>,
    ) -> Result<String, McpError> {
        let graph = edges_to_skill_graph(&p.edges);
        let result = graph.level_parallelization().map_err(|cycle| {
            McpError::internal_error(format!("Cycle detected: {cycle:?}"), None)
        })?;
        serde_json::to_string_pretty(&serde_json::json!({ "levels": result }))
            .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))
    }
}

#[tool_handler]
impl ServerHandler for RskMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "RSK MCP Server — microgram decision tree runtime, statistical tests, \
                 decision engine, and graph operations. Use mcg_* tools for microgram \
                 operations, stats_* for statistical inference, decision_tree_run for \
                 tree processing, and graph_* for DAG operations."
                .to_string(),
        )
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Helpers
// ════════════════════════════════════════════════════════════════════════════

/// Parse optional JSON input into HashMap<String, Value>
fn parse_json_input(
    input: Option<serde_json::Value>,
) -> Result<std::collections::HashMap<String, rsk::Value>, McpError> {
    match input {
        Some(serde_json::Value::Object(map)) => {
            let mut result = std::collections::HashMap::new();
            for (k, v) in map {
                result.insert(k, json_to_rsk_value(v));
            }
            Ok(result)
        }
        Some(_) => Err(McpError::invalid_params(
            "Input must be a JSON object",
            None,
        )),
        None => Ok(std::collections::HashMap::new()),
    }
}

/// Build a SkillGraph from edge pairs for graph tools
fn edges_to_skill_graph(edges: &[(String, String)]) -> rsk::modules::graph::SkillGraph {
    let mut adj: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (from, to) in edges {
        adj.entry(from.clone()).or_default().push(to.clone());
        adj.entry(to.clone()).or_default(); // ensure target node exists
    }
    rsk::modules::graph::SkillGraph::from(adj)
}

/// Expand `~` to the user's home directory
fn expand_home(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}

/// Recursively search YAML files for micrograms matching a query
fn search_recursive(
    dir: &std::path::Path,
    query: &str,
    results: &mut Vec<serde_json::Value>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("Cannot read dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Dir entry error: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            search_recursive(&path, query, results)?;
        } else if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
            if let Ok(mg) = rsk::modules::microgram::Microgram::load(&path) {
                let name_lower = mg.name.to_lowercase();
                let desc_lower = mg.description.to_lowercase();
                let prim_lower = mg
                    .primitive_signature
                    .as_ref()
                    .map(|ps| ps.dominant.to_lowercase())
                    .unwrap_or_default();

                if name_lower.contains(query)
                    || desc_lower.contains(query)
                    || prim_lower.contains(query)
                {
                    results.push(serde_json::json!({
                        "name": mg.name,
                        "description": mg.description,
                        "path": path.display().to_string(),
                        "primitive": mg.primitive_signature.as_ref().map(|ps| &ps.dominant),
                        "tests": mg.tests.len(),
                    }));
                }
            }
        }
    }
    Ok(())
}

/// Convert serde_json::Value to rsk::Value
fn json_to_rsk_value(v: serde_json::Value) -> rsk::Value {
    match v {
        serde_json::Value::Null => rsk::Value::Null,
        serde_json::Value::Bool(b) => rsk::Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rsk::Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                rsk::Value::Float(f)
            } else {
                rsk::Value::Null
            }
        }
        serde_json::Value::String(s) => rsk::Value::String(s),
        serde_json::Value::Array(arr) => {
            rsk::Value::Array(arr.into_iter().map(json_to_rsk_value).collect())
        }
        serde_json::Value::Object(map) => {
            let mut hm = std::collections::HashMap::new();
            for (k, val) in map {
                hm.insert(k, json_to_rsk_value(val));
            }
            rsk::Value::Object(hm)
        }
    }
}
