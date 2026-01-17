//! Python Bindings Module - PyO3 bridge for Python integration
//!
//! This module provides Python bindings for rsk functions via PyO3.
//! It's only compiled when the `python` feature is enabled.
//!
//! ## Building the Python Extension
//!
//! ```bash
//! # Build with maturin (recommended)
//! pip install maturin
//! maturin build --release --features python
//!
//! # Or build manually
//! cargo build --release --features python
//! ```
//!
//! ## Usage from Python
//!
//! ```python
//! import rsk
//!
//! # Levenshtein distance
//! result = rsk.levenshtein("kitten", "sitting")
//! print(f"Distance: {result['distance']}, Similarity: {result['similarity']}")
//!
//! # SHA-256 hashing
//! hash_result = rsk.sha256("hello world")
//! print(f"Hash: {hash_result['hex']}")
//!
//! # Fuzzy search
//! matches = rsk.fuzzy_search("test", ["test-skill", "testing", "best"], 3)
//! print(f"Matches: {matches}")
//!
//! # Text processing (NEW in 0.4.3)
//! tokens = rsk.tokenize("Hello, World! How are you?")
//! print(f"Tokens: {tokens['tokens']}, Count: {tokens['count']}")
//!
//! normalized = rsk.normalize("  Hello,  WORLD!  ", remove_punctuation=True)
//! print(f"Normalized: {normalized['text']}")
//!
//! freq = rsk.word_frequency("the quick brown fox jumps over the lazy dog", top_n=5)
//! print(f"Top words: {freq['top_words']}")
//!
//! entropy = rsk.text_entropy("aaaaaaaaaa")  # Low entropy = compressible
//! print(f"Entropy: {entropy['entropy_estimate']}, Compressibility: {entropy['compressibility']}")
//!
//! # Compression (NEW in 0.4.3)
//! compressed = rsk.gzip_compress("Hello " * 100, level="best")
//! print(f"Ratio: {compressed['ratio']}, Savings: {compressed['savings_percent']}%")
//!
//! decompressed = rsk.gzip_decompress(compressed['data'])
//! print(f"Decompressed: {decompressed[:20]}...")
//!
//! ratio = rsk.estimate_compressibility(b"aaaaaaaaaa")  # 0.0 = highly compressible
//! print(f"Estimated ratio: {ratio}")
//! ```

#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::PyDict;
#[cfg(feature = "python")]
use std::collections::HashMap;

#[cfg(feature = "python")]
use crate::modules::{
    builder::{build_skill, verify_skill},
    chain::{
        parse_inline as chain_parse_inline, parse_yaml as chain_parse_yaml,
        validate_chain, execute_chain_with_fn, Chain, ChainStep, StepType,
        ExecutorConfig, SkillExecutionResult, ValidationResult as ChainValidationResult,
    },
    stats::{
        chi_square_test, t_test_independent, proportion_test, correlation_test,
        ChiSquareInput, TTestInput, ProportionInput, CorrelationInput,
    },
    anti_pattern::{
        detect_anti_patterns, Features, DetectionConfig, AntiPattern, Symptom, SymptomType,
        create_god_object_pattern, create_paper_constructs_pattern,
    },
    code_generator::{generate_validation_rules, generate_test_scaffold, generate_rust_stub, generate_decision_tree},
    compression::{gzip_compress_string, gzip_decompress_string, estimate_compressibility, CompressionLevel},
    crypto::{sha256_hash, sha256_verify},
    execution_engine::{ExecutionModule, EffortSize, build_execution_plan},
    graph::{SkillGraph, SkillNode},
    json_processor::{parse_json, serialize_json, query_path, set_path, merge_json, diff_json, flatten_json, unflatten_json},
    levenshtein::{fuzzy_search, levenshtein},
    math::{calculate_variance, is_prime},
    intent::classify_intent,
    session_tracker::{
        load_state as session_load, save_state as session_save,
        track_execution as session_track, track_completion as session_complete,
        track_failure as session_fail, append_log as session_log,
        route_skill as session_route, SessionState,
    },
    state_manager::{CheckpointManager, ExecutionContext},
    taxonomy::{query_taxonomy, list_taxonomy},
    text_processor::{extract_smst, parse_frontmatter, tokenize, normalize, word_frequency, analyze_compressibility},
    yaml_processor::parse_yaml,
    decision_engine::{DecisionEngine, DecisionTree, DecisionContext, ExecutionResult, Value},
    epistemic::{validate_claim, validate_claims, get_hedging_suggestions, EpistemicResult, ConfidenceLevel},
};

// ============================================================================
// Levenshtein Operations
// ============================================================================

/// Calculate Levenshtein edit distance between two strings
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "levenshtein")]
fn py_levenshtein(py: Python<'_>, source: &str, target: &str) -> PyResult<PyObject> {
    let result = levenshtein(source, target);
    let dict = PyDict::new(py);
    dict.set_item("distance", result.distance)?;
    dict.set_item("similarity", result.similarity)?;
    dict.set_item("source_len", result.source_len)?;
    dict.set_item("target_len", result.target_len)?;
    Ok(dict.into())
}

/// Fuzzy search for best matches
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "fuzzy_search")]
fn py_fuzzy_search(py: Python<'_>, query: &str, candidates: Vec<String>, limit: usize) -> PyResult<PyObject> {
    let results = fuzzy_search(query, &candidates, limit);
    let list: Vec<PyObject> = results.iter().map(|r| {
        let dict = PyDict::new(py);
        dict.set_item("candidate", &r.candidate).expect("Failed to set candidate");
        dict.set_item("distance", r.distance).expect("Failed to set distance");
        dict.set_item("similarity", r.similarity).expect("Failed to set similarity");
        dict.into()
    }).collect();
    Ok(list.into_pyobject(py)?.into())
}

// ============================================================================
// Crypto Operations
// ============================================================================

/// Calculate SHA-256 hash of a string
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "sha256")]
fn py_sha256(py: Python<'_>, input: &str) -> PyResult<PyObject> {
    let result = sha256_hash(input);
    let dict = PyDict::new(py);
    dict.set_item("algorithm", &result.algorithm)?;
    dict.set_item("hex", &result.hex)?;
    dict.set_item("bytes_hashed", result.bytes_hashed)?;
    Ok(dict.into())
}

/// Verify a string against an expected SHA-256 hash
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "sha256_verify")]
fn py_sha256_verify(_py: Python<'_>, input: &str, expected: &str) -> PyResult<bool> {
    Ok(sha256_verify(input, expected))
}

// ============================================================================
// Math Operations
// ============================================================================

/// Calculate variance between actual and target values
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "variance")]
fn py_variance(py: Python<'_>, actual: f64, target: f64) -> PyResult<PyObject> {
    let result = calculate_variance(actual, target);
    let dict = PyDict::new(py);
    dict.set_item("absolute", result.absolute)?;
    dict.set_item("percentage", result.percentage)?;
    Ok(dict.into())
}

/// Check if a number is prime
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "is_prime")]
fn py_is_prime(py: Python<'_>, n: i64) -> PyResult<PyObject> {
    let result = is_prime(n);
    let dict = PyDict::new(py);
    dict.set_item("is_prime", result.is_prime)?;
    dict.set_item("number", result.number)?;
    dict.set_item("reason", result.reason)?;
    Ok(dict.into())
}

/// Classify skill intent into pattern and complexity
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "classify_intent")]
fn py_classify_intent(py: Python<'_>, intent: &str) -> PyResult<PyObject> {
    match classify_intent(intent) {
        Ok(result) => {
            let dict = PyDict::new(py);
            dict.set_item("pattern", format!("{:?}", result.pattern).to_uppercase())?;
            dict.set_item("complexity", format!("{:?}", result.complexity).to_uppercase())?;
            dict.set_item("rsk_modules", result.rsk_modules)?;
            Ok(dict.into())
        }
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
    }
}

// ============================================================================
// Taxonomy Operations
// ============================================================================

/// Query taxonomy by type and key
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "query_taxonomy")]
fn py_query_taxonomy(py: Python<'_>, taxonomy_type: &str, key: &str) -> PyResult<PyObject> {
    let result = query_taxonomy(taxonomy_type, key);
    let dict = PyDict::new(py);
    dict.set_item("query_type", &result.query_type)?;
    dict.set_item("key", &result.key)?;
    dict.set_item("found", result.found)?;
    
    if let Some(data) = result.data {
        // We still use JSON as an intermediate for complex dynamic data 
        // until we have a full PyO3 trait for our Value types, but 
        // we return the dict directly.
        let json_str = serde_json::to_string(&data).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let json_module = py.import("json")?;
        let py_data = json_module.call_method1("loads", (json_str,))?;
        dict.set_item("data", py_data)?;
    } else {
        dict.set_item("data", py.None())?;
    }
    
    Ok(dict.into())
}

/// List all entries in a taxonomy
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "list_taxonomy")]
fn py_list_taxonomy(py: Python<'_>, taxonomy_type: &str) -> PyResult<PyObject> {
    let result = list_taxonomy(taxonomy_type);
    let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let json_module = py.import("json")?;
    let py_dict = json_module.call_method1("loads", (json_str,))?;
    Ok(py_dict.into())
}

// ============================================================================
// SKILL.md Operations
// ============================================================================

/// Extract SMST from SKILL.md content
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "extract_smst")]
fn py_extract_smst(py: Python<'_>, content: &str) -> PyResult<PyObject> {
    let result = extract_smst(content);

    let json_module = py.import("json")?;

    // Build frontmatter dict via JSON for completeness (includes adjacencies)
    let fm_json = serde_json::to_string(&result.frontmatter)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Frontmatter serialization failed: {}", e)))?;
    let frontmatter_dict = json_module.call_method1("loads", (fm_json,))?;

    // Build spec dict directly
    let spec = &result.spec;
    let spec_dict = PyDict::new(py);
    spec_dict.set_item("inputs", &spec.inputs)?;
    spec_dict.set_item("outputs", &spec.outputs)?;
    spec_dict.set_item("state", &spec.state)?;
    spec_dict.set_item("operator_mode", &spec.operator_mode)?;
    spec_dict.set_item("performance", &spec.performance)?;
    spec_dict.set_item("invariants", &spec.invariants)?;
    spec_dict.set_item("failure_modes", &spec.failure_modes)?;
    spec_dict.set_item("telemetry", &spec.telemetry)?;

    // Build score dict directly
    let score = &result.score;
    let score_dict = PyDict::new(py);
    score_dict.set_item("total_score", score.total_score)?;
    score_dict.set_item("sections_present", score.sections_present)?;
    score_dict.set_item("sections_required", score.sections_required)?;
    score_dict.set_item("has_frontmatter", score.has_frontmatter)?;
    score_dict.set_item("has_machine_spec", score.has_machine_spec)?;
    score_dict.set_item("compliance_level", &score.compliance_level)?;
    score_dict.set_item("missing_sections", &score.missing_sections)?;

    // Build result dict
    let dict = PyDict::new(py);
    dict.set_item("frontmatter", frontmatter_dict)?;
    dict.set_item("spec", spec_dict)?;
    dict.set_item("score", score_dict)?;
    dict.set_item("is_diamond_compliant", result.is_diamond_compliant)?;

    Ok(dict.into())
}

