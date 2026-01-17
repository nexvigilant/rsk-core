//! Integration tests for rsk (Rust Skill Kernel)
//!
//! Tests full workflows: SKILL.md → SMST → Validation → Code Generation

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Helper to run rsk command and capture output
fn run_rsk(args: &[&str]) -> (String, String, bool) {
    let rsk_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // Go up to workspace root (.gemini/rust)
        .unwrap()
        .join("target")
        .join("debug")
        .join("rsk");

    let output = Command::new(&rsk_path)
        .args(args)
        .output()
        .expect("Failed to execute rsk");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
}

/// Sample SKILL.md content for testing
const SAMPLE_SKILL_MD: &str = r#"---
name: test-integration-skill
description: A skill for integration testing
version: 1.0.0
compliance-level: diamond
categories:
  - testing
  - integration
author: test-author
user-invocable: true
---

# test-integration-skill

This is a test skill for integration testing.

## Machine Specification

### 1. INPUTS

- `input_path` (String): Path to input file
- `threshold` (i32): Score threshold value
- `options` (Object): Configuration options

### 2. OUTPUTS

- `result` (String): Processing result
- `score` (f64): Calculated score
- `passed` (bool): Whether validation passed

### 3. STATE

- `cache` (Object): Internal cache for memoization
- `execution_count` (i64): Number of executions

### 4. OPERATOR MODE

| Mode | Behavior |
|------|----------|
| validate | Validate inputs only |
| execute | Full execution with output |
| dry-run | Show what would be done |

### 5. PERFORMANCE

- Latency: <50ms p95
- Throughput: 100 req/s
- Delegated to: rsk kernel

### 6. INVARIANTS

- Score must be between 0 and 100
- Input path must be a valid file path
- Threshold must be positive
- Cache must be invalidated on configuration change

### 7. FAILURE MODES

- FM-001: File not found (recoverable)
- FM-002: Invalid JSON format (recoverable)
- FM-003: Score exceeds threshold (warning)
- FM-004: Out of memory (critical)

### 8. TELEMETRY

- execution_time_ms: Time taken for execution
- score_value: Calculated score
- failure_count: Number of failures
- cache_hit_rate: Cache efficiency metric
"#;

mod smst_extraction {
    use super::*;

    #[test]
    fn test_full_smst_extraction_workflow() {
        // Create temp file with SKILL.md content
        let temp_dir = std::env::temp_dir();
        let skill_path = temp_dir.join("test_skill").join("SKILL.md");
        fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
        fs::write(&skill_path, SAMPLE_SKILL_MD).unwrap();

        // Test text smst command
        let (stdout, stderr, success) = run_rsk(&["text", "smst", skill_path.to_str().unwrap()]);

        assert!(success, "Command failed: {}", stderr);
        assert!(stdout.contains("\"name\": \"test-integration-skill\""));
        assert!(stdout.contains("\"compliance_level\": \"diamond\""));
        assert!(stdout.contains("\"total_score\""));
        assert!(stdout.contains("\"is_diamond_compliant\": true"));

        // Cleanup
        fs::remove_dir_all(temp_dir.join("test_skill")).ok();
    }

    #[test]
    fn test_verify_command_passes_diamond() {
        let temp_dir = std::env::temp_dir();
        let base_path = temp_dir.join("test_verify_skill");
        let skill_path = base_path.join("SKILL.md");
        fs::create_dir_all(&base_path).unwrap();
        
        // Create required directories for Diamond compliance
        fs::create_dir_all(base_path.join("scripts")).unwrap();
        fs::create_dir_all(base_path.join("references")).unwrap();
        fs::create_dir_all(base_path.join("templates")).unwrap();
        fs::create_dir_all(base_path.join("tests")).unwrap();
        
        // Create required artifacts
        fs::write(base_path.join("logic.yaml"), "nodes: {}").unwrap();
        fs::write(base_path.join("validation_rules.json"), "{}").unwrap();
        fs::write(base_path.join("tests/scaffold.rs"), "// test").unwrap();
        
        // Create dummy verify script that succeeds
        let verify_script = base_path.join("scripts/verify");
        fs::write(&verify_script, "#!/bin/sh\nexit 0").unwrap();
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&verify_script, fs::Permissions::from_mode(0o755)).unwrap();
        }
        
        fs::write(&skill_path, SAMPLE_SKILL_MD).unwrap();

        let (stdout, stderr, success) =
            run_rsk(&["verify", base_path.to_str().unwrap(), "--threshold", "85"]);

        assert!(success, "Verify failed: {}", stderr);
        assert!(stdout.contains("\"passed\": true"));
        assert!(stdout.contains("\"compliance_level\": \"diamond\""));

        fs::remove_dir_all(&base_path).ok();
    }

    #[test]
    fn test_verify_threshold_failure() {
        // Create a minimal SKILL.md that won't pass diamond threshold
        let minimal_skill = r#"---
name: minimal-skill
description: Minimal skill
---

# minimal-skill

No machine spec.
"#;
        let temp_dir = std::env::temp_dir();
        let skill_path = temp_dir.join("test_minimal_skill").join("SKILL.md");
        fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
        fs::write(&skill_path, minimal_skill).unwrap();

        let (stdout, _, success) = run_rsk(&[
            "verify",
            skill_path.to_str().unwrap(),
            "--threshold",
            "85",
            "--format",
            "json",
        ]);

        // Should complete but report failure
        assert!(stdout.contains("\"passed\": false"));

        fs::remove_dir_all(temp_dir.join("test_minimal_skill")).ok();
    }
}

