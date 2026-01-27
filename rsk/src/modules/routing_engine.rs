//! # Routing Engine
//!
//! Multi-strategy skill routing engine for intelligent skill discovery and navigation.
//!
//! ## Strategies
//! - **Adjacency**: Graph-based routing using weighted edges from skill adjacency index
//! - **Capability**: Pattern matching on triggers, handles, and capability declarations
//! - **Semantic**: Keyword-based similarity matching using Levenshtein distance
//! - **Hybrid**: Weighted combination of all strategies (default)
//!
//! ## Performance Targets
//! - Route calculation: < 1ms
//! - Graph loading: < 10ms for 500 skills
//! - Semantic index build: < 50ms
//!
//! ## Weights (Hybrid Mode)
//! - Adjacency: 0.5 (strongest signal - explicit graph edges)
//! - Capability: 0.3 (second - declared capabilities)
//! - Semantic: 0.2 (fallback - keyword similarity)
//!
//! ## Example
//! ```rust,ignore
//! use rsk::modules::routing_engine::{route, RoutingStrategy, RoutingRequest};
//!
//! let request = RoutingRequest {
//!     source: "proceed".to_string(),
//!     context: "I need to validate my skill".to_string(),
//!     strategy: RoutingStrategy::Hybrid,
//!     limit: 5,
//! };
//!
//! let result = route(request)?;
//! println!("Top recommendation: {}", result.recommendations[0].target);
//! ```

use crate::modules::graph::SkillGraph;
use crate::modules::levenshtein::levenshtein;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Routing strategy to use for skill discovery
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// Graph-based routing using adjacency weights
    Adjacency,
    /// Pattern matching on skill capabilities
    Capability,
    /// Keyword similarity using Levenshtein distance
    Semantic,
    /// Weighted combination of all strategies
    Hybrid,
}

impl Default for RoutingStrategy {
    fn default() -> Self {
        Self::Hybrid
    }
}

impl RoutingStrategy {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "adjacency" | "adj" | "graph" => Some(Self::Adjacency),
            "capability" | "cap" | "pattern" => Some(Self::Capability),
            "semantic" | "sem" | "keyword" => Some(Self::Semantic),
            "hybrid" | "all" | "combined" => Some(Self::Hybrid),
            _ => None,
        }
    }

    /// Get the weight for this strategy in hybrid mode
    pub fn hybrid_weight(&self) -> f32 {
        match self {
            Self::Adjacency => 0.5,
            Self::Capability => 0.3,
            Self::Semantic => 0.2,
            Self::Hybrid => 1.0, // Not used directly
        }
    }
}

/// A routing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRequest {
    /// Source skill (current context)
    pub source: String,
    /// Natural language context/query
    pub context: String,
    /// Strategy to use
    pub strategy: RoutingStrategy,
    /// Maximum number of results
    pub limit: usize,
}

impl Default for RoutingRequest {
    fn default() -> Self {
        Self {
            source: String::new(),
            context: String::new(),
            strategy: RoutingStrategy::default(),
            limit: 5,
        }
    }
}

/// A scored routing recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingScore {
    /// Target skill name
    pub target: String,
    /// Combined score (0.0 - 1.0)
    pub score: f32,
    /// Confidence in this recommendation (0.0 - 1.0)
    pub confidence: f32,
    /// Human-readable reasoning
    pub reasoning: String,
    /// Individual strategy scores (for debugging)
    pub strategy_scores: HashMap<String, f32>,
}

/// Result of a routing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingResult {
    /// Original source skill
    pub source: String,
    /// Query context
    pub context: String,
    /// Strategy used
    pub strategy: RoutingStrategy,
    /// Sorted recommendations (best first)
    pub recommendations: Vec<RoutingScore>,
    /// Total skills considered
    pub total_considered: usize,
    /// Time taken in milliseconds
    pub duration_ms: u64,
}

/// Skill capability entry for capability-based routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCapability {
    /// Skill name
    pub name: String,
    /// Trigger phrases
    pub triggers: Vec<String>,
    /// What this skill handles (capability descriptions)
    pub handles: Vec<String>,
    /// Keywords extracted from skill
    pub keywords: Vec<String>,
    /// Category (e.g., "algorithms", "code-analysis")
    pub category: String,
}

