use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;

fn get_base_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn get_skills_dir() -> PathBuf {
    PathBuf::from("/home/matthew/.claude/skills")
}

fn get_registry_file() -> PathBuf {
    get_base_dir().join("e2e_registry_rust.json")
}

fn ensure_registry() {
    let registry_file = get_registry_file();
    if !registry_file.exists() {
        let mut cmd = Command::cargo_bin("rsk").unwrap();
        cmd.arg("skills")
           .arg("scan")
           .arg(get_skills_dir().to_str().unwrap())
           .arg("--output")
           .arg(registry_file.to_str().unwrap());
        cmd.assert().success();
    }
}

#[test]
fn test_e2e_001_ecosystem_scan() {
    let mut cmd = Command::cargo_bin("rsk").unwrap();
    let skills_dir = get_skills_dir();
    let registry_file = get_registry_file();

    cmd.arg("skills")
       .arg("scan")
       .arg(skills_dir.to_str().unwrap())
       .arg("--output")
       .arg(registry_file.to_str().unwrap());

    cmd.assert()
       .success()
       .stdout(predicate::str::contains("\"status\": \"success\""))
       .stdout(predicate::str::contains("\"count\""));
    
    assert!(registry_file.exists());
}

#[test]
fn test_e2e_002_chain_integrity() {
    ensure_registry();
    let mut cmd = Command::cargo_bin("rsk").unwrap();
    let registry_file = get_registry_file();

    cmd.arg("chain")
       .arg("validate")
       .arg("strategy-engine")
       .arg("--registry")
       .arg(registry_file.to_str().unwrap());

    cmd.assert()
       .success()
       .stdout(predicate::str::contains("strategy-engine"));
    // Note: Don't check for FULL CHAIN VALIDATED if we expect gaps in the real ecosystem
}

#[test]
fn test_e2e_003_logic_synthesis() {
    ensure_registry();
    let mut cmd = Command::cargo_bin("rsk").unwrap();
    let registry_file = get_registry_file();

    cmd.arg("evolve")
       .arg("is-prime")
       .arg("--registry")
       .arg(registry_file.to_str().unwrap());

    cmd.assert()
       .success()
       .stdout(predicate::str::contains("\"status\": \"evolved\""))
       .stdout(predicate::str::contains("dynamic_intrinsics.rs"));
}

#[test]
fn test_e2e_004_deterministic_execution() {
    ensure_registry();
    let mut cmd = Command::cargo_bin("rsk").unwrap();
    let registry_file = get_registry_file();
    let inputs = r#"{"n": 17}"#;

    cmd.arg("skills")
       .arg("execute")
       .arg("is-prime")
       .arg("--input")
       .arg(inputs)
       .arg("--registry")
       .arg(registry_file.to_str().unwrap());

    cmd.assert()
       .success()
       .stdout(predicate::str::contains("\"status\": \"success\""))
       .stdout(predicate::str::contains("\"skill\": \"is-prime\""));
}

#[test]
fn test_e2e_005_strategic_optimization() {
    let mut cmd = Command::cargo_bin("rsk").unwrap();
    
    let strategy_data = r#"{        "fields": [
            {"id": "F1", "market_size": 1000.0, "growth_rate": 0.1, "capability_fit": 0.9, "competitive_intensity": 0.1}
        ],
        "tactics": [
            {"id": "T1", "differentiation": 0.9, "cost_advantage": 0.5, "execution_risk": 0.1}
        ]
    }"#;
    
    let logic_path = "/home/matthew/.claude/skills/strategy-engine/logic.yaml";
    if !PathBuf::from(logic_path).exists() {
        return; // Skip if environment not set up
    }
    let logic_tree = fs::read_to_string(logic_path).expect("Failed to read strategy logic");
    
    // In Rust we use the CLI to execute logic
    let test_tree = logic_tree.replace("start: validate_input", "start: exponential_optimization");
    
    cmd.arg("yaml")
       .arg("execute-logic")
       .arg("--tree")
       .arg(test_tree)
       .arg("--input")
       .arg(format!(r#"{{"strategy_data": {}}}"#, strategy_data));

    cmd.assert()
       .success()
       .stdout(predicate::str::contains("\"status\": \"success\""))
       .stdout(predicate::str::contains("llm_fallback"))
       .stdout(predicate::str::contains("PHASE 1: WINNING ASPIRATION"));
}
