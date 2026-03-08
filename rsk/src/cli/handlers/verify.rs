//! Verify/validate command handler.

use crate::cli::utils::resolve_skill_paths;
use serde_json::json;

/// Handle verify/validate command (they're aliases for the same functionality).
pub fn handle_verify(
    path: &str,
    threshold: f64,
    format: &str,
    export_jsonschema: bool,
    verbose: bool,
) {
    // Handle JSON Schema export mode
    if export_jsonschema {
        // Output SMST structure as JSON Schema Draft 2020-12
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://claude.ai/schemas/smst-v2.json",
            "title": "SMST - Skill Machine Specification Template v2",
            "description": "Schema for Claude Code skill machine specifications (Diamond v2 compliance)",
            "type": "object",
            "properties": {
                "frontmatter": {
                    "type": "object",
                    "description": "YAML frontmatter metadata",
                    "properties": {
                        "name": { "type": "string", "description": "Skill identifier (kebab-case)" },
                        "description": { "type": "string", "description": "One-line skill description" },
                        "version": { "type": "string", "pattern": "^\\d+\\.\\d+\\.\\d+$" },
                        "compliance-level": {
                            "type": "string",
                            "enum": ["Bronze", "Silver", "Gold", "Platinum", "Diamond"]
                        },
                        "categories": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "triggers": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Patterns that activate this skill"
                        },
                        "author": { "type": "string" },
                        "user-invocable": { "type": "boolean", "default": true },
                        "context": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Required context (codebase, conversation)"
                        },
                        "depends-on": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Skill dependencies"
                        }
                    },
                    "required": ["name", "version", "compliance-level"]
                },
                "spec": {
                    "type": "object",
                    "description": "Machine Specification sections",
                    "properties": {
                        "inputs": { "type": "string", "description": "INPUTS section content" },
                        "outputs": { "type": "string", "description": "OUTPUTS section content" },
                        "state": { "type": "string", "description": "STATE section content" },
                        "operator_mode": { "type": "string", "description": "OPERATOR MODE section" },
                        "performance": { "type": "string", "description": "PERFORMANCE section" },
                        "invariants": { "type": "string", "description": "INVARIANTS section" },
                        "failure_modes": { "type": "string", "description": "FAILURE_MODES section" },
                        "telemetry": { "type": "string", "description": "TELEMETRY section" }
                    }
                },
                "score": {
                    "type": "object",
                    "description": "SMST compliance scoring",
                    "properties": {
                        "total_score": { "type": "number", "minimum": 0, "maximum": 100 },
                        "sections_present": { "type": "integer", "minimum": 0 },
                        "sections_required": { "type": "integer", "minimum": 8 },
                        "has_frontmatter": { "type": "boolean" },
                        "has_machine_spec": { "type": "boolean" },
                        "compliance_level": { "type": "string" },
                        "missing_sections": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "is_diamond_compliant": { "type": "boolean" }
            },
            "required": ["frontmatter", "spec", "score"]
        });
        println!("{}", serde_json::to_string_pretty(&schema).unwrap_or_default());
        return;
    }

    // Resolve paths to validate
    let paths = resolve_skill_paths(path);

    let mut results = Vec::new();
    let mut all_passed = true;

    let diamond_threshold = rsk::lookup_compliance_level("diamond")
        .map(|l| f64::from(l.min_score))
        .unwrap_or(threshold);

    for skill_path in &paths {
        // If the path is a file (SKILL.md), get its parent directory
        let p = std::path::Path::new(skill_path);
        let skill_dir = if p.is_file() {
            p.parent().unwrap_or(std::path::Path::new("."))
        } else {
            p
        };

        let result = rsk::verify_skill(skill_dir);
        // Status is "success" if it meets the Diamond threshold and has all artifacts
        if result.status == "failed" || result.score < diamond_threshold {
            all_passed = false;
        }
        if verbose {
            results.push(json!(result));
        } else {
            results.push(json!({
                "name": result.skill_name,
                "score": result.score,
                "compliance_level": result.compliance_level,
                "passed": result.status == "success"
            }));
        }
    }

    match format {
        "json" => {
            if results.len() == 1 {
                println!("{}", serde_json::to_string_pretty(&results[0]).unwrap_or_default());
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": if all_passed { "success" } else { "failed" },
                        "total": results.len(),
                        "passed": results.iter().filter(|r| r["passed"] == true).count(),
                        "results": results,
                    }))
                    .unwrap_or_default()
                );
            }
        }
        "summary" => {
            let passed_count = results.iter().filter(|r| r["passed"] == true).count();
            println!(
                "Validation Summary: {}/{} skills passed (threshold: {}%)",
                passed_count,
                results.len(),
                threshold
            );
            for r in &results {
                let status = if r["passed"] == true { "✓" } else { "✗" };
                println!("  {} {} - {:.1}%", status, r["name"], r["score"]);
            }
        }
        "minimal" => {
            // Just exit code - 0 if all passed, 1 if any failed
            if !all_passed {
                std::process::exit(1);
            }
        }
        "report" => {
            for r in &results {
                if let Some(err) = r.get("error") {
                    println!("❌ ERROR: {err}");
                    continue;
                }

                let name = r["name"].as_str().unwrap_or("unknown");
                let score = r["score"].as_f64().unwrap_or(0.0);
                let passed = r["passed"].as_bool().unwrap_or(false);
                let missing = r["missing_sections"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();

                println!("═══════════════════════════════════════════════════════════════════");
                println!("SMST VALIDATION REPORT: {name}");
                println!("═══════════════════════════════════════════════════════════════════");
                println!();
                println!(
                    "OVERALL: {}",
                    if passed {
                        "✅ DIAMOND READY"
                    } else {
                        "❌ NOT READY"
                    }
                );
                println!("Score: {score:.1}/100 (Threshold: {threshold}%)");
                println!();

                println!("───────────────────────────────────────────────────────────────────");
                println!("COMPONENT STATUS");
                println!("───────────────────────────────────────────────────────────────────");

                let all_sections = [
                    "INPUTS",
                    "OUTPUTS",
                    "STATE",
                    "OPERATOR MODE",
                    "PERFORMANCE",
                    "INVARIANTS",
                    "FAILURE MODES",
                    "TELEMETRY",
                ];

                for section in all_sections {
                    let is_missing = missing.iter().any(|m| m.as_str() == Some(section));
                    let status = if is_missing { "□ ✗" } else { "■ ✓" };
                    println!("{section:<15} [{status}]");
                }

                if !missing.is_empty() {
                    println!();
                    println!("───────────────────────────────────────────────────────────────────");
                    println!("GAPS TO DIAMOND");
                    println!("───────────────────────────────────────────────────────────────────");
                    for m in missing {
                        println!(
                            "- {}: Missing required section",
                            m.as_str().unwrap_or("UNKNOWN")
                        );
                    }
                }
                println!("═══════════════════════════════════════════════════════════════════\n");
            }
        }
        _ => {
            eprintln!("Unknown format: {format}");
            std::process::exit(1);
        }
    }
}

/// Handle build command.
pub fn handle_build(path: &str, dry_run: bool) {
    use crate::cli::utils::resolve_build_paths;

    let paths = resolve_build_paths(path);

    let mut all_results = Vec::new();
    let mut all_success = true;

    for skill_dir in &paths {
        let p = std::path::Path::new(skill_dir);
        let result = rsk::build_skill(p, dry_run);
        if result.status == "failed" {
            all_success = false;
        }
        all_results.push(result);
    }

    if all_results.len() == 1 {
        println!("{}", serde_json::to_string_pretty(&all_results[0]).unwrap_or_default());
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "status": if all_success { "success" } else { "failed" },
                "count": all_results.len(),
                "results": all_results
            }))
            .unwrap_or_default()
        );
    }

    if !all_success {
        std::process::exit(1);
    }
}