/// Parse frontmatter from SKILL.md content
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "parse_frontmatter")]
fn py_parse_frontmatter(py: Python<'_>, content: &str) -> PyResult<PyObject> {
    let fm = parse_frontmatter(content);
    let val = fm.flatten_to_json();

    // Convert back to Python dict via PyO3 compatible JSON conversion
    let json_str = serde_json::to_string(&val).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let json_module = py.import("json")?;
    let py_dict = json_module.call_method1("loads", (json_str,))?;
    Ok(py_dict.into())
}

// ============================================================================
// Text Processing Operations
// ============================================================================

/// Tokenize text into words
///
/// Args:
///     text: Input text to tokenize
///
/// Returns:
///     dict with 'tokens' (list), 'count' (int), 'unique_count' (int)
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "tokenize")]
fn py_tokenize(py: Python<'_>, text: &str) -> PyResult<PyObject> {
    let result = tokenize(text);
    let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let json_module = py.import("json")?;
    let py_dict = json_module.call_method1("loads", (json_str,))?;
    Ok(py_dict.into())
}

/// Normalize text for comparison
///
/// Args:
///     text: Input text to normalize
///     remove_punctuation: Whether to remove punctuation (default: True)
///
/// Returns:
///     dict with 'text' (str), 'original_length' (int), 'normalized_length' (int)
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "normalize", signature = (text, remove_punctuation=true))]
fn py_normalize(py: Python<'_>, text: &str, remove_punctuation: bool) -> PyResult<PyObject> {
    let result = normalize(text, remove_punctuation);
    let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let json_module = py.import("json")?;
    let py_dict = json_module.call_method1("loads", (json_str,))?;
    Ok(py_dict.into())
}

/// Calculate word frequencies in text
///
/// Args:
///     text: Input text to analyze
///     top_n: Number of top words to return (default: 10)
///
/// Returns:
///     dict with 'frequencies' (dict), 'total_words' (int), 'unique_words' (int), 'top_words' (list)
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "word_frequency", signature = (text, top_n=10))]
fn py_word_frequency(py: Python<'_>, text: &str, top_n: usize) -> PyResult<PyObject> {
    let result = word_frequency(text, top_n);
    let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let json_module = py.import("json")?;
    let py_dict = json_module.call_method1("loads", (json_str,))?;
    Ok(py_dict.into())
}

/// Calculate text entropy (compressibility analysis)
///
/// Args:
///     text: Input text to analyze
///
/// Returns:
///     dict with 'original_chars' (int), 'unique_chars' (int), 'entropy_estimate' (float), 'compressibility' (str)
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "text_entropy")]
fn py_text_entropy(py: Python<'_>, text: &str) -> PyResult<PyObject> {
    let result = analyze_compressibility(text);
    let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let json_module = py.import("json")?;
    let py_dict = json_module.call_method1("loads", (json_str,))?;
    Ok(py_dict.into())
}

// ============================================================================
// Compression Operations
// ============================================================================

/// Compress text using gzip
///
/// Args:
///     text: Input text to compress
///     level: Compression level - "fast", "default", or "best" (default: "default")
///
/// Returns:
///     dict with 'original_size' (int), 'compressed_size' (int), 'ratio' (float),
///           'savings_percent' (float), 'data' (bytes as base64 string)
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "gzip_compress", signature = (text, level="default"))]
fn py_gzip_compress(py: Python<'_>, text: &str, level: &str) -> PyResult<PyObject> {
    let compression_level = match level.to_lowercase().as_str() {
        "fast" => CompressionLevel::Fast,
        "best" => CompressionLevel::Best,
        _ => CompressionLevel::Default,
    };

    let result = gzip_compress_string(text, compression_level);
    let dict = PyDict::new(py);
    dict.set_item("original_size", result.original_size)?;
    dict.set_item("compressed_size", result.compressed_size)?;
    dict.set_item("ratio", result.ratio)?;
    dict.set_item("savings_percent", result.savings_percent)?;
    // Return bytes directly for Python to handle
    dict.set_item("data", pyo3::types::PyBytes::new(py, &result.data))?;
    Ok(dict.into())
}

/// Decompress gzip data to text
///
/// Args:
///     data: Compressed bytes (as bytes object)
///
/// Returns:
///     Decompressed text as string
///
/// Raises:
///     ValueError: If decompression fails or data is not valid gzip
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "gzip_decompress")]
fn py_gzip_decompress(_py: Python<'_>, data: &[u8]) -> PyResult<String> {
    gzip_decompress_string(data)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

/// Estimate compressibility of data without actually compressing
///
/// Args:
///     data: Input bytes or string to analyze
///
/// Returns:
///     float: Estimated compression ratio (0.0 = highly compressible, 1.0 = not compressible)
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "estimate_compressibility")]
fn py_estimate_compressibility(_py: Python<'_>, data: &[u8]) -> PyResult<f64> {
    Ok(estimate_compressibility(data))
}

// ============================================================================
// Graph Operations (NEW: C2 Sprint 1)
// ============================================================================

/// Perform topological sort on a graph
///
/// Args:
///     graph: Dict mapping node names to lists of successor node names
///            (edges point FROM dependencies TO dependents)
///
/// Returns:
///     dict with 'sorted' (list of node names in dependency order) and 'order' (indices)
///
/// Example:
///     >>> result = topological_sort({"a": ["b"], "b": ["c"], "c": []})
///     >>> result['sorted']
///     ['a', 'b', 'c']
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "topological_sort")]
fn py_topological_sort(py: Python<'_>, graph: std::collections::HashMap<String, Vec<String>>) -> PyResult<PyObject> {
    // Convert Python dict to SkillGraph
    // Input format: node -> [successors] (what this node points to)
    // We need to invert to dependencies format for SkillGraph

    // Collect all nodes (including those only in successor lists)
    let mut all_nodes: std::collections::HashSet<String> = graph.keys().cloned().collect();
    for successors in graph.values() {
        for s in successors {
            all_nodes.insert(s.clone());
        }
    }

    // Build reverse mapping: for each node, find its dependencies
    let mut dependencies: std::collections::HashMap<String, Vec<String>> =
        all_nodes.iter().map(|n| (n.clone(), vec![])).collect();

    for (node, successors) in &graph {
        for successor in successors {
            dependencies.get_mut(successor).map(|deps| deps.push(node.clone()));
        }
    }

    // Build SkillGraph
    let mut skill_graph = SkillGraph::new();
    for node in &all_nodes {
        skill_graph.add_node(SkillNode {
            name: node.clone(),
            dependencies: dependencies.get(node).cloned().unwrap_or_default(),
            outputs: vec![],
            adjacencies: vec![],
        });
    }

    match skill_graph.topological_sort() {
        Ok(sorted) => {
            let dict = PyDict::new(py);
            let order: Vec<usize> = (0..sorted.len()).collect();
            dict.set_item("sorted", &sorted)?;
            dict.set_item("order", order)?;
            dict.set_item("status", "success")?;
            Ok(dict.into())
        }
        Err(cycle) => {
            let dict = PyDict::new(py);
            dict.set_item("error", "Graph contains a cycle")?;
            dict.set_item("cycle", cycle)?;
            dict.set_item("sorted", Vec::<String>::new())?;
            dict.set_item("order", Vec::<usize>::new())?;
            Ok(dict.into())
        }
    }
}

/// Compute parallel execution levels for DAG vertices
///
/// Args:
///     graph: Dict mapping node names to lists of successor node names
///            (edges point FROM dependencies TO dependents)
///
/// Returns:
///     dict with 'levels' (list of lists, each inner list can execute in parallel)
///     and 'total_levels' (int)
///
/// Example:
///     >>> result = level_parallelization({"a": ["b", "c"], "b": ["d"], "c": ["d"], "d": []})
///     >>> result['levels']
///     [['a'], ['b', 'c'], ['d']]
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "level_parallelization")]
fn py_level_parallelization(py: Python<'_>, graph: std::collections::HashMap<String, Vec<String>>) -> PyResult<PyObject> {
    // Same conversion logic as topological_sort
    let mut all_nodes: std::collections::HashSet<String> = graph.keys().cloned().collect();
    for successors in graph.values() {
        for s in successors {
            all_nodes.insert(s.clone());
        }
    }

    let mut dependencies: std::collections::HashMap<String, Vec<String>> =
        all_nodes.iter().map(|n| (n.clone(), vec![])).collect();

    for (node, successors) in &graph {
        for successor in successors {
            dependencies.get_mut(successor).map(|deps| deps.push(node.clone()));
        }
    }

    let mut skill_graph = SkillGraph::new();
    for node in &all_nodes {
        skill_graph.add_node(SkillNode {
            name: node.clone(),
            dependencies: dependencies.get(node).cloned().unwrap_or_default(),
            outputs: vec![],
            adjacencies: vec![],
        });
    }

    match skill_graph.level_parallelization() {
        Ok(levels) => {
            let dict = PyDict::new(py);
            dict.set_item("levels", &levels)?;
            dict.set_item("total_levels", levels.len())?;
            dict.set_item("status", "success")?;
            Ok(dict.into())
        }
        Err(cycle) => {
            let dict = PyDict::new(py);
            dict.set_item("error", "Graph contains a cycle")?;
            dict.set_item("cycle", cycle)?;
            dict.set_item("levels", Vec::<Vec<String>>::new())?;
            dict.set_item("total_levels", 0)?;
            Ok(dict.into())
        }
    }
}

