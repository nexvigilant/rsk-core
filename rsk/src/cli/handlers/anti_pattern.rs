//! Anti-pattern CLI handler.

use crate::cli::actions::AntiPatternAction;
use rsk::modules::anti_pattern::{
    DetectionConfig, Features, PatternRegistry, pattern_from_observation,
};
use serde_json::json;
use std::collections::HashMap;

/// Handle anti-pattern subcommands.
pub fn handle_anti_pattern(action: &AntiPatternAction) {
    let registry_path = PatternRegistry::default_path();

    match action {
        AntiPatternAction::Detect {
            features,
            threshold,
        } => {
            let mut registry = match PatternRegistry::load_or_create(&registry_path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            let numeric: HashMap<String, f64> = match serde_json::from_str(features) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Invalid JSON: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            let feat = Features {
                numeric,
                ..Default::default()
            };

            let context = HashMap::new();
            let config = DetectionConfig {
                threshold: *threshold,
                ..Default::default()
            };

            let result = registry.detect(&feat, &context, &config);

            // Persist updated detection count
            if let Err(e) = registry.save(&registry_path) {
                eprintln!("Warning: could not save registry: {e}");
            }

            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_default()
            );
        }
        AntiPatternAction::Add {
            name,
            category,
            description,
            metric,
            threshold,
            direction,
            remediation,
        } => {
            let mut registry = match PatternRegistry::load_or_create(&registry_path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            let pattern = pattern_from_observation(
                name,
                category,
                description,
                metric,
                *threshold,
                direction,
                vec![remediation.clone()],
            );

            if registry.register(pattern) {
                if let Err(e) = registry.save(&registry_path) {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
                println!(
                    "{}",
                    json!({
                        "status": "registered",
                        "pattern": name,
                        "total_patterns": registry.len(),
                    })
                );
            } else {
                println!(
                    "{}",
                    json!({
                        "status": "duplicate",
                        "message": format!("Pattern '{}' already exists", name),
                    })
                );
            }
        }
        AntiPatternAction::List => {
            let registry = match PatternRegistry::load_or_create(&registry_path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "patterns": registry.patterns.iter().map(|p| json!({
                        "name": p.name,
                        "category": p.category,
                        "severity": p.base_severity,
                        "symptoms": p.symptoms.len(),
                    })).collect::<Vec<_>>(),
                    "total": registry.len(),
                }))
                .unwrap_or_default()
            );
        }
        AntiPatternAction::Stats => {
            let registry = match PatternRegistry::load_or_create(&registry_path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            let mut by_category: HashMap<String, usize> = HashMap::new();
            for p in &registry.patterns {
                *by_category.entry(p.category.clone()).or_insert(0) += 1;
            }

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "total_patterns": registry.len(),
                    "total_detections": registry.total_detections,
                    "by_category": by_category,
                    "version": registry.version,
                }))
                .unwrap_or_default()
            );
        }
    }
}
