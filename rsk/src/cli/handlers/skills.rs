//! Skills registry and chain handler.

use crate::cli::actions::{ChainAction, SkillsAction};
use crate::cli::utils::default_registry_path;
use rsk::{DecisionContext, DecisionEngine, ExecutionResult, SkillRegistry, Value};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Handle skills subcommands.
pub fn handle_skills(action: &SkillsAction) {
    let default_registry = default_registry_path();

    match action {
        SkillsAction::Scan { path, output } => {
            let mut registry = SkillRegistry::new();
            if let Err(e) = registry.load_from_directory(path) {
                eprintln!("{}", json!({"status": "error", "message": e}));
                std::process::exit(1);
            }

            let out_path = output
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or(default_registry);
            if let Some(parent) = out_path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            if let Err(e) = registry.save(&out_path) {
                eprintln!("{}", json!({"status": "error", "message": e}));
                std::process::exit(1);
            }

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "status": "success",
                    "message": format!("Scanned {} skills and saved to {:?}", registry.skills.len(), out_path),
                    "count": registry.skills.len(),
                }))
                .unwrap()
            );
        }
        SkillsAction::List { registry, strategy } => {
            let reg_path = registry
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or(default_registry);
            let reg = match SkillRegistry::load(&reg_path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Failed to load registry: {}", e)})
                    );
                    std::process::exit(1);
                }
            };

            let mut skills = reg.list();
            if let Some(s) = strategy {
                skills.retain(|entry| {
                    format!("{:?}", entry.strategy)
                        .to_lowercase()
                        .contains(&s.to_lowercase())
                });
            }

            println!("{}", serde_json::to_string_pretty(&skills).unwrap());
        }
        SkillsAction::Info { name, registry } => {
            let reg_path = registry
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or(default_registry);
            let reg = match SkillRegistry::load(&reg_path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            match reg.get(name) {
                Some(skill) => println!("{}", serde_json::to_string_pretty(skill).unwrap()),
                None => {
                    eprintln!("{}", json!({"status": "not_found", "name": name}));
                    std::process::exit(1);
                }
            }
        }
        SkillsAction::Execute {
            name,
            input,
            registry,
        } => {
            let reg_path = registry
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or(default_registry);
            let reg = match SkillRegistry::load(&reg_path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            let skill = match reg.get(name) {
                Some(s) => s,
                None => {
                    eprintln!("{}", json!({"status": "not_found", "name": name}));
                    std::process::exit(1);
                }
            };

            if let Some(logic_path) = &skill.logic_path {
                let logic_content = fs::read_to_string(logic_path).unwrap();
                let tree: rsk::DecisionTree = serde_yaml::from_str(&logic_content).unwrap();
                let engine = DecisionEngine::new(tree);

                let variables: HashMap<String, Value> = serde_json::from_str(input).unwrap();
                let mut ctx = DecisionContext {
                    variables,
                    execution_path: Vec::new(),
                };

                let result = engine.execute(&mut ctx);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": "success",
                        "skill": name,
                        "execution_path": ctx.execution_path,
                        "result": match result {
                            ExecutionResult::Value(v) => json!(v),
                            ExecutionResult::LlmRequest { prompt, .. } => json!({"llm_fallback": prompt}),
                            ExecutionResult::Error(e) => json!({"error": e}),
                        }
                    }))
                    .unwrap()
                );
            } else {
                println!(
                    "{}",
                    json!({
                        "status": "unsupported",
                        "message": "Skill has no logic.yaml and cannot be executed deterministically yet",
                        "strategy": format!("{:?}", skill.strategy),
                    })
                );
            }
        }
    }
}

/// Handle chain subcommands.
pub fn handle_chain(action: &ChainAction) {
    let default_registry = default_registry_path();

    match action {
        ChainAction::Validate {
            name,
            depth,
            registry,
        } => {
            let reg_path = registry
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or(default_registry);
            let reg = match SkillRegistry::load(&reg_path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            println!("═══════════════════════════════════════════════════════════════════");
            println!("CHAIN VALIDATION: {} (Depth: {})", name, depth);
            println!("═══════════════════════════════════════════════════════════════════");
            println!();

            let chain_results = reg.validate_chain(name, *depth);
            let mut all_diamond = true;

            for (skill, passed, score) in chain_results {
                let status = if passed {
                    "✅ DIAMOND"
                } else {
                    "❌ NOT READY"
                };
                if !passed {
                    all_diamond = false;
                }
                println!("{:<30} | {:<15} | {:.1}%", skill, status, score);
            }

            println!();
            if all_diamond {
                println!("STATUS: 💎 FULL CHAIN VALIDATED");
            } else {
                println!("STATUS: ⚠️ GAPS DETECTED IN CHAIN");
            }
            println!("═══════════════════════════════════════════════════════════════════");
        }
    }
}

/// Handle evolve command.
pub fn handle_evolve(name: &str, registry: &Option<String>) {
    let default_registry = default_registry_path();
    let reg_path = registry
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or(default_registry);
    let reg = match SkillRegistry::load(&reg_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", json!({"status": "error", "message": e}));
            std::process::exit(1);
        }
    };

    let skill = match reg.get(name) {
        Some(s) => s,
        None => {
            eprintln!("{}", json!({"status": "not_found", "name": name}));
            std::process::exit(1);
        }
    };

    if let Some(logic_path) = &skill.logic_path {
        let logic_content = fs::read_to_string(logic_path).unwrap();
        let tree: rsk::DecisionTree = serde_yaml::from_str(&logic_content).unwrap();

        // 1. Synthesize Code
        let code = rsk::synthesize_intrinsic(name, &tree);
        let out_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/modules/dynamic_intrinsics.rs");

        fs::write(&out_path, code).unwrap();

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "status": "evolved",
                "skill": name,
                "generated_file": out_path.to_string_lossy(),
                "message": "Logic synthesized. Run 'cargo build' to integrate.",
            }))
            .unwrap()
        );
    } else {
        eprintln!(
            "{}",
            json!({"status": "error", "message": "Skill has no logic to evolve"})
        );
    }
}
