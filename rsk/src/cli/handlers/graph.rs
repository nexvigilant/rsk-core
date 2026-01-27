//! Graph operations handler.

use crate::cli::actions::GraphAction;
use crate::cli::utils::load_graph;
use serde_json::json;

/// Handle graph subcommands.
pub fn handle_graph(action: &GraphAction) {
    match action {
        GraphAction::TopSort { input } => match load_graph(input) {
            Ok(graph) => match graph.topological_sort() {
                Ok(sorted) => println!("{}", json!({"status": "success", "result": sorted})),
                Err(cycle) => println!(
                    "{}",
                    json!({"status": "error", "message": "Cycle detected", "cycle": cycle})
                ),
            },
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        GraphAction::ShortestPath { input, start, end } => match load_graph(input) {
            Ok(graph) => match graph.shortest_path(start, end) {
                Some((path, cost)) => println!(
                    "{}",
                    json!({"status": "success", "path": path, "cost": cost})
                ),
                None => println!("{}", json!({"status": "error", "message": "No path found"})),
            },
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        GraphAction::Levels { input } => match load_graph(input) {
            Ok(graph) => match graph.level_parallelization() {
                Ok(levels) => {
                    let level_info: Vec<_> = levels
                        .iter()
                        .enumerate()
                        .map(|(i, nodes)| {
                            json!({
                                "level": i,
                                "parallel_count": nodes.len(),
                                "nodes": nodes
                            })
                        })
                        .collect();
                    println!(
                        "{}",
                        json!({
                            "status": "success",
                            "total_levels": levels.len(),
                            "max_parallelism": levels.iter().map(|l| l.len()).max().unwrap_or(0),
                            "levels": level_info
                        })
                    );
                }
                Err(cycle) => println!(
                    "{}",
                    json!({"status": "error", "message": "Cycle detected", "cycle": cycle})
                ),
            },
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
    }
}
