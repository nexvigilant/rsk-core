//! Taxonomy Module - Compile-time static lookup tables using phf
//!
//! This module provides O(1) lookup performance for skill taxonomies,
//! compliance levels, and SMST components via perfect hash functions.
//!
//! ## Features
//!
//! - Zero runtime overhead (compile-time computed)
//! - Type-safe taxonomy lookups
//! - Compliance level validation
//! - SMST component registry
//!
//! ## Example
//!
//! ```rust
//! use rsk::{lookup_compliance_level, lookup_smst_component};
//!
//! // O(1) compliance level lookup
//! if let Some(level) = lookup_compliance_level("diamond") {
//!     assert_eq!(level.min_score, 85);
//! }
//!
//! // O(1) SMST component lookup
//! if let Some(component) = lookup_smst_component("INPUTS") {
//!     assert!(component.required);
//! }
//! ```

use phf::phf_map;
use serde::{Deserialize, Serialize, Serializer};

/// Helper to serialize static string slices as JSON arrays
fn serialize_static_slice<S>(slice: &&'static [&'static str], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_seq(*slice)
}

// ============================================================================
// Compliance Levels
// ============================================================================

/// Compliance level metadata
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ComplianceLevel {
    /// Level name (Bronze, Silver, Gold, Platinum, Diamond)
    pub name: &'static str,
    /// Badge emoji
    pub badge: &'static str,
    /// Numeric tier (1-5)
    pub tier: u8,
    /// Minimum SMST score required
    pub min_score: u8,
    /// Required directories
    #[serde(serialize_with = "serialize_static_slice")]
    pub required_dirs: &'static [&'static str],
    /// Required files
    #[serde(serialize_with = "serialize_static_slice")]
    pub required_files: &'static [&'static str],
    /// Description
    pub description: &'static str,
}

/// Compile-time compliance level lookup table
pub static COMPLIANCE_LEVELS: phf::Map<&'static str, ComplianceLevel> = phf_map! {
    "bronze" => ComplianceLevel {
        name: "Bronze",
        badge: "🥉",
        tier: 1,
        min_score: 0,
        required_dirs: &[],
        required_files: &["SKILL.md"],
        description: "SKILL.md with valid YAML frontmatter",
    },
    "silver" => ComplianceLevel {
        name: "Silver",
        badge: "🥈",
        tier: 2,
        min_score: 25,
        required_dirs: &["scripts"],
        required_files: &["SKILL.md"],
        description: "Bronze + scripts/ directory",
    },
    "gold" => ComplianceLevel {
        name: "Gold",
        badge: "🥇",
        tier: 3,
        min_score: 50,
        required_dirs: &["scripts", "references", "templates"],
        required_files: &["SKILL.md"],
        description: "Silver + references/ + templates/",
    },
    "platinum" => ComplianceLevel {
        name: "Platinum",
        badge: "💎",
        tier: 4,
        min_score: 70,
        required_dirs: &["scripts", "references", "templates"],
        required_files: &["SKILL.md"],
        description: "Gold + functional tests pass",
    },
    "diamond" => ComplianceLevel {
        name: "Diamond",
        badge: "💠",
        tier: 5,
        min_score: 85,
        required_dirs: &["scripts", "references", "templates"],
        required_files: &["SKILL.md"],
        description: "Platinum + SMST score >= 85%",
    },
};

/// Lookup compliance level by name (case-insensitive)
pub fn lookup_compliance_level(name: &str) -> Option<&'static ComplianceLevel> {
    COMPLIANCE_LEVELS.get(&name.to_lowercase())
}

/// Get all compliance levels ordered by tier
pub fn all_compliance_levels() -> Vec<&'static ComplianceLevel> {
    let mut levels: Vec<_> = COMPLIANCE_LEVELS.values().collect();
    levels.sort_by_key(|l| l.tier);
    levels
}

// ============================================================================
// SMST Components
// ============================================================================

/// SMST component metadata
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SmstComponent {
    /// Component name (INPUTS, OUTPUTS, etc.)
    pub name: &'static str,
    /// Whether this component is required for Diamond v2
    pub required: bool,
    /// Weight in SMST scoring (0.0 - 1.0)
    pub weight: f64,
    /// Description
    pub description: &'static str,
    /// Expected content format
    pub format: &'static str,
}

