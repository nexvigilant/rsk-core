//! Text processing handler.

use crate::cli::actions::TextAction;
use serde_json::json;
use std::fs;

/// Handle text subcommands.
pub fn handle_text(action: &TextAction) {
    match action {
        TextAction::Parse { path } => match fs::read_to_string(path) {
            Ok(content) => {
                let result = rsk::parse_skill_md(&content);
                println!("{}", json!(result));
            }
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        TextAction::Validate { path } => match fs::read_to_string(path) {
            Ok(content) => {
                let parse_result = rsk::parse_skill_md(&content);
                let errors = rsk::validate_diamond_spec(&parse_result);
                if errors.is_empty() {
                    println!(
                        "{}",
                        json!({"status": "success", "message": "Diamond v2 compliant"})
                    );
                } else {
                    println!("{}", json!({"status": "error", "errors": errors}));
                }
            }
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        TextAction::Smst { path } => match fs::read_to_string(path) {
            Ok(content) => {
                let result = rsk::extract_smst(&content);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            }
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        TextAction::Tokenize { text } => {
            let result = rsk::tokenize(text);
            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_default()
            );
        }
        TextAction::Normalize {
            text,
            strip_punctuation,
        } => {
            let result = rsk::normalize(text, *strip_punctuation);
            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_default()
            );
        }
        TextAction::Frequency { text, top } => {
            let result = rsk::word_frequency(text, *top);
            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_default()
            );
        }
        TextAction::Entropy { text } => {
            let result = rsk::analyze_compressibility(text);
            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_default()
            );
        }
        TextAction::Ngrams { text, n, words } => {
            let ngrams = rsk::extract_ngrams(text, *n, *words);
            println!(
                "{}",
                json!({
                    "n": n,
                    "mode": if *words { "word" } else { "character" },
                    "count": ngrams.len(),
                    "ngrams": ngrams,
                })
            );
        }
        TextAction::Slugify { text } => {
            let result = rsk::slugify(text);
            println!(
                "{}",
                json!({
                    "original": text,
                    "slug": result,
                })
            );
        }
    }
}
