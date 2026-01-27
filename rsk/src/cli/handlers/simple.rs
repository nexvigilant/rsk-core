//! Simple CLI handlers (Variance, Levenshtein, Fuzzy, Sha256, Version).

use crate::cli::actions::Sha256Action;
use rsk::{calculate_variance, fuzzy_search, levenshtein, sha256_hash, sha256_verify};
use serde_json::json;
use std::fs;

/// Handle the variance command.
pub fn handle_variance(actual: f64, target: f64) {
    let result = calculate_variance(actual, target);
    println!("{}", json!(result));
}

/// Handle the levenshtein command.
pub fn handle_levenshtein(source: &str, target: &str) {
    let result = levenshtein(source, target);
    println!("{}", json!(result));
}

/// Handle the fuzzy command.
pub fn handle_fuzzy(query: &str, candidates: &str, limit: usize) {
    let candidate_list: Vec<String> = candidates
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let results = fuzzy_search(query, &candidate_list, limit);
    println!("{}", json!({"status": "success", "matches": results}));
}

/// Handle the sha256 subcommands.
pub fn handle_sha256(action: &Sha256Action) {
    match action {
        Sha256Action::Hash { input } => {
            let result = sha256_hash(input);
            println!("{}", json!(result));
        }
        Sha256Action::File { path } => match fs::read(path) {
            Ok(bytes) => {
                let result = rsk::sha256_bytes(&bytes);
                println!("{}", json!(result));
            }
            Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
        },
        Sha256Action::Verify { input, expected } => {
            let matches = sha256_verify(input, expected);
            println!("{}", json!({"matches": matches}));
        }
    }
}

/// Handle the version command.
pub fn handle_version() {
    println!("rsk version {}", rsk::version());
}