/// Find shortest path between two nodes in a weighted graph (Dijkstra)
///
/// Args:
///     graph: Dict mapping node names to lists of (target, weight) tuples
///            representing weighted edges
///     start: Starting node name
///     end: Target node name
///
/// Returns:
///     dict with 'path' (list of node names), 'cost' (float), and 'status'
///     Returns error dict if no path exists or nodes not found
///
/// Example:
///     >>> graph = {"a": [("b", 1.0), ("c", 3.0)], "b": [("c", 1.0)], "c": []}
///     >>> result = shortest_path(graph, "a", "c")
///     >>> result['path']
///     ['a', 'b', 'c']
///     >>> result['cost']
///     2.0
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "shortest_path")]
fn py_shortest_path(
    py: Python<'_>,
    graph: std::collections::HashMap<String, Vec<(String, f32)>>,
    start: &str,
    end: &str,
) -> PyResult<PyObject> {
    // Build SkillGraph with weighted adjacencies
    let mut skill_graph = SkillGraph::new();

    // Collect all nodes
    let mut all_nodes: std::collections::HashSet<String> = graph.keys().cloned().collect();
    for edges in graph.values() {
        for (target, _) in edges {
            all_nodes.insert(target.clone());
        }
    }

    // Add nodes with adjacencies
    for node_name in &all_nodes {
        let adjacencies: Vec<crate::modules::graph::Adjacency> = graph
            .get(node_name)
            .map(|edges| {
                edges.iter().map(|(target, weight)| {
                    crate::modules::graph::Adjacency {
                        target: target.clone(),
                        weight: *weight,
                        when: "success".to_string(),
                        action: "".to_string(),
                    }
                }).collect()
            })
            .unwrap_or_default();

        skill_graph.add_node(SkillNode {
            name: node_name.clone(),
            dependencies: vec![],
            outputs: vec![],
            adjacencies,
        });
    }

    let dict = PyDict::new(py);

    match skill_graph.shortest_path(start, end) {
        Some((path, cost)) => {
            dict.set_item("path", &path)?;
            dict.set_item("cost", cost)?;
            dict.set_item("status", "success")?;
        }
        None => {
            // Determine reason for failure
            let error_msg = if !all_nodes.contains(start) {
                format!("Start node '{}' not in graph", start)
            } else if !all_nodes.contains(end) {
                format!("End node '{}' not in graph", end)
            } else {
                format!("No path from '{}' to '{}'", start, end)
            };
            dict.set_item("error", error_msg)?;
            dict.set_item("path", Vec::<String>::new())?;
            dict.set_item("cost", -1.0f32)?;
            dict.set_item("status", "error")?;
        }
    }

    Ok(dict.into())
}

// ============================================================================
// YAML Operations (NEW: C2 Sprint 1)
// ============================================================================

/// Parse YAML string to JSON-compatible Python dict
///
/// Args:
///     content: YAML content as string
///
/// Returns:
///     dict with 'status', 'format', 'data', 'keys', 'depth'
///
/// Example:
///     >>> result = parse_yaml_string("name: test\\nversion: '1.0'")
///     >>> result['data']['name']
///     'test'
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "parse_yaml_string")]
fn py_parse_yaml_string(py: Python<'_>, content: &str) -> PyResult<PyObject> {
    match parse_yaml(content) {
        Ok(result) => {
            // Convert the Rust ParseResult to Python dict via JSON
            let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let json_module = py.import("json")?;
            let py_dict = json_module.call_method1("loads", (json_str,))?;
            Ok(py_dict.into())
        }
        Err(e) => {
            let dict = PyDict::new(py);
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            Ok(dict.into())
        }
    }
}

// ============================================================================
// Execution Engine Operations (NEW: C2 Sprint 2)
// ============================================================================

/// Build an execution plan from a list of modules.
///
/// Args:
///     modules: List of dicts, each with:
///         - id: Unique module identifier
///         - name: Human-readable name
///         - dependencies: List of module IDs this depends on
///         - effort: Optional effort size ("S", "M", "L", "XL") - defaults to "M"
///         - risk: Optional risk score 0.0-1.0 - defaults to 0.3
///         - critical: Optional bool - defaults to False
///
/// Returns:
///     dict with:
///         - execution_order: List of module IDs in topological order
///         - levels: List of lists (parallel execution groups)
///         - critical_path: List of module IDs on critical path
///         - estimated_duration_minutes: Total estimated duration
///         - status: "success" or error info
///
/// Example:
///     >>> modules = [
///     ...     {"id": "M1", "name": "Root task", "dependencies": []},
///     ...     {"id": "M2", "name": "Depends on M1", "dependencies": ["M1"]},
///     ... ]
///     >>> plan = build_execution_plan(modules)
///     >>> plan['execution_order']
///     ['M1', 'M2']
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "build_execution_plan")]
fn py_build_execution_plan(
    py: Python<'_>,
    modules: Vec<std::collections::HashMap<String, pyo3::Py<pyo3::PyAny>>>,
) -> PyResult<PyObject> {
    // Convert Python dicts to ExecutionModule structs
    let mut rust_modules: Vec<ExecutionModule> = Vec::with_capacity(modules.len());

    for module_dict in &modules {
        // Extract required fields
        let id: String = module_dict.get("id")
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Module missing 'id' field"))?
            .extract(py)?;

        let name: String = module_dict.get("name")
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Module missing 'name' field"))?
            .extract(py)?;

        let dependencies: Vec<String> = module_dict.get("dependencies")
            .map(|v| v.extract(py))
            .transpose()?
            .unwrap_or_default();

        // Extract optional fields with defaults
        let effort_str: String = module_dict.get("effort")
            .map(|v| v.extract(py))
            .transpose()?
            .unwrap_or_else(|| "M".to_string());

        let effort = EffortSize::from_str(&effort_str).unwrap_or(EffortSize::M);

        let risk: f32 = module_dict.get("risk")
            .map(|v| v.extract(py))
            .transpose()?
            .unwrap_or(0.3);

        let critical: bool = module_dict.get("critical")
            .map(|v| v.extract(py))
            .transpose()?
            .unwrap_or(false);

        let purpose: String = module_dict.get("purpose")
            .map(|v| v.extract(py))
            .transpose()?
            .unwrap_or_default();

        let resources: Vec<String> = module_dict.get("resources")
            .map(|v| v.extract(py))
            .transpose()?
            .unwrap_or_default();

        let deliverables: Vec<String> = module_dict.get("deliverables")
            .map(|v| v.extract(py))
            .transpose()?
            .unwrap_or_default();

        let mut module = ExecutionModule::new(&id, &name, dependencies)
            .with_effort(effort)
            .with_risk(risk)
            .with_resources(resources)
            .with_deliverables(deliverables);

        if !purpose.is_empty() {
            module = module.with_purpose(&purpose);
        }
        if critical {
            module = module.critical();
        }

        rust_modules.push(module);
    }

    // Build execution plan
    let dict = PyDict::new(py);

    match build_execution_plan(rust_modules) {
        Ok(plan) => {
            dict.set_item("execution_order", &plan.execution_order)?;
            dict.set_item("levels", &plan.levels)?;
            dict.set_item("critical_path", &plan.critical_path)?;
            dict.set_item("estimated_duration_minutes", plan.estimated_duration_minutes)?;
            dict.set_item("module_count", plan.modules.len())?;
            dict.set_item("status", "success")?;
        }
        Err(e) => {
            dict.set_item("error", e.to_string())?;
            dict.set_item("execution_order", Vec::<String>::new())?;
            dict.set_item("levels", Vec::<Vec<String>>::new())?;
            dict.set_item("critical_path", Vec::<String>::new())?;
            dict.set_item("estimated_duration_minutes", 0)?;
            dict.set_item("module_count", 0)?;
            dict.set_item("status", "error")?;
        }
    }

    Ok(dict.into())
}

// ============================================================================
// Code Generator Operations (NEW: C2 Sprint 4)
// ============================================================================

/// Generate validation rules from SMST content
///
/// Args:
///     content: SKILL.md content containing SMST specification
///
/// Returns:
///     dict with 'skill_name', 'invariant_rules', 'failure_mode_rules',
///     'input_rules', 'output_rules', 'total_rules'
///
/// Example:
///     >>> rules = generate_validation_rules(skill_content)
///     >>> rules['total_rules']
///     12
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "generate_validation_rules")]
fn py_generate_validation_rules(py: Python<'_>, content: &str) -> PyResult<PyObject> {
    let smst = extract_smst(content);
    let rules = generate_validation_rules(&smst);

    // Convert to Python dict
    let json_str = serde_json::to_string(&rules)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Rules serialization failed: {}", e)))?;
    let json_module = py.import("json")?;
    let py_dict = json_module.call_method1("loads", (json_str,))?;
    Ok(py_dict.into())
}

/// Generate test scaffold from SMST content
///
/// Args:
///     content: SKILL.md content containing SMST specification
///
/// Returns:
///     dict with 'skill_name', 'module_path', 'test_cases', 'rust_code'
///
/// Example:
///     >>> scaffold = generate_test_scaffold(skill_content)
///     >>> len(scaffold['test_cases'])
///     5
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "generate_test_scaffold")]
fn py_generate_test_scaffold(py: Python<'_>, content: &str) -> PyResult<PyObject> {
    let smst = extract_smst(content);
    let scaffold = generate_test_scaffold(&smst);

    let json_str = serde_json::to_string(&scaffold).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let json_module = py.import("json")?;
    let py_dict = json_module.call_method1("loads", (json_str,))?;
    Ok(py_dict.into())
}

/// Generate Rust code stub from SMST content
///
/// Args:
///     content: SKILL.md content containing SMST specification
///
/// Returns:
///     dict with 'skill_name', 'module_name', 'structs', 'functions', 'full_code'
///
/// Example:
///     >>> stub = generate_rust_stub(skill_content)
///     >>> print(stub['full_code'])
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "generate_rust_stub")]
fn py_generate_rust_stub(py: Python<'_>, content: &str) -> PyResult<PyObject> {
    let smst = extract_smst(content);
    let stub = generate_rust_stub(&smst);

    let json_str = serde_json::to_string(&stub).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let json_module = py.import("json")?;
    let py_dict = json_module.call_method1("loads", (json_str,))?;
    Ok(py_dict.into())
}

/// Generate a decision tree YAML from SMST content
///
/// Args:
///     content: SKILL.md content
///
/// Returns:
///     YAML string of the generated decision tree
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "generate_logic")]
fn py_generate_logic(_py: Python<'_>, content: &str) -> PyResult<String> {
    let smst = extract_smst(content);
    let tree = generate_decision_tree(&smst);
    
    serde_yaml::to_string(&tree)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("YAML serialization failed: {}", e)))
}

