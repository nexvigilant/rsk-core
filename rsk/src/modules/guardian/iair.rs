//! IAIR - Individual Algorithm Incident Report
//!
//! Implements the 8-block IAIR schema for standardized incident reporting
//! for AI/algorithm incidents, adapted from ToV §55.

use crate::tov::{ACACase, ACACausalityCategory, HarmType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// INCIDENT CATEGORY CODES (Claude-Specific)
// ============================================================================

/// Claude-specific incident categories mapping to ToV harm types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IncidentCategory {
    /// CL-CONFAB: Confident confabulation (fluent bullshit)
    Confabulation,
    /// CL-MOTREASON: Motivated reasoning (apparent rigor, wrong conclusion)
    MotivatedReasoning,
    /// CL-VULNCODE: Vulnerable code (security flaw in generated code)
    VulnerableCode,
    /// CL-MANIP: Unintended manipulation (persuasion without awareness)
    Manipulation,
    /// CL-FALSESYNTH: False synthesis (imposed coherence on contradictory sources)
    FalseSynthesis,
    /// CL-APOPH: Apophenia (false pattern detection)
    Apophenia,
    /// CL-BADFOLLOW: Harmful instruction following
    BadFollow,
    /// CL-ERRORPROP: Error propagation (early error compounded)
    ErrorPropagation,
    /// CL-OVERCONF: Miscalibrated confidence
    Overconfidence,
    /// CL-HALLUCITE: Hallucinated citation
    HallucinatedCitation,
}

impl IncidentCategory {
    /// Get the code string (e.g., "CL-CONFAB")
    pub fn code(&self) -> &'static str {
        match self {
            Self::Confabulation => "CL-CONFAB",
            Self::MotivatedReasoning => "CL-MOTREASON",
            Self::VulnerableCode => "CL-VULNCODE",
            Self::Manipulation => "CL-MANIP",
            Self::FalseSynthesis => "CL-FALSESYNTH",
            Self::Apophenia => "CL-APOPH",
            Self::BadFollow => "CL-BADFOLLOW",
            Self::ErrorPropagation => "CL-ERRORPROP",
            Self::Overconfidence => "CL-OVERCONF",
            Self::HallucinatedCitation => "CL-HALLUCITE",
        }
    }

    /// Map to ToV harm type
    pub fn to_harm_type(&self) -> HarmType {
        match self {
            Self::Confabulation => HarmType::OffTarget,
            Self::MotivatedReasoning => HarmType::Acute,
            Self::VulnerableCode => HarmType::Interaction,
            Self::Manipulation => HarmType::Idiosyncratic,
            Self::FalseSynthesis => HarmType::Cumulative,
            Self::Apophenia => HarmType::OffTarget,
            Self::BadFollow => HarmType::Acute,
            Self::ErrorPropagation => HarmType::Saturation,
            Self::Overconfidence => HarmType::Cumulative,
            Self::HallucinatedCitation => HarmType::OffTarget,
        }
    }

    /// Parse from code string
    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_uppercase().as_str() {
            "CL-CONFAB" | "CONFAB" | "CONFABULATION" => Some(Self::Confabulation),
            "CL-MOTREASON" | "MOTREASON" | "MOTIVATED_REASONING" => Some(Self::MotivatedReasoning),
            "CL-VULNCODE" | "VULNCODE" | "VULNERABLE_CODE" => Some(Self::VulnerableCode),
            "CL-MANIP" | "MANIP" | "MANIPULATION" => Some(Self::Manipulation),
            "CL-FALSESYNTH" | "FALSESYNTH" | "FALSE_SYNTHESIS" => Some(Self::FalseSynthesis),
            "CL-APOPH" | "APOPH" | "APOPHENIA" => Some(Self::Apophenia),
            "CL-BADFOLLOW" | "BADFOLLOW" | "BAD_FOLLOW" => Some(Self::BadFollow),
            "CL-ERRORPROP" | "ERRORPROP" | "ERROR_PROPAGATION" => Some(Self::ErrorPropagation),
            "CL-OVERCONF" | "OVERCONF" | "OVERCONFIDENCE" => Some(Self::Overconfidence),
            "CL-HALLUCITE" | "HALLUCITE" | "HALLUCINATED_CITATION" => {
                Some(Self::HallucinatedCitation)
            }
            _ => None,
        }
    }
}

// ============================================================================
// BLOCK A: METADATA
// ============================================================================

/// Reporter type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReporterType {
    User,
    Developer,
    Automated,
    SelfReport,
}

/// Interface type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceType {
    ClaudeCode,
    Api,
    Web,
    Mobile,
}

