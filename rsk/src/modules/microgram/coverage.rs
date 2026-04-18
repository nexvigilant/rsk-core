use super::{Microgram, load_all};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct CoverageResult {
    pub name: String,
    pub total_nodes: usize,
    pub covered_nodes: usize,
    pub coverage_pct: f64,
    pub uncovered: Vec<String>,
    pub paths_taken: Vec<Vec<String>>,
}

/// Analyze path coverage of a microgram's self-tests
pub fn coverage(mg: &Microgram) -> CoverageResult {
    let total_nodes = mg.tree.nodes.len();
    let mut visited: HashMap<String, bool> =
        mg.tree.nodes.keys().map(|k| (k.clone(), false)).collect();
    let mut paths_taken = Vec::new();

    for test in &mg.tests {
        let result = mg.run(test.input.clone());
        for node_name in &result.path {
            visited.insert(node_name.clone(), true);
        }
        paths_taken.push(result.path);
    }

    let covered_nodes = visited.values().filter(|&&v| v).count();
    let uncovered: Vec<String> = visited
        .iter()
        .filter(|(_, v)| !*v)
        .map(|(k, _)| k.clone())
        .collect();

    let coverage_pct = if total_nodes == 0 {
        100.0
    } else {
        #[allow(clippy::as_conversions)] // usize→f64 for coverage percentage
        let pct = (covered_nodes as f64 / total_nodes as f64) * 100.0;
        pct
    };

    CoverageResult {
        name: mg.name.clone(),
        total_nodes,
        covered_nodes,
        coverage_pct,
        uncovered,
        paths_taken,
    }
}

/// Coverage for all micrograms in a directory
pub fn coverage_all(dir: &Path) -> Result<Vec<CoverageResult>, String> {
    let all = load_all(dir)?;
    Ok(all.iter().map(coverage).collect())
}
