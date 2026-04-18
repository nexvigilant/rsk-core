//! Execution engine handler.

use crate::cli::actions::ExecAction;
use crate::cli::utils::default_state_dir;
use rsk::{
    CheckpointManager, EffortSize, ExecutionModule, build_execution_plan, detect_resource_conflicts,
};
use serde_json::json;
use std::fs;

/// Handle exec subcommands.
pub fn handle_exec(action: &ExecAction) {
    match action {
        ExecAction::Plan { modules } => {
            // Load modules from JSON string or file
            let content = if modules.ends_with(".json") {
                fs::read_to_string(modules).unwrap_or_else(|e| {
                    eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                    std::process::exit(1);
                })
            } else {
                modules.clone()
            };

            // Parse modules
            let module_list: Vec<serde_json::Value> = match serde_json::from_str(&content) {
                Ok(list) => list,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Invalid JSON: {}", e)})
                    );
                    std::process::exit(1);
                }
            };

            // Convert to ExecutionModule
            let exec_modules: Vec<ExecutionModule> = module_list
                .iter()
                .map(|m| {
                    let id = m["id"].as_str().unwrap_or("unknown").to_string();
                    let name = m["name"].as_str().unwrap_or(&id).to_string();
                    let deps: Vec<String> = m["dependencies"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    let effort = m["effort"]
                        .as_str()
                        .and_then(EffortSize::parse_str)
                        .unwrap_or(EffortSize::M);
                    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
                    // f64→f32 acceptable for risk score display
                    let risk = m["risk"].as_f64().unwrap_or(0.3) as f32;

                    ExecutionModule::new(&id, &name, deps)
                        .with_effort(effort)
                        .with_risk(risk)
                })
                .collect();

            // Build execution plan
            match build_execution_plan(exec_modules) {
                Ok(plan) => {
                    let conflicts = detect_resource_conflicts(&plan);
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "status": "success",
                            "plan": {
                                "total_modules": plan.modules.len(),
                                "execution_order": plan.execution_order,
                                "levels": plan.levels,
                                "critical_path": plan.critical_path,
                                "estimated_duration_minutes": plan.estimated_duration_minutes,
                                "resource_conflicts": conflicts.len(),
                            }
                        }))
                        .unwrap_or_default()
                    );
                }
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                    std::process::exit(1);
                }
            }
        }
        ExecAction::Status { id } => {
            let state_dir = default_state_dir();

            match CheckpointManager::new(state_dir.to_str().unwrap_or("/tmp/chain-state")) {
                Ok(manager) => match manager.load(id) {
                    Ok(Some(ctx)) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "status": "found",
                                "context": {
                                    "id": ctx.id,
                                    "name": ctx.name,
                                    "status": format!("{:?}", ctx.status),
                                    "progress_percent": ctx.progress_percent(),
                                    "completed_steps": ctx.completed_steps.len(),
                                    "failed_steps": ctx.failed_steps.len(),
                                    "total_steps": ctx.total_steps,
                                }
                            }))
                            .unwrap_or_default()
                        );
                    }
                    Ok(None) => {
                        println!("{}", json!({"status": "not_found", "id": id}));
                    }
                    Err(e) => {
                        eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                    }
                },
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                }
            }
        }
        ExecAction::Resume { id } => {
            let state_dir = default_state_dir();

            match CheckpointManager::new(state_dir.to_str().unwrap_or("/tmp/chain-state")) {
                Ok(manager) => match manager.load(id) {
                    Ok(Some(ctx)) => {
                        let next = ctx.next_step();
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "status": "resumable",
                                "context": {
                                    "id": ctx.id,
                                    "name": ctx.name,
                                    "next_step": next,
                                    "completed_steps": ctx.completed_steps,
                                    "total_steps": ctx.total_steps,
                                }
                            }))
                            .unwrap_or_default()
                        );
                    }
                    Ok(None) => {
                        println!("{}", json!({"status": "not_found", "id": id}));
                    }
                    Err(e) => {
                        eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                    }
                },
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                }
            }
        }
        ExecAction::Validate { modules } => {
            // Same as Plan but just validates without saving
            let content = if modules.ends_with(".json") {
                fs::read_to_string(modules).unwrap_or_else(|e| {
                    eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                    std::process::exit(1);
                })
            } else {
                modules.clone()
            };

            let module_list: Vec<serde_json::Value> = match serde_json::from_str(&content) {
                Ok(list) => list,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Invalid JSON: {}", e)})
                    );
                    std::process::exit(1);
                }
            };

            let exec_modules: Vec<ExecutionModule> = module_list
                .iter()
                .map(|m| {
                    let id = m["id"].as_str().unwrap_or("unknown").to_string();
                    let name = m["name"].as_str().unwrap_or(&id).to_string();
                    let deps: Vec<String> = m["dependencies"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    ExecutionModule::new(&id, &name, deps)
                })
                .collect();

            match build_execution_plan(exec_modules) {
                Ok(plan) => {
                    let conflicts = detect_resource_conflicts(&plan);
                    println!(
                        "{}",
                        json!({
                            "status": "valid",
                            "modules": plan.modules.len(),
                            "levels": plan.levels.len(),
                            "conflicts": conflicts.len(),
                        })
                    );
                }
                Err(e) => {
                    println!("{}", json!({"status": "invalid", "error": e.to_string()}));
                    std::process::exit(1);
                }
            }
        }
    }
}
