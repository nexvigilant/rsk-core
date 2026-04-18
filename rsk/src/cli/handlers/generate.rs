//! Code generation handler.

use crate::cli::actions::GenerateAction;
use rsk::{generate_test_scaffold, generate_validation_rules};
use serde_json::json;
use std::fs;

/// Handle generate subcommands.
pub fn handle_generate(action: &GenerateAction) {
    match action {
        GenerateAction::Rules { path, format } => match fs::read_to_string(path) {
            Ok(content) => {
                let smst = rsk::extract_smst(&content);
                let rules = generate_validation_rules(&smst);
                match format.as_str() {
                    "rust" => {
                        // Generate Rust validation code
                        println!(
                            "// Auto-generated validation rules for {}",
                            rules.skill_name
                        );
                        println!("// Total rules: {}\n", rules.total_rules);
                        for rule in &rules.invariant_rules {
                            println!("/// {}", rule.description);
                            println!("fn {}() -> bool {{", rule.id);
                            println!("    // Condition: {}", rule.condition);
                            println!("    todo!(\"Implement validation\")");
                            println!("}}\n");
                        }
                        for rule in &rules.failure_mode_rules {
                            println!("/// {} [{}]", rule.description, rule.severity);
                            println!("fn {}() -> Result<(), &'static str> {{", rule.id);
                            println!("    // Error: {}", rule.error_message);
                            println!("    todo!(\"Implement error handling\")");
                            println!("}}\n");
                        }
                    }
                    _ => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&rules).unwrap_or_default()
                        );
                    }
                }
            }
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        GenerateAction::Tests { path, format } => match fs::read_to_string(path) {
            Ok(content) => {
                let smst = rsk::extract_smst(&content);
                let scaffold = generate_test_scaffold(&smst);
                match format.as_str() {
                    "rust" => {
                        println!("{}", scaffold.rust_code);
                    }
                    _ => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&scaffold).unwrap_or_default()
                        );
                    }
                }
            }
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        GenerateAction::Stub { path } => match fs::read_to_string(path) {
            Ok(content) => {
                let smst = rsk::extract_smst(&content);
                let stub = rsk::generate_rust_stub(&smst);
                println!("{}", stub.full_code);
            }
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        GenerateAction::Logic { path } => match fs::read_to_string(path) {
            Ok(content) => {
                let smst = rsk::extract_smst(&content);
                let tree = rsk::generate_decision_tree(&smst);
                match serde_yaml::to_string(&tree) {
                    Ok(yaml) => println!("{yaml}"),
                    Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
                }
            }
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
    }
}
