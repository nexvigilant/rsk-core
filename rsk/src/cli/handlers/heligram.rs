//! Heligram CLI handler.

use crate::cli::actions::HeligramAction;
use rsk::modules::decision_engine::Value as RskValue;
use rsk::modules::heligram::{Heligram, chain, dna, forge, load_all, promote};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;

pub fn handle_heligram(action: &HeligramAction) {
    match action {
        HeligramAction::Run { path, input } => {
            let h = match Heligram::load(Path::new(path)) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

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

            let result = h.run(variables);
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "name": result.name,
                    "success": result.success,
                    "agreement": result.agreement,
                    "sense": result.sense_output,
                    "antisense": result.antisense_output,
                    "output": result.resolved_output,
                    "duration_us": result.duration_us,
                }))
                .unwrap_or_default()
            );
        }
        HeligramAction::Test { path } => {
            let h = match Heligram::load(Path::new(path)) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            let result = h.test();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "name": result.name,
                    "total": result.total,
                    "passed": result.passed,
                    "failed": result.failed,
                    "results": result.results,
                }))
                .unwrap_or_default()
            );

            if result.failed > 0 {
                std::process::exit(1);
            }
        }
        HeligramAction::TestAll { dir } => {
            let heligrams = match load_all(Path::new(dir)) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            if heligrams.is_empty() {
                println!(
                    "{}",
                    json!({"status": "ok", "message": "No heligrams found", "total": 0})
                );
                return;
            }

            let mut total_pass = 0usize;
            let mut total_fail = 0usize;
            let mut total_tests = 0usize;
            let mut results = Vec::new();

            for h in &heligrams {
                let r = h.test();
                total_pass += r.passed;
                total_fail += r.failed;
                total_tests += r.total;
                results.push(json!({
                    "name": r.name,
                    "total": r.total,
                    "passed": r.passed,
                    "failed": r.failed,
                }));
            }

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "status": if total_fail == 0 { "ok" } else { "fail" },
                    "heligrams": heligrams.len(),
                    "total_tests": total_tests,
                    "passed": total_pass,
                    "failed": total_fail,
                    "results": results,
                }))
                .unwrap_or_default()
            );

            if total_fail > 0 {
                std::process::exit(1);
            }
        }
        HeligramAction::List { dir } => {
            let heligrams = match load_all(Path::new(dir)) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            let entries: Vec<_> = heligrams
                .iter()
                .map(|h| {
                    json!({
                        "name": h.name,
                        "version": h.version,
                        "description": h.description,
                        "twist_rate": h.helix.twist_rate,
                        "base_pairs": h.helix.base_pairs.len(),
                        "tests": h.tests.len(),
                    })
                })
                .collect();

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "count": entries.len(),
                    "heligrams": entries,
                }))
                .unwrap_or_default()
            );
        }
        HeligramAction::Encode { path } => {
            let yaml_bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Cannot read {path}: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            let result = dna::encode_heligram(&yaml_bytes);
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "sense": result.sense,
                    "antisense": result.antisense,
                    "nucleotides": result.nucleotides,
                    "codons": result.codons,
                    "bytes": result.bytes,
                    "complement_verified": dna::complement(&result.sense) == result.antisense,
                }))
                .unwrap_or_default()
            );
        }
        HeligramAction::Decode { dna: dna_str } => {
            match dna::decode(dna_str) {
                Ok(bytes) => {
                    let yaml = String::from_utf8_lossy(&bytes);
                    // Try to parse as heligram to verify
                    match Heligram::parse(&yaml) {
                        Ok(h) => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&json!({
                                    "status": "ok",
                                    "name": h.name,
                                    "type": h.heligram_type,
                                    "tests": h.tests.len(),
                                    "yaml_preview": &yaml[..yaml.len().min(200)],
                                }))
                                .unwrap_or_default()
                            );
                        }
                        Err(e) => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&json!({
                                    "status": "decoded_but_invalid_heligram",
                                    "error": e,
                                    "yaml_preview": &yaml[..yaml.len().min(200)],
                                    "bytes": bytes.len(),
                                }))
                                .unwrap_or_default()
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            }
        }
        HeligramAction::Promote { path, output } => {
            let mg_yaml = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Cannot read {path}: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            let mg: rsk::modules::microgram::Microgram = match serde_yaml::from_str(&mg_yaml) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Parse error: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            let heligram = match promote::promote(&mg) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Promotion failed: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            let yaml_out = match promote::to_yaml(&heligram) {
                Ok(y) => y,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Serialization failed: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            match output {
                Some(out_path) => {
                    if let Err(e) = std::fs::write(out_path, &yaml_out) {
                        eprintln!(
                            "{}",
                            json!({"status": "error", "message": format!("Cannot write {out_path}: {e}")})
                        );
                        std::process::exit(1);
                    }
                    println!(
                        "{}",
                        json!({
                            "status": "ok",
                            "promoted": heligram.name,
                            "output": out_path,
                            "base_pairs": heligram.helix.base_pairs.len(),
                            "resolution_rules": heligram.resolution.rules.len(),
                            "tests": heligram.tests.len(),
                        })
                    );
                }
                None => {
                    print!("{yaml_out}");
                }
            }
        }
        HeligramAction::Forge { path, output } => {
            let mg_yaml = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Cannot read {path}: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            let mg: rsk::modules::microgram::Microgram = match serde_yaml::from_str(&mg_yaml) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Parse error: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            let heligram = match forge::forge(&mg) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Forge failed: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            let yaml_out = match promote::to_yaml(&heligram) {
                Ok(y) => y,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Serialization failed: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            match output {
                Some(out_path) => {
                    if let Err(e) = std::fs::write(out_path, &yaml_out) {
                        eprintln!(
                            "{}",
                            json!({"status": "error", "message": format!("Cannot write {out_path}: {e}")})
                        );
                        std::process::exit(1);
                    }
                    println!(
                        "{}",
                        json!({
                            "status": "ok",
                            "forged": heligram.name,
                            "output": out_path,
                            "description": heligram.description,
                            "base_pairs": heligram.helix.base_pairs.len(),
                            "resolution_rules": heligram.resolution.rules.len(),
                            "tests": heligram.tests.len(),
                        })
                    );
                }
                None => {
                    print!("{yaml_out}");
                }
            }
        }
        HeligramAction::Chain {
            chain: chain_str,
            dir,
            input,
        } => {
            let names: Vec<&str> = chain_str.split("->").map(|s| s.trim()).collect();

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

            match chain(&names, Path::new(dir), variables) {
                Ok(result) => {
                    let step_summaries: Vec<_> = result
                        .steps
                        .iter()
                        .map(|s| {
                            json!({
                                "name": s.name,
                                "agreement": s.agreement,
                                "verdict": s.resolved_output.get("verdict"),
                                "confidence": s.resolved_output.get("confidence"),
                                "duration_us": s.duration_us,
                            })
                        })
                        .collect();

                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "success": result.success,
                            "steps": step_summaries,
                            "consensus_ratio": result.consensus_ratio,
                            "final_output": result.final_output,
                            "total_duration_us": result.total_duration_us,
                        }))
                        .unwrap_or_default()
                    );
                }
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            }
        }
    }
}
