//! Attenuation Theorem (T10.2) - Implementation
//!
//! This module provides the implementation of the Attenuation Theorem
//! from the Theory of Vigilance.
//!
//! ## Theorem Statement
//!
//! Under the Markov assumption (Axiom 5), if all propagation probabilities P_{i->i+1} < 1,
//! then the harm probability at level H is:
//!
//! P(H|delta_s1) = e^{-alpha(H-1)}
//!
//! where alpha = -log(geometric mean of propagation probabilities)

use serde::{Deserialize, Serialize};

/// Propagation probability (must be in (0, 1))
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PropagationProbability {
    value: f64,
}

impl PropagationProbability {
    /// Create a new propagation probability
    ///
    /// # Panics
    /// Panics if value is not in (0, 1)
    pub fn new(value: f64) -> Self {
        assert!(
            value > 0.0 && value < 1.0,
            "Probability must be in (0, 1), got {value}"
        );
        Self { value }
    }

    /// Get the probability value
    pub fn get(&self) -> f64 {
        self.value
    }
}

/// Result of attenuation analysis
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttenuationResult {
    /// The attenuation rate alpha
    pub alpha: f64,
    /// Harm probability using exact product formula
    pub harm_probability: f64,
    /// Harm probability using exponential approximation
    pub harm_probability_exponential: f64,
    /// Uniform bound (P_max^{H-1})
    pub uniform_bound: f64,
    /// Whether attenuation property holds
    pub attenuation_verified: bool,
}

/// Compute product of probabilities (prod P_i)
pub fn product(probs: &[f64]) -> f64 {
    probs.iter().product()
}

/// Compute harm probability using product formula (Axiom 5)
///
/// P(H|delta_s1) = prod_i P_{i->i+1}
pub fn harm_probability(probs: &[PropagationProbability]) -> f64 {
    probs.iter().map(|p| p.get()).product()
}

/// Compute attenuation rate alpha = -log(P_bar)
///
/// where P_bar is the geometric mean of propagation probabilities
pub fn attenuation_rate(probs: &[PropagationProbability]) -> f64 {
    if probs.is_empty() {
        return 0.0;
    }
    let log_sum: f64 = probs.iter().map(|p| p.get().ln()).sum();
    #[allow(clippy::as_conversions)] // usize→f64 for ratio
    let count = probs.len() as f64;
    -log_sum / count
}

/// Compute harm probability using exponential form (Theorem 10.2 Version D)
///
/// P(H) = e^{-alpha(H-1)}
pub fn harm_probability_exponential(alpha: f64, harm_level: usize) -> f64 {
    #[allow(clippy::as_conversions)] // usize→f64 for math
    let level = harm_level as f64;
    (-alpha * (level - 1.0)).exp()
}

/// Compute protective depth for target probability (Corollary)
///
/// Returns minimum H such that P(H) < target_probability
///
/// Formula: H >= 1 + log(1/epsilon)/alpha
pub fn protective_depth(target_probability: f64, attenuation_rate: f64) -> usize {
    assert!(
        target_probability > 0.0 && target_probability < 1.0,
        "Target probability must be in (0, 1)"
    );
    assert!(attenuation_rate > 0.0, "Attenuation rate must be positive");
    let min_depth = 1.0 + (-target_probability.ln()) / attenuation_rate;
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation, clippy::cast_sign_loss)] // f64→usize: ceil of positive value
    { min_depth.ceil() as usize }
}

/// Maximum probability in a slice
pub fn max_probability(probs: &[PropagationProbability]) -> f64 {
    probs
        .iter()
        .map(|p| p.get())
        .fold(0.0, |acc, x| if x > acc { x } else { acc })
}

/// Compute uniform bound (Theorem 10.2 Version A)
///
/// P(H) <= P_max^{H-1}
pub fn uniform_bound(probs: &[PropagationProbability]) -> f64 {
    let p_max = max_probability(probs);
    let h_minus_1 = probs.len();
    let exp = i32::try_from(h_minus_1).unwrap_or(i32::MAX);
    p_max.powi(exp)
}

/// Verify attenuation property: harm decreases with depth
pub fn verify_attenuation(probs: &[PropagationProbability]) -> bool {
    if probs.is_empty() {
        return true;
    }

    // Compute harm probabilities for increasing depths
    let mut last_hp = 1.0;
    for i in 1..=probs.len() {
        let hp = harm_probability(&probs[..i]);
        if hp >= last_hp {
            return false;
        }
        last_hp = hp;
    }
    true
}

/// Perform complete attenuation analysis
pub fn analyze_attenuation(probs: &[PropagationProbability]) -> AttenuationResult {
    let alpha = attenuation_rate(probs);
    let hp = harm_probability(probs);
    let hp_exp = harm_probability_exponential(alpha, probs.len() + 1);
    let bound = uniform_bound(probs);
    let verified = verify_attenuation(probs);

    AttenuationResult {
        alpha,
        harm_probability: hp,
        harm_probability_exponential: hp_exp,
        uniform_bound: bound,
        attenuation_verified: verified,
    }
}

/// Calculate protective depth recommendations for common target probabilities
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtectiveDepthRecommendations {
    /// Depth for 10% harm probability
    pub depth_10_percent: usize,
    /// Depth for 1% harm probability
    pub depth_1_percent: usize,
    /// Depth for 0.1% harm probability
    pub depth_01_percent: usize,
    /// The attenuation rate used
    pub alpha: f64,
}