// ============================================================================
// State Manager Operations (NEW: C2 Sprint 4)
// ============================================================================

/// Create a new checkpoint manager for a state directory
///
/// Args:
///     state_dir: Path to directory for storing checkpoints
///
/// Returns:
///     dict with 'status' and 'state_dir' on success, or 'error' on failure
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "checkpoint_create_manager")]
fn py_checkpoint_create_manager(py: Python<'_>, state_dir: &str) -> PyResult<PyObject> {
    let dict = PyDict::new(py);

    match CheckpointManager::new(state_dir) {
        Ok(_) => {
            dict.set_item("status", "success")?;
            dict.set_item("state_dir", state_dir)?;
        }
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
        }
    }

    Ok(dict.into())
}

/// Create a new execution context
///
/// Args:
///     name: Human-readable name for the execution
///     total_steps: Total number of steps in the pipeline
///
/// Returns:
///     dict with context details including 'id', 'name', 'status', 'total_steps'
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "checkpoint_create_context")]
fn py_checkpoint_create_context(py: Python<'_>, name: &str, total_steps: usize) -> PyResult<PyObject> {
    let ctx = ExecutionContext::new(name, total_steps);

    let json_str = serde_json::to_string(&ctx).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let json_module = py.import("json")?;
    let py_dict = json_module.call_method1("loads", (json_str,))?;
    Ok(py_dict.into())
}

/// Save a context to disk
///
/// Args:
///     state_dir: Path to checkpoint directory
///     context_json: JSON string of the execution context
///
/// Returns:
///     dict with 'status', 'path' on success, or 'error' on failure
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "checkpoint_save")]
fn py_checkpoint_save(py: Python<'_>, state_dir: &str, context_json: &str) -> PyResult<PyObject> {
    let dict = PyDict::new(py);

    let mut manager = match CheckpointManager::new(state_dir) {
        Ok(m) => m,
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            return Ok(dict.into());
        }
    };

    let context: ExecutionContext = match serde_json::from_str(context_json) {
        Ok(c) => c,
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", format!("Invalid context JSON: {}", e))?;
            return Ok(dict.into());
        }
    };

    match manager.save(&context) {
        Ok(path) => {
            dict.set_item("status", "success")?;
            dict.set_item("path", path.to_string_lossy().to_string())?;
        }
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
        }
    }

    Ok(dict.into())
}

/// Load a context by ID
///
/// Args:
///     state_dir: Path to checkpoint directory
///     context_id: Unique identifier of the context
///
/// Returns:
///     dict with context details or None if not found
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "checkpoint_load")]
fn py_checkpoint_load(py: Python<'_>, state_dir: &str, context_id: &str) -> PyResult<PyObject> {
    let dict = PyDict::new(py);

    let manager = match CheckpointManager::new(state_dir) {
        Ok(m) => m,
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            return Ok(dict.into());
        }
    };

    match manager.load(context_id) {
        Ok(Some(ctx)) => {
            let json_str = serde_json::to_string(&ctx).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let json_module = py.import("json")?;
            let py_dict = json_module.call_method1("loads", (json_str,))?;
            Ok(py_dict.into())
        }
        Ok(None) => {
            dict.set_item("status", "not_found")?;
            dict.set_item("context_id", context_id)?;
            Ok(dict.into())
        }
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            Ok(dict.into())
        }
    }
}

/// Find a resumable context by name
///
/// Args:
///     state_dir: Path to checkpoint directory
///     name: Name of the pipeline to find
///
/// Returns:
///     dict with context details or None if no resumable context found
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "checkpoint_find_resumable")]
fn py_checkpoint_find_resumable(py: Python<'_>, state_dir: &str, name: &str) -> PyResult<PyObject> {
    let dict = PyDict::new(py);

    let manager = match CheckpointManager::new(state_dir) {
        Ok(m) => m,
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            return Ok(dict.into());
        }
    };

    match manager.find_resumable(name) {
        Ok(Some(ctx)) => {
            let json_str = serde_json::to_string(&ctx).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let json_module = py.import("json")?;
            let py_dict = json_module.call_method1("loads", (json_str,))?;
            Ok(py_dict.into())
        }
        Ok(None) => {
            dict.set_item("status", "not_found")?;
            dict.set_item("name", name)?;
            Ok(dict.into())
        }
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            Ok(dict.into())
        }
    }
}

/// List all checkpoints in a directory
///
/// Args:
///     state_dir: Path to checkpoint directory
///
/// Returns:
///     dict with 'contexts' (list) and 'count'
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "checkpoint_list")]
fn py_checkpoint_list(py: Python<'_>, state_dir: &str) -> PyResult<PyObject> {
    let dict = PyDict::new(py);

    let manager = match CheckpointManager::new(state_dir) {
        Ok(m) => m,
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            return Ok(dict.into());
        }
    };

    match manager.list() {
        Ok(contexts) => {
            let json_str = serde_json::to_string(&contexts).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let json_module = py.import("json")?;
            let py_list = json_module.call_method1("loads", (json_str,))?;
            dict.set_item("contexts", py_list)?;
            dict.set_item("count", contexts.len())?;
            dict.set_item("status", "success")?;
        }
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
        }
    }

    Ok(dict.into())
}

/// Get checkpoint statistics
///
/// Args:
///     state_dir: Path to checkpoint directory
///
/// Returns:
///     dict with 'total', 'created', 'running', 'paused', 'completed', 'failed', 'cancelled'
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "checkpoint_stats")]
fn py_checkpoint_stats(py: Python<'_>, state_dir: &str) -> PyResult<PyObject> {
    let dict = PyDict::new(py);

    let manager = match CheckpointManager::new(state_dir) {
        Ok(m) => m,
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            return Ok(dict.into());
        }
    };

    match manager.stats() {
        Ok(stats) => {
            dict.set_item("total", stats.total)?;
            dict.set_item("created", stats.created)?;
            dict.set_item("running", stats.running)?;
            dict.set_item("paused", stats.paused)?;
            dict.set_item("completed", stats.completed)?;
            dict.set_item("failed", stats.failed)?;
            dict.set_item("cancelled", stats.cancelled)?;
            dict.set_item("status", "success")?;
        }
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
        }
    }

    Ok(dict.into())
}

/// Delete a checkpoint by ID
///
/// Args:
///     state_dir: Path to checkpoint directory
///     context_id: ID of the context to delete
///
/// Returns:
///     dict with 'deleted' (bool) and 'status'
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "checkpoint_delete")]
fn py_checkpoint_delete(py: Python<'_>, state_dir: &str, context_id: &str) -> PyResult<PyObject> {
    let dict = PyDict::new(py);

    let mut manager = match CheckpointManager::new(state_dir) {
        Ok(m) => m,
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            return Ok(dict.into());
        }
    };

    match manager.delete(context_id) {
        Ok(deleted) => {
            dict.set_item("deleted", deleted)?;
            dict.set_item("status", "success")?;
        }
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
        }
    }

    Ok(dict.into())
}

/// Cleanup old checkpoints
///
/// Args:
///     state_dir: Path to checkpoint directory
///     max_age_days: Maximum age in days for completed/cancelled checkpoints
///
/// Returns:
///     dict with 'removed' count and 'status'
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "checkpoint_cleanup")]
fn py_checkpoint_cleanup(py: Python<'_>, state_dir: &str, max_age_days: u32) -> PyResult<PyObject> {
    let dict = PyDict::new(py);

    let mut manager = match CheckpointManager::new(state_dir) {
        Ok(m) => m,
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            return Ok(dict.into());
        }
    };

    match manager.cleanup(max_age_days) {
        Ok(removed) => {
            dict.set_item("removed", removed)?;
            dict.set_item("status", "success")?;
        }
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
        }
    }

    Ok(dict.into())
}

// ============================================================================
// JSON Processing Operations (NEW: C2 Sprint 5)
// ============================================================================

/// Parse JSON string to Python dict/list
///
/// Args:
///     content: JSON content as string
///
/// Returns:
///     dict with 'status', 'data', 'keys', 'depth', 'value_type'
///
/// Example:
///     >>> result = parse_json_string('{"name": "test", "value": 42}')
///     >>> result['data']['name']
///     'test'
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "parse_json_string")]
fn py_parse_json_string(py: Python<'_>, content: &str) -> PyResult<PyObject> {
    match parse_json(content) {
        Ok(result) => {
            let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let json_module = py.import("json")?;
            let py_dict = json_module.call_method1("loads", (json_str,))?;
            Ok(py_dict.into())
        }
        Err(e) => {
            let dict = PyDict::new(py);
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            Ok(dict.into())
        }
    }
}

/// Serialize Python dict/list to JSON string
///
/// Args:
///     data: Python dict, list, or primitive to serialize
///     pretty: Whether to format with indentation (default: False)
///
/// Returns:
///     dict with 'status', 'json', 'size_bytes', 'pretty'
///
/// Example:
///     >>> result = serialize_json({"name": "test"}, pretty=True)
///     >>> print(result['json'])
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "serialize_json", signature = (data, pretty=false))]
fn py_serialize_json(py: Python<'_>, data: &Bound<'_, pyo3::PyAny>, pretty: bool) -> PyResult<PyObject> {
    // Convert Python object to JSON string, then parse to serde_json::Value
    let json_module = py.import("json")?;
    let json_str: String = json_module.call_method1("dumps", (data,))?.extract()?;

    let value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    match serialize_json(&value, pretty) {
        Ok(result) => {
            let dict = PyDict::new(py);
            dict.set_item("status", "success")?;
            dict.set_item("json", &result.json)?;
            dict.set_item("size_bytes", result.size_bytes)?;
            dict.set_item("pretty", result.pretty)?;
            Ok(dict.into())
        }
        Err(e) => {
            let dict = PyDict::new(py);
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            Ok(dict.into())
        }
    }
}

