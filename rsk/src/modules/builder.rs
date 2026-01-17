use crate::modules::text_processor::{extract_smst};
use crate::modules::code_generator::{generate_validation_rules, generate_test_scaffold, generate_decision_tree};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildResult {
    pub skill_name: String,
    pub path: PathBuf,
    pub artifacts_created: Vec<String>,
    pub status: String,
    pub error: Option<String>,
}

pub fn build_skill(path: &Path, dry_run: bool) -> BuildResult {
    let skill_md_path = path.join("SKILL.md");
    
    if !skill_md_path.exists() {
        return BuildResult {
            skill_name: "unknown".to_string(),
            path: path.to_path_buf(),
            artifacts_created: vec![],
            status: "failed".to_string(),
            error: Some("SKILL.md not found".to_string()),
        };
    }

    let content = match fs::read_to_string(&skill_md_path) {
        Ok(c) => c,
        Err(e) => return BuildResult {
            skill_name: "unknown".to_string(),
            path: path.to_path_buf(),
            artifacts_created: vec![],
            status: "failed".to_string(),
            error: Some(format!("Failed to read SKILL.md: {}", e)),
        },
    };

    let smst = extract_smst(&content);
    let skill_name = smst.frontmatter.name.clone();
    
    if skill_name == "unknown" || skill_name.is_empty() {
        return BuildResult {
            skill_name: "unknown".to_string(),
            path: path.to_path_buf(),
            artifacts_created: vec![],
            status: "failed".to_string(),
            error: Some("Invalid skill name in frontmatter".to_string()),
        };
    }

    let mut artifacts_created = Vec::new();

    if dry_run {
        return BuildResult {
            skill_name,
            path: path.to_path_buf(),
            artifacts_created: vec!["logic.yaml (planned)".to_string(), "validation_rules.json (planned)".to_string(), "tests/scaffold.rs (planned)".to_string()],
            status: "dry_run".to_string(),
            error: None,
        };
    }

    // Ensure directory structure exists
    if let Err(e) = fs::create_dir_all(path) {
        return BuildResult {
            skill_name,
            path: path.to_path_buf(),
            artifacts_created: vec![],
            status: "failed".to_string(),
            error: Some(format!("Failed to create skill directory: {}", e)),
        };
    }

    // 1. Generate logic.yaml
    let logic_path = path.join("logic.yaml");
    let tree = generate_decision_tree(&smst);
    match serde_yaml::to_string(&tree) {
        Ok(yaml) => {
            if let Err(e) = fs::write(&logic_path, yaml) {
                return BuildResult {
                    skill_name: skill_name.clone(),
                    path: path.to_path_buf(),
                    artifacts_created,
                    status: "failed".to_string(),
                    error: Some(format!("Failed to write logic.yaml: {}", e)),
                };
            }
            artifacts_created.push("logic.yaml".to_string());
        }
        Err(e) => return BuildResult {
            skill_name: skill_name.clone(),
            path: path.to_path_buf(),
            artifacts_created,
            status: "failed".to_string(),
            error: Some(format!("Failed to serialize logic.yaml: {}", e)),
        },
    }

    // 2. Generate validation_rules.json
    let rules_path = path.join("validation_rules.json");
    let rules = generate_validation_rules(&smst);
    match serde_json::to_string_pretty(&rules) {
        Ok(json) => {
            if let Err(e) = fs::write(&rules_path, json) {
                return BuildResult {
                    skill_name: skill_name.clone(),
                    path: path.to_path_buf(),
                    artifacts_created: artifacts_created.clone(),
                    status: "failed".to_string(),
                    error: Some(format!("Failed to write validation_rules.json: {}", e)),
                };
            }
            artifacts_created.push("validation_rules.json".to_string());
        }
        Err(e) => return BuildResult {
            skill_name: skill_name.clone(),
            path: path.to_path_buf(),
            artifacts_created,
            status: "failed".to_string(),
            error: Some(format!("Failed to serialize validation_rules.json: {}", e)),
        },
    }

    // 3. Generate test_scaffold.rs
    let tests_dir = path.join("tests");
    if !tests_dir.exists() {
        if let Err(e) = fs::create_dir_all(&tests_dir) {
             return BuildResult {
                skill_name: skill_name.clone(),
                path: path.to_path_buf(),
                artifacts_created,
                status: "failed".to_string(),
                error: Some(format!("Failed to create tests directory: {}", e)),
            };
        }
    }
    let test_scaffold = generate_test_scaffold(&smst);
    let scaffold_path = tests_dir.join("scaffold.rs");
    if let Err(e) = fs::write(&scaffold_path, test_scaffold.rust_code) {
        return BuildResult {
            skill_name: skill_name.clone(),
            path: path.to_path_buf(),
            artifacts_created,
            status: "failed".to_string(),
            error: Some(format!("Failed to write test_scaffold.rs: {}", e)),
        };
    }
    artifacts_created.push("tests/scaffold.rs".to_string());

    // 4. Ensure scripts directory exists for verify.py/build.py delegation
    let scripts_dir = path.join("scripts");
    if !scripts_dir.exists() {
        let _ = fs::create_dir_all(&scripts_dir);
    }

    BuildResult {
        skill_name,
        path: path.to_path_buf(),
        artifacts_created,
        status: "success".to_string(),
        error: None,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillCheck {
    pub name: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyResult {
    pub skill_name: String,
    pub score: f64,
    pub compliance_level: String,
    pub checks: Vec<SkillCheck>,
    pub status: String,
}

pub fn verify_skill_file(path: &Path) -> VerifyResult {
    let mut checks = Vec::new();

    if !path.exists() {
        return VerifyResult {
            skill_name: "unknown".to_string(),
            score: 0.0,
            compliance_level: "none".to_string(),
            checks: vec![SkillCheck {
                name: "File existence".to_string(),
                status: "failed".to_string(),
                message: format!("{:?} not found", path),
            }],
            status: "failed".to_string(),
        };
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return VerifyResult {
            skill_name: "unknown".to_string(),
            score: 0.0,
            compliance_level: "none".to_string(),
            checks: vec![SkillCheck {
                name: "File read".to_string(),
                status: "failed".to_string(),
                message: format!("Failed to read file: {}", e),
            }],
            status: "failed".to_string(),
        },
    };

    let smst = extract_smst(&content);
    let skill_name = smst.frontmatter.name.clone();

    checks.push(SkillCheck {
        name: "SMST v2.0 Compliance".to_string(),
        status: if smst.is_diamond_compliant { "passed".to_string() } else { "partial".to_string() },
        message: format!("Score: {:.1}/100, Level: {}", smst.score.total_score, smst.score.compliance_level),
    });

    // For a single file, we skip the directory-based checks for now
    // or we could look for artifacts in its parent directory.

    VerifyResult {
        skill_name,
        score: smst.score.total_score,
        compliance_level: smst.score.compliance_level,
        checks,
        status: if smst.is_diamond_compliant { "success".to_string() } else { "partial".to_string() },
    }
}

pub fn verify_skill(path: &Path) -> VerifyResult {
    let skill_md_path = path.join("SKILL.md");
    let mut checks = Vec::new();

    if !skill_md_path.exists() {
        return VerifyResult {
            skill_name: "unknown".to_string(),
            score: 0.0,
            compliance_level: "none".to_string(),
            checks: vec![SkillCheck {
                name: "SKILL.md existence".to_string(),
                status: "failed".to_string(),
                message: "SKILL.md not found".to_string(),
            }],
            status: "failed".to_string(),
        };
    }

    let content = match fs::read_to_string(&skill_md_path) {
        Ok(c) => c,
        Err(e) => return VerifyResult {
            skill_name: "unknown".to_string(),
            score: 0.0,
            compliance_level: "none".to_string(),
            checks: vec![SkillCheck {
                name: "SKILL.md read".to_string(),
                status: "failed".to_string(),
                message: format!("Failed to read SKILL.md: {}", e),
            }],
            status: "failed".to_string(),
        },
    };

    let smst = extract_smst(&content);
    let skill_name = smst.frontmatter.name.clone();

    checks.push(SkillCheck {
        name: "SMST v2.0 Compliance".to_string(),
        status: if smst.is_diamond_compliant { "passed".to_string() } else { "partial".to_string() },
        message: format!("Score: {:.1}/100, Level: {}", smst.score.total_score, smst.score.compliance_level),
    });

    // Check for artifacts
    let artifacts = [
        ("logic.yaml", "Deterministic logic tree"),
        ("validation_rules.json", "Extracted validation rules"),
        ("tests/scaffold.rs", "Rust test scaffold"),
    ];

    let mut artifacts_missing = false;
    for (file, desc) in artifacts {
        let file_path = path.join(file);
        let exists = file_path.exists();
        if !exists { artifacts_missing = true; }
        checks.push(SkillCheck {
            name: format!("Artifact: {}", file),
            status: if exists { "passed".to_string() } else { "missing".to_string() },
            message: desc.to_string(),
        });
    }

    // Check for scripts directory
    let scripts_dir = path.join("scripts");
    let scripts_missing = !scripts_dir.is_dir();
    let mut functional_tests_passed = false;

    if !scripts_missing {
        // Look for verification script in order of preference
        let verify_paths = [
            scripts_dir.join("verify"),
            scripts_dir.join("verify.sh"),
            scripts_dir.join("verify.py"),
        ];

        if let Some(verify_path) = verify_paths.iter().find(|p| p.exists()) {
            let mut cmd = if verify_path.extension().map_or(false, |ext| ext == "py") {
                let mut c = std::process::Command::new("python3");
                c.arg(verify_path);
                c
            } else if verify_path.extension().map_or(false, |ext| ext == "sh") {
                let mut c = std::process::Command::new("bash");
                c.arg(verify_path);
                c
            } else {
                std::process::Command::new(verify_path)
            };

            if let Ok(out) = cmd.output() {
                functional_tests_passed = out.status.success();
            }
        }

        checks.push(SkillCheck {
            name: "Scripts Directory".to_string(),
            status: "passed".to_string(),
            message: "Mandatory scripts/ directory exists".to_string(),
        });
    } else {
        checks.push(SkillCheck {
            name: "Scripts Directory".to_string(),
            status: "missing".to_string(),
            message: "Missing mandatory scripts/ directory".to_string(),
        });
    }

    if functional_tests_passed {
        checks.push(SkillCheck {
            name: "Functional Verification".to_string(),
            status: "passed".to_string(),
            message: "All internal tests and examples passed".to_string(),
        });
    } else if !scripts_missing {
        checks.push(SkillCheck {
            name: "Functional Verification".to_string(),
            status: "failed".to_string(),
            message: "Self-test failed or verify script not found".to_string(),
        });
    }

    // Strict Gold Standard
    let all_passed = checks.iter().any(|c| c.name == "SMST v2.0 Compliance" && (c.status == "passed" || c.status == "partial"))
        && !artifacts_missing 
        && !scripts_missing
        && functional_tests_passed;

    VerifyResult {
        skill_name,
        score: smst.score.total_score,
        compliance_level: smst.score.compliance_level,
        checks,
        status: if all_passed { "success".to_string() } else { "failed".to_string() },
    }
}