/// Get protective depth recommendations for a given attenuation rate
pub fn protective_depth_recommendations(alpha: f64) -> ProtectiveDepthRecommendations {
    assert!(alpha > 0.0, "Attenuation rate must be positive");
    ProtectiveDepthRecommendations {
        depth_10_percent: protective_depth(0.10, alpha),
        depth_1_percent: protective_depth(0.01, alpha),
        depth_01_percent: protective_depth(0.001, alpha),
        alpha,
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propagation_probability_bounds() {
        let p = PropagationProbability::new(0.5);
        assert!(p.get() > 0.0 && p.get() < 1.0);
    }

    #[test]
    #[should_panic]
    fn test_propagation_probability_rejects_1() {
        let _ = PropagationProbability::new(1.0);
    }

    #[test]
    #[should_panic]
    fn test_propagation_probability_rejects_0() {
        let _ = PropagationProbability::new(0.0);
    }

    #[test]
    fn test_harm_probability_is_product() {
        let probs = vec![
            PropagationProbability::new(0.5),
            PropagationProbability::new(0.3),
            PropagationProbability::new(0.2),
        ];
        let hp = harm_probability(&probs);
        let expected = 0.5 * 0.3 * 0.2;
        assert!((hp - expected).abs() < 1e-10);
    }

    #[test]
    fn test_harm_probability_monotonic_decrease() {
        let p = PropagationProbability::new(0.7);

        let hp1 = harm_probability(&[p]);
        let hp2 = harm_probability(&[p, p]);
        let hp3 = harm_probability(&[p, p, p]);
        let hp4 = harm_probability(&[p, p, p, p]);

        assert!(hp1 > hp2, "H=2 should have lower probability than H=1");
        assert!(hp2 > hp3, "H=3 should have lower probability than H=2");
        assert!(hp3 > hp4, "H=4 should have lower probability than H=3");
    }

    #[test]
    fn test_attenuation_rate_positive() {
        let probs: Vec<_> = vec![
            PropagationProbability::new(0.5),
            PropagationProbability::new(0.4),
            PropagationProbability::new(0.3),
        ];
        let alpha = attenuation_rate(&probs);
        assert!(alpha > 0.0, "Attenuation rate should be positive");
    }

    #[test]
    fn test_exponential_decay_formula() {
        let p_val = 0.6;
        let levels = 5;

        let probs: Vec<_> = (0..levels)
            .map(|_| PropagationProbability::new(p_val))
            .collect();

        let actual = harm_probability(&probs);
        #[allow(clippy::as_conversions, clippy::cast_possible_truncation)] // test: small i32 literal
        let expected = p_val.powi(levels as i32);

        assert!(
            (actual - expected).abs() < 1e-10,
            "Harm probability should equal P^n for uniform P"
        );
    }

    #[test]
    fn test_protective_depth_achieves_target() {
        let alpha = 0.5;
        let target = 0.05;

        let depth = protective_depth(target, alpha);
        let actual_prob = harm_probability_exponential(alpha, depth);

        assert!(
            actual_prob < target,
            "Protective depth {} should achieve probability {} < {}",
            depth,
            actual_prob,
            target
        );
    }

    #[test]
    fn test_protective_depth_increases_with_stricter_target() {
        let alpha = 1.0;

        let depth_10pct = protective_depth(0.10, alpha);
        let depth_1pct = protective_depth(0.01, alpha);
        let depth_01pct = protective_depth(0.001, alpha);

        assert!(depth_1pct > depth_10pct);
        assert!(depth_01pct > depth_1pct);
    }

    #[test]
    fn test_attenuation_stronger_with_lower_probabilities() {
        let high_p: Vec<_> = (0..3).map(|_| PropagationProbability::new(0.8)).collect();
        let low_p: Vec<_> = (0..3).map(|_| PropagationProbability::new(0.2)).collect();

        let alpha_high = attenuation_rate(&high_p);
        let alpha_low = attenuation_rate(&low_p);

        assert!(
            alpha_low > alpha_high,
            "Lower probabilities should give stronger attenuation"
        );
    }

    #[test]
    fn test_uniform_bound() {
        let probs = vec![
            PropagationProbability::new(0.3),
            PropagationProbability::new(0.5),
            PropagationProbability::new(0.2),
        ];

        let hp = harm_probability(&probs);
        let bound = uniform_bound(&probs);

        assert!(
            hp <= bound,
            "Harm probability {} should be bounded by {}",
            hp,
            bound
        );
    }

    #[test]
    fn test_verify_attenuation() {
        let probs: Vec<_> = (0..5).map(|_| PropagationProbability::new(0.5)).collect();

        assert!(
            verify_attenuation(&probs),
            "Attenuation property should hold"
        );
    }

    #[test]
    fn test_analyze_attenuation() {
        let probs: Vec<_> = vec![
            PropagationProbability::new(0.5),
            PropagationProbability::new(0.4),
            PropagationProbability::new(0.3),
        ];

        let result = analyze_attenuation(&probs);

        assert!(result.alpha > 0.0);
        assert!(result.harm_probability < 1.0);
        assert!(result.attenuation_verified);
    }

    #[test]
    fn test_protective_depth_recommendations() {
        let recs = protective_depth_recommendations(1.0);

        assert!(recs.depth_1_percent > recs.depth_10_percent);
        assert!(recs.depth_01_percent > recs.depth_1_percent);
    }
}