/// Compile-time SMST component lookup table
pub static SMST_COMPONENTS: phf::Map<&'static str, SmstComponent> = phf_map! {
    "INPUTS" => SmstComponent {
        name: "INPUTS",
        required: true,
        weight: 0.125,
        description: "Input parameters and their types",
        format: "name: type — description",
    },
    "OUTPUTS" => SmstComponent {
        name: "OUTPUTS",
        required: true,
        weight: 0.125,
        description: "Output values and their types",
        format: "name: type — description",
    },
    "STATE" => SmstComponent {
        name: "STATE",
        required: true,
        weight: 0.125,
        description: "Internal state variables",
        format: "name: type — description",
    },
    "OPERATOR_MODE" => SmstComponent {
        name: "OPERATOR_MODE",
        required: true,
        weight: 0.125,
        description: "Behavioral modes (STANDARD, CAUTIOUS, AGGRESSIVE)",
        format: "MODE: description",
    },
    "PERFORMANCE" => SmstComponent {
        name: "PERFORMANCE",
        required: true,
        weight: 0.125,
        description: "Computational complexity and resource bounds",
        format: "TIME: O(n), SPACE: O(n), MEMORY: limit",
    },
    "INVARIANTS" => SmstComponent {
        name: "INVARIANTS",
        required: true,
        weight: 0.125,
        description: "Conditions that must always hold",
        format: "- description",
    },
    "FAILURE_MODES" => SmstComponent {
        name: "FAILURE_MODES",
        required: true,
        weight: 0.125,
        description: "Known failure conditions and responses",
        format: "CONDITION → RESPONSE [SEVERITY]",
    },
    "TELEMETRY" => SmstComponent {
        name: "TELEMETRY",
        required: true,
        weight: 0.125,
        description: "Metrics and observability points",
        format: "metric_name: type — description",
    },
};

/// Lookup SMST component by name (case-insensitive, handles underscores/spaces)
pub fn lookup_smst_component(name: &str) -> Option<&'static SmstComponent> {
    let normalized = name.to_uppercase().replace([' ', '-'], "_");
    SMST_COMPONENTS.get(normalized.as_str())
}

/// Get all required SMST components
pub fn required_smst_components() -> Vec<&'static SmstComponent> {
    SMST_COMPONENTS
        .values()
        .filter(|c| c.required)
        .collect()
}

/// Get all SMST components
pub fn all_smst_components() -> Vec<&'static SmstComponent> {
    SMST_COMPONENTS.values().collect()
}

// ============================================================================
// Skill Categories
// ============================================================================

/// Skill category metadata
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SkillCategory {
    /// Category name
    pub name: &'static str,
    /// Short description
    pub description: &'static str,
    /// Example skills in this category
    #[serde(serialize_with = "serialize_static_slice")]
    pub examples: &'static [&'static str],
    /// Whether this category has compute-intensive operations
    pub compute_intensive: bool,
}

/// Compile-time skill category lookup table
pub static SKILL_CATEGORIES: phf::Map<&'static str, SkillCategory> = phf_map! {
    "algorithms" => SkillCategory {
        name: "algorithms",
        description: "Computational algorithms and data structures",
        examples: &["binary-search", "topological-sort", "dijkstra", "kmp-search"],
        compute_intensive: true,
    },
    "validation" => SkillCategory {
        name: "validation",
        description: "Input validation and schema checking",
        examples: &["smst-validator", "construct-validator", "schema-sync"],
        compute_intensive: false,
    },
    "text-processing" => SkillCategory {
        name: "text-processing",
        description: "Text manipulation and parsing",
        examples: &["levenshtein", "text-compression", "lexicon-builder"],
        compute_intensive: true,
    },
    "graph" => SkillCategory {
        name: "graph",
        description: "Graph algorithms and DAG operations",
        examples: &["cycle-detection", "critical-path", "level-parallelization"],
        compute_intensive: true,
    },
    "security" => SkillCategory {
        name: "security",
        description: "Security scanning and vulnerability detection",
        examples: &["security-scan", "trivy", "schemathesis"],
        compute_intensive: false,
    },
    "code-generation" => SkillCategory {
        name: "code-generation",
        description: "Code scaffolding and generation",
        examples: &["skill-new", "sop-generator", "cli-scaffolder"],
        compute_intensive: false,
    },
    "orchestration" => SkillCategory {
        name: "orchestration",
        description: "Workflow and pipeline orchestration",
        examples: &["chain-skills", "proceed", "ralphy-engine"],
        compute_intensive: false,
    },
    "analysis" => SkillCategory {
        name: "analysis",
        description: "Code and system analysis",
        examples: &["code-analysis", "anti-pattern-detector", "boundary-detector"],
        compute_intensive: false,
    },
    "planning" => SkillCategory {
        name: "planning",
        description: "Project and strategy planning",
        examples: &["project-planning", "strategy-engine", "epic-optimizer"],
        compute_intensive: false,
    },
    "documentation" => SkillCategory {
        name: "documentation",
        description: "Documentation generation and management",
        examples: &["dev-docs", "doc-lookup", "api-documenter"],
        compute_intensive: false,
    },
};

