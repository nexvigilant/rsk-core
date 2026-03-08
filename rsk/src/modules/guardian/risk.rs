//! Risk Assessment for Guardian-AV
//!
//! Implements risk scoring, therapeutic window calculation, and
//! risk minimization recommendations.

use serde::{Deserialize, Serialize};

use super::iair::{CheckabilityLevel, ExpertiseLevel, OutputTreatment, StakesLevel};

// ============================================================================
// CONTEXT RISK SCORING
// ============================================================================

/// Context risk parameters for therapeutic window calculation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextRiskParams {
    pub stakes: StakesLevel,
    pub expertise: ExpertiseLevel,
    pub checkability: CheckabilityLevel,
    pub output_treatment: OutputTreatment,
}

impl ContextRiskParams {
    /// Calculate context risk score: stakes × (1 - expertise) × (1 - checkability)
    pub fn risk_score(&self) -> f64 {
        self.stakes.as_factor() * self.expertise.as_factor() * self.checkability.as_factor()
    }

    /// Check if within therapeutic window
    pub fn is_therapeutic(&self) -> bool {
        // Therapeutic window conditions:
        // 1. Expertise is high or moderate
        // 2. Checkability is high or moderate
        // 3. Output is reviewed or draft (not direct use or published)
        let expertise_ok = matches!(
            self.expertise,
            ExpertiseLevel::High | ExpertiseLevel::Moderate
        );
        let checkability_ok = matches!(
            self.checkability,
            CheckabilityLevel::High | CheckabilityLevel::Moderate
        );
        let output_ok = matches!(
            self.output_treatment,
            OutputTreatment::Draft | OutputTreatment::Reviewed
        );

        expertise_ok && checkability_ok && output_ok
    }
}

/// Risk score result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskScoreResult {
    pub score: f64,
    pub level: RiskLevel,
    pub therapeutic_window: bool,
    pub recommendations: Vec<String>,
}

/// Risk level classification
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Moderate,
    High,
    Critical,
}

impl RiskLevel {
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s < 0.2 => RiskLevel::Low,
            s if s < 0.4 => RiskLevel::Moderate,
            s if s < 0.7 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }
}

/// Calculate risk score with recommendations
pub fn calculate_risk(params: &ContextRiskParams) -> RiskScoreResult {
    let score = params.risk_score();
    let level = RiskLevel::from_score(score);
    let therapeutic_window = params.is_therapeutic();

    let mut recommendations = Vec::new();

    // Generate recommendations based on risk factors
    if !therapeutic_window {
        recommendations
            .push("Outside therapeutic window - consider additional safeguards".to_string());
    }

    match params.expertise {
        ExpertiseLevel::Low => {
            recommendations
                .push("User has low domain expertise - recommend expert review".to_string());
        }
        ExpertiseLevel::Unknown => {
            recommendations
                .push("User expertise unknown - assume low and recommend verification".to_string());
        }
        _ => {}
    }

    match params.checkability {
        CheckabilityLevel::Low => {
            recommendations
                .push("Output is difficult to verify - recommend multiple sources".to_string());
        }
        CheckabilityLevel::Unfalsifiable => {
            recommendations.push(
                "Claims cannot be verified - strongly recommend expert consultation".to_string(),
            );
        }
        _ => {}
    }

    match params.output_treatment {
        OutputTreatment::DirectUse => {
            recommendations.push("Output used directly - recommend review before use".to_string());
        }
        OutputTreatment::Published => {
            recommendations
                .push("Output will be published - recommend thorough fact-checking".to_string());
        }
        _ => {}
    }

    match params.stakes {
        StakesLevel::High => {
            recommendations
                .push("High stakes decision - recommend additional validation".to_string());
        }
        StakesLevel::Critical => {
            recommendations
                .push("Critical stakes - strongly recommend human expert involvement".to_string());
        }
        _ => {}
    }

    RiskScoreResult {
        score,
        level,
        therapeutic_window,
        recommendations,
    }
}

// ============================================================================
// RISK MINIMIZATION LEVELS (ToV §59 / §41)
// ============================================================================

/// Risk minimization level (ordered by severity)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskMinimizationLevel {
    /// Level 1: Enhanced information/warnings
    InformationEnhancement,
    /// Level 2: Usage guidance
    UsageGuidance,
    /// Level 3: Controlled access
    ControlledAccess,
    /// Level 4: Additional monitoring
    AdditionalMonitoring,
    /// Level 5: Usage restriction
    UsageRestriction,
    /// Level 6: Conditional use
    ConditionalUse,
    /// Level 7: Capability withdrawal
    Withdrawal,
}