/// Block A: Report metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockA {
    pub report_id: String,
    pub session_id: String,
    pub report_timestamp: DateTime<Utc>,
    pub incident_timestamp: DateTime<Utc>,
    pub reporter_type: ReporterType,
    pub interface: InterfaceType,
}

// ============================================================================
// BLOCK B: MODEL IDENTIFICATION
// ============================================================================

/// Block B: Model identification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockB {
    pub model_name: String,
    pub model_version: String,
    pub context_window: Option<usize>,
    pub tools_enabled: Vec<String>,
    pub mcp_servers: Vec<String>,
    pub system_prompt_hash: Option<String>,
    pub hooks_active: Vec<String>,
}

// ============================================================================
// BLOCK C: CONTEXT CHARACTERISTICS (θ Parameters)
// ============================================================================

/// Expertise level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpertiseLevel {
    High,
    Moderate,
    Low,
    Unknown,
}

impl ExpertiseLevel {
    pub fn as_factor(&self) -> f64 {
        match self {
            Self::High => 0.1,
            Self::Moderate => 0.5,
            Self::Low => 0.9,
            Self::Unknown => 0.7,
        }
    }
}

/// Stakes level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StakesLevel {
    Low,
    Moderate,
    High,
    Critical,
}

impl StakesLevel {
    pub fn as_factor(&self) -> f64 {
        match self {
            Self::Low => 0.1,
            Self::Moderate => 0.4,
            Self::High => 0.7,
            Self::Critical => 1.0,
        }
    }
}

/// Checkability level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckabilityLevel {
    High,
    Moderate,
    Low,
    Unfalsifiable,
}

impl CheckabilityLevel {
    pub fn as_factor(&self) -> f64 {
        match self {
            Self::High => 0.1,
            Self::Moderate => 0.4,
            Self::Low => 0.7,
            Self::Unfalsifiable => 1.0,
        }
    }
}

/// Output treatment
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputTreatment {
    Draft,
    Reviewed,
    DirectUse,
    Published,
}

/// Block C: Context characteristics (θ parameters)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockC {
    pub user_expertise: ExpertiseLevel,
    pub domain: String,
    pub stakes: StakesLevel,
    pub checkability: CheckabilityLevel,
    pub iteration_expected: Option<bool>,
    pub output_treatment: OutputTreatment,
    pub time_pressure: Option<StakesLevel>,
}

// ============================================================================
// BLOCK D: INCIDENT DESCRIPTION
// ============================================================================

/// Block D: Incident description
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockD {
    pub incident_category: IncidentCategory,
    pub prompt_summary: String,
    pub prompt_hash: String,
    pub response_summary: String,
    pub harm_pathway: String,
    pub user_action: String,
    pub detection_method: String,
    pub contributing_factors: Vec<String>,
}

// ============================================================================
// BLOCK E: OUTCOME INFORMATION
// ============================================================================

/// Outcome type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutcomeType {
    NearMiss,
    NoHarm,
    TimeWasted,
    MinorError,
    SignificantError,
    Reputational,
    Financial,
    SecurityBreach,
    DecisionHarm,
    PropagatedHarm,
}

impl OutcomeType {
    /// Get severity range (min, max)
    pub fn severity_range(&self) -> (f64, f64) {
        match self {
            Self::NearMiss => (0.0, 0.0),
            Self::NoHarm => (0.0, 0.0),
            Self::TimeWasted => (0.0, 0.2),
            Self::MinorError => (0.1, 0.3),
            Self::SignificantError => (0.3, 0.5),
            Self::Reputational => (0.3, 0.6),
            Self::Financial => (0.4, 0.8),
            Self::SecurityBreach => (0.5, 0.9),
            Self::DecisionHarm => (0.5, 0.9),
            Self::PropagatedHarm => (0.6, 1.0),
        }
    }
}

/// Reversibility
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reversibility {
    FullyReversible,
    PartiallyReversible,
    Irreversible,
}

/// Block E: Outcome information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockE {
    pub outcome_type: OutcomeType,
    pub severity: f64,
    pub reversibility: Reversibility,
    pub propagation: Option<String>,
}

// ============================================================================
// BLOCK F: CAUSALITY ASSESSMENT
// ============================================================================

/// Lemma evaluation result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LemmaEvaluation {
    pub satisfied: bool,
    pub points: Option<i32>,
    pub notes: Option<String>,
}