/// Lookup skill category by name
pub fn lookup_skill_category(name: &str) -> Option<&'static SkillCategory> {
    SKILL_CATEGORIES.get(&name.to_lowercase().replace('_', "-"))
}

/// Get all skill categories
pub fn all_skill_categories() -> Vec<&'static SkillCategory> {
    SKILL_CATEGORIES.values().collect()
}

/// Get compute-intensive categories (candidates for Rust delegation)
pub fn compute_intensive_categories() -> Vec<&'static SkillCategory> {
    SKILL_CATEGORIES
        .values()
        .filter(|c| c.compute_intensive)
        .collect()
}

// ============================================================================
// Decision Tree Node Types
// ============================================================================

/// Decision tree node type metadata
#[derive(Debug, Clone, Copy, Serialize)]
pub struct NodeType {
    /// Node type name
    pub name: &'static str,
    /// Whether this is a terminal (leaf) node
    pub terminal: bool,
    /// Description
    pub description: &'static str,
}

/// Compile-time decision tree node type lookup
pub static NODE_TYPES: phf::Map<&'static str, NodeType> = phf_map! {
    "root" => NodeType {
        name: "root",
        terminal: false,
        description: "Entry point of decision tree",
    },
    "decision" => NodeType {
        name: "decision",
        terminal: false,
        description: "Branch node with condition",
    },
    "action" => NodeType {
        name: "action",
        terminal: true,
        description: "Terminal node with action to take",
    },
    "delegate" => NodeType {
        name: "delegate",
        terminal: true,
        description: "Terminal node delegating to another skill",
    },
    "error" => NodeType {
        name: "error",
        terminal: true,
        description: "Terminal error/failure node",
    },
};

/// Lookup decision tree node type
pub fn lookup_node_type(name: &str) -> Option<&'static NodeType> {
    NODE_TYPES.get(&name.to_lowercase())
}

// ============================================================================
// Taxonomy Query Results
// ============================================================================

/// Result of a taxonomy query
#[derive(Debug, Serialize, Deserialize)]
pub struct TaxonomyQueryResult {
    /// Query type (compliance, smst, category, node)
    pub query_type: String,
    /// Query key
    pub key: String,
    /// Whether the lookup succeeded
    pub found: bool,
    /// Result data (JSON-serializable)
    pub data: Option<serde_json::Value>,
}