/// Query a value at a JSON path (dot/bracket notation)
///
/// Args:
///     data: JSON data (dict or list)
///     path: Path to query (e.g., "a.b.c" or "items[0].name")
///
/// Returns:
///     dict with 'status', 'found', 'value', 'path', 'value_type'
///
/// Example:
///     >>> result = json_query({"users": [{"name": "Alice"}]}, "users[0].name")
///     >>> result['value']
///     'Alice'
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "json_query")]
fn py_json_query(py: Python<'_>, data: &Bound<'_, pyo3::PyAny>, path: &str) -> PyResult<PyObject> {
    // Convert Python object to serde_json::Value
    let json_module = py.import("json")?;
    let json_str: String = json_module.call_method1("dumps", (data,))?.extract()?;
    let value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let result = query_path(&value, path);

    let dict = PyDict::new(py);
    dict.set_item("status", &result.status)?;
    dict.set_item("found", result.found)?;
    dict.set_item("path", &result.path)?;

    if let Some(v) = result.value {
        let value_str = serde_json::to_string(&v).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let py_value = json_module.call_method1("loads", (value_str,))?;
        dict.set_item("value", py_value)?;
    } else {
        dict.set_item("value", py.None())?;
    }

    if let Some(vt) = result.value_type {
        dict.set_item("value_type", vt)?;
    } else {
        dict.set_item("value_type", py.None())?;
    }

    Ok(dict.into())
}

/// Set a value at a JSON path (creates intermediate objects/arrays)
///
/// Args:
///     data: JSON data (dict or list) to modify
///     path: Path to set (e.g., "a.b.c" or "items[0]")
///     value: Value to set
///
/// Returns:
///     Modified data structure
///
/// Example:
///     >>> result = json_set({}, "a.b.c", 42)
///     >>> result['a']['b']['c']
///     42
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "json_set")]
fn py_json_set(py: Python<'_>, data: &Bound<'_, pyo3::PyAny>, path: &str, new_value: &Bound<'_, pyo3::PyAny>) -> PyResult<PyObject> {
    let json_module = py.import("json")?;

    // Convert data to serde_json::Value
    let data_str: String = json_module.call_method1("dumps", (data,))?.extract()?;
    let mut json_data: serde_json::Value = serde_json::from_str(&data_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    // Convert new_value to serde_json::Value
    let value_str: String = json_module.call_method1("dumps", (new_value,))?.extract()?;
    let json_value: serde_json::Value = serde_json::from_str(&value_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    set_path(&mut json_data, path, json_value)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    // Convert back to Python
    let result_str = serde_json::to_string(&json_data).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let py_result = json_module.call_method1("loads", (result_str,))?;
    Ok(py_result.into())
}

/// Deep merge two JSON objects
///
/// Args:
///     target: Base JSON object
///     source: JSON object to merge into target
///
/// Returns:
///     dict with 'status', 'data', 'keys_added', 'keys_overwritten'
///
/// Example:
///     >>> result = json_merge({"a": 1}, {"b": 2})
///     >>> result['data']
///     {'a': 1, 'b': 2}
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "json_merge")]
fn py_json_merge(py: Python<'_>, target: &Bound<'_, pyo3::PyAny>, source: &Bound<'_, pyo3::PyAny>) -> PyResult<PyObject> {
    let json_module = py.import("json")?;

    let target_str: String = json_module.call_method1("dumps", (target,))?.extract()?;
    let target_value: serde_json::Value = serde_json::from_str(&target_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let source_str: String = json_module.call_method1("dumps", (source,))?.extract()?;
    let source_value: serde_json::Value = serde_json::from_str(&source_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let result = merge_json(&target_value, &source_value);

    let dict = PyDict::new(py);
    dict.set_item("status", &result.status)?;
    dict.set_item("keys_added", result.keys_added)?;
    dict.set_item("keys_overwritten", result.keys_overwritten)?;

    let data_str = serde_json::to_string(&result.data).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let py_data = json_module.call_method1("loads", (data_str,))?;
    dict.set_item("data", py_data)?;

    Ok(dict.into())
}

/// Compare two JSON values and return differences
///
/// Args:
///     left: First JSON value
///     right: Second JSON value
///
/// Returns:
///     dict with 'status', 'added', 'removed', 'modified', 'unchanged'
///
/// Example:
///     >>> result = json_diff({"a": 1, "b": 2}, {"a": 1, "c": 3})
///     >>> result['removed']
///     ['b']
///     >>> result['added']
///     ['c']
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "json_diff")]
fn py_json_diff(py: Python<'_>, left: &Bound<'_, pyo3::PyAny>, right: &Bound<'_, pyo3::PyAny>) -> PyResult<PyObject> {
    let json_module = py.import("json")?;

    let left_str: String = json_module.call_method1("dumps", (left,))?.extract()?;
    let left_value: serde_json::Value = serde_json::from_str(&left_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let right_str: String = json_module.call_method1("dumps", (right,))?.extract()?;
    let right_value: serde_json::Value = serde_json::from_str(&right_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let result = diff_json(&left_value, &right_value);

    let dict = PyDict::new(py);
    dict.set_item("status", &result.status)?;
    dict.set_item("added", &result.added)?;
    dict.set_item("removed", &result.removed)?;
    dict.set_item("modified", &result.modified)?;
    dict.set_item("unchanged", &result.unchanged)?;

    Ok(dict.into())
}

/// Flatten nested JSON into dot-notation keys
///
/// Args:
///     data: Nested JSON object
///
/// Returns:
///     dict with 'status', 'data' (flattened), 'total_keys'
///
/// Example:
///     >>> result = json_flatten({"a": {"b": 1}})
///     >>> result['data']['a.b']
///     1
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "json_flatten")]
fn py_json_flatten(py: Python<'_>, data: &Bound<'_, pyo3::PyAny>) -> PyResult<PyObject> {
    let json_module = py.import("json")?;

    let data_str: String = json_module.call_method1("dumps", (data,))?.extract()?;
    let value: serde_json::Value = serde_json::from_str(&data_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let result = flatten_json(&value);

    let dict = PyDict::new(py);
    dict.set_item("status", &result.status)?;
    dict.set_item("total_keys", result.total_keys)?;

    // Convert HashMap to Python dict
    let flat_str = serde_json::to_string(&result.data).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let py_flat = json_module.call_method1("loads", (flat_str,))?;
    dict.set_item("data", py_flat)?;

    Ok(dict.into())
}

/// Unflatten dot-notation keys back into nested JSON
///
/// Args:
///     data: Flattened dict with dot-notation keys
///
/// Returns:
///     Nested JSON structure
///
/// Example:
///     >>> result = json_unflatten({"a.b": 1, "a.c": 2})
///     >>> result['a']['b']
///     1
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "json_unflatten")]
fn py_json_unflatten(py: Python<'_>, data: std::collections::HashMap<String, pyo3::Py<pyo3::PyAny>>) -> PyResult<PyObject> {
    let json_module = py.import("json")?;

    // Convert Python values to serde_json::Value
    let mut rust_map: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
    for (key, py_value) in data {
        let value_str: String = json_module.call_method1("dumps", (py_value,))?.extract()?;
        let json_value: serde_json::Value = serde_json::from_str(&value_str)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        rust_map.insert(key, json_value);
    }

    match unflatten_json(&rust_map) {
        Ok(result) => {
            let result_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let py_result = json_module.call_method1("loads", (result_str,))?;
            Ok(py_result.into())
        }
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
    }
}

// ============================================================================
// Decision Engine Operations (NEW: C2 Sprint 6)
// ============================================================================

/// Execute a decision tree with a given context
///
/// Args:
///     tree_yaml: Decision tree definition in YAML format
///     inputs: Initial context variables as a dict
///
/// Returns:
///     dict with 'status', 'value' (if successful), 'execution_path', and 'error' info
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "execute_logic")]
fn py_execute_logic(py: Python<'_>, tree_yaml: &str, inputs: &Bound<'_, pyo3::PyAny>) -> PyResult<PyObject> {
    let json_module = py.import("json")?;

    // Parse tree from YAML
    let tree: DecisionTree = serde_yaml::from_str(tree_yaml)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid decision tree YAML: {}", e)))?;

    // Convert inputs dict to HashMap<String, Value> via JSON
    let inputs_str: String = json_module.call_method1("dumps", (inputs,))?.extract()?;
    let variables: HashMap<String, Value> = serde_json::from_str(&inputs_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid inputs format: {}", e)))?;

    let mut context = DecisionContext {
        variables,
        execution_path: Vec::new(),
    };

    let engine = DecisionEngine::new(tree);
    let result = engine.execute(&mut context);

    let dict = PyDict::new(py);
    dict.set_item("execution_path", &context.execution_path)?;

    match result {
        ExecutionResult::Value(val) => {
            dict.set_item("status", "success")?;
            let val_json = serde_json::to_string(&val).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let py_val = json_module.call_method1("loads", (val_json,))?;
            dict.set_item("value", py_val)?;
        },
        ExecutionResult::LlmRequest { prompt, context: llm_ctx } => {
            dict.set_item("status", "llm_fallback")?;
            dict.set_item("prompt", prompt)?;
            let ctx_json = serde_json::to_string(&llm_ctx).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let py_ctx = json_module.call_method1("loads", (ctx_json,))?;
            dict.set_item("context", py_ctx)?;
        },
        ExecutionResult::Error(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e)?;
        }
    }

    Ok(dict.into())
}

// ============================================================================
// Skill Builder Operations (NEW: C2 Sprint 7)
// ============================================================================

/// Build a skill from its directory
///
/// Args:
///     path: Path to the skill directory
///     dry_run: Whether to perform a dry run (default: False)
///
/// Returns:
///     dict with build results
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "build_skill", signature = (path, dry_run=false))]
fn py_build_skill(py: Python<'_>, path: &str, dry_run: bool) -> PyResult<PyObject> {
    let p = std::path::Path::new(path);
    let result = build_skill(p, dry_run);

    let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let json_module = py.import("json")?;
    let py_dict = json_module.call_method1("loads", (json_str,))?;
    Ok(py_dict.into())
}

/// Verify a skill for compliance
///
/// Args:
///     path: Path to the skill directory
///
/// Returns:
///     dict with verification results
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "verify_skill")]
fn py_verify_skill(py: Python<'_>, path: &str) -> PyResult<PyObject> {
    let p = std::path::Path::new(path);
    let result = verify_skill(p);

    let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let json_module = py.import("json")?;
    let py_dict = json_module.call_method1("loads", (json_str,))?;
    Ok(py_dict.into())
}

// ============================================================================
// Chain Operations (NEW: Rust Migration)
// ============================================================================

