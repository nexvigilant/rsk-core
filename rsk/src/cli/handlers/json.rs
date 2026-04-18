//! CLI handler for JSON processing operations.

use crate::cli::actions::JsonAction;
use rsk::modules::json_processor;
use serde_json::json;

/// Handle the json subcommands.
pub fn handle_json(action: &JsonAction) {
    match action {
        JsonAction::Parse { input } => match json_processor::parse_json(input) {
            Ok(result) => println!("{}", json!(result)),
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        JsonAction::Query { input, path } => match json_processor::parse_json(input) {
            Ok(parsed) => {
                let result = json_processor::query_path(&parsed.data, path);
                println!("{}", json!(result));
            }
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        JsonAction::Diff { left, right } => {
            let left_parsed = json_processor::parse_json(left);
            let right_parsed = json_processor::parse_json(right);
            match (left_parsed, right_parsed) {
                (Ok(l), Ok(r)) => {
                    let result = json_processor::diff_json(&l.data, &r.data);
                    println!("{}", json!(result));
                }
                (Err(e), _) | (_, Err(e)) => {
                    println!("{}", json!({"status": "error", "message": e.to_string()}));
                }
            }
        }
        JsonAction::Merge { target, source } => {
            let target_parsed = json_processor::parse_json(target);
            let source_parsed = json_processor::parse_json(source);
            match (target_parsed, source_parsed) {
                (Ok(t), Ok(s)) => {
                    let result = json_processor::merge_json(&t.data, &s.data);
                    println!("{}", json!(result));
                }
                (Err(e), _) | (_, Err(e)) => {
                    println!("{}", json!({"status": "error", "message": e.to_string()}));
                }
            }
        }
        JsonAction::Flatten { input } => match json_processor::parse_json(input) {
            Ok(parsed) => {
                let result = json_processor::flatten_json(&parsed.data);
                println!("{}", json!(result));
            }
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        JsonAction::Keys { input } => match json_processor::parse_json(input) {
            Ok(parsed) => {
                let keys = json_processor::get_keys(&parsed.data);
                println!("{}", json!({"status": "success", "keys": keys}));
            }
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        JsonAction::TypeCheck { input, expected } => match json_processor::parse_json(input) {
            Ok(parsed) => {
                let valid = json_processor::validate_type(&parsed.data, expected);
                println!(
                    "{}",
                    json!({"status": "success", "valid": valid, "expected": expected, "actual": parsed.value_type})
                );
            }
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
    }
}