impl RiskMinimizationLevel {
    /// Get recommendation description
    pub fn description(&self) -> &'static str {
        match self {
            Self::InformationEnhancement => "Add explicit warnings and uncertainty indicators",
            Self::UsageGuidance => "Provide specific guidance on appropriate use cases",
            Self::ControlledAccess => "Restrict access to authorized/trained users",
            Self::AdditionalMonitoring => "Implement enhanced monitoring and logging",
            Self::UsageRestriction => "Limit capability to specific contexts",
            Self::ConditionalUse => "Require approval or verification before use",
            Self::Withdrawal => "Disable capability pending review",
        }
    }

    /// Get ToV signal effect
    pub fn signal_effect(&self) -> RiskMinimizationEffect {
        match self {
            Self::InformationEnhancement => RiskMinimizationEffect::IncreaseRecognition,
            Self::UsageGuidance => RiskMinimizationEffect::IncreaseRecognition,
            Self::ControlledAccess => RiskMinimizationEffect::ReducePerturbation,
            Self::AdditionalMonitoring => RiskMinimizationEffect::IncreaseRecognition,
            Self::UsageRestriction => RiskMinimizationEffect::ReducePerturbation,
            Self::ConditionalUse => RiskMinimizationEffect::ReducePerturbation,
            Self::Withdrawal => RiskMinimizationEffect::ZeroPerturbation,
        }
    }
}

/// Effect of risk minimization on ToV parameters
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskMinimizationEffect {
    /// Increases R (recognition presence) in S = U × R × T
    IncreaseRecognition,
    /// Reduces u (perturbation) in harm probability
    ReducePerturbation,
    /// Eliminates perturbation entirely
    ZeroPerturbation,
}

/// Get recommended risk minimization level based on risk score
pub fn recommend_minimization(risk_score: f64, incident_count: usize) -> RiskMinimizationLevel {
    match (risk_score, incident_count) {
        (s, _) if s < 0.2 => RiskMinimizationLevel::InformationEnhancement,
        (s, c) if s < 0.4 && c < 5 => RiskMinimizationLevel::UsageGuidance,
        (s, c) if s < 0.4 && c >= 5 => RiskMinimizationLevel::AdditionalMonitoring,
        (s, c) if s < 0.6 && c < 10 => RiskMinimizationLevel::ControlledAccess,
        (s, c) if s < 0.6 && c >= 10 => RiskMinimizationLevel::UsageRestriction,
        (s, _) if s < 0.8 => RiskMinimizationLevel::ConditionalUse,
        _ => RiskMinimizationLevel::Withdrawal,
    }
}

// ============================================================================
// THERAPEUTIC WINDOW ANALYSIS
// ============================================================================

/// Therapeutic window boundaries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TherapeuticWindow {
    /// Maximum risk score for safe use
    pub max_risk_score: f64,
    /// Required minimum expertise level
    pub min_expertise: ExpertiseLevel,
    /// Required minimum checkability
    pub min_checkability: CheckabilityLevel,
    /// Allowed output treatments
    pub allowed_treatments: Vec<OutputTreatment>,
}

impl Default for TherapeuticWindow {
    fn default() -> Self {
        Self {
            max_risk_score: 0.4,
            min_expertise: ExpertiseLevel::Moderate,
            min_checkability: CheckabilityLevel::Moderate,
            allowed_treatments: vec![OutputTreatment::Draft, OutputTreatment::Reviewed],
        }
    }
}

impl TherapeuticWindow {
    /// Check if context is within therapeutic window
    pub fn contains(&self, params: &ContextRiskParams) -> bool {
        let risk_ok = params.risk_score() <= self.max_risk_score;
        let expertise_ok = matches!(
            (&params.expertise, &self.min_expertise),
            (ExpertiseLevel::High, _)
                | (ExpertiseLevel::Moderate, ExpertiseLevel::Moderate | ExpertiseLevel::Low)
                | (ExpertiseLevel::Low, ExpertiseLevel::Low)
        );
        let checkability_ok = matches!(
            (&params.checkability, &self.min_checkability),
            (CheckabilityLevel::High, _)
                | (CheckabilityLevel::Moderate, CheckabilityLevel::Moderate | CheckabilityLevel::Low)
                | (CheckabilityLevel::Low, CheckabilityLevel::Low)
        );
        let treatment_ok = self.allowed_treatments.contains(&params.output_treatment);

        risk_ok && expertise_ok && checkability_ok && treatment_ok
    }