/// Routing engine state
#[derive(Debug, Clone, Default)]
pub struct RoutingEngine {
    /// Skill adjacency graph
    pub graph: Option<SkillGraph>,
    /// Skill capabilities index
    pub capabilities: HashMap<String, SkillCapability>,
    /// Semantic index (keyword -> skills)
    pub semantic_index: HashMap<String, Vec<String>>,
    /// All known skill names
    pub skill_names: Vec<String>,
}

/// Error types for routing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingError {
    /// Graph not loaded
    GraphNotLoaded,
    /// Source skill not found
    SourceNotFound(String),
    /// Invalid strategy
    InvalidStrategy(String),
    /// IO error
    IoError(String),
    /// Parse error
    ParseError(String),
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GraphNotLoaded => write!(f, "Skill graph not loaded"),
            Self::SourceNotFound(s) => write!(f, "Source skill not found: {}", s),
            Self::InvalidStrategy(s) => write!(f, "Invalid routing strategy: {}", s),
            Self::IoError(s) => write!(f, "IO error: {}", s),
            Self::ParseError(s) => write!(f, "Parse error: {}", s),
        }
    }
}

impl std::error::Error for RoutingError {}

// ═══════════════════════════════════════════════════════════════════════════
// ROUTING ENGINE IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════════════════

impl RoutingEngine {
    /// Create a new routing engine
    pub fn new() -> Self {
        Self::default()
    }