/// Parse an inline chain definition
///
/// Args:
///     input: Inline chain syntax (e.g., "skill1 -> skill2 -> skill3")
///
/// Returns:
///     dict with 'status', 'name', 'steps', 'step_count' on success,
///     or 'error' on failure
///
/// Example:
///     >>> result = parse_chain_inline("analyze -> transform -> output")
///     >>> result['step_count']
///     3
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "parse_chain_inline")]
fn py_parse_chain_inline(py: Python<'_>, input: &str) -> PyResult<PyObject> {
    let dict = PyDict::new(py);

    match chain_parse_inline(input) {
        Ok(chain) => {
            dict.set_item("status", "success")?;
            dict.set_item("name", &chain.name)?;
            dict.set_item("step_count", chain.len())?;

            // Extract step names
            let step_names: Vec<String> = chain.steps.iter().filter_map(|s| {
                match s {
                    StepType::Regular(step) => Some(step.skill.clone()),
                    StepType::Conditional(c) => Some(format!("conditional:{}", c.then_step.skill)),
                }
            }).collect();
            dict.set_item("steps", step_names)?;

            // Serialize full chain as JSON for advanced use
            let json_module = py.import("json")?;
            let chain_json = serde_json::to_string(&chain).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let py_chain = json_module.call_method1("loads", (chain_json,))?;
            dict.set_item("chain", py_chain)?;
        }
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
        }
    }

    Ok(dict.into())
}

/// Parse a YAML chain definition
///
/// Args:
///     content: YAML chain definition string
///
/// Returns:
///     dict with 'status', 'name', 'steps', 'step_count' on success,
///     or 'error' on failure
///
/// Example:
///     >>> yaml = "name: my-pipeline\\nsteps:\\n  - skill: analyze\\n  - skill: transform"
///     >>> result = parse_chain_yaml(yaml)
///     >>> result['name']
///     'my-pipeline'
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "parse_chain_yaml")]
fn py_parse_chain_yaml(py: Python<'_>, content: &str) -> PyResult<PyObject> {
    let dict = PyDict::new(py);

    match chain_parse_yaml(content) {
        Ok(chain) => {
            dict.set_item("status", "success")?;
            dict.set_item("name", &chain.name)?;
            dict.set_item("step_count", chain.len())?;

            // Extract step names
            let step_names: Vec<String> = chain.steps.iter().filter_map(|s| {
                match s {
                    StepType::Regular(step) => Some(step.skill.clone()),
                    StepType::Conditional(c) => Some(format!("conditional:{}", c.then_step.skill)),
                }
            }).collect();
            dict.set_item("steps", step_names)?;

            // Serialize full chain as JSON for advanced use
            let json_module = py.import("json")?;
            let chain_json = serde_json::to_string(&chain).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let py_chain = json_module.call_method1("loads", (chain_json,))?;
            dict.set_item("chain", py_chain)?;
        }
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
        }
    }

    Ok(dict.into())
}

/// Validate a chain for correctness
///
/// Args:
///     chain_json: JSON representation of a chain (from parse_chain_inline/yaml)
///
/// Returns:
///     dict with 'valid', 'issues' (list), 'warnings', 'errors'
///
/// Example:
///     >>> chain = parse_chain_inline("a -> b")
///     >>> result = validate_chain_json(chain['chain'])
///     >>> result['valid']
///     True
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "validate_chain_json")]
fn py_validate_chain_json(py: Python<'_>, chain_json: &Bound<'_, pyo3::PyAny>) -> PyResult<PyObject> {
    let json_module = py.import("json")?;

    // Convert Python dict to JSON string, then parse to Chain
    let json_str: String = json_module.call_method1("dumps", (chain_json,))?.extract()?;
    let chain: Chain = serde_json::from_str(&json_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid chain JSON: {}", e)))?;

    let validation = validate_chain(&chain);

    let dict = PyDict::new(py);
    dict.set_item("valid", validation.valid)?;

    // Serialize issues
    let issues_json = serde_json::to_string(&validation.issues).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let py_issues = json_module.call_method1("loads", (issues_json,))?;
    dict.set_item("issues", py_issues)?;

    // Count by severity
    let warnings = validation.issues.iter().filter(|i| i.severity == crate::modules::chain::Severity::Warning).count();
    let errors = validation.issues.iter().filter(|i| i.severity == crate::modules::chain::Severity::Error).count();
    dict.set_item("warnings", warnings)?;
    dict.set_item("errors", errors)?;

    Ok(dict.into())
}

/// Execute a chain with a custom skill executor
///
/// Args:
///     chain_json: JSON representation of a chain
///     dry_run: Whether to perform a dry run (default: False)
///     max_parallel: Maximum parallel executions (default: 4)
///
/// Returns:
///     dict with execution results including 'success', 'steps', 'duration_ms'
///
/// Note: This uses a mock executor - for real execution, use the chain module
/// directly from Rust or provide a custom executor via Python callback.
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "execute_chain_dry_run", signature = (chain_json, max_parallel=4))]
fn py_execute_chain_dry_run(py: Python<'_>, chain_json: &Bound<'_, pyo3::PyAny>, max_parallel: usize) -> PyResult<PyObject> {
    let json_module = py.import("json")?;

    // Convert Python dict to Chain
    let json_str: String = json_module.call_method1("dumps", (chain_json,))?.extract()?;
    let chain: Chain = serde_json::from_str(&json_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid chain JSON: {}", e)))?;

    // Configure for dry run
    let config = ExecutorConfig {
        dry_run: true,
        max_parallel,
        ..Default::default()
    };

    // Execute with mock executor (dry run returns would-execute status)
    let result = execute_chain_with_fn(
        &chain,
        |skill, _args, _ctx| SkillExecutionResult {
            success: true,
            output: serde_json::json!({"skill": skill, "status": "would_execute"}),
            error: None,
            duration_ms: 0,
        },
        &config,
    );

    // Serialize result
    let result_json = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let py_result = json_module.call_method1("loads", (result_json,))?;
    Ok(py_result.into())
}

/// Create a new chain programmatically
///
/// Args:
///     name: Chain name
///     steps: List of skill names
///
/// Returns:
///     dict with the chain JSON representation
///
/// Example:
///     >>> chain = create_chain("my-pipeline", ["analyze", "transform", "output"])
///     >>> chain['step_count']
///     3
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "create_chain")]
fn py_create_chain(py: Python<'_>, name: &str, steps: Vec<String>) -> PyResult<PyObject> {
    let mut chain = Chain::new(name);

    for skill_name in steps {
        chain = chain.with_step(StepType::Regular(ChainStep::new(&skill_name)));
    }

    let dict = PyDict::new(py);
    dict.set_item("status", "success")?;
    dict.set_item("name", &chain.name)?;
    dict.set_item("step_count", chain.len())?;

    // Serialize full chain
    let json_module = py.import("json")?;
    let chain_json = serde_json::to_string(&chain).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let py_chain = json_module.call_method1("loads", (chain_json,))?;
    dict.set_item("chain", py_chain)?;

    Ok(dict.into())
}

// ============================================================================
// Statistics Operations (NEW: Rust Migration)
// ============================================================================

/// Chi-square test for independence (2x2 contingency table)
///
/// Args:
///     a: Exposed + event count
///     b: Exposed + no event count
///     c: Not exposed + event count
///     d: Not exposed + no event count
///
/// Returns:
///     dict with test results including p-value, effect size, and epistemic interpretation
///
/// Example:
///     >>> result = chi_square(47, 12000, 23, 45000)
///     >>> result['p_value']
///     0.0012
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "chi_square")]
fn py_chi_square(py: Python<'_>, a: i64, b: i64, c: i64, d: i64) -> PyResult<PyObject> {
    let input = ChiSquareInput { a, b, c, d };
    let result = chi_square_test(&input);

    let json_module = py.import("json")?;
    let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let py_result = json_module.call_method1("loads", (json_str,))?;
    Ok(py_result.into())
}

/// Welch's independent samples t-test
///
/// Args:
///     group1: List of values for group 1
///     group2: List of values for group 2
///
/// Returns:
///     dict with test results including t-statistic, p-value, Cohen's d, and CI
///
/// Example:
///     >>> result = t_test([1, 2, 3, 4, 5], [6, 7, 8, 9, 10])
///     >>> result['p_value'] < 0.05
///     True
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "t_test")]
fn py_t_test(py: Python<'_>, group1: Vec<f64>, group2: Vec<f64>) -> PyResult<PyObject> {
    let input = TTestInput { group1, group2 };

    match t_test_independent(&input) {
        Ok(result) => {
            let json_module = py.import("json")?;
            let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let py_result = json_module.call_method1("loads", (json_str,))?;
            Ok(py_result.into())
        }
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e)),
    }
}

/// One-sample proportion z-test
///
/// Args:
///     successes: Number of successes
///     n: Total sample size
///     null: Null hypothesis proportion (default: 0.5)
///
/// Returns:
///     dict with z-statistic, p-value, CI, and epistemic interpretation
///
/// Example:
///     >>> result = proportion_z_test(60, 100, null=0.5)
///     >>> result['p_value'] < 0.1
///     True
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "proportion_z_test", signature = (successes, n, null=0.5))]
fn py_proportion_z_test(py: Python<'_>, successes: i64, n: i64, null: f64) -> PyResult<PyObject> {
    let input = ProportionInput {
        successes,
        n,
        null: Some(null),
    };

    match proportion_test(&input) {
        Ok(result) => {
            let json_module = py.import("json")?;
            let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let py_result = json_module.call_method1("loads", (json_str,))?;
            Ok(py_result.into())
        }
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e)),
    }
}

/// Pearson correlation with significance test
///
/// Args:
///     x: List of x values
///     y: List of y values
///
/// Returns:
///     dict with correlation coefficient, p-value, CI, and interpretation
///
/// Example:
///     >>> result = pearson_correlation([1, 2, 3, 4, 5], [2, 4, 6, 8, 10])
///     >>> result['test_statistic']  # r value
///     1.0
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "pearson_correlation")]
fn py_pearson_correlation(py: Python<'_>, x: Vec<f64>, y: Vec<f64>) -> PyResult<PyObject> {
    let input = CorrelationInput { x, y };

    match correlation_test(&input) {
        Ok(result) => {
            let json_module = py.import("json")?;
            let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let py_result = json_module.call_method1("loads", (json_str,))?;
            Ok(py_result.into())
        }
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e)),
    }
}

// ============================================================================
// Anti-Pattern Detection Operations (NEW: Rust Migration)
// ============================================================================