    /// Get violation reasons if outside window
    pub fn get_violations(&self, params: &ContextRiskParams) -> Vec<String> {
        let mut violations = Vec::new();

        if params.risk_score() > self.max_risk_score {
            violations.push(format!(
                "Risk score {} exceeds maximum {}",
                params.risk_score(),
                self.max_risk_score
            ));
        }

        if !self.allowed_treatments.contains(&params.output_treatment) {
            violations.push(format!(
                "Output treatment {:?} not in allowed list",
                params.output_treatment
            ));
        }

        violations
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_score_calculation() {
        let params = ContextRiskParams {
            stakes: StakesLevel::High,
            expertise: ExpertiseLevel::Low,
            checkability: CheckabilityLevel::Low,
            output_treatment: OutputTreatment::DirectUse,
        };

        let score = params.risk_score();
        // High (0.7) * Low expertise (0.9) * Low checkability (0.7) = 0.441
        assert!(score > 0.4 && score < 0.5);
    }

    #[test]
    fn test_risk_score_low() {
        let params = ContextRiskParams {
            stakes: StakesLevel::Low,
            expertise: ExpertiseLevel::High,
            checkability: CheckabilityLevel::High,
            output_treatment: OutputTreatment::Reviewed,
        };

        let score = params.risk_score();
        // Low (0.1) * High expertise (0.1) * High checkability (0.1) = 0.001
        assert!(score < 0.01);
    }

    #[test]
    fn test_therapeutic_window() {
        let safe_params = ContextRiskParams {
            stakes: StakesLevel::Moderate,
            expertise: ExpertiseLevel::High,
            checkability: CheckabilityLevel::High,
            output_treatment: OutputTreatment::Reviewed,
        };
        assert!(safe_params.is_therapeutic());

        let unsafe_params = ContextRiskParams {
            stakes: StakesLevel::High,
            expertise: ExpertiseLevel::Low,
            checkability: CheckabilityLevel::Low,
            output_treatment: OutputTreatment::DirectUse,
        };
        assert!(!unsafe_params.is_therapeutic());
    }

    #[test]
    fn test_risk_level_classification() {
        assert_eq!(RiskLevel::from_score(0.1), RiskLevel::Low);
        assert_eq!(RiskLevel::from_score(0.3), RiskLevel::Moderate);
        assert_eq!(RiskLevel::from_score(0.5), RiskLevel::High);
        assert_eq!(RiskLevel::from_score(0.8), RiskLevel::Critical);
    }

    #[test]
    fn test_risk_minimization_ordering() {
        assert!(RiskMinimizationLevel::InformationEnhancement < RiskMinimizationLevel::Withdrawal);
        assert!(RiskMinimizationLevel::ControlledAccess < RiskMinimizationLevel::UsageRestriction);
    }

    #[test]
    fn test_recommend_minimization() {
        assert_eq!(
            recommend_minimization(0.1, 2),
            RiskMinimizationLevel::InformationEnhancement
        );
        assert_eq!(
            recommend_minimization(0.3, 3),
            RiskMinimizationLevel::UsageGuidance
        );
        assert_eq!(
            recommend_minimization(0.9, 20),
            RiskMinimizationLevel::Withdrawal
        );
    }

    #[test]
    fn test_therapeutic_window_contains() {
        let window = TherapeuticWindow::default();

        let safe = ContextRiskParams {
            stakes: StakesLevel::Low,
            expertise: ExpertiseLevel::High,
            checkability: CheckabilityLevel::High,
            output_treatment: OutputTreatment::Reviewed,
        };
        assert!(window.contains(&safe));

        let unsafe_treatment = ContextRiskParams {
            stakes: StakesLevel::Low,
            expertise: ExpertiseLevel::High,
            checkability: CheckabilityLevel::High,
            output_treatment: OutputTreatment::DirectUse,
        };
        assert!(!window.contains(&unsafe_treatment));
    }

    #[test]
    fn test_calculate_risk_recommendations() {
        let params = ContextRiskParams {
            stakes: StakesLevel::Critical,
            expertise: ExpertiseLevel::Low,
            checkability: CheckabilityLevel::Unfalsifiable,
            output_treatment: OutputTreatment::Published,
        };

        let result = calculate_risk(&params);
        assert_eq!(result.level, RiskLevel::Critical);
        assert!(!result.therapeutic_window);
        assert!(!result.recommendations.is_empty());
    }
}