    /// Load skill graph from file
    pub fn load_graph(&mut self, path: &Path) -> Result<(), RoutingError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| RoutingError::IoError(e.to_string()))?;

        let graph: SkillGraph =
            serde_json::from_str(&content).map_err(|e| RoutingError::ParseError(e.to_string()))?;

        self.skill_names = graph.nodes.keys().cloned().collect();
        self.graph = Some(graph);
        Ok(())
    }

    /// Add a skill capability to the index
    pub fn add_capability(&mut self, capability: SkillCapability) {
        // Add to semantic index
        for keyword in &capability.keywords {
            self.semantic_index
                .entry(keyword.to_lowercase())
                .or_default()
                .push(capability.name.clone());
        }

        // Add to skill names if not present
        if !self.skill_names.contains(&capability.name) {
            self.skill_names.push(capability.name.clone());
        }

        self.capabilities
            .insert(capability.name.clone(), capability);
    }

    /// Build semantic index from all capabilities
    pub fn build_semantic_index(&mut self) {
        self.semantic_index.clear();

        for capability in self.capabilities.values() {
            for keyword in &capability.keywords {
                self.semantic_index
                    .entry(keyword.to_lowercase())
                    .or_default()
                    .push(capability.name.clone());
            }

            // Also index triggers
            for trigger in &capability.triggers {
                for word in trigger.split_whitespace() {
                    if word.len() > 2 {
                        self.semantic_index
                            .entry(word.to_lowercase())
                            .or_default()
                            .push(capability.name.clone());
                    }
                }
            }
        }
    }

    /// Route to find best matching skills
    pub fn route(&self, request: &RoutingRequest) -> Result<RoutingResult, RoutingError> {
        let start = std::time::Instant::now();

        let scores = match request.strategy {
            RoutingStrategy::Adjacency => self.route_adjacency(request)?,
            RoutingStrategy::Capability => self.route_capability(request)?,
            RoutingStrategy::Semantic => self.route_semantic(request)?,
            RoutingStrategy::Hybrid => self.route_hybrid(request)?,
        };

        // Sort by score descending and take top N
        let mut recommendations: Vec<RoutingScore> = scores.into_iter().collect();
        recommendations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        recommendations.truncate(request.limit);

        Ok(RoutingResult {
            source: request.source.clone(),
            context: request.context.clone(),
            strategy: request.strategy,
            recommendations,
            total_considered: self.skill_names.len(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Adjacency-based routing using graph edges
    fn route_adjacency(&self, request: &RoutingRequest) -> Result<Vec<RoutingScore>, RoutingError> {
        let graph = self.graph.as_ref().ok_or(RoutingError::GraphNotLoaded)?;

        let mut scores = Vec::new();

        // If source is specified and exists, use its adjacencies
        if !request.source.is_empty() {
            if let Some(node) = graph.nodes.get(&request.source) {
                for adj in &node.adjacencies {
                    scores.push(RoutingScore {
                        target: adj.target.clone(),
                        score: adj.weight,
                        confidence: 0.9, // High confidence for explicit edges
                        reasoning: format!(
                            "Adjacent to {} with weight {:.2}",
                            request.source, adj.weight
                        ),
                        strategy_scores: {
                            let mut m = HashMap::new();
                            m.insert("adjacency".to_string(), adj.weight);
                            m
                        },
                    });
                }
            }
        }

        Ok(scores)
    }

    /// Capability-based routing using pattern matching
    fn route_capability(
        &self,
        request: &RoutingRequest,
    ) -> Result<Vec<RoutingScore>, RoutingError> {
        let mut scores = Vec::new();
        let query_lower = request.context.to_lowercase();

        for capability in self.capabilities.values() {
            let mut score = 0.0f32;
            let mut matched = Vec::new();

            // Check triggers
            for trigger in &capability.triggers {
                if query_lower.contains(&trigger.to_lowercase()) {
                    score += 0.5;
                    matched.push(format!("trigger: {}", trigger));
                }
            }

            // Check handles
            for handle in &capability.handles {
                if query_lower.contains(&handle.to_lowercase()) {
                    score += 0.3;
                    matched.push(format!("handles: {}", handle));
                }
            }

            // Check keywords
            for keyword in &capability.keywords {
                if query_lower.contains(&keyword.to_lowercase()) {
                    score += 0.1;
                    matched.push(format!("keyword: {}", keyword));
                }
            }

            if score > 0.0 {
                scores.push(RoutingScore {
                    target: capability.name.clone(),
                    score: score.min(1.0),
                    confidence: (matched.len() as f32 * 0.2).min(0.9),
                    reasoning: matched.join(", "),
                    strategy_scores: {
                        let mut m = HashMap::new();
                        m.insert("capability".to_string(), score.min(1.0));
                        m
                    },
                });
            }
        }

        Ok(scores)
    }

    /// Semantic routing using keyword similarity
    fn route_semantic(&self, request: &RoutingRequest) -> Result<Vec<RoutingScore>, RoutingError> {
        let mut skill_scores: HashMap<String, (f32, Vec<String>)> = HashMap::new();

        // Extract words from query
        let query_words: Vec<&str> = request
            .context
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
            .collect();

        // Check semantic index for each query word
        for word in &query_words {
            let word_lower = word.to_lowercase();

            // Direct match
            if let Some(skills) = self.semantic_index.get(&word_lower) {
                for skill in skills {
                    let entry = skill_scores
                        .entry(skill.clone())
                        .or_insert((0.0, Vec::new()));
                    entry.0 += 0.3;
                    entry.1.push(format!("exact: {}", word));
                }
            }

            // Fuzzy match using Levenshtein
            for (indexed_word, skills) in &self.semantic_index {
                let distance = levenshtein(&word_lower, indexed_word).distance;
                if distance <= 2 && distance > 0 {
                    let similarity =
                        1.0 - (distance as f32 / word_lower.len().max(indexed_word.len()) as f32);
                    for skill in skills {
                        let entry = skill_scores
                            .entry(skill.clone())
                            .or_insert((0.0, Vec::new()));
                        entry.0 += similarity * 0.2;
                        entry.1.push(format!("fuzzy: {} ~ {}", word, indexed_word));
                    }
                }
            }
        }

        // Convert to RoutingScores
        let scores: Vec<RoutingScore> = skill_scores
            .into_iter()
            .map(|(skill, (score, matches))| RoutingScore {
                target: skill,
                score: score.min(1.0),
                confidence: (matches.len() as f32 * 0.15).min(0.8),
                reasoning: matches.into_iter().take(3).collect::<Vec<_>>().join(", "),
                strategy_scores: {
                    let mut m = HashMap::new();
                    m.insert("semantic".to_string(), score.min(1.0));
                    m
                },
            })
            .collect();

        Ok(scores)
    }

    /// Hybrid routing combining all strategies
    fn route_hybrid(&self, request: &RoutingRequest) -> Result<Vec<RoutingScore>, RoutingError> {
        let adj_scores = self.route_adjacency(request).unwrap_or_default();
        let cap_scores = self.route_capability(request).unwrap_or_default();
        let sem_scores = self.route_semantic(request).unwrap_or_default();

        // Merge scores by target skill
        let mut merged: HashMap<String, RoutingScore> = HashMap::new();

        for score in adj_scores {
            let entry = merged.entry(score.target.clone()).or_insert(RoutingScore {
                target: score.target.clone(),
                score: 0.0,
                confidence: 0.0,
                reasoning: String::new(),
                strategy_scores: HashMap::new(),
            });
            entry.score += score.score * RoutingStrategy::Adjacency.hybrid_weight();
            entry.confidence = entry.confidence.max(score.confidence);
            entry
                .strategy_scores
                .insert("adjacency".to_string(), score.score);
            if !score.reasoning.is_empty() {
                if !entry.reasoning.is_empty() {
                    entry.reasoning.push_str("; ");
                }
                entry.reasoning.push_str(&score.reasoning);
            }
        }

        for score in cap_scores {
            let entry = merged.entry(score.target.clone()).or_insert(RoutingScore {
                target: score.target.clone(),
                score: 0.0,
                confidence: 0.0,
                reasoning: String::new(),
                strategy_scores: HashMap::new(),
            });
            entry.score += score.score * RoutingStrategy::Capability.hybrid_weight();
            entry.confidence = entry.confidence.max(score.confidence);
            entry
                .strategy_scores
                .insert("capability".to_string(), score.score);
            if !score.reasoning.is_empty() {
                if !entry.reasoning.is_empty() {
                    entry.reasoning.push_str("; ");
                }
                entry.reasoning.push_str(&score.reasoning);
            }
        }

        for score in sem_scores {
            let entry = merged.entry(score.target.clone()).or_insert(RoutingScore {
                target: score.target.clone(),
                score: 0.0,
                confidence: 0.0,
                reasoning: String::new(),
                strategy_scores: HashMap::new(),
            });
            entry.score += score.score * RoutingStrategy::Semantic.hybrid_weight();
            entry.confidence = entry.confidence.max(score.confidence);
            entry
                .strategy_scores
                .insert("semantic".to_string(), score.score);
            if !score.reasoning.is_empty() {
                if !entry.reasoning.is_empty() {
                    entry.reasoning.push_str("; ");
                }
                entry.reasoning.push_str(&score.reasoning);
            }
        }

        Ok(merged.into_values().collect())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONVENIENCE FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Route using a pre-configured engine (convenience function)
pub fn route(
    engine: &RoutingEngine,
    source: &str,
    context: &str,
    strategy: RoutingStrategy,
) -> Result<RoutingResult, RoutingError> {
    let request = RoutingRequest {
        source: source.to_string(),
        context: context.to_string(),
        strategy,
        limit: 5,
    };
    engine.route(&request)
}

/// Quick fuzzy skill name lookup
pub fn fuzzy_skill_lookup(
    skill_names: &[String],
    query: &str,
    limit: usize,
) -> Vec<(String, usize)> {
    let mut results: Vec<(String, usize)> = skill_names
        .iter()
        .map(|name| {
            let distance = levenshtein(query, name).distance;
            (name.clone(), distance)
        })
        .filter(|(_, d)| *d <= 3)
        .collect();

    results.sort_by_key(|(_, d)| *d);
    results.truncate(limit);
    results
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_engine() -> RoutingEngine {
        let mut engine = RoutingEngine::new();

        // Add some test capabilities
        engine.add_capability(SkillCapability {
            name: "proceed".to_string(),
            triggers: vec![
                "proceed".to_string(),
                "execute".to_string(),
                "run tasks".to_string(),
            ],
            handles: vec!["task execution".to_string(), "DAG processing".to_string()],
            keywords: vec![
                "execution".to_string(),
                "dag".to_string(),
                "tasks".to_string(),
            ],
            category: "orchestration".to_string(),
        });

        engine.add_capability(SkillCapability {
            name: "skill-validator".to_string(),
            triggers: vec!["validate".to_string(), "check skill".to_string()],
            handles: vec![
                "skill validation".to_string(),
                "compliance checking".to_string(),
            ],
            keywords: vec![
                "validate".to_string(),
                "compliance".to_string(),
                "check".to_string(),
            ],
            category: "validation".to_string(),
        });

        engine.add_capability(SkillCapability {
            name: "topological-sort".to_string(),
            triggers: vec!["topsort".to_string(), "sort dependencies".to_string()],
            handles: vec![
                "graph sorting".to_string(),
                "dependency ordering".to_string(),
            ],
            keywords: vec![
                "graph".to_string(),
                "sort".to_string(),
                "dependencies".to_string(),
            ],
            category: "algorithms".to_string(),
        });

        engine.build_semantic_index();
        engine
    }

    // ───────────────────────────────────────────────────────────────────────
    // POSITIVE TESTS
    // ───────────────────────────────────────────────────────────────────────

    #[test]
    fn test_capability_routing_exact_trigger() {
        let engine = create_test_engine();
        let request = RoutingRequest {
            source: String::new(),
            context: "I want to validate my skill".to_string(),
            strategy: RoutingStrategy::Capability,
            limit: 5,
        };

        let result = engine.route(&request).unwrap();
        assert!(!result.recommendations.is_empty());
        assert_eq!(result.recommendations[0].target, "skill-validator");
    }

    #[test]
    fn test_semantic_routing() {
        let engine = create_test_engine();
        let request = RoutingRequest {
            source: String::new(),
            context: "help me with compliance checking".to_string(),
            strategy: RoutingStrategy::Semantic,
            limit: 5,
        };

        let result = engine.route(&request).unwrap();
        assert!(!result.recommendations.is_empty());
    }

    #[test]
    fn test_hybrid_routing() {
        let engine = create_test_engine();
        let request = RoutingRequest {
            source: String::new(),
            context: "execute my tasks in the dag".to_string(),
            strategy: RoutingStrategy::Hybrid,
            limit: 5,
        };

        let result = engine.route(&request).unwrap();
        assert!(!result.recommendations.is_empty());
        // Should find "proceed" which handles DAG execution
    }

    #[test]
    fn test_fuzzy_skill_lookup() {
        let skills = vec![
            "proceed".to_string(),
            "process".to_string(),
            "validate".to_string(),
        ];

        let results = fuzzy_skill_lookup(&skills, "procede", 3); // Typo
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "proceed"); // Should find despite typo
    }

    #[test]
    fn test_routing_strategy_weights() {
        assert_eq!(RoutingStrategy::Adjacency.hybrid_weight(), 0.5);
        assert_eq!(RoutingStrategy::Capability.hybrid_weight(), 0.3);
        assert_eq!(RoutingStrategy::Semantic.hybrid_weight(), 0.2);
    }

    // ───────────────────────────────────────────────────────────────────────
    // EDGE CASES
    // ───────────────────────────────────────────────────────────────────────

    #[test]
    fn test_empty_context() {
        let engine = create_test_engine();
        let request = RoutingRequest {
            source: String::new(),
            context: String::new(),
            strategy: RoutingStrategy::Capability,
            limit: 5,
        };

        let result = engine.route(&request).unwrap();
        assert!(result.recommendations.is_empty());
    }

    #[test]
    fn test_no_matches() {
        let engine = create_test_engine();
        let request = RoutingRequest {
            source: String::new(),
            context: "completely unrelated query about cooking".to_string(),
            strategy: RoutingStrategy::Capability,
            limit: 5,
        };

        let result = engine.route(&request).unwrap();
        // May or may not find matches - just shouldn't panic
        assert!(result.total_considered > 0);
    }

    #[test]
    fn test_strategy_from_str() {
        assert_eq!(
            RoutingStrategy::from_str("adjacency"),
            Some(RoutingStrategy::Adjacency)
        );
        assert_eq!(
            RoutingStrategy::from_str("HYBRID"),
            Some(RoutingStrategy::Hybrid)
        );
        assert_eq!(RoutingStrategy::from_str("invalid"), None);
    }
}
