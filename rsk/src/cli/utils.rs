//! Shared CLI utilities.
//!
//! This module provides common functions used across CLI handlers.

use rsk::{SkillGraph, SkillNode};
use std::fs;

/// Load a skill graph from JSON string or file path.
///
/// If input ends with ".json", reads from file; otherwise treats as JSON string.
/// Supports both direct SkillGraph format and Vec<SkillNode> format.
pub fn load_graph(input: &str) -> Result<SkillGraph, Box<dyn std::error::Error>> {
    let content = if input.ends_with(".json") {
        fs::read_to_string(input)?
    } else {
        input.to_string()
    };

    // Check if it's a direct SkillGraph or a list of nodes
    if let Ok(graph) = serde_json::from_str::<SkillGraph>(&content) {
        return Ok(graph);
    }

    if let Ok(nodes) = serde_json::from_str::<Vec<SkillNode>>(&content) {
        let mut graph = SkillGraph::new();
        for node in nodes {
            graph.add_node(node);
        }
        return Ok(graph);
    }

    Err("Invalid graph format".into())
}

/// Get the default path for the skill registry.
pub fn default_registry_path() -> std::path::PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".rsk/skills.json"))
        .unwrap_or_else(|| std::path::PathBuf::from("skills.json"))
}

/// Get the default path for chain state storage.
pub fn default_state_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".claude/chain-state"))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/chain-state"))
}

/// Resolve skill paths from a given path.
///
/// Smart path detection:
/// 1. If path is a .md file, use it directly
/// 2. If path is a skill directory (has SKILL.md), validate that skill
/// 3. If path is a parent directory, scan for skill subdirectories
pub fn resolve_skill_paths(path: &str) -> Vec<String> {
    let p = std::path::Path::new(path);

    if path.ends_with(".md") || p.is_file() {
        // Direct file path
        vec![path.to_string()]
    } else if p.is_dir() {
        let skill_md_direct = p.join("SKILL.md");
        if skill_md_direct.exists() {
            // This directory IS a skill - validate it directly
            vec![skill_md_direct.to_string_lossy().to_string()]
        } else {
            // This is a parent directory - scan for skill subdirectories
            std::fs::read_dir(path)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.path().join("SKILL.md"))
                .filter(|p| p.exists())
                .map(|p| p.to_string_lossy().to_string())
                .collect()
        }
    } else {
        vec![path.to_string()]
    }
}

/// Resolve build paths from a given path (returns skill directories, not SKILL.md files).
pub fn resolve_build_paths(path: &str) -> Vec<String> {
    let p = std::path::Path::new(path);

    if path.ends_with(".md") || p.is_file() {
        // Not standard for build, but handle gracefully
        vec![
            p.parent()
                .unwrap_or(std::path::Path::new("."))
                .to_string_lossy()
                .to_string(),
        ]
    } else if p.is_dir() {
        let skill_md_direct = p.join("SKILL.md");
        if skill_md_direct.exists() {
            vec![path.to_string()]
        } else {
            std::fs::read_dir(path)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.path().to_string_lossy().to_string())
                .filter(|d| std::path::Path::new(d).join("SKILL.md").exists())
                .collect()
        }
    } else {
        vec![path.to_string()]
    }
}
