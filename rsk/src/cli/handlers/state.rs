//! State/checkpoint management handler.

use crate::cli::actions::StateAction;
use crate::cli::utils::default_state_dir;
use rsk::CheckpointManager;
use serde_json::json;

/// Handle state subcommands.
pub fn handle_state(action: &StateAction) {
    let state_dir = default_state_dir();

    match CheckpointManager::new(state_dir.to_str().unwrap_or("/tmp/chain-state")) {
        Ok(mut manager) => match action {
            StateAction::List { name, status } => {
                let contexts = if let Some(n) = name {
                    manager.list_by_name(n).unwrap_or_default()
                } else {
                    manager.list().unwrap_or_default()
                };

                // Filter by status if specified
                let filtered: Vec<_> = if let Some(s) = status {
                    contexts
                        .into_iter()
                        .filter(|ctx| {
                            format!("{:?}", ctx.status)
                                .to_lowercase()
                                .contains(&s.to_lowercase())
                        })
                        .collect()
                } else {
                    contexts
                };

                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": "success",
                        "count": filtered.len(),
                        "checkpoints": filtered.iter().map(|ctx| json!({
                            "id": ctx.id,
                            "name": ctx.name,
                            "status": format!("{:?}", ctx.status),
                            "progress": ctx.progress_percent(),
                            "updated_at": ctx.updated_at.to_rfc3339(),
                        })).collect::<Vec<_>>(),
                    }))
                    .unwrap()
                );
            }
            StateAction::Show { id } => match manager.load(id) {
                Ok(Some(ctx)) => {
                    println!("{}", serde_json::to_string_pretty(&ctx).unwrap());
                }
                Ok(None) => {
                    println!("{}", json!({"status": "not_found", "id": id}));
                }
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                }
            },
            StateAction::Delete { id } => match manager.delete(id) {
                Ok(true) => {
                    println!("{}", json!({"status": "deleted", "id": id}));
                }
                Ok(false) => {
                    println!("{}", json!({"status": "not_found", "id": id}));
                }
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                }
            },
            StateAction::Cleanup { max_age } => match manager.cleanup(*max_age) {
                Ok(count) => {
                    println!(
                        "{}",
                        json!({
                            "status": "success",
                            "removed": count,
                            "max_age_days": max_age,
                        })
                    );
                }
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                }
            },
            StateAction::Stats => match manager.stats() {
                Ok(stats) => {
                    println!("{}", serde_json::to_string_pretty(&stats).unwrap());
                }
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                }
            },
        },
        Err(e) => {
            eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
            std::process::exit(1);
        }
    }
}
