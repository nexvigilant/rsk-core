//! Skills registry and chain handler.

use crate::cli::actions::{ChainAction, SkillsAction};
use crate::cli::utils::default_registry_path;
use rsk::modules::chain::{ExecutionContext, SkillExecutionResult};
use rsk::{
    DecisionContext, DecisionEngine, DecisionTree, ExecutionResult, SkillRegistry,
    Value as RskValue,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Shared executor: resolves skill names through registry, runs logic trees.
/// Falls back to pass-through for unregistered skills.
fn registry_executor(
    registry: &Option<SkillRegistry>,
    skill_name: &str,
    _args: Option<&str>,
    ctx: &ExecutionContext,
) -> SkillExecutionResult {
    if let Some(reg) = registry
        && let Some(entry) = reg.get(skill_name)
        && let Some(logic_path) = &entry.logic_path
        && let Ok(content) = fs::read_to_string(logic_path)
        && let Ok(tree) = serde_yaml::from_str::<DecisionTree>(&content)
    {
        let engine = DecisionEngine::new(tree);
        let variables: HashMap<String, RskValue> = ctx
            .variables
            .iter()
            .filter_map(|(k, v)| {
                serde_json::from_value::<RskValue>(v.clone())
                    .ok()
                    .map(|rv| (k.clone(), rv))
            })
            .collect();
        let mut dctx = DecisionContext {
            variables,
            execution_path: Vec::new(),
        };
        let exec_result = engine.execute(&mut dctx);
        return match exec_result {
            ExecutionResult::Value(v) => SkillExecutionResult {
                success: true,
                output: json!({
                    "skill": skill_name,
                    "path": dctx.execution_path,
                    "result": v,
                }),
                error: None,
                duration_ms: 0,
            },
            ExecutionResult::Error(e) => SkillExecutionResult {
                success: false,
                output: Value::Null,
                error: Some(e),
                duration_ms: 0,
            },
            ExecutionResult::LlmRequest { prompt, .. } => SkillExecutionResult {
                success: true,
                output: json!({
                    "skill": skill_name,
                    "llm_fallback": prompt,
                }),
                error: None,
                duration_ms: 0,
            },
        };
    }
    // Fallback: skill not in registry or has no logic tree
    SkillExecutionResult {
        success: true,
        output: json!({"skill": skill_name, "status": "pass-through"}),
        error: None,
        duration_ms: 0,
    }
}

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
                .unwrap_or_default()
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

            println!("{}", serde_json::to_string_pretty(&skills).unwrap_or_default());
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
                Some(skill) => {
                    println!("{}", serde_json::to_string_pretty(skill).unwrap_or_default());
                }
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

            let Some(skill) = reg.get(name) else {
                eprintln!("{}", json!({"status": "not_found", "name": name}));
                std::process::exit(1);
            };

            if let Some(logic_path) = &skill.logic_path {
                let logic_content = match fs::read_to_string(logic_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!(
                            "{}",
                            json!({"status": "error", "message": format!("Cannot read {}: {e}", logic_path.display())})
                        );
                        std::process::exit(1);
                    }
                };
                let tree: rsk::DecisionTree = match serde_yaml::from_str(&logic_content) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!(
                            "{}",
                            json!({"status": "error", "message": format!("YAML parse error: {e}")})
                        );
                        std::process::exit(1);
                    }
                };
                let engine = DecisionEngine::new(tree);

                let variables: HashMap<String, RskValue> = match serde_json::from_str(input) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!(
                            "{}",
                            json!({"status": "error", "message": format!("Invalid input JSON: {e}")})
                        );
                        std::process::exit(1);
                    }
                };
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
                    .unwrap_or_default()
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
            println!("CHAIN VALIDATION: {name} (Depth: {depth})");
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
                println!("{skill:<30} | {status:<15} | {score:.1}%");
            }

            println!();
            if all_diamond {
                println!("STATUS: 💎 FULL CHAIN VALIDATED");
            } else {
                println!("STATUS: ⚠️ GAPS DETECTED IN CHAIN");
            }
            println!("═══════════════════════════════════════════════════════════════════");
        }
        ChainAction::Run {
            chain,
            dry_run,
            fail_fast,
        } => {
            use rsk::modules::chain::{ExecutorConfig, execute_chain_with_fn, parse_inline};

            let parsed = match parse_inline(chain) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Parse error: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            let reg_path = default_registry_path();
            let registry = SkillRegistry::load(&reg_path).ok();

            let config = ExecutorConfig {
                dry_run: *dry_run,
                fail_fast: *fail_fast,
                ..Default::default()
            };

            let result = execute_chain_with_fn(
                &parsed,
                |skill_name, args, ctx| registry_executor(&registry, skill_name, args, ctx),
                &config,
            );

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "chain": parsed.name,
                    "success": result.success,
                    "dry_run": result.dry_run,
                    "steps": result.steps.iter().map(|s| json!({
                        "skill": s.skill,
                        "status": format!("{:?}", s.status),
                        "output": s.output,
                        "duration_ms": s.duration_ms,
                    })).collect::<Vec<_>>(),
                    "duration_ms": result.duration_ms,
                }))
                .unwrap_or_default()
            );
        }
        ChainAction::RunYaml { path, dry_run } => {
            use rsk::modules::chain::{
                ExecutorConfig, execute_chain_with_fn, parse_yaml,
            };

            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Read error: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            let parsed = match parse_yaml(&content) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Parse error: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            let reg_path = default_registry_path();
            let registry = SkillRegistry::load(&reg_path).ok();

            let config = ExecutorConfig {
                dry_run: *dry_run,
                ..Default::default()
            };

            let result = execute_chain_with_fn(
                &parsed,
                |skill_name, args, ctx| registry_executor(&registry, skill_name, args, ctx),
                &config,
            );

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "chain": parsed.name,
                    "success": result.success,
                    "dry_run": result.dry_run,
                    "steps": result.steps.iter().map(|s| json!({
                        "skill": s.skill,
                        "status": format!("{:?}", s.status),
                        "output": s.output,
                        "duration_ms": s.duration_ms,
                    })).collect::<Vec<_>>(),
                    "duration_ms": result.duration_ms,
                }))
                .unwrap_or_default()
            );
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

    let Some(skill) = reg.get(name) else {
        eprintln!("{}", json!({"status": "not_found", "name": name}));
        std::process::exit(1);
    };

    if let Some(logic_path) = &skill.logic_path {
        let logic_content = match fs::read_to_string(logic_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "{}",
                    json!({"status": "error", "message": format!("Cannot read {}: {e}", logic_path.display())})
                );
                std::process::exit(1);
            }
        };
        let tree: rsk::DecisionTree = match serde_yaml::from_str(&logic_content) {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "{}",
                    json!({"status": "error", "message": format!("YAML parse error: {e}")})
                );
                std::process::exit(1);
            }
        };

        // 1. Synthesize Code
        let code = rsk::synthesize_intrinsic(name, &tree);
        let out_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/modules/dynamic_intrinsics.rs");

        if let Err(e) = fs::write(&out_path, code) {
            eprintln!(
                "{}",
                json!({"status": "error", "message": format!("Cannot write {}: {e}", out_path.display())})
            );
            std::process::exit(1);
        }

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "status": "evolved",
                "skill": name,
                "generated_file": out_path.to_string_lossy(),
                "message": "Logic synthesized. Run 'cargo build' to integrate.",
            }))
            .unwrap_or_default()
        );
    } else {
        eprintln!(
            "{}",
            json!({"status": "error", "message": "Skill has no logic to evolve"})
        );
    }
}