/// Block F: Causality assessment (Claude ACA)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockF {
    pub logic_engine_case: ACACase,
    pub lemma_l1_temporal: LemmaEvaluation,
    pub lemma_l2_cognition: LemmaEvaluation,
    pub lemma_l3_action: LemmaEvaluation,
    pub lemma_l4_harm: LemmaEvaluation,
    pub lemma_l5_mechanism: LemmaEvaluation,
    pub lemma_l6_reproduction: LemmaEvaluation,
    pub lemma_l7_validation: LemmaEvaluation,
    pub lemma_l8_ground_truth: LemmaEvaluation,
    pub total_score: i32,
    pub causality_category: ACACausalityCategory,
}

impl BlockF {
    /// Calculate total score from lemma evaluations
    pub fn calculate_score(&self) -> i32 {
        let mut score = 0;
        if let Some(p) = self.lemma_l2_cognition.points {
            score += p;
        }
        if let Some(p) = self.lemma_l5_mechanism.points {
            score += p;
        }
        if let Some(p) = self.lemma_l6_reproduction.points {
            score += p;
        }
        if let Some(p) = self.lemma_l7_validation.points {
            score += p;
        }
        if let Some(p) = self.lemma_l8_ground_truth.points {
            score += p;
        }
        score
    }

    /// Check if required lemmas are satisfied
    pub fn required_lemmas_satisfied(&self) -> bool {
        self.lemma_l1_temporal.satisfied
            && self.lemma_l3_action.satisfied
            && self.lemma_l4_harm.satisfied
    }
}

// ============================================================================
// BLOCK G: SIGNAL INDICATORS
// ============================================================================

/// Block G: Signal indicators
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockG {
    pub signal_flag: bool,
    pub similar_incidents: Vec<String>,
    pub capability_involved: String,
    pub context_risk_score: f64,
    pub therapeutic_window_violation: bool,
    pub drift_indicator: bool,
}

// ============================================================================
// BLOCK H: ADMINISTRATIVE
// ============================================================================

/// Block H: Administrative
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockH {
    pub reported_to_anthropic: bool,
    pub corrective_action: Option<String>,
    pub preventive_measure: Option<String>,
    pub follow_up_required: bool,
    pub related_documentation: Vec<String>,
}

// ============================================================================
// COMPLETE IAIR
// ============================================================================

/// Complete Individual Algorithm Incident Report
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IAIR {
    pub block_a: BlockA,
    pub block_b: BlockB,
    pub block_c: BlockC,
    pub block_d: BlockD,
    pub block_e: BlockE,
    pub block_f: BlockF,
    pub block_g: BlockG,
    pub block_h: BlockH,
}

impl IAIR {
    /// Generate a new report ID using timestamp nanoseconds for uniqueness
    pub fn generate_report_id() -> String {
        let now = Utc::now();
        // Use nanoseconds for uniqueness (modulo 10000 for readability)
        let nanos = now.timestamp_subsec_nanos() % 10000;
        format!("CLAUDE-IAIR-{}-{:04}", now.format("%Y%m%d"), nanos)
    }

    /// Calculate context risk score from Block C
    pub fn calculate_context_risk_score(block_c: &BlockC) -> f64 {
        let stakes = block_c.stakes.as_factor();
        let expertise_inverse = block_c.user_expertise.as_factor();
        let checkability_inverse = block_c.checkability.as_factor();
        stakes * expertise_inverse * checkability_inverse
    }

    /// Determine if this is within the therapeutic window
    pub fn is_within_therapeutic_window(&self) -> bool {
        // Inside therapeutic window if:
        // - Expertise is high or moderate AND
        // - Checkability is high or moderate AND
        // - Output was reviewed or draft
        let expertise_ok = matches!(
            self.block_c.user_expertise,
            ExpertiseLevel::High | ExpertiseLevel::Moderate
        );
        let checkability_ok = matches!(
            self.block_c.checkability,
            CheckabilityLevel::High | CheckabilityLevel::Moderate
        );
        let output_ok = matches!(
            self.block_c.output_treatment,
            OutputTreatment::Draft | OutputTreatment::Reviewed
        );

        expertise_ok && checkability_ok && output_ok
    }

    /// Map ACA case to ToV propagation
    pub fn get_tov_case(&self) -> ACACase {
        self.block_f.logic_engine_case
    }

    /// Get the associated harm type
    pub fn get_harm_type(&self) -> HarmType {
        self.block_d.incident_category.to_harm_type()
    }
}

// ============================================================================
// IAIR BUILDER
// ============================================================================

/// Builder for creating IAIRs
#[derive(Default)]
pub struct IAIRBuilder {
    session_id: Option<String>,
    model_name: Option<String>,
    model_version: Option<String>,
    tools_enabled: Vec<String>,
    user_expertise: Option<ExpertiseLevel>,
    domain: Option<String>,
    stakes: Option<StakesLevel>,
    checkability: Option<CheckabilityLevel>,
    output_treatment: Option<OutputTreatment>,
    incident_category: Option<IncidentCategory>,
    prompt_summary: Option<String>,
    response_summary: Option<String>,
    harm_pathway: Option<String>,
    outcome_type: Option<OutcomeType>,
    severity: Option<f64>,
}

