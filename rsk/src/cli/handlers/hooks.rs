//! File organization hooks handler.

use crate::cli::actions::HooksAction;
use rsk::hooks::{
    blindspot::BlindspotCheck,
    policy::PolicyFile,
    scanner::{format_scan_result, scan_directory},
    staleness::{check_staleness, format_staleness_result},
    validation::{categorize_file, format_validation_result, validate_file},
};
use std::path::PathBuf;

/// Handle hooks subcommands.
pub fn handle_hooks(action: &HooksAction) {
    let policy = PolicyFile::load_or_default(None);

    match action {
        HooksAction::Validate { path, format } => {
            let path = PathBuf::from(&path);
            let result = validate_file(&path, &policy);

            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            } else {
                let formatted = format_validation_result(&result);
                if !formatted.is_empty() {
                    println!("{}", formatted);
                } else {
                    println!("[OK] {} - no policy violations", path.display());
                }
            }
        }
        HooksAction::Staleness { path, format } => {
            let path = PathBuf::from(&path);
            let result = check_staleness(&path, &policy);

            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            } else {
                println!("{}", format_staleness_result(&result));
            }
        }
        HooksAction::Categorize { path } => {
            let path = PathBuf::from(&path);
            let category = categorize_file(&path, &policy);
            println!("{}", category);
        }
        HooksAction::Scan {
            path,
            depth,
            format,
        } => {
            let path = PathBuf::from(&path);
            let result = scan_directory(&path, *depth, &policy);

            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            } else {
                println!("{}", format_scan_result(&result));
            }
        }
        HooksAction::Policy => {
            println!("=== File Organization Policy ===\n");

            if let Some(settings) = &policy.settings {
                println!("Settings:");
                println!("  Mode: {}", settings.mode.as_deref().unwrap_or("advisory"));
                println!(
                    "  Stale action: {}",
                    settings.stale_action.as_deref().unwrap_or("report")
                );
                println!();
            }

            if let Some(rules) = &policy.placement_rules {
                println!("Placement Rules ({} categories):", rules.len());
                for (name, rule) in rules {
                    println!("  {}:", name);
                    println!("    Patterns: {:?}", rule.patterns);
                    if !rule.forbidden_paths.is_empty() {
                        println!("    Forbidden: {:?}", rule.forbidden_paths);
                    }
                    if !rule.recommended_paths.is_empty() {
                        println!("    Recommended: {:?}", rule.recommended_paths);
                    }
                }
                println!();
            }

            if let Some(staleness) = &policy.staleness {
                println!("Staleness Rules:");
                println!("  Default: {} days", staleness.default_days.unwrap_or(30));
                if let Some(rules) = &staleness.path_rules {
                    for (pattern, rule) in rules {
                        println!(
                            "  {}: {} days ({})",
                            pattern,
                            rule.days.unwrap_or(30),
                            rule.action.as_deref().unwrap_or("report")
                        );
                    }
                }
            }
        }
        HooksAction::Blindspot { path, format } => {
            let path = PathBuf::from(&path);
            let check = BlindspotCheck::for_file(&path, &policy);

            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&check).unwrap());
            } else {
                println!("{}", check.message);
                println!("\nChecklist:");
                for item in &check.items {
                    println!("  - {}", item);
                }
            }
        }
        HooksAction::SubagentReview {
            agent_type,
            description,
        } => {
            let check = BlindspotCheck::for_subagent(agent_type, description);
            println!("{}", check.message);
        }
        HooksAction::SchemaVersion => {
            println!("{}", rsk::hooks::SCHEMA_VERSION);
        }
    }
}
