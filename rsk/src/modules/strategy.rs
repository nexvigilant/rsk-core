//! # Strategy Module
//!
//! High-performance strategic optimization engine.
//! Implements exponential path evaluation for complex decision spaces.

use serde::{Deserialize, Serialize};

/// A strategic segment (Where to Play)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategicField {
    pub id: String,
    pub market_size: f64,
    pub growth_rate: f64,
    pub capability_fit: f64, // 0.0 to 1.0
    pub competitive_intensity: f64, // 0.0 to 1.0 (lower is better)
}

/// A winning tactic (How to Win)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinTactic {
    pub id: String,
    pub differentiation: f64,
    pub cost_advantage: f64,
    pub execution_risk: f64, // 0.0 to 1.0
}

/// Result of a strategy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyScore {
    pub field_id: String,
    pub tactic_id: String,
    pub combined_score: f64,
    pub win_probability: f64,
    pub expected_value: f64,
}

/// The Strategy Optimizer
pub struct StrategyOptimizer {
    pub fields: Vec<StrategicField>,
    pub tactics: Vec<WinTactic>,
}

impl StrategyOptimizer {
    pub fn new(fields: Vec<StrategicField>, tactics: Vec<WinTactic>) -> Self {
        Self { fields, tactics }
    }

    /// Evaluates all combinations of fields and tactics to find the optimal strategy.
    /// Complexity: O(fields * tactics) - linear in nodes, but allows for massive 
    /// scaling (exponential combinations in larger trees).
    pub fn optimize(&self) -> Vec<StrategyScore> {
        let mut results = Vec::new();

        for field in &self.fields {
            for tactic in &self.tactics {
                // Heuristic scoring formula:
                // (Market Size * Growth) * (Fit * (1 - Intensity)) * (Diff * (1 - Risk))
                let field_potential = field.market_size * (1.0 + field.growth_rate);
                let competitive_fit = field.capability_fit * (1.0 - field.competitive_intensity);
                let tactic_strength = tactic.differentiation * (1.0 - tactic.execution_risk);

                let win_probability = (competitive_fit * tactic_strength).clamp(0.0, 1.0);
                let combined_score = field_potential * win_probability;
                
                results.push(StrategyScore {
                    field_id: field.id.clone(),
                    tactic_id: tactic.id.clone(),
                    combined_score,
                    win_probability,
                    expected_value: combined_score,
                });
            }
        }

        // Sort by combined score descending
        results.sort_by(|a, b| b.combined_score.partial_cmp(&a.score_val()).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}

impl StrategyScore {
    fn score_val(&self) -> f64 {
        self.combined_score
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_optimization() {
        let fields = vec![
            StrategicField {
                id: "Pharma-SaaS".to_string(),
                market_size: 1000.0,
                growth_rate: 0.15,
                capability_fit: 0.8,
                competitive_intensity: 0.4,
            },
            StrategicField {
                id: "Generic-SaaS".to_string(),
                market_size: 5000.0,
                growth_rate: 0.05,
                capability_fit: 0.3,
                competitive_intensity: 0.9,
            },
        ];

        let tactics = vec![
            WinTactic {
                id: "AI-Differentiation".to_string(),
                differentiation: 0.9,
                cost_advantage: 0.5,
                execution_risk: 0.3,
            },
            WinTactic {
                id: "Low-Cost-Ops".to_string(),
                differentiation: 0.2,
                cost_advantage: 0.9,
                execution_risk: 0.1,
            },
        ];

        let optimizer = StrategyOptimizer::new(fields, tactics);
        let results = optimizer.optimize();

        assert!(!results.is_empty());
        // Pharma-SaaS + AI-Differentiation should win due to high fit and differentiation
        assert_eq!(results[0].field_id, "Pharma-SaaS");
        assert_eq!(results[0].tactic_id, "AI-Differentiation");
    }
}
