//! YAML/TOML processing handler.

use crate::cli::actions::YamlAction;
use rsk::{
    DecisionContext, DecisionEngine, ExecutionResult, Value, analyze_decision_tree,
    extract_taxonomy_schema, parse_toml, parse_yaml, parse_yaml_frontmatter, validate_schema,
};
use serde_json::json;
use std::collections::HashMap;
use std::fs;

/// Handle yaml subcommands.
pub fn handle_yaml(action: &YamlAction) {
    match action {
        YamlAction::Parse { path } => match fs::read_to_string(path) {
            Ok(content) => match parse_yaml(&content) {
                Ok(result) => {
                    println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
                }
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        YamlAction::ParseStdin => {
            use std::io::{self, Read};
            let mut content = String::new();
            match io::stdin().read_to_string(&mut content) {
                Ok(_) => match parse_yaml(&content) {
                    Ok(result) => {
                        println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
                    }
                    Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
                },
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            }
        }
        YamlAction::Toml { path } => match fs::read_to_string(path) {
            Ok(content) => match parse_toml(&content) {
                Ok(result) => {
                    println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
                }
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        YamlAction::Validate { path, schema } => match fs::read_to_string(path) {
            Ok(content) => {
                let result = validate_schema(&content, schema.as_deref());
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            }
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        YamlAction::DecisionTree { path } => match fs::read_to_string(path) {
            Ok(content) => match analyze_decision_tree(&content) {
                Ok(analysis) => {
                    println!("{}", serde_json::to_string_pretty(&analysis).unwrap_or_default());
                }
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        YamlAction::Taxonomy { path } => match fs::read_to_string(path) {
            Ok(content) => match extract_taxonomy_schema(&content) {
                Ok(schema) => {
                    println!("{}", serde_json::to_string_pretty(&schema).unwrap_or_default());
                }
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        YamlAction::Frontmatter { path } => match fs::read_to_string(path) {
            Ok(content) => match parse_yaml_frontmatter(&content) {
                Ok(frontmatter) => {
                    println!("{}", serde_json::to_string_pretty(&frontmatter).unwrap_or_default());
                }
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        YamlAction::ExecuteLogic { tree, input } => {
            let tree_content = if tree.ends_with(".yaml") || tree.ends_with(".yml") {
                fs::read_to_string(tree).unwrap_or_else(|_| tree.clone())
            } else {
                tree.clone()
            };

            let tree: rsk::DecisionTree = match serde_yaml::from_str(&tree_content) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Invalid logic tree: {}", e)})
                    );
                    std::process::exit(1);
                }
            };

            let engine = DecisionEngine::new(tree);
            let variables: HashMap<String, Value> = serde_json::from_str(input).unwrap_or_default();
            let mut ctx = DecisionContext {
                variables,
                execution_path: Vec::new(),
            };

            let result = engine.execute(&mut ctx);
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "status": "success",
                    "execution_path": ctx.execution_path,
                    "result": match result {
                        ExecutionResult::Value(v) => json!(v),
                        ExecutionResult::LlmRequest { prompt, .. } => json!({"llm_fallback": prompt}),
                        ExecutionResult::Error(e) => json!({"error": e}),
                    }
                }))
                .unwrap_or_default()
            );
        }
    }
}