impl IAIRBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }

    pub fn model(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.model_name = Some(name.into());
        self.model_version = Some(version.into());
        self
    }

    pub fn tools(mut self, tools: Vec<String>) -> Self {
        self.tools_enabled = tools;
        self
    }

    pub fn context(
        mut self,
        expertise: ExpertiseLevel,
        stakes: StakesLevel,
        checkability: CheckabilityLevel,
    ) -> Self {
        self.user_expertise = Some(expertise);
        self.stakes = Some(stakes);
        self.checkability = Some(checkability);
        self
    }

    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    pub fn output_treatment(mut self, treatment: OutputTreatment) -> Self {
        self.output_treatment = Some(treatment);
        self
    }

    pub fn incident(mut self, category: IncidentCategory) -> Self {
        self.incident_category = Some(category);
        self
    }

    pub fn description(
        mut self,
        prompt: impl Into<String>,
        response: impl Into<String>,
        pathway: impl Into<String>,
    ) -> Self {
        self.prompt_summary = Some(prompt.into());
        self.response_summary = Some(response.into());
        self.harm_pathway = Some(pathway.into());
        self
    }

    pub fn outcome(mut self, outcome_type: OutcomeType, severity: f64) -> Self {
        self.outcome_type = Some(outcome_type);
        self.severity = Some(severity);
        self
    }

    /// Build a minimal IAIR (for quick incident logging)
    pub fn build_minimal(self) -> Result<IAIR, String> {
        let now = Utc::now();
        let report_id = IAIR::generate_report_id();

        let block_c = BlockC {
            user_expertise: self.user_expertise.unwrap_or(ExpertiseLevel::Unknown),
            domain: self.domain.unwrap_or_else(|| "unknown".to_string()),
            stakes: self.stakes.unwrap_or(StakesLevel::Moderate),
            checkability: self.checkability.unwrap_or(CheckabilityLevel::Moderate),
            iteration_expected: None,
            output_treatment: self.output_treatment.unwrap_or(OutputTreatment::DirectUse),
            time_pressure: None,
        };

        let context_risk = IAIR::calculate_context_risk_score(&block_c);

        Ok(IAIR {
            block_a: BlockA {
                report_id,
                session_id: self.session_id.unwrap_or_else(|| "unknown".to_string()),
                report_timestamp: now,
                incident_timestamp: now,
                reporter_type: ReporterType::User,
                interface: InterfaceType::ClaudeCode,
            },
            block_b: BlockB {
                model_name: self.model_name.unwrap_or_else(|| "Claude".to_string()),
                model_version: self.model_version.unwrap_or_else(|| "unknown".to_string()),
                context_window: None,
                tools_enabled: self.tools_enabled,
                mcp_servers: vec![],
                system_prompt_hash: None,
                hooks_active: vec![],
            },
            block_c,
            block_d: BlockD {
                incident_category: self.incident_category.ok_or("incident_category required")?,
                prompt_summary: self.prompt_summary.unwrap_or_default(),
                prompt_hash: "".to_string(),
                response_summary: self.response_summary.unwrap_or_default(),
                harm_pathway: self.harm_pathway.unwrap_or_default(),
                user_action: "".to_string(),
                detection_method: "manual".to_string(),
                contributing_factors: vec![],
            },
            block_e: BlockE {
                outcome_type: self.outcome_type.unwrap_or(OutcomeType::NearMiss),
                severity: self.severity.unwrap_or(0.0),
                reversibility: Reversibility::FullyReversible,
                propagation: None,
            },
            block_f: BlockF {
                logic_engine_case: ACACase::CaseIII, // Default to signal
                lemma_l1_temporal: LemmaEvaluation {
                    satisfied: true,
                    points: None,
                    notes: None,
                },
                lemma_l2_cognition: LemmaEvaluation {
                    satisfied: false,
                    points: Some(0),
                    notes: None,
                },
                lemma_l3_action: LemmaEvaluation {
                    satisfied: false,
                    points: None,
                    notes: None,
                },
                lemma_l4_harm: LemmaEvaluation {
                    satisfied: false,
                    points: None,
                    notes: None,
                },
                lemma_l5_mechanism: LemmaEvaluation {
                    satisfied: false,
                    points: Some(0),
                    notes: None,
                },
                lemma_l6_reproduction: LemmaEvaluation {
                    satisfied: false,
                    points: Some(0),
                    notes: None,
                },
                lemma_l7_validation: LemmaEvaluation {
                    satisfied: false,
                    points: Some(0),
                    notes: None,
                },
                lemma_l8_ground_truth: LemmaEvaluation {
                    satisfied: false,
                    points: Some(0),
                    notes: None,
                },
                total_score: 0,
                causality_category: ACACausalityCategory::Unlikely,
            },
            block_g: BlockG {
                signal_flag: true,
                similar_incidents: vec![],
                capability_involved: "unknown".to_string(),
                context_risk_score: context_risk,
                therapeutic_window_violation: context_risk > 0.5,
                drift_indicator: false,
            },
            block_h: BlockH {
                reported_to_anthropic: false,
                corrective_action: None,
                preventive_measure: None,
                follow_up_required: false,
                related_documentation: vec![],
            },
        })
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incident_category_codes() {
        assert_eq!(IncidentCategory::Confabulation.code(), "CL-CONFAB");
        assert_eq!(IncidentCategory::VulnerableCode.code(), "CL-VULNCODE");
    }

    #[test]
    fn test_incident_category_from_code() {
        assert_eq!(
            IncidentCategory::from_code("CL-CONFAB"),
            Some(IncidentCategory::Confabulation)
        );
        assert_eq!(
            IncidentCategory::from_code("confab"),
            Some(IncidentCategory::Confabulation)
        );
        assert_eq!(IncidentCategory::from_code("invalid"), None);
    }

    #[test]
    fn test_incident_category_to_harm_type() {
        assert_eq!(
            IncidentCategory::Confabulation.to_harm_type(),
            HarmType::OffTarget
        );
        assert_eq!(
            IncidentCategory::VulnerableCode.to_harm_type(),
            HarmType::Interaction
        );
    }

    #[test]
    fn test_context_risk_score() {
        let block_c = BlockC {
            user_expertise: ExpertiseLevel::Low,
            domain: "legal".to_string(),
            stakes: StakesLevel::High,
            checkability: CheckabilityLevel::Low,
            iteration_expected: None,
            output_treatment: OutputTreatment::DirectUse,
            time_pressure: None,
        };

        let risk = IAIR::calculate_context_risk_score(&block_c);
        // High stakes (0.7) * low expertise (0.9) * low checkability (0.7) = 0.441
        assert!(risk > 0.4 && risk < 0.5);
    }

    #[test]
    fn test_therapeutic_window() {
        let mut iair = IAIRBuilder::new()
            .session_id("test")
            .model("Claude", "4.5")
            .context(
                ExpertiseLevel::High,
                StakesLevel::Low,
                CheckabilityLevel::High,
            )
            .domain("testing")
            .output_treatment(OutputTreatment::Reviewed)
            .incident(IncidentCategory::Confabulation)
            .build_minimal()
            .unwrap();

        assert!(iair.is_within_therapeutic_window());

        // Change to outside therapeutic window
        iair.block_c.user_expertise = ExpertiseLevel::Low;
        iair.block_c.checkability = CheckabilityLevel::Low;
        iair.block_c.output_treatment = OutputTreatment::DirectUse;

        assert!(!iair.is_within_therapeutic_window());
    }

    #[test]
    fn test_iair_builder() {
        let iair = IAIRBuilder::new()
            .session_id("test-session")
            .model("Claude Opus 4.5", "claude-opus-4-5-20251101")
            .tools(vec!["Read".to_string(), "Edit".to_string()])
            .context(
                ExpertiseLevel::Moderate,
                StakesLevel::High,
                CheckabilityLevel::Low,
            )
            .domain("legal_research")
            .output_treatment(OutputTreatment::DirectUse)
            .incident(IncidentCategory::Confabulation)
            .description(
                "Asked about statute",
                "Provided incorrect interpretation",
                "User relied on bad advice",
            )
            .outcome(OutcomeType::Financial, 0.6)
            .build_minimal()
            .unwrap();

        assert!(iair.block_a.report_id.starts_with("CLAUDE-IAIR-"));
        assert_eq!(
            iair.block_d.incident_category,
            IncidentCategory::Confabulation
        );
        assert_eq!(iair.block_e.severity, 0.6);
    }

    #[test]
    fn test_report_id_generation() {
        let id1 = IAIR::generate_report_id();
        let id2 = IAIR::generate_report_id();

        assert!(id1.starts_with("CLAUDE-IAIR-"));
        assert!(id2.starts_with("CLAUDE-IAIR-"));
        // IDs should be unique (with high probability)
        assert_ne!(id1, id2);
    }
}