/// Query the taxonomy by type and key
pub fn query_taxonomy(query_type: &str, key: &str) -> TaxonomyQueryResult {
    match query_type.to_lowercase().as_str() {
        "compliance" | "level" => {
            if let Some(level) = lookup_compliance_level(key) {
                TaxonomyQueryResult {
                    query_type: "compliance".to_string(),
                    key: key.to_string(),
                    found: true,
                    data: Some(serde_json::to_value(level).unwrap()),
                }
            } else {
                TaxonomyQueryResult {
                    query_type: "compliance".to_string(),
                    key: key.to_string(),
                    found: false,
                    data: None,
                }
            }
        }
        "smst" | "component" => {
            if let Some(component) = lookup_smst_component(key) {
                TaxonomyQueryResult {
                    query_type: "smst".to_string(),
                    key: key.to_string(),
                    found: true,
                    data: Some(serde_json::to_value(component).unwrap()),
                }
            } else {
                TaxonomyQueryResult {
                    query_type: "smst".to_string(),
                    key: key.to_string(),
                    found: false,
                    data: None,
                }
            }
        }
        "category" => {
            if let Some(category) = lookup_skill_category(key) {
                TaxonomyQueryResult {
                    query_type: "category".to_string(),
                    key: key.to_string(),
                    found: true,
                    data: Some(serde_json::to_value(category).unwrap()),
                }
            } else {
                TaxonomyQueryResult {
                    query_type: "category".to_string(),
                    key: key.to_string(),
                    found: false,
                    data: None,
                }
            }
        }
        "node" | "node_type" => {
            if let Some(node) = lookup_node_type(key) {
                TaxonomyQueryResult {
                    query_type: "node_type".to_string(),
                    key: key.to_string(),
                    found: true,
                    data: Some(serde_json::to_value(node).unwrap()),
                }
            } else {
                TaxonomyQueryResult {
                    query_type: "node_type".to_string(),
                    key: key.to_string(),
                    found: false,
                    data: None,
                }
            }
        }
        _ => TaxonomyQueryResult {
            query_type: query_type.to_string(),
            key: key.to_string(),
            found: false,
            data: None,
        },
    }
}

/// List all entries in a taxonomy category
#[derive(Debug, Serialize, Deserialize)]
pub struct TaxonomyListResult {
    /// Taxonomy type
    pub taxonomy_type: String,
    /// Number of entries
    pub count: usize,
    /// All entries
    pub entries: Vec<serde_json::Value>,
}

