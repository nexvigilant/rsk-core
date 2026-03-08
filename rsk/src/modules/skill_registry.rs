//! # Skill Registry
//!
//! Centralized registry for discovering, validating, and routing skills.
//! Handles the mapping between high-level skill names and their underlying
//! implementations (Rust intrinsics, YAML logic trees, or LLM fallbacks).

use crate::modules::decision_engine::DecisionTree;
use crate::modules::text_processor::extract_smst;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Represents the execution strategy for a skill
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStrategy {
    /// Pure Rust implementation (intrinsic)
    RustIntrinsic,
    /// Deterministic logic tree (no LLM)
    DeterministicLogic,
    /// Hybrid (Deterministic logic with LLM fallbacks)
    Hybrid,
    /// Pure LLM prompting
    PureLlm,
}

/// Metadata and implementation pointers for a single skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub name: String,
    pub version: String,
    pub smst_score: f64,
    pub strategy: ExecutionStrategy,
    pub logic_path: Option<PathBuf>,
    pub skill_md_path: PathBuf,
}

/// The Skill Registry
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SkillRegistry {
    pub skills: HashMap<String, SkillEntry>,
    #[serde(skip)]
    pub base_dir: Option<PathBuf>,
}

// ═══════════════════════════════════════════════════════════════════════════
// IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════════════════

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load skills from a root directory recursively
    pub fn load_from_directory<P: AsRef<Path>>(&mut self, root: P) -> Result<(), String> {
        let root_path = root.as_ref();
        self.base_dir = Some(root_path.to_path_buf());

        // Find all SKILL.md files
        let walker = walkdir::WalkDir::new(root_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() == "SKILL.md");

        for entry in walker {
            let path = entry.path();
            if let Err(e) = self.register_skill_from_path(path) {
                eprintln!("Warning: Failed to register skill at {path:?}: {e}");
            }
        }

        Ok(())
    }

    fn register_skill_from_path(&mut self, skill_md_path: &Path) -> Result<(), String> {
        let content = fs::read_to_string(skill_md_path).map_err(|e| e.to_string())?;

        let smst = extract_smst(&content);
        let name = smst.frontmatter.name.clone();

        // Check for logic.yaml in the same directory
        let dir = skill_md_path.parent().unwrap_or(skill_md_path);
        let logic_path = dir.join("logic.yaml");
        let has_logic = logic_path.exists();

        // Determine strategy
        let strategy = if has_logic {
            let logic_content = fs::read_to_string(&logic_path).unwrap_or_default();
            if let Ok(tree) = serde_yaml::from_str::<DecisionTree>(&logic_content) {
                // Check if any node is an LLM fallback
                let has_llm = tree.nodes.values().any(|n| {
                    matches!(
                        n,
                        crate::modules::decision_engine::DecisionNode::LlmFallback { .. }
                    )
                });
                if has_llm {
                    ExecutionStrategy::Hybrid
                } else {
                    ExecutionStrategy::DeterministicLogic
                }
            } else {
                ExecutionStrategy::PureLlm
            }
        } else {
            // Some skills might be built-in intrinsics even without logic.yaml
            // We can add a lookup table here for known intrinsics
            match name.as_str() {
                "is-prime" | "topological-sort" | "levenshtein" | "sha256" => {
                    ExecutionStrategy::RustIntrinsic
                }
                _ => ExecutionStrategy::PureLlm,
            }
        };

        self.skills.insert(
            name.clone(),
            SkillEntry {
                name,
                version: smst
                    .frontmatter
                    .version
                    .clone()
                    .unwrap_or_else(|| "0.1.0".to_string()),
                smst_score: smst.score.total_score,
                strategy,
                logic_path: if has_logic { Some(logic_path) } else { None },
                skill_md_path: skill_md_path.to_path_buf(),
            },
        );

        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&SkillEntry> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<&SkillEntry> {
        self.skills.values().collect()
    }

    /// Save the registry to a JSON file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Load the registry from a JSON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let json = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&json).map_err(|e| e.to_string())
    }

    /// Validate a chain of skills starting from a root skill
    pub fn validate_chain(&self, start_skill: &str, depth: usize) -> Vec<(String, bool, f64)> {
        let mut results = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        queue.push_back((start_skill.to_string(), 0));

        while let Some((skill_name, current_depth)) = queue.pop_front() {
            if current_depth > depth || visited.contains(&skill_name) {
                continue;
            }

            visited.insert(skill_name.clone());

            if let Some(entry) = self.get(&skill_name) {
                let passed = entry.smst_score >= 85.0;
                results.push((skill_name.clone(), passed, entry.smst_score));

                // Add children from adjacencies
                let content = fs::read_to_string(&entry.skill_md_path).unwrap_or_default();
                let smst = extract_smst(&content);

                for adj in smst.frontmatter.adjacencies {
                    queue.push_back((adj.target, current_depth + 1));
                }
            } else {
                results.push((skill_name.clone(), false, 0.0));
            }
        }

        results
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_registry_load() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("test-skill");
        fs::create_dir(&skill_dir).unwrap();

        let skill_md = skill_dir.join("SKILL.md");
        fs::write(
            skill_md,
            "---
name: test-skill
version: 1.0.0
compliance-level: diamond
---
# test-skill
## Machine Specification
### 1. INPUTS
",
        )
        .unwrap();

        let mut registry = SkillRegistry::new();
        registry.load_from_directory(dir.path()).unwrap();

        assert!(registry.get("test-skill").is_some());
        let entry = registry.get("test-skill").unwrap();
        assert_eq!(entry.strategy, ExecutionStrategy::PureLlm);
    }

    #[test]
    fn test_registry_deterministic_logic() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("is-prime");
        fs::create_dir(&skill_dir).unwrap();

        fs::write(
            skill_dir.join("SKILL.md"),
            "---
name: is-prime
version: 1.0.0
compliance-level: diamond
---",
        )
        .unwrap();

        fs::write(
            skill_dir.join("logic.yaml"),
            "
start: check
nodes:
  check:
    type: return
    value: true
",
        )
        .unwrap();

        let mut registry = SkillRegistry::new();
        registry.load_from_directory(dir.path()).unwrap();

        let entry = registry.get("is-prime").unwrap();
        assert_eq!(entry.strategy, ExecutionStrategy::DeterministicLogic);
    }
}