/// Detect anti-patterns in code/process artifacts
///
/// Args:
///     features: Dict with numeric features (e.g., {"method_count": 25, "line_count": 500})
///     context: Dict with boolean context (e.g., {"is_critical_path": True})
///     threshold: Detection confidence threshold (default: 0.3)
///     artifact_name: Name of artifact being analyzed
///
/// Returns:
///     dict with detection results including health status and pattern matches
///
/// Example:
///     >>> result = detect_anti_patterns(
///     ...     {"method_count": 25, "line_count": 600},
///     ...     {"is_critical_path": True},
///     ... )
///     >>> result['overall_health']
///     'NEEDS_ATTENTION'
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "detect_anti_patterns", signature = (features, context, threshold=0.3, artifact_name="artifact"))]
fn py_detect_anti_patterns(
    py: Python<'_>,
    features: HashMap<String, f64>,
    context: HashMap<String, bool>,
    threshold: f64,
    artifact_name: &str,
) -> PyResult<PyObject> {
    // Use built-in patterns for now
    let patterns = vec![
        create_god_object_pattern(),
        create_paper_constructs_pattern(),
    ];

    let mut feat = Features::default();
    feat.numeric = features;

    let config = DetectionConfig {
        threshold,
        artifact_name: artifact_name.to_string(),
        categories: None,
    };

    let result = detect_anti_patterns(&feat, &context, &patterns, &config);

    let json_module = py.import("json")?;
    let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let py_result = json_module.call_method1("loads", (json_str,))?;
    Ok(py_result.into())
}

/// Detect anti-patterns with custom patterns
///
/// Args:
///     features: Dict with numeric features
///     text_features: Dict with text features (e.g., {"text_content": "// TODO: fix"})
///     context: Dict with boolean context
///     patterns_json: JSON string of pattern definitions
///     threshold: Detection confidence threshold (default: 0.3)
///     artifact_name: Name of artifact being analyzed
///
/// Returns:
///     dict with detection results
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "detect_anti_patterns_custom", signature = (features, text_features, context, patterns_json, threshold=0.3, artifact_name="artifact"))]
fn py_detect_anti_patterns_custom(
    py: Python<'_>,
    features: HashMap<String, f64>,
    text_features: HashMap<String, String>,
    context: HashMap<String, bool>,
    patterns_json: &str,
    threshold: f64,
    artifact_name: &str,
) -> PyResult<PyObject> {
    // Parse patterns from JSON
    let patterns: Vec<AntiPattern> = serde_json::from_str(patterns_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid patterns JSON: {}", e)))?;

    let mut feat = Features::default();
    feat.numeric = features;
    feat.text = text_features;

    let config = DetectionConfig {
        threshold,
        artifact_name: artifact_name.to_string(),
        categories: None,
    };

    let result = detect_anti_patterns(&feat, &context, &patterns, &config);

    let json_module = py.import("json")?;
    let json_str = serde_json::to_string(&result).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let py_result = json_module.call_method1("loads", (json_str,))?;
    Ok(py_result.into())
}

/// Get built-in anti-pattern definitions as JSON
///
/// Returns:
///     JSON string containing built-in pattern definitions
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "get_builtin_patterns")]
fn py_get_builtin_patterns(py: Python<'_>) -> PyResult<PyObject> {
    let patterns = vec![
        create_god_object_pattern(),
        create_paper_constructs_pattern(),
    ];

    let json_module = py.import("json")?;
    let json_str = serde_json::to_string(&patterns).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let py_result = json_module.call_method1("loads", (json_str,))?;
    Ok(py_result.into())
}

// ============================================================================
// Session Tracker Operations (NEW: Rust Migration - skill_router.py consolidation)
// ============================================================================

/// Load session state from a JSON file
///
/// Args:
///     path: Path to the session state file
///
/// Returns:
///     dict with session state including 'session_id', 'current_skill', 'execution_history'
///
/// Example:
///     >>> state = session_load("~/.claude/skills/my-skill/session-state.json")
///     >>> state['current_skill']
///     'my-skill'
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "session_load")]
fn py_session_load(py: Python<'_>, path: &str) -> PyResult<PyObject> {
    let p = std::path::Path::new(path);

    match session_load(p) {
        Ok(state) => {
            let json_module = py.import("json")?;
            let json_str = serde_json::to_string(&state).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let py_result = json_module.call_method1("loads", (json_str,))?;
            Ok(py_result.into())
        }
        Err(e) => {
            let dict = PyDict::new(py);
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            Ok(dict.into())
        }
    }
}

/// Save session state to a JSON file (atomic write)
///
/// Args:
///     path: Path to save the session state
///     state_json: JSON string or dict of the session state
///
/// Returns:
///     dict with 'status' ("success" or "error")
///
/// Example:
///     >>> result = session_save("/path/to/state.json", {"session_id": "abc", "execution_history": []})
///     >>> result['status']
///     'success'
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "session_save")]
fn py_session_save(py: Python<'_>, path: &str, state_json: &Bound<'_, pyo3::PyAny>) -> PyResult<PyObject> {
    let json_module = py.import("json")?;
    let dict = PyDict::new(py);

    // Convert Python dict to SessionState
    let json_str: String = json_module.call_method1("dumps", (state_json,))?.extract()?;
    let state: SessionState = serde_json::from_str(&json_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid state JSON: {}", e)))?;

    let p = std::path::Path::new(path);

    match session_save(p, &state) {
        Ok(()) => {
            dict.set_item("status", "success")?;
        }
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
        }
    }

    Ok(dict.into())
}

/// Track a skill execution (load, update, save in one operation)
///
/// Args:
///     state_path: Path to the session state file
///     skill_name: Name of the skill being executed
///     context: Optional context/notes for the execution
///
/// Returns:
///     dict with updated session state
///
/// Example:
///     >>> state = session_track_execution("/path/to/state.json", "my-skill", "Processing data")
///     >>> len(state['execution_history'])
///     1
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "session_track_execution", signature = (state_path, skill_name, context=None))]
fn py_session_track_execution(
    py: Python<'_>,
    state_path: &str,
    skill_name: &str,
    context: Option<&str>,
) -> PyResult<PyObject> {
    let p = std::path::Path::new(state_path);

    match session_track(p, skill_name, context) {
        Ok(state) => {
            let json_module = py.import("json")?;
            let json_str = serde_json::to_string(&state).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let py_result = json_module.call_method1("loads", (json_str,))?;
            Ok(py_result.into())
        }
        Err(e) => {
            let dict = PyDict::new(py);
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            Ok(dict.into())
        }
    }
}

/// Track execution completion
///
/// Args:
///     state_path: Path to the session state file
///     duration_ms: Optional duration in milliseconds
///
/// Returns:
///     dict with updated session state
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "session_complete", signature = (state_path, duration_ms=None))]
fn py_session_complete(
    py: Python<'_>,
    state_path: &str,
    duration_ms: Option<u64>,
) -> PyResult<PyObject> {
    let p = std::path::Path::new(state_path);

    match session_complete(p, duration_ms) {
        Ok(state) => {
            let json_module = py.import("json")?;
            let json_str = serde_json::to_string(&state).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let py_result = json_module.call_method1("loads", (json_str,))?;
            Ok(py_result.into())
        }
        Err(e) => {
            let dict = PyDict::new(py);
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            Ok(dict.into())
        }
    }
}

/// Track execution failure
///
/// Args:
///     state_path: Path to the session state file
///     error: Optional error message
///
/// Returns:
///     dict with updated session state
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "session_fail", signature = (state_path, error=None))]
fn py_session_fail(
    py: Python<'_>,
    state_path: &str,
    error: Option<&str>,
) -> PyResult<PyObject> {
    let p = std::path::Path::new(state_path);

    match session_fail(p, error) {
        Ok(state) => {
            let json_module = py.import("json")?;
            let json_str = serde_json::to_string(&state).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let py_result = json_module.call_method1("loads", (json_str,))?;
            Ok(py_result.into())
        }
        Err(e) => {
            let dict = PyDict::new(py);
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            Ok(dict.into())
        }
    }
}

/// Append to log file
///
/// Args:
///     log_path: Path to the log file
///     skill_name: Name of the skill
///     message: Log message
///
/// Returns:
///     dict with 'status'
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "session_append_log")]
fn py_session_append_log(py: Python<'_>, log_path: &str, skill_name: &str, message: &str) -> PyResult<PyObject> {
    let p = std::path::Path::new(log_path);
    let dict = PyDict::new(py);

    match session_log(p, skill_name, message) {
        Ok(()) => {
            dict.set_item("status", "success")?;
        }
        Err(e) => {
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
        }
    }

    Ok(dict.into())
}

/// Full skill router replacement: track + log in one call
///
/// This is the primary replacement for skill_router.py files.
/// It tracks skill execution and optionally logs to a file.
///
/// Args:
///     state_path: Path to the session state file
///     log_path: Optional path to log file
///     skill_name: Name of the skill being executed
///     context: Optional context/notes
///
/// Returns:
///     dict with updated session state
///
/// Example:
///     >>> state = session_route(
///     ...     "~/.claude/skills/my-skill/session-state.json",
///     ...     "~/.claude/skills/skill-execution.log",
///     ...     "my-skill",
///     ...     "Starting execution"
///     ... )
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "session_route", signature = (state_path, log_path=None, skill_name="unknown", context=None))]
fn py_session_route(
    py: Python<'_>,
    state_path: &str,
    log_path: Option<&str>,
    skill_name: &str,
    context: Option<&str>,
) -> PyResult<PyObject> {
    let state_p = std::path::Path::new(state_path);
    let log_p = log_path.map(std::path::Path::new);

    match session_route(state_p, log_p, skill_name, context) {
        Ok(state) => {
            let json_module = py.import("json")?;
            let json_str = serde_json::to_string(&state).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let py_result = json_module.call_method1("loads", (json_str,))?;
            Ok(py_result.into())
        }
        Err(e) => {
            let dict = PyDict::new(py);
            dict.set_item("status", "error")?;
            dict.set_item("error", e.to_string())?;
            Ok(dict.into())
        }
    }
}

// ============================================================================
// Epistemic Rigor Operations
// ============================================================================