/// List all entries in a taxonomy
pub fn list_taxonomy(taxonomy_type: &str) -> TaxonomyListResult {
    match taxonomy_type.to_lowercase().as_str() {
        "compliance" | "levels" => {
            let entries: Vec<_> = all_compliance_levels()
                .iter()
                .map(|l| serde_json::to_value(l).unwrap())
                .collect();
            TaxonomyListResult {
                taxonomy_type: "compliance".to_string(),
                count: entries.len(),
                entries,
            }
        }
        "smst" | "components" => {
            let entries: Vec<_> = all_smst_components()
                .iter()
                .map(|c| serde_json::to_value(c).unwrap())
                .collect();
            TaxonomyListResult {
                taxonomy_type: "smst".to_string(),
                count: entries.len(),
                entries,
            }
        }
        "category" | "categories" => {
            let entries: Vec<_> = all_skill_categories()
                .iter()
                .map(|c| serde_json::to_value(c).unwrap())
                .collect();
            TaxonomyListResult {
                taxonomy_type: "category".to_string(),
                count: entries.len(),
                entries,
            }
        }
        "node" | "node_types" => {
            let entries: Vec<_> = NODE_TYPES
                .values()
                .map(|n| serde_json::to_value(n).unwrap())
                .collect();
            TaxonomyListResult {
                taxonomy_type: "node_type".to_string(),
                count: entries.len(),
                entries,
            }
        }
        _ => TaxonomyListResult {
            taxonomy_type: taxonomy_type.to_string(),
            count: 0,
            entries: vec![],
        },
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // === POSITIVE TESTS ===

    #[test]
    fn test_lookup_compliance_level_diamond() {
        let level = lookup_compliance_level("diamond").unwrap();
        assert_eq!(level.name, "Diamond");
        assert_eq!(level.tier, 5);
        assert_eq!(level.min_score, 85);
    }

    #[test]
    fn test_lookup_compliance_level_case_insensitive() {
        let level1 = lookup_compliance_level("DIAMOND").unwrap();
        let level2 = lookup_compliance_level("Diamond").unwrap();
        let level3 = lookup_compliance_level("diamond").unwrap();
        assert_eq!(level1.tier, level2.tier);
        assert_eq!(level2.tier, level3.tier);
    }

    #[test]
    fn test_all_compliance_levels_ordered() {
        let levels = all_compliance_levels();
        assert_eq!(levels.len(), 5);
        assert_eq!(levels[0].name, "Bronze");
        assert_eq!(levels[4].name, "Diamond");
    }

    #[test]
    fn test_lookup_smst_component() {
        let component = lookup_smst_component("INPUTS").unwrap();
        assert!(component.required);
        assert_eq!(component.weight, 0.125);
    }

    #[test]
    fn test_lookup_smst_component_normalized() {
        let c1 = lookup_smst_component("operator_mode").unwrap();
        let c2 = lookup_smst_component("OPERATOR_MODE").unwrap();
        let c3 = lookup_smst_component("operator-mode").unwrap();
        assert_eq!(c1.name, c2.name);
        assert_eq!(c2.name, c3.name);
    }

    #[test]
    fn test_required_smst_components() {
        let required = required_smst_components();
        assert_eq!(required.len(), 8); // All 8 SMST components are required
    }

    #[test]
    fn test_lookup_skill_category() {
        let category = lookup_skill_category("algorithms").unwrap();
        assert!(category.compute_intensive);
        assert!(!category.examples.is_empty());
    }

    #[test]
    fn test_compute_intensive_categories() {
        let compute = compute_intensive_categories();
        assert!(compute.len() >= 3); // algorithms, text-processing, graph
        assert!(compute.iter().all(|c| c.compute_intensive));
    }

    #[test]
    fn test_lookup_node_type() {
        let root = lookup_node_type("root").unwrap();
        assert!(!root.terminal);

        let action = lookup_node_type("action").unwrap();
        assert!(action.terminal);
    }

    #[test]
    fn test_query_taxonomy_compliance() {
        let result = query_taxonomy("compliance", "gold");
        assert!(result.found);
        assert_eq!(result.query_type, "compliance");
    }

    #[test]
    fn test_query_taxonomy_smst() {
        let result = query_taxonomy("smst", "TELEMETRY");
        assert!(result.found);
        assert!(result.data.is_some());
    }

    #[test]
    fn test_list_taxonomy_compliance() {
        let result = list_taxonomy("compliance");
        assert_eq!(result.count, 5);
        assert_eq!(result.entries.len(), 5);
    }

    #[test]
    fn test_list_taxonomy_smst() {
        let result = list_taxonomy("smst");
        assert_eq!(result.count, 8);
    }

    // === NEGATIVE TESTS ===

    #[test]
    fn test_lookup_nonexistent_compliance_level() {
        assert!(lookup_compliance_level("mythril").is_none());
    }

    #[test]
    fn test_lookup_nonexistent_smst_component() {
        assert!(lookup_smst_component("NONEXISTENT").is_none());
    }

    #[test]
    fn test_lookup_nonexistent_category() {
        assert!(lookup_skill_category("nonexistent").is_none());
    }

    #[test]
    fn test_query_taxonomy_not_found() {
        let result = query_taxonomy("compliance", "nonexistent");
        assert!(!result.found);
        assert!(result.data.is_none());
    }

    #[test]
    fn test_query_taxonomy_invalid_type() {
        let result = query_taxonomy("invalid_type", "key");
        assert!(!result.found);
    }

    #[test]
    fn test_list_taxonomy_invalid_type() {
        let result = list_taxonomy("invalid");
        assert_eq!(result.count, 0);
        assert!(result.entries.is_empty());
    }

    // === EDGE CASE TESTS ===

    #[test]
    fn test_lookup_empty_string() {
        assert!(lookup_compliance_level("").is_none());
        assert!(lookup_smst_component("").is_none());
        assert!(lookup_skill_category("").is_none());
    }

    #[test]
    fn test_smst_weights_sum_to_one() {
        let total_weight: f64 = all_smst_components()
            .iter()
            .map(|c| c.weight)
            .sum();
        assert!((total_weight - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_compliance_tiers_unique() {
        let levels = all_compliance_levels();
        let tiers: Vec<u8> = levels.iter().map(|l| l.tier).collect();
        let unique_tiers: std::collections::HashSet<u8> = tiers.iter().cloned().collect();
        assert_eq!(tiers.len(), unique_tiers.len());
    }

    #[test]
    fn test_compliance_scores_increasing() {
        let levels = all_compliance_levels();
        for i in 1..levels.len() {
            assert!(levels[i].min_score >= levels[i - 1].min_score);
        }
    }
}
