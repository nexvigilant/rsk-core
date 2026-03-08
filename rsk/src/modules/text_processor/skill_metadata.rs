//! Skill metadata parsing (frontmatter and sections).
//!
//! This module provides types and functions for parsing SKILL.md metadata:
//! - YAML frontmatter extraction and parsing
//! - Skill section representation

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::modules::graph::Adjacency;

// ═══════════════════════════════════════════════════════════════════════════
// PRECOMPILED REGEX PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

/// Frontmatter extraction pattern
pub(crate) static RE_FRONTMATTER: LazyLock<Regex> = LazyLock::new(|| {
    // SAFETY: Pattern is a compile-time string literal verified to be a valid regex;
    // Regex::new on a valid literal pattern cannot fail at runtime.
    #[allow(clippy::unwrap_used)]
    Regex::new(r"(?s)---\s*(.*?)\s*---").unwrap()
});

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// A section within a SKILL.md file
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillSection {
    pub name: String,
    pub content: String,
}

/// YAML frontmatter metadata from SKILL.md
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SkillFrontmatter {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(rename = "compliance-level", default)]
    pub compliance_level: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(rename = "user-invocable", default)]
    pub user_invocable: bool,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(rename = "depends-on", default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub adjacencies: Vec<Adjacency>,
    /// Capture any other fields (tags, dependencies, etc.)
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

impl SkillFrontmatter {
    /// Flatten the 'extra' map into a single JSON Value for downstream consumption
    pub fn flatten_to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();

        // 1. Add 'extra' fields first
        for (k, v) in &self.extra {
            if let Ok(json_v) = serde_json::to_value(v) {
                obj.insert(k.clone(), json_v);
            }
        }

        // 2. Overwrite with protected top-level fields
        obj.insert(
            "name".to_string(),
            serde_json::Value::String(self.name.clone()),
        );
        if let Some(v) = &self.description {
            obj.insert(
                "description".to_string(),
                serde_json::Value::String(v.clone()),
            );
        }
        if let Some(v) = &self.version {
            obj.insert("version".to_string(), serde_json::Value::String(v.clone()));
        }
        if let Some(v) = &self.compliance_level {
            obj.insert(
                "compliance-level".to_string(),
                serde_json::Value::String(v.clone()),
            );
        }
        // SAFETY: Vec<String> and Vec<Adjacency> (which derives Serialize) cannot produce
        // a non-serializable value; serde_json::to_value on these types never fails.
        #[allow(clippy::unwrap_used)]
        obj.insert(
            "categories".to_string(),
            serde_json::to_value(&self.categories).unwrap(),
        );
        if let Some(v) = &self.author {
            obj.insert("author".to_string(), serde_json::Value::String(v.clone()));
        }
        obj.insert(
            "user-invocable".to_string(),
            serde_json::Value::Bool(self.user_invocable),
        );
        if let Some(v) = &self.context {
            obj.insert("context".to_string(), serde_json::Value::String(v.clone()));
        }
        #[allow(clippy::unwrap_used)]
        obj.insert(
            "depends-on".to_string(),
            serde_json::to_value(&self.depends_on).unwrap(),
        );
        #[allow(clippy::unwrap_used)]
        obj.insert(
            "triggers".to_string(),
            serde_json::to_value(&self.triggers).unwrap(),
        );
        #[allow(clippy::unwrap_used)]
        obj.insert(
            "keywords".to_string(),
            serde_json::to_value(&self.keywords).unwrap(),
        );
        #[allow(clippy::unwrap_used)]
        obj.insert(
            "adjacencies".to_string(),
            serde_json::to_value(&self.adjacencies).unwrap(),
        );

        serde_json::Value::Object(obj)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PARSING FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Parse YAML frontmatter from SKILL.md content
///
/// Uses proper YAML parsing via serde_yaml to correctly handle:
/// - Literal block scalars (|)
/// - Folded block scalars (>)
/// - Multiline strings
/// - All YAML 1.1 features
pub fn parse_frontmatter(content: &str) -> SkillFrontmatter {
    let mut frontmatter = SkillFrontmatter::default();

    // Extract frontmatter block between --- delimiters using precompiled regex
    let fm_content = match RE_FRONTMATTER.captures(content) {
        Some(cap) => cap[1].to_string(),
        None => return frontmatter,
    };

    // Parse using serde_yaml for proper YAML handling
    let yaml_value: serde_yaml::Value = match serde_yaml::from_str(&fm_content) {
        Ok(v) => v,
        Err(_) => return frontmatter,
    };

    // Convert directly to struct using serde
    match serde_yaml::from_value::<SkillFrontmatter>(yaml_value.clone()) {
        Ok(fm) => {
            frontmatter = fm;
        }
        Err(_) => {
            // Resilient fallback for partial parsing
            let get_string = |key: &str| -> Option<String> {
                yaml_value
                    .get(key)
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
            };

            frontmatter.name = get_string("name").unwrap_or_else(|| "unknown".to_string());
            frontmatter.description = get_string("description");
            frontmatter.version = get_string("version");
            frontmatter.compliance_level = get_string("compliance-level");
            frontmatter.author = get_string("author");
            frontmatter.context = get_string("context");
            // Basic extraction for critical boolean
            frontmatter.user_invocable = yaml_value
                .get("user-invocable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        }
    }

    frontmatter
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_complete() {
        let content = r#"---
name: test-skill
description: |
  Multi-line description
  with more text
version: 1.0.0
compliance-level: diamond
categories:
  - orchestration
  - testing
author: Claude
user-invocable: true
context: fork
depends-on:
  - skill-a
  - skill-b
triggers:
  - /test
  - test skill
keywords:
  - test
  - demo
---

# Content
"#;
        let fm = parse_frontmatter(content);

        assert_eq!(fm.name, "test-skill");
        assert!(fm.description.is_some());
        assert!(fm.description.unwrap().contains("Multi-line"));
        assert_eq!(fm.version, Some("1.0.0".to_string()));
        assert_eq!(fm.compliance_level, Some("diamond".to_string()));
        assert_eq!(fm.categories.len(), 2);
        assert_eq!(fm.author, Some("Claude".to_string()));
        assert!(fm.user_invocable);
        assert_eq!(fm.context, Some("fork".to_string()));
        assert_eq!(fm.depends_on.len(), 2);
        assert_eq!(fm.triggers.len(), 2);
        assert_eq!(fm.keywords.len(), 2);
    }

    #[test]
    fn test_parse_frontmatter_minimal() {
        let content = "---\nname: minimal\n---\n# Minimal";
        let fm = parse_frontmatter(content);

        assert_eq!(fm.name, "minimal");
        assert!(fm.description.is_none());
        assert!(!fm.user_invocable);
        assert!(fm.triggers.is_empty());
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let content = "# Just a title\n\nNo frontmatter here.";
        let fm = parse_frontmatter(content);

        assert_eq!(fm.name, "");
    }

    #[test]
    fn test_parse_frontmatter_literal_block_scalar() {
        // This test validates the fix for YAML literal block scalars (|)
        // which preserve newlines and formatting
        let content = r#"---
name: literal-block-test
description: |
  This is a multiline description
  that uses the literal block scalar syntax.

  It preserves newlines and spacing.
version: 1.0.0
compliance-level: Gold
---

# Content
"#;
        let fm = parse_frontmatter(content);

        assert_eq!(fm.name, "literal-block-test");
        assert!(fm.description.is_some());
        let desc = fm.description.unwrap();
        // Literal block scalar should preserve the multiline content
        assert!(desc.contains("multiline description"));
        assert!(desc.contains("literal block scalar syntax"));
        assert_eq!(fm.version, Some("1.0.0".to_string()));
        assert_eq!(fm.compliance_level, Some("Gold".to_string()));
    }

    #[test]
    fn test_parse_frontmatter_folded_block_scalar() {
        // Test folded block scalar (>) which folds newlines into spaces
        let content = r#"---
name: folded-block-test
description: >
  This is a folded description
  that uses the folded block scalar.
  Lines are joined with spaces.
version: 2.0.0
---

# Content
"#;
        let fm = parse_frontmatter(content);

        assert_eq!(fm.name, "folded-block-test");
        assert!(fm.description.is_some());
        let desc = fm.description.unwrap();
        assert!(desc.contains("folded description"));
        assert_eq!(fm.version, Some("2.0.0".to_string()));
    }

    #[test]
    fn test_flatten_to_json_collision() {
        use serde_yaml::Value as YamlValue;
        let mut fm = SkillFrontmatter::default();
        fm.name = "test-skill".to_string();
        fm.version = Some("1.0.0".to_string());

        // Add colliding key in extra
        fm.extra.insert(
            "version".to_string(),
            YamlValue::String("2.0.0".to_string()),
        );
        fm.extra.insert(
            "custom-tag".to_string(),
            YamlValue::String("val".to_string()),
        );

        let json = fm.flatten_to_json();
        let obj = json.as_object().unwrap();

        // Protected key should be preserved from top-level
        assert_eq!(obj.get("version").unwrap().as_str().unwrap(), "1.0.0");
        // Extra key should be present
        assert_eq!(obj.get("custom-tag").unwrap().as_str().unwrap(), "val");
    }
}