mod code_generation {
    use super::*;

    #[test]
    fn test_generate_rules_json() {
        let temp_dir = std::env::temp_dir();
        let skill_path = temp_dir.join("test_gen_rules").join("SKILL.md");
        fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
        fs::write(&skill_path, SAMPLE_SKILL_MD).unwrap();

        let (stdout, stderr, success) =
            run_rsk(&["generate", "rules", skill_path.to_str().unwrap()]);

        assert!(success, "Generate rules failed: {}", stderr);
        assert!(stdout.contains("\"skill_name\": \"test-integration-skill\""));
        assert!(stdout.contains("\"invariant_rules\""));
        assert!(stdout.contains("\"failure_mode_rules\""));
        assert!(stdout.contains("\"total_rules\""));

        fs::remove_dir_all(temp_dir.join("test_gen_rules")).ok();
    }

    #[test]
    fn test_generate_tests_rust() {
        let temp_dir = std::env::temp_dir();
        let skill_path = temp_dir.join("test_gen_tests").join("SKILL.md");
        fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
        fs::write(&skill_path, SAMPLE_SKILL_MD).unwrap();

        let (stdout, stderr, success) = run_rsk(&[
            "generate",
            "tests",
            skill_path.to_str().unwrap(),
            "--format",
            "rust",
        ]);

        assert!(success, "Generate tests failed: {}", stderr);
        assert!(stdout.contains("#[test]"));
        assert!(stdout.contains("fn test_test_integration_skill"));
        assert!(stdout.contains("#[cfg(test)]"));

        fs::remove_dir_all(temp_dir.join("test_gen_tests")).ok();
    }

    #[test]
    fn test_generate_stub() {
        let temp_dir = std::env::temp_dir();
        let skill_path = temp_dir.join("test_gen_stub").join("SKILL.md");
        fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
        fs::write(&skill_path, SAMPLE_SKILL_MD).unwrap();

        let (stdout, stderr, success) =
            run_rsk(&["generate", "stub", skill_path.to_str().unwrap()]);

        assert!(success, "Generate stub failed: {}", stderr);
        assert!(stdout.contains("pub struct TestIntegrationSkillInput"));
        assert!(stdout.contains("pub struct TestIntegrationSkillOutput"));
        assert!(stdout.contains("pub fn test_integration_skill"));
        assert!(stdout.contains("use serde::{Deserialize, Serialize}"));

        fs::remove_dir_all(temp_dir.join("test_gen_stub")).ok();
    }
}

mod graph_operations {
    use super::*;

    #[test]
    fn test_graph_levels_json() {
        // JSON must be on one line for shell parsing
        let graph_json = r#"[{"name":"a","dependencies":[],"adjacencies":[]},{"name":"b","dependencies":["a"],"adjacencies":[]},{"name":"c","dependencies":["a"],"adjacencies":[]},{"name":"d","dependencies":["b","c"],"adjacencies":[]}]"#;

        let (stdout, stderr, success) = run_rsk(&["graph", "levels", "-i", graph_json]);

        assert!(success, "Graph levels failed: {}", stderr);
        assert!(stdout.contains("\"status\":\"success\""), "Got: {}", stdout);
        assert!(stdout.contains("\"total_levels\":3"));
        assert!(stdout.contains("\"max_parallelism\":2"));
    }

    #[test]
    fn test_graph_levels_cycle_detection() {
        let graph_json = r#"[{"name":"a","dependencies":["b"],"adjacencies":[]},{"name":"b","dependencies":["a"],"adjacencies":[]}]"#;

        let (stdout, _, _) = run_rsk(&["graph", "levels", "-i", graph_json]);

        assert!(stdout.contains("\"status\":\"error\""), "Got: {}", stdout);
        assert!(stdout.contains("Cycle detected"));
    }

    #[test]
    fn test_graph_topsort_json() {
        let graph_json = r#"[{"name":"c","dependencies":["b"],"adjacencies":[]},{"name":"b","dependencies":["a"],"adjacencies":[]},{"name":"a","dependencies":[],"adjacencies":[]}]"#;

        let (stdout, stderr, success) = run_rsk(&["graph", "top-sort", "-i", graph_json]);

        assert!(success, "TopSort failed: {}", stderr);
        assert!(stdout.contains("\"status\":\"success\""), "Got: {}", stdout);
        // Order should be a, b, c
        let result_pos = stdout.find("\"result\"").unwrap();
        let a_pos = stdout[result_pos..].find("\"a\"").unwrap();
        let b_pos = stdout[result_pos..].find("\"b\"").unwrap();
        let c_pos = stdout[result_pos..].find("\"c\"").unwrap();
        assert!(a_pos < b_pos && b_pos < c_pos);
    }
}

