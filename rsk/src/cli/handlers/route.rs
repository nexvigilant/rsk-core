//! Skill routing handler.

use crate::cli::actions::RouteAction;
use rsk::{RoutingEngine, RoutingRequest, RoutingStrategy, fuzzy_search};
use serde_json::json;

/// Handle route subcommands.
pub fn handle_route(action: &RouteAction) {
    match action {
        RouteAction::Find {
            query,
            source,
            strategy,
            limit,
        } => {
            let engine = RoutingEngine::new();
            // Note: In production, engine would be loaded with skill capabilities
            // For now, show what the interface would return

            let strat = RoutingStrategy::from_str(strategy).unwrap_or(RoutingStrategy::Hybrid);
            let request = RoutingRequest {
                source: source.clone().unwrap_or_default(),
                context: query.clone(),
                strategy: strat,
                limit: *limit,
            };

            match engine.route(&request) {
                Ok(result) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "status": "success",
                            "query": query,
                            "strategy": format!("{:?}", result.strategy),
                            "recommendations": result.recommendations.iter().map(|r| json!({
                                "target": r.target,
                                "score": r.score,
                                "confidence": r.confidence,
                                "reasoning": r.reasoning,
                            })).collect::<Vec<_>>(),
                            "total_considered": result.total_considered,
                            "duration_ms": result.duration_ms,
                        }))
                        .unwrap()
                    );
                }
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                }
            }
        }
        RouteAction::Strategies => {
            println!(
                "{}",
                json!({
                    "strategies": [
                        {
                            "name": "adjacency",
                            "weight": 0.5,
                            "description": "Graph-based routing using skill adjacency edges"
                        },
                        {
                            "name": "capability",
                            "weight": 0.3,
                            "description": "Pattern matching on skill triggers and handles"
                        },
                        {
                            "name": "semantic",
                            "weight": 0.2,
                            "description": "Keyword similarity using Levenshtein distance"
                        },
                        {
                            "name": "hybrid",
                            "weight": 1.0,
                            "description": "Weighted combination of all strategies (default)"
                        }
                    ]
                })
            );
        }
        RouteAction::Fuzzy { query, limit } => {
            // This uses the existing fuzzy_search functionality
            // In production, would load actual skill names from index
            let example_skills = vec![
                "proceed".to_string(),
                "process".to_string(),
                "skill-validator".to_string(),
                "topological-sort".to_string(),
                "level-parallelization".to_string(),
                "execution-engine".to_string(),
            ];

            let results = fuzzy_search(query, &example_skills, *limit);
            println!(
                "{}",
                json!({
                    "status": "success",
                    "query": query,
                    "matches": results,
                })
            );
        }
    }
}
