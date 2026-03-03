//! Microgram CLI handler.

use crate::cli::actions::MicrogramAction;
use rsk::modules::decision_engine::Value as RskValue;
use rsk::modules::microgram::{
    auto_execute, bench_all, catalog, clone_mutated, compose, coverage_all, diff,
    evolve_tests, load_all, matrix, merge, pipe, pipe_chain, shrink, snapshot_restore,
    snapshot_save, stress_all, test_all, CompositionGoal, Microgram, MicrogramSpec,
};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;

pub fn handle_microgram(action: &MicrogramAction) {
    match action {
        MicrogramAction::Run { path, input } => {
            let mg = match Microgram::load(Path::new(path)) {
                Ok(m) => m,
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

            let result = mg.run(variables);
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "name": result.name,
                    "success": result.success,
                    "path": result.path,
                    "output": result.output,
                    "duration_us": result.duration_us,
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Test { path } => {
            let mg = match Microgram::load(Path::new(path)) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            let result = mg.test();
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
        MicrogramAction::TestAll { dir } => {
            let results = match test_all(Path::new(dir)) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            let total: usize = results.iter().map(|r| r.total).sum();
            let passed: usize = results.iter().map(|r| r.passed).sum();
            let failed: usize = results.iter().map(|r| r.failed).sum();

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "micrograms": results.len(),
                    "total_tests": total,
                    "passed": passed,
                    "failed": failed,
                    "results": results.iter().map(|r| json!({
                        "name": r.name,
                        "passed": r.passed,
                        "failed": r.failed,
                    })).collect::<Vec<_>>(),
                }))
                .unwrap_or_default()
            );

            if failed > 0 {
                std::process::exit(1);
            }
        }
        MicrogramAction::List { dir } => {
            let micrograms = match load_all(Path::new(dir)) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "micrograms": micrograms.iter().map(|mg| json!({
                        "name": mg.name,
                        "description": mg.description,
                        "version": mg.version,
                        "nodes": mg.tree.nodes.len(),
                        "tests": mg.tests.len(),
                    })).collect::<Vec<_>>(),
                    "total": micrograms.len(),
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Chain { chain, dir, input } => {
            use rsk::modules::microgram::chain_by_names;

            let names: Vec<&str> = chain.split("->").map(|s| s.trim()).collect();

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

            let result = match chain_by_names(Path::new(dir), &names, variables) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "success": result.success,
                    "steps": result.steps.iter().map(|s| json!({
                        "name": s.name,
                        "path": s.path,
                        "output": s.output,
                        "duration_us": s.duration_us,
                    })).collect::<Vec<_>>(),
                    "final_output": result.final_output,
                    "total_duration_us": result.total_duration_us,
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Generate {
            name, desc, var, op, threshold, true_label, false_label, out_dir,
        } => {
            let spec = MicrogramSpec {
                name: name.clone(),
                description: desc.clone(),
                variable: var.clone(),
                operator: op.clone(),
                threshold: RskValue::Int(*threshold),
                true_label: true_label.clone(),
                true_value: RskValue::Bool(true),
                false_label: false_label.clone(),
                false_value: RskValue::Bool(false),
            };

            let yaml = match spec.to_yaml() {
                Ok(y) => y,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            // Write to file
            let dir = Path::new(out_dir);
            if !dir.exists() {
                if let Err(e) = std::fs::create_dir_all(dir) {
                    eprintln!("{}", json!({"status": "error", "message": format!("Cannot create dir: {e}")}));
                    std::process::exit(1);
                }
            }

            let file_path = dir.join(format!("{}.yaml", name));
            if let Err(e) = std::fs::write(&file_path, &yaml) {
                eprintln!("{}", json!({"status": "error", "message": format!("Cannot write: {e}")}));
                std::process::exit(1);
            }

            // Verify: load and run self-tests
            let mg = Microgram::load(&file_path).unwrap_or_else(|e| {
                eprintln!("{}", json!({"status": "error", "message": e}));
                std::process::exit(1);
            });
            let test_result = mg.test();

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "generated": file_path.display().to_string(),
                    "name": name,
                    "tests": test_result.total,
                    "passed": test_result.passed,
                    "failed": test_result.failed,
                }))
                .unwrap_or_default()
            );

            if test_result.failed > 0 {
                std::process::exit(1);
            }
        }
        MicrogramAction::Evolve { path } => {
            let mg = match Microgram::load(Path::new(path)) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            let new_tests = evolve_tests(&mg);

            if new_tests.is_empty() {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "name": mg.name,
                        "existing_tests": mg.tests.len(),
                        "new_tests": 0,
                        "message": "No additional boundary tests suggested",
                    }))
                    .unwrap_or_default()
                );
                return;
            }

            // Merge new tests into the microgram
            let mut evolved = mg.clone();
            evolved.tests.extend(new_tests.clone());

            // Write back
            let yaml = match serde_yaml::to_string(&evolved) {
                Ok(y) => y,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": format!("Serialize error: {e}")}));
                    std::process::exit(1);
                }
            };
            if let Err(e) = std::fs::write(path, &yaml) {
                eprintln!("{}", json!({"status": "error", "message": format!("Write error: {e}")}));
                std::process::exit(1);
            }

            // Verify evolved version
            let test_result = evolved.test();

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "name": evolved.name,
                    "existing_tests": mg.tests.len(),
                    "new_tests": new_tests.len(),
                    "total_tests": test_result.total,
                    "passed": test_result.passed,
                    "failed": test_result.failed,
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Compose { require, dir, input } => {
            let required: Vec<String> = require.split(',').map(|s| s.trim().to_string()).collect();

            let initial_input: HashMap<String, RskValue> = match serde_json::from_str(input) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Invalid input JSON: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            let goal = CompositionGoal {
                required_outputs: required,
                initial_input,
            };

            let plan = match compose(Path::new(dir), &goal) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "feasible": plan.feasible,
                    "chain": plan.chain,
                    "coverage": plan.coverage,
                    "missing": plan.missing,
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Bench { dir, iterations } => {
            let results = match bench_all(Path::new(dir), *iterations) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "iterations": iterations,
                    "results": results.iter().map(|r| json!({
                        "name": r.name,
                        "min_us": r.min_us,
                        "max_us": r.max_us,
                        "avg_us": format!("{:.1}", r.avg_us),
                        "p95_us": r.p95_us,
                        "tests_pass": r.tests_pass,
                    })).collect::<Vec<_>>(),
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Auto { require, dir, input } => {
            let required: Vec<String> = require.split(',').map(|s| s.trim().to_string()).collect();

            let initial_input: HashMap<String, RskValue> = match serde_json::from_str(input) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({"status": "error", "message": format!("Invalid input JSON: {e}")})
                    );
                    std::process::exit(1);
                }
            };

            let goal = CompositionGoal {
                required_outputs: required,
                initial_input,
            };

            let result = match auto_execute(Path::new(dir), &goal) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            let exec_json = result.execution.as_ref().map(|exec| json!({
                "success": exec.success,
                "steps": exec.steps.iter().map(|s| json!({
                    "name": s.name,
                    "path": s.path,
                    "output": s.output,
                    "duration_us": s.duration_us,
                })).collect::<Vec<_>>(),
                "final_output": exec.final_output,
                "total_duration_us": exec.total_duration_us,
            }));

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "plan": {
                        "feasible": result.plan.feasible,
                        "chain": result.plan.chain,
                        "coverage": result.plan.coverage,
                        "missing": result.plan.missing,
                    },
                    "execution": exec_json,
                    "verified": result.verified,
                    "total_duration_us": result.duration_us,
                }))
                .unwrap_or_default()
            );

            if !result.verified {
                std::process::exit(1);
            }
        }
        MicrogramAction::Catalog { dir } => {
            let cat = match catalog(Path::new(dir)) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "total_micrograms": cat.total_micrograms,
                    "total_tests": cat.total_tests,
                    "all_pass": cat.all_pass,
                    "entries": cat.entries.iter().map(|e| json!({
                        "name": e.name,
                        "description": e.description,
                        "inputs": e.inputs,
                        "outputs": e.outputs,
                        "tests": e.test_count,
                        "pass": e.tests_pass,
                    })).collect::<Vec<_>>(),
                    "connections": cat.connections.iter().map(|(a, b)| {
                        format!("{} -> {}", a, b)
                    }).collect::<Vec<_>>(),
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Diff { left, right } => {
            let a = match Microgram::load(Path::new(left)) {
                Ok(m) => m,
                Err(e) => { eprintln!("{}", json!({"status": "error", "message": e})); std::process::exit(1); }
            };
            let b = match Microgram::load(Path::new(right)) {
                Ok(m) => m,
                Err(e) => { eprintln!("{}", json!({"status": "error", "message": e})); std::process::exit(1); }
            };

            let d = diff(&a, &b);
            println!("{}", serde_json::to_string_pretty(&d).unwrap_or_default());
        }
        MicrogramAction::Merge { left, right, name, desc, out_dir } => {
            let a = match Microgram::load(Path::new(left)) {
                Ok(m) => m,
                Err(e) => { eprintln!("{}", json!({"status": "error", "message": e})); std::process::exit(1); }
            };
            let b = match Microgram::load(Path::new(right)) {
                Ok(m) => m,
                Err(e) => { eprintln!("{}", json!({"status": "error", "message": e})); std::process::exit(1); }
            };

            let merged = merge(&a, &b, name, desc);

            // Serialize and write
            let yaml = match serde_yaml::to_string(&merged) {
                Ok(y) => y,
                Err(e) => { eprintln!("{}", json!({"status": "error", "message": format!("{e}")})); std::process::exit(1); }
            };

            let dir = Path::new(out_dir);
            if !dir.exists() {
                let _ = std::fs::create_dir_all(dir);
            }
            let file_path = dir.join(format!("{}.yaml", name));
            if let Err(e) = std::fs::write(&file_path, &yaml) {
                eprintln!("{}", json!({"status": "error", "message": format!("{e}")}));
                std::process::exit(1);
            }

            // Verify
            let test_result = merged.test();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "merged": file_path.display().to_string(),
                    "name": name,
                    "nodes": merged.tree.nodes.len(),
                    "tests": test_result.total,
                    "passed": test_result.passed,
                    "failed": test_result.failed,
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Pipe { target, dir, inputs } => {
            let input_data: Vec<HashMap<String, RskValue>> = match serde_json::from_str(inputs) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": format!("Invalid JSON array: {e}")}));
                    std::process::exit(1);
                }
            };

            if target.contains("->") {
                // Chain mode
                let names: Vec<&str> = target.split("->").map(|s| s.trim()).collect();
                let result = match pipe_chain(Path::new(dir), &names, &input_data) {
                    Ok(r) => r,
                    Err(e) => { eprintln!("{}", json!({"status": "error", "message": e})); std::process::exit(1); }
                };
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            } else {
                // Single microgram mode — load by name from dir
                let all = match load_all(Path::new(dir)) {
                    Ok(a) => a,
                    Err(e) => { eprintln!("{}", json!({"status": "error", "message": e})); std::process::exit(1); }
                };
                let mg = match all.iter().find(|m| m.name == *target) {
                    Some(m) => m,
                    None => {
                        eprintln!("{}", json!({"status": "error", "message": format!("Microgram '{}' not found", target)}));
                        std::process::exit(1);
                    }
                };
                let result = pipe(mg, &input_data);
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            }
        }
        MicrogramAction::Snapshot { dir, out } => {
            let snap = match snapshot_save(Path::new(dir), Path::new(out)) {
                Ok(s) => s,
                Err(e) => { eprintln!("{}", json!({"status": "error", "message": e})); std::process::exit(1); }
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "file": out,
                    "micrograms": snap.micrograms.len(),
                    "total_tests": snap.total_tests,
                    "all_pass": snap.all_pass,
                    "timestamp": snap.timestamp,
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Restore { snap, dir } => {
            let count = match snapshot_restore(Path::new(snap), Path::new(dir)) {
                Ok(c) => c,
                Err(e) => { eprintln!("{}", json!({"status": "error", "message": e})); std::process::exit(1); }
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "restored": count,
                    "from": snap,
                    "to": dir,
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Stress { dir, iterations, seed } => {
            let results = match stress_all(Path::new(dir), *iterations, *seed) {
                Ok(r) => r,
                Err(e) => { eprintln!("{}", json!({"status": "error", "message": e})); std::process::exit(1); }
            };

            let total_errors: usize = results.iter().map(|r| r.errored).sum();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "micrograms": results.len(),
                    "iterations_each": iterations,
                    "total_errors": total_errors,
                    "results": results.iter().map(|r| json!({
                        "name": r.name,
                        "succeeded": r.succeeded,
                        "errored": r.errored,
                        "avg_us": format!("{:.1}", r.avg_us),
                        "max_us": r.max_us,
                    })).collect::<Vec<_>>(),
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Matrix { dir } => {
            let result = match matrix(Path::new(dir)) {
                Ok(r) => r,
                Err(e) => { eprintln!("{}", json!({"status": "error", "message": e})); std::process::exit(1); }
            };

            // Compact: only show interesting cells (self-match or cross-match)
            let interesting: Vec<_> = result.cells.iter()
                .filter(|c| c.matched > 0)
                .map(|c| json!({
                    "runner": c.runner,
                    "tests_from": c.test_from,
                    "matched": format!("{}/{}", c.matched, c.total),
                }))
                .collect();

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "total_runs": result.total_runs,
                    "cross_matches": result.cross_matches,
                    "matches": interesting,
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Coverage { dir } => {
            let results = match coverage_all(Path::new(dir)) {
                Ok(r) => r,
                Err(e) => { eprintln!("{}", json!({"status": "error", "message": e})); std::process::exit(1); }
            };

            let avg_cov: f64 = if results.is_empty() { 0.0 } else {
                results.iter().map(|r| r.coverage_pct).sum::<f64>() / results.len() as f64
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "avg_coverage_pct": format!("{:.1}", avg_cov),
                    "results": results.iter().map(|r| json!({
                        "name": r.name,
                        "nodes": r.total_nodes,
                        "covered": r.covered_nodes,
                        "pct": format!("{:.0}%", r.coverage_pct),
                        "uncovered": r.uncovered,
                    })).collect::<Vec<_>>(),
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Clone { source, name, delta, out_dir } => {
            let mg = match Microgram::load(Path::new(source)) {
                Ok(m) => m,
                Err(e) => { eprintln!("{}", json!({"status": "error", "message": e})); std::process::exit(1); }
            };

            let mutant = clone_mutated(&mg, name, *delta);

            let yaml = match serde_yaml::to_string(&mutant) {
                Ok(y) => y,
                Err(e) => { eprintln!("{}", json!({"status": "error", "message": format!("{e}")})); std::process::exit(1); }
            };

            let dir = Path::new(out_dir);
            if !dir.exists() { let _ = std::fs::create_dir_all(dir); }
            let file_path = dir.join(format!("{}.yaml", name));
            if let Err(e) = std::fs::write(&file_path, yaml) {
                eprintln!("{}", json!({"status": "error", "message": format!("{e}")}));
                std::process::exit(1);
            }

            let test_result = mutant.test();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "cloned": file_path.display().to_string(),
                    "source": mg.name,
                    "delta": delta,
                    "tests": test_result.total,
                    "passed": test_result.passed,
                }))
                .unwrap_or_default()
            );
        }
        MicrogramAction::Shrink { path, input } => {
            let mg = match Microgram::load(Path::new(path)) {
                Ok(m) => m,
                Err(e) => { eprintln!("{}", json!({"status": "error", "message": e})); std::process::exit(1); }
            };

            let input_map: HashMap<String, RskValue> = match serde_json::from_str(input) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": format!("Invalid JSON: {e}")}));
                    std::process::exit(1);
                }
            };

            let original = mg.run(input_map.clone());
            let minimal = shrink(&mg, &input_map);
            let shrunk = mg.run(minimal.clone());

            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "original_input": input_map,
                    "minimal_input": minimal,
                    "output": shrunk.output,
                    "same_output": original.output == shrunk.output,
                }))
                .unwrap_or_default()
            );
        }
    }
}