mod algorithm_operations {
    use super::*;

    #[test]
    fn test_levenshtein() {
        let (stdout, stderr, success) = run_rsk(&["levenshtein", "kitten", "sitting"]);

        assert!(success, "Levenshtein failed: {}", stderr);
        // JSON uses compact format (no spaces after colons)
        assert!(stdout.contains("\"distance\":3"), "Got: {}", stdout);
        assert!(stdout.contains("\"similarity\""));
    }

    #[test]
    fn test_fuzzy_search() {
        let (stdout, stderr, success) = run_rsk(&[
            "fuzzy",
            "verify",
            "-c",
            "verify,build,validate,version,verilog",
            "-l",
            "3",
        ]);

        assert!(success, "Fuzzy search failed: {}", stderr);
        assert!(stdout.contains("\"status\":\"success\""), "Got: {}", stdout);
        assert!(stdout.contains("\"matches\""));
    }

    #[test]
    fn test_sha256_hash() {
        let (stdout, stderr, success) = run_rsk(&["sha256", "hash", "hello"]);

        assert!(success, "SHA256 failed: {}", stderr);
        // SHA256 of "hello" is known
        assert!(
            stdout.contains("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
        );
    }

    #[test]
    fn test_variance() {
        let (stdout, stderr, success) = run_rsk(&["variance", "100", "80"]);

        assert!(success, "Variance failed: {}", stderr);
        assert!(stdout.contains("\"absolute\":20"), "Got: {}", stdout);
        assert!(stdout.contains("\"percentage\":25"));
    }
}

mod end_to_end {
    use super::*;

    #[test]
    fn test_full_skill_validation_pipeline() {
        // This test simulates the full workflow:
        // 1. Parse SKILL.md
        // 2. Extract SMST
        // 3. Verify compliance
        // 4. Generate validation rules
        // 5. Generate test scaffold

        let temp_dir = std::env::temp_dir();
        let skill_dir = temp_dir.join("test_e2e_skill");
        let skill_path = skill_dir.join("SKILL.md");
        fs::create_dir_all(&skill_dir).unwrap();
        
        // Create required directories for Diamond compliance
        fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        fs::create_dir_all(skill_dir.join("references")).unwrap();
        fs::create_dir_all(skill_dir.join("templates")).unwrap();
        fs::create_dir_all(skill_dir.join("tests")).unwrap();
        
        // Create required artifacts
        fs::write(skill_dir.join("logic.yaml"), "nodes: {}").unwrap();
        fs::write(skill_dir.join("validation_rules.json"), "{}").unwrap();
        fs::write(skill_dir.join("tests/scaffold.rs"), "// test").unwrap();
        
        // Create dummy verify script
        let verify_script = skill_dir.join("scripts/verify");
        fs::write(&verify_script, "#!/bin/sh\nexit 0").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&verify_script, fs::Permissions::from_mode(0o755)).unwrap();
        }
        
        fs::write(&skill_path, SAMPLE_SKILL_MD).unwrap();

        // Step 1: Parse (compact JSON format)
        let (stdout, _, success) = run_rsk(&["text", "parse", skill_path.to_str().unwrap()]);
        assert!(success, "Parse failed");
        assert!(
            stdout.contains("\"skill_name\":\"test-integration-skill\""),
            "Parse output: {}",
            stdout
        );
        assert!(stdout.contains("\"has_machine_spec\":true"));

        // Step 2: SMST extraction (pretty-printed JSON)
        let (stdout, _, success) = run_rsk(&["text", "smst", skill_path.to_str().unwrap()]);
        assert!(success, "SMST extraction failed");
        let smst: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(smst["is_diamond_compliant"].as_bool().unwrap());

        // Step 3: Verify (pretty-printed JSON)
        let (stdout, _, success) =
            run_rsk(&["verify", skill_dir.to_str().unwrap(), "--threshold", "85"]);
        assert!(success, "Verify failed");
        let verify: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(verify["passed"].as_bool().unwrap());

        // Step 4: Generate rules (pretty-printed JSON)
        let (stdout, _, success) = run_rsk(&["generate", "rules", skill_path.to_str().unwrap()]);
        assert!(success, "Generate rules failed");
        let rules: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(rules["total_rules"].as_u64().unwrap() > 0);

        // Step 5: Generate tests (Rust code)
        let (stdout, _, success) = run_rsk(&[
            "generate",
            "tests",
            skill_path.to_str().unwrap(),
            "--format",
            "rust",
        ]);
        assert!(success, "Generate tests failed");
        assert!(stdout.contains("#[test]"));

        // Cleanup
        fs::remove_dir_all(&skill_dir).ok();
    }
}