/// Validate a claim for epistemic rigor
///
/// Checks for overconfident language and missing citations.
///
/// Args:
///     claim: The claim text to validate
///
/// Returns:
///     dict with validation results including:
///     - valid: Whether the claim passes validation
///     - issues: List of issues found
///     - suggestions: List of suggestions for improvement
///     - confidence_level: "high", "medium", or "low"
///     - detected_words: List of overconfident words found
///
/// Example:
///     >>> result = validate_epistemic_claim("This will always work perfectly.")
///     >>> print(result['valid'])  # False
///     >>> print(result['issues'])  # ["Uses overconfident language: 'always'"]
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "validate_epistemic_claim")]
fn py_validate_epistemic_claim(py: Python<'_>, claim: &str) -> PyResult<PyObject> {
    let result = validate_claim(claim);
    let dict = PyDict::new(py);
    dict.set_item("claim", result.claim)?;
    dict.set_item("valid", result.valid)?;
    dict.set_item("issues", result.issues)?;
    dict.set_item("suggestions", result.suggestions)?;
    dict.set_item("confidence_level", match result.confidence_level {
        ConfidenceLevel::High => "high",
        ConfidenceLevel::Medium => "medium",
        ConfidenceLevel::Low => "low",
    })?;
    dict.set_item("detected_words", result.detected_words)?;
    Ok(dict.into())
}

/// Batch validate multiple claims for epistemic rigor
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "validate_epistemic_claims")]
fn py_validate_epistemic_claims(py: Python<'_>, claims: Vec<String>) -> PyResult<PyObject> {
    let claim_refs: Vec<&str> = claims.iter().map(|s| s.as_str()).collect();
    let results = validate_claims(&claim_refs);
    let json_module = py.import("json")?;
    let json_str = serde_json::to_string(&results).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let py_result = json_module.call_method1("loads", (json_str,))?;
    Ok(py_result.into())
}

/// Log skill execution details to the unified kernel telemetry stream
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "log_skill_execution")]
fn py_log_skill_execution(py: Python<'_>, skill_name: &str, session_id: &str, status: &str) -> PyResult<PyObject> {
    tracing::info!(
        skill = %skill_name,
        session = %session_id,
        status = %status,
        "Skill execution tracked"
    );
    let dict = PyDict::new(py);
    dict.set_item("status", "logged")?;
    Ok(dict.into())
}

/// Compile a decision tree YAML into deterministic Rust code
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "compile_logic_to_rust")]
fn py_compile_logic_to_rust(_py: Python<'_>, yaml_content: &str) -> PyResult<String> {
    let tree = crate::modules::decision_engine::load_tree(yaml_content)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    Ok(crate::modules::evolution::compile_logic_to_rust(&tree))
}

/// Get hedging suggestions for overconfident words

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "get_hedging_suggestions")]
fn py_get_hedging_suggestions(py: Python<'_>) -> PyResult<PyObject> {
    let suggestions = get_hedging_suggestions();
    let dict = PyDict::new(py);
    for (word, alternatives) in suggestions {
        dict.set_item(word, alternatives)?;
    }
    Ok(dict.into())
}

// ============================================================================
// Module Registration
// ============================================================================

/// Python module definition
#[cfg(feature = "python")]
#[pymodule]
pub fn rsk(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Levenshtein operations
    m.add_function(wrap_pyfunction!(py_levenshtein, m)?)?;
    m.add_function(wrap_pyfunction!(py_fuzzy_search, m)?)?;

    // Crypto operations
    m.add_function(wrap_pyfunction!(py_sha256, m)?)?;
    m.add_function(wrap_pyfunction!(py_sha256_verify, m)?)?;

    // Math operations
    m.add_function(wrap_pyfunction!(py_variance, m)?)?;
    m.add_function(wrap_pyfunction!(py_is_prime, m)?)?;
    m.add_function(wrap_pyfunction!(py_classify_intent, m)?)?;

    // Taxonomy operations
    m.add_function(wrap_pyfunction!(py_query_taxonomy, m)?)?;
    m.add_function(wrap_pyfunction!(py_list_taxonomy, m)?)?;

    // SKILL.md operations
    m.add_function(wrap_pyfunction!(py_extract_smst, m)?)?;
    m.add_function(wrap_pyfunction!(py_parse_frontmatter, m)?)?;

    // Text processing operations
    m.add_function(wrap_pyfunction!(py_tokenize, m)?)?;
    m.add_function(wrap_pyfunction!(py_normalize, m)?)?;
    m.add_function(wrap_pyfunction!(py_word_frequency, m)?)?;
    m.add_function(wrap_pyfunction!(py_text_entropy, m)?)?;

    // Compression operations
    m.add_function(wrap_pyfunction!(py_gzip_compress, m)?)?;
    m.add_function(wrap_pyfunction!(py_gzip_decompress, m)?)?;
    m.add_function(wrap_pyfunction!(py_estimate_compressibility, m)?)?;

    // Graph operations (NEW: C2 Sprint 1)
    m.add_function(wrap_pyfunction!(py_topological_sort, m)?)?;
    m.add_function(wrap_pyfunction!(py_level_parallelization, m)?)?;
    m.add_function(wrap_pyfunction!(py_shortest_path, m)?)?;

    // YAML operations (NEW: C2 Sprint 1)
    m.add_function(wrap_pyfunction!(py_parse_yaml_string, m)?)?;

    // Execution engine operations (NEW: C2 Sprint 2)
    m.add_function(wrap_pyfunction!(py_build_execution_plan, m)?)?;

    // Code generator operations (NEW: C2 Sprint 4)
    m.add_function(wrap_pyfunction!(py_generate_validation_rules, m)?)?;
    m.add_function(wrap_pyfunction!(py_generate_test_scaffold, m)?)?;
    m.add_function(wrap_pyfunction!(py_generate_rust_stub, m)?)?;
    m.add_function(wrap_pyfunction!(py_generate_logic, m)?)?;

    // State manager operations (NEW: C2 Sprint 4)
    m.add_function(wrap_pyfunction!(py_checkpoint_create_manager, m)?)?;
    m.add_function(wrap_pyfunction!(py_checkpoint_create_context, m)?)?;
    m.add_function(wrap_pyfunction!(py_checkpoint_save, m)?)?;
    m.add_function(wrap_pyfunction!(py_checkpoint_load, m)?)?;
    m.add_function(wrap_pyfunction!(py_checkpoint_find_resumable, m)?)?;
    m.add_function(wrap_pyfunction!(py_checkpoint_list, m)?)?;
    m.add_function(wrap_pyfunction!(py_checkpoint_stats, m)?)?;
    m.add_function(wrap_pyfunction!(py_checkpoint_delete, m)?)?;
    m.add_function(wrap_pyfunction!(py_checkpoint_cleanup, m)?)?;

    // JSON processing operations (NEW: C2 Sprint 5)
    m.add_function(wrap_pyfunction!(py_parse_json_string, m)?)?;
    m.add_function(wrap_pyfunction!(py_serialize_json, m)?)?;
    m.add_function(wrap_pyfunction!(py_json_query, m)?)?;
    m.add_function(wrap_pyfunction!(py_json_set, m)?)?;
    m.add_function(wrap_pyfunction!(py_json_merge, m)?)?;
    m.add_function(wrap_pyfunction!(py_json_diff, m)?)?;
    m.add_function(wrap_pyfunction!(py_json_flatten, m)?)?;
    m.add_function(wrap_pyfunction!(py_json_unflatten, m)?)?;

    // Decision Engine operations (NEW: C2 Sprint 6)
    m.add_function(wrap_pyfunction!(py_execute_logic, m)?)?;

    // Skill Builder operations (NEW: C2 Sprint 7)
    m.add_function(wrap_pyfunction!(py_build_skill, m)?)?;
    m.add_function(wrap_pyfunction!(py_verify_skill, m)?)?;

    // Chain operations (NEW: Rust Migration)
    m.add_function(wrap_pyfunction!(py_parse_chain_inline, m)?)?;
    m.add_function(wrap_pyfunction!(py_parse_chain_yaml, m)?)?;
    m.add_function(wrap_pyfunction!(py_validate_chain_json, m)?)?;
    m.add_function(wrap_pyfunction!(py_execute_chain_dry_run, m)?)?;
    m.add_function(wrap_pyfunction!(py_create_chain, m)?)?;

    // Statistics operations (NEW: Rust Migration)
    m.add_function(wrap_pyfunction!(py_chi_square, m)?)?;
    m.add_function(wrap_pyfunction!(py_t_test, m)?)?;
    m.add_function(wrap_pyfunction!(py_proportion_z_test, m)?)?;
    m.add_function(wrap_pyfunction!(py_pearson_correlation, m)?)?;

    // Anti-pattern detection (NEW: Rust Migration)
    m.add_function(wrap_pyfunction!(py_detect_anti_patterns, m)?)?;
    m.add_function(wrap_pyfunction!(py_detect_anti_patterns_custom, m)?)?;
    m.add_function(wrap_pyfunction!(py_get_builtin_patterns, m)?)?;

    // Session tracker operations (NEW: Rust Migration - skill_router.py consolidation)
    m.add_function(wrap_pyfunction!(py_session_load, m)?)?;
    m.add_function(wrap_pyfunction!(py_session_save, m)?)?;
    m.add_function(wrap_pyfunction!(py_session_track_execution, m)?)?;
    m.add_function(wrap_pyfunction!(py_session_complete, m)?)?;
    m.add_function(wrap_pyfunction!(py_session_fail, m)?)?;
    m.add_function(wrap_pyfunction!(py_session_append_log, m)?)?;
    m.add_function(wrap_pyfunction!(py_session_route, m)?)?;

    // Epistemic rigor validation (NEW: Rust Migration)
    m.add_function(wrap_pyfunction!(py_validate_epistemic_claim, m)?)?;
    m.add_function(wrap_pyfunction!(py_validate_epistemic_claims, m)?)?;
    m.add_function(wrap_pyfunction!(py_get_hedging_suggestions, m)?)?;
    m.add_function(wrap_pyfunction!(py_log_skill_execution, m)?)?;
    m.add_function(wrap_pyfunction!(py_compile_logic_to_rust, m)?)?;

    // Module info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__doc__", "Rust Skill Kernel - High-performance Python bindings")?;

    Ok(())
}

// ============================================================================
// Tests (only when python feature is not enabled, to avoid PyO3 test issues)
// ============================================================================

#[cfg(all(test, not(feature = "python")))]
mod tests {
    // Tests would go here, but they require Python runtime
    // Use pytest for testing the Python bindings instead

    #[test]
    fn test_module_compiles_without_python_feature() {
        // This test just verifies the module compiles without the python feature
        assert!(true);
    }
}
