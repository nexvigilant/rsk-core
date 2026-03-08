//! Theory of Vigilance - Core Types and Classification
//!
//! This module provides the core types and functions for the Theory of Vigilance
//! framework, including harm classification, conservation laws, and causality assessment.

use serde::{Deserialize, Serialize};

// ============================================================================
// HARM CLASSIFICATION (Section 9)
// ============================================================================

/// Harm type classification (8 types from 2^3 combinations)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HarmType {
    /// Type A: Immediate severe harm from high-magnitude perturbation
    Acute,
    /// Type B: Gradual harm from accumulated exposure
    Cumulative,
    /// Type C: Unintended effects on non-target components
    OffTarget,
    /// Type D: Propagating failure across interconnected components
    Cascade,
    /// Type E: Rare harm due to unusual susceptibility
    Idiosyncratic,
    /// Type F: Harm from exceeding processing capacity
    Saturation,
    /// Type G: Harm from combining multiple perturbations
    Interaction,
    /// Type H: Differential harm across subgroups
    Population,
}

/// Perturbation multiplicity
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Multiplicity {
    Single,
    Multiple,
}

/// Temporal profile
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Temporal {
    Acute,
    Chronic,
}

/// Response determinism
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Determinism {
    Deterministic,
    Stochastic,
}

/// Harm characteristics (2^3 = 8 combinations)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarmCharacteristics {
    pub multiplicity: Multiplicity,
    pub temporal: Temporal,
    pub determinism: Determinism,
}

/// A harm event with its observable characteristics
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CharacterizedHarmEvent {
    pub characteristics: HarmCharacteristics,
}

/// Classify a characterized harm event into one of 8 types
///
/// This implements the bijection from (Multiplicity x Temporal x Determinism)
/// to HarmType A-H as defined in Section 9.0.
pub fn classify_harm(event: CharacterizedHarmEvent) -> HarmType {
    use Determinism::*;
    use Multiplicity::*;
    use Temporal::*;

    match (
        event.characteristics.multiplicity,
        event.characteristics.temporal,
        event.characteristics.determinism,
    ) {
        (Single, Acute, Deterministic) => HarmType::Acute, // A
        (Single, Chronic, Deterministic) => HarmType::Cumulative, // B
        (Single, Acute, Stochastic) => HarmType::Idiosyncratic, // E
        (Single, Chronic, Stochastic) => HarmType::OffTarget, // C
        (Multiple, Acute, Deterministic) => HarmType::Cascade, // D
        (Multiple, Chronic, Deterministic) => HarmType::Saturation, // F
        (Multiple, Acute, Stochastic) => HarmType::Interaction, // G
        (Multiple, Chronic, Stochastic) => HarmType::Population, // H
    }
}

/// Get the characteristics for a given harm type (inverse of classify_harm)
pub fn harm_type_characteristics(harm_type: HarmType) -> HarmCharacteristics {
    match harm_type {
        HarmType::Acute => HarmCharacteristics {
            multiplicity: Multiplicity::Single,
            temporal: Temporal::Acute,
            determinism: Determinism::Deterministic,
        },
        HarmType::Cumulative => HarmCharacteristics {
            multiplicity: Multiplicity::Single,
            temporal: Temporal::Chronic,
            determinism: Determinism::Deterministic,
        },
        HarmType::Idiosyncratic => HarmCharacteristics {
            multiplicity: Multiplicity::Single,
            temporal: Temporal::Acute,
            determinism: Determinism::Stochastic,
        },
        HarmType::OffTarget => HarmCharacteristics {
            multiplicity: Multiplicity::Single,
            temporal: Temporal::Chronic,
            determinism: Determinism::Stochastic,
        },
        HarmType::Cascade => HarmCharacteristics {
            multiplicity: Multiplicity::Multiple,
            temporal: Temporal::Acute,
            determinism: Determinism::Deterministic,
        },
        HarmType::Saturation => HarmCharacteristics {
            multiplicity: Multiplicity::Multiple,
            temporal: Temporal::Chronic,
            determinism: Determinism::Deterministic,
        },
        HarmType::Interaction => HarmCharacteristics {
            multiplicity: Multiplicity::Multiple,
            temporal: Temporal::Acute,
            determinism: Determinism::Stochastic,
        },
        HarmType::Population => HarmCharacteristics {
            multiplicity: Multiplicity::Multiple,
            temporal: Temporal::Chronic,
            determinism: Determinism::Stochastic,
        },
    }
}

// ============================================================================
// CONSERVATION LAWS (Section 8)
// ============================================================================

/// Mathematical type of conservation law
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LawType {
    /// dC/dt = 0 along trajectories (first integral)
    StrictConservation,
    /// g(s,u,theta) <= 0 (feasibility constraint)
    InequalityConstraint,
    /// dV/dt <= 0 (Lyapunov stability)
    LyapunovFunction,
    /// I: S -> D constant (topological invariant)
    StructuralInvariant,
}

/// The 11 conservation laws
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConservationLaw {
    /// Law 1: dM/dt = J_in - J_out
    Mass,
    /// Law 2: dV/dt <= 0
    Energy,
    /// Law 3: sum(p_i) = 1
    State,
    /// Law 4: sum(J_in) = sum(J_out)
    Flux,
    /// Law 5: [E]_final = [E]_initial
    Catalyst,
    /// Law 6: dA_i/dt = net flux
    Rate,
    /// Law 7: ds/dt -> 0
    Equilibrium,
    /// Law 8: v <= V_max
    Saturation,
    /// Law 9: dS_total >= 0
    Entropy,
    /// Law 10: X in {0, q, 2q, ...}
    Discretization,
    /// Law 11: Sigma(s(t)) = Sigma(s(0))
    Structure,
}

impl ConservationLaw {
    /// Get the index (1-11) for this law
    pub fn index(&self) -> u8 {
        match self {
            ConservationLaw::Mass => 1,
            ConservationLaw::Energy => 2,
            ConservationLaw::State => 3,
            ConservationLaw::Flux => 4,
            ConservationLaw::Catalyst => 5,
            ConservationLaw::Rate => 6,
            ConservationLaw::Equilibrium => 7,
            ConservationLaw::Saturation => 8,
            ConservationLaw::Entropy => 9,
            ConservationLaw::Discretization => 10,
            ConservationLaw::Structure => 11,
        }
    }

    /// Get the mathematical type for this law
    pub fn law_type(&self) -> LawType {
        match self {
            ConservationLaw::Mass => LawType::StrictConservation,
            ConservationLaw::Energy => LawType::LyapunovFunction,
            ConservationLaw::State => LawType::StrictConservation,
            ConservationLaw::Flux => LawType::StrictConservation,
            ConservationLaw::Catalyst => LawType::StrictConservation,
            ConservationLaw::Rate => LawType::StrictConservation,
            ConservationLaw::Equilibrium => LawType::LyapunovFunction,
            ConservationLaw::Saturation => LawType::InequalityConstraint,
            ConservationLaw::Entropy => LawType::InequalityConstraint,
            ConservationLaw::Discretization => LawType::StructuralInvariant,
            ConservationLaw::Structure => LawType::StructuralInvariant,
        }
    }
}

/// Connection: Harm type to primary conservation law
pub fn harm_law_connection(harm_type: HarmType) -> ConservationLaw {
    match harm_type {
        HarmType::Acute => ConservationLaw::Mass,
        HarmType::Cumulative => ConservationLaw::Mass,
        HarmType::OffTarget => ConservationLaw::Energy,
        HarmType::Cascade => ConservationLaw::Flux,
        HarmType::Idiosyncratic => ConservationLaw::Structure,
        HarmType::Saturation => ConservationLaw::Saturation,
        HarmType::Interaction => ConservationLaw::Catalyst,
        HarmType::Population => ConservationLaw::State,
    }
}

// ============================================================================
// DOMAINS (Section 11-15)
// ============================================================================

/// ToV Domains
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    /// Cloud computing infrastructure
    Cloud,
    /// Pharmacovigilance (drug safety)
    Pharmacovigilance,
    /// Algorithmovigilance (AI safety)
    Algorithmovigilance,
}

// ============================================================================
// ACA - ALGORITHM CAUSALITY ASSESSMENT (Section 51-53)
// ============================================================================

/// ACA Four-Case Logic Engine
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ACACase {
    /// Case I: Algorithm wrong + Followed + Harm = Incident
    CaseI,
    /// Case II: Algorithm correct + Overrode + Harm = Exculpated
    CaseII,
    /// Case III: Algorithm wrong + Overrode = Signal (near-miss)
    CaseIII,
    /// Case IV: Algorithm correct + Followed + Good = Baseline
    CaseIV,
}

/// Algorithm correctness
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlgorithmCorrectness {
    Correct,
    Wrong,
}

/// Clinician response to algorithm
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClinicianResponse {
    Followed,
    Overrode,
}

/// Clinical outcome
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClinicalOutcome {
    Good,
    Harm,
}

/// Determine the ACA case from inputs
pub fn determine_aca_case(
    correctness: AlgorithmCorrectness,
    response: ClinicianResponse,
    outcome: ClinicalOutcome,
) -> ACACase {
    use AlgorithmCorrectness::*;
    use ClinicalOutcome::*;
    use ClinicianResponse::*;

    match (correctness, response, outcome) {
        (Wrong, Followed, Harm) => ACACase::CaseI,
        (Correct, Overrode, Harm) => ACACase::CaseII,
        (Wrong, Overrode, _) => ACACase::CaseIII,
        (Correct, Followed, Good) => ACACase::CaseIV,
        // Default cases
        (Correct, Followed, Harm) => ACACase::CaseI, // Unexpected harm
        (Wrong, Followed, Good) => ACACase::CaseIV,  // Lucky outcome
        (Correct, Overrode, Good) => ACACase::CaseIV, // Override worked
    }
}

/// Get the propagation factor for a case
pub fn case_propagation_factor(case: ACACase) -> f64 {
    match case {
        ACACase::CaseI => 1.0,   // Full propagation
        ACACase::CaseII => 0.0,  // No propagation
        ACACase::CaseIII => 0.5, // Partial propagation (near-miss)
        ACACase::CaseIV => -0.1, // Negative evidence
    }
}

/// ACA Causality Category
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ACACausalityCategory {
    /// Score >= 6
    Definite,
    /// Score 4-5
    Probable,
    /// Score 2-3
    Possible,
    /// Score < 2
    Unlikely,
}

/// Categorize ACA score
pub fn categorize_aca_score(score: i32) -> ACACausalityCategory {
    match score {
        s if s >= 6 => ACACausalityCategory::Definite,
        s if s >= 4 => ACACausalityCategory::Probable,
        s if s >= 2 => ACACausalityCategory::Possible,
        _ => ACACausalityCategory::Unlikely,
    }
}

/// ACA Lemmas
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ACALemma {
    /// L1: Temporal - Required
    L1Temporal,
    /// L2: Cognition
    L2Cognition,
    /// L3: Action - Required
    L3Action,
    /// L4: Harm - Required
    L4Harm,
    /// L5: Mechanism
    L5Mechanism,
    /// L6: Rechallenge
    L6Rechallenge,
    /// L7: Alternatives
    L7Alternatives,
    /// L8: Ground Truth
    L8GroundTruth,
}

/// Check if lemma is required
pub fn lemma_required(lemma: ACALemma) -> bool {
    matches!(
        lemma,
        ACALemma::L1Temporal | ACALemma::L3Action | ACALemma::L4Harm
    )
}

/// Get points for a lemma
pub fn lemma_points(lemma: ACALemma) -> i32 {
    match lemma {
        ACALemma::L1Temporal => 0, // Required, no points
        ACALemma::L2Cognition => 1,
        ACALemma::L3Action => 0, // Required, no points
        ACALemma::L4Harm => 0,   // Required, no points
        ACALemma::L5Mechanism => 1,
        ACALemma::L6Rechallenge => 2,
        ACALemma::L7Alternatives => 1,
        ACALemma::L8GroundTruth => 2,
    }
}

// ============================================================================
// ARCHITECTURE ADJACENCY (Section 61)
// ============================================================================

/// Architecture relationship type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchitectureRelationship {
    /// Same model family (e.g., GPT-3 variants)
    SameFamily,
    /// Same base model (e.g., same fine-tune base)
    SameBase,
    /// Same architectural pattern (e.g., transformer-based)
    SamePattern,
    /// Different architecture
    Different,
}

/// Get architecture adjacency score
pub fn architecture_adjacency(relationship: ArchitectureRelationship) -> f64 {
    match relationship {
        ArchitectureRelationship::SameFamily => 0.9,
        ArchitectureRelationship::SameBase => 0.7,
        ArchitectureRelationship::SamePattern => 0.4,
        ArchitectureRelationship::Different => 0.1,
    }
}

// ============================================================================
// FAILURE ATTRIBUTION (Section 62)
// ============================================================================

/// Failure attribution result
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureAttribution {
    /// Failure attributed to AI model
    AIModel,
    /// Failure attributed to infrastructure
    Infrastructure,
    /// Compound failure (both)
    Compound,
    /// Unknown attribution
    Unknown,
}

/// Attribute failure based on system state
pub fn attribute_failure(
    infra_healthy: bool,
    model_validated: bool,
    recent_deployment: bool,
) -> FailureAttribution {
    match (infra_healthy, model_validated, recent_deployment) {
        (false, _, _) => FailureAttribution::Infrastructure,
        (true, false, _) => FailureAttribution::AIModel,
        (true, true, true) => FailureAttribution::Compound,
        (true, true, false) => FailureAttribution::Unknown,
    }
}

// ============================================================================
// KHS_AI - KINETICS HEALTH SCORE FOR AI (Section 62)
// ============================================================================

/// KHS_AI Score
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct KHSAI {
    pub overall: u8,
    pub latency_stability: u8,
    pub accuracy_stability: u8,
    pub resource_efficiency: u8,
    pub drift_score: u8,
}

impl KHSAI {
    /// Calculate KHS_AI from component scores (each 0-100)
    pub fn calculate(latency: u8, accuracy: u8, resource: u8, drift: u8) -> Self {
        let sum = u16::from(latency) + u16::from(accuracy) + u16::from(resource) + u16::from(drift);
        #[allow(clippy::as_conversions, clippy::cast_possible_truncation)] // u16→u8: max average of 4 u8 values is 255, fits in u8
        let overall = (sum / 4) as u8;
        KHSAI {
            overall,
            latency_stability: latency,
            accuracy_stability: accuracy,
            resource_efficiency: resource,
            drift_score: drift,
        }
    }
}

/// KHS_AI Status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KHSAIStatus {
    /// Score >= 80
    Healthy,
    /// Score 60-79
    Monitor,
    /// Score 40-59
    Investigate,
    /// Score < 40
    Intervene,
}

/// Interpret KHS_AI score
pub fn interpret_khs_ai(score: u8) -> KHSAIStatus {
    match score {
        s if s >= 80 => KHSAIStatus::Healthy,
        s if s >= 60 => KHSAIStatus::Monitor,
        s if s >= 40 => KHSAIStatus::Investigate,
        _ => KHSAIStatus::Intervene,
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harm_classification_exhaustive() {
        let test_cases = [
            (
                Multiplicity::Single,
                Temporal::Acute,
                Determinism::Deterministic,
                HarmType::Acute,
            ),
            (
                Multiplicity::Single,
                Temporal::Chronic,
                Determinism::Deterministic,
                HarmType::Cumulative,
            ),
            (
                Multiplicity::Single,
                Temporal::Acute,
                Determinism::Stochastic,
                HarmType::Idiosyncratic,
            ),
            (
                Multiplicity::Single,
                Temporal::Chronic,
                Determinism::Stochastic,
                HarmType::OffTarget,
            ),
            (
                Multiplicity::Multiple,
                Temporal::Acute,
                Determinism::Deterministic,
                HarmType::Cascade,
            ),
            (
                Multiplicity::Multiple,
                Temporal::Chronic,
                Determinism::Deterministic,
                HarmType::Saturation,
            ),
            (
                Multiplicity::Multiple,
                Temporal::Acute,
                Determinism::Stochastic,
                HarmType::Interaction,
            ),
            (
                Multiplicity::Multiple,
                Temporal::Chronic,
                Determinism::Stochastic,
                HarmType::Population,
            ),
        ];

        for (mult, temp, det, expected) in test_cases {
            let event = CharacterizedHarmEvent {
                characteristics: HarmCharacteristics {
                    multiplicity: mult,
                    temporal: temp,
                    determinism: det,
                },
            };
            assert_eq!(classify_harm(event), expected);
        }
    }

    #[test]
    fn test_harm_type_roundtrip() {
        for harm_type in [
            HarmType::Acute,
            HarmType::Cumulative,
            HarmType::OffTarget,
            HarmType::Cascade,
            HarmType::Idiosyncratic,
            HarmType::Saturation,
            HarmType::Interaction,
            HarmType::Population,
        ] {
            let chars = harm_type_characteristics(harm_type);
            let event = CharacterizedHarmEvent {
                characteristics: chars,
            };
            assert_eq!(classify_harm(event), harm_type);
        }
    }

    #[test]
    fn test_conservation_law_indices() {
        assert_eq!(ConservationLaw::Mass.index(), 1);
        assert_eq!(ConservationLaw::Structure.index(), 11);
    }

    #[test]
    fn test_aca_case_determination() {
        assert_eq!(
            determine_aca_case(
                AlgorithmCorrectness::Wrong,
                ClinicianResponse::Followed,
                ClinicalOutcome::Harm
            ),
            ACACase::CaseI
        );
        assert_eq!(
            determine_aca_case(
                AlgorithmCorrectness::Correct,
                ClinicianResponse::Overrode,
                ClinicalOutcome::Harm
            ),
            ACACase::CaseII
        );
        assert_eq!(
            determine_aca_case(
                AlgorithmCorrectness::Wrong,
                ClinicianResponse::Overrode,
                ClinicalOutcome::Good
            ),
            ACACase::CaseIII
        );
        assert_eq!(
            determine_aca_case(
                AlgorithmCorrectness::Correct,
                ClinicianResponse::Followed,
                ClinicalOutcome::Good
            ),
            ACACase::CaseIV
        );
    }

    #[test]
    fn test_case_propagation_factors() {
        assert_eq!(case_propagation_factor(ACACase::CaseI), 1.0);
        assert_eq!(case_propagation_factor(ACACase::CaseII), 0.0);
        assert_eq!(case_propagation_factor(ACACase::CaseIII), 0.5);
        assert!(case_propagation_factor(ACACase::CaseIV) < 0.0);
    }

    #[test]
    fn test_aca_causality_categories() {
        assert_eq!(categorize_aca_score(7), ACACausalityCategory::Definite);
        assert_eq!(categorize_aca_score(5), ACACausalityCategory::Probable);
        assert_eq!(categorize_aca_score(3), ACACausalityCategory::Possible);
        assert_eq!(categorize_aca_score(1), ACACausalityCategory::Unlikely);
    }

    #[test]
    fn test_architecture_adjacency_ordering() {
        let same_family = architecture_adjacency(ArchitectureRelationship::SameFamily);
        let same_base = architecture_adjacency(ArchitectureRelationship::SameBase);
        let same_pattern = architecture_adjacency(ArchitectureRelationship::SamePattern);
        let different = architecture_adjacency(ArchitectureRelationship::Different);

        assert!(same_family > same_base);
        assert!(same_base > same_pattern);
        assert!(same_pattern > different);
    }

    #[test]
    fn test_failure_attribution() {
        assert_eq!(
            attribute_failure(false, true, false),
            FailureAttribution::Infrastructure
        );
        assert_eq!(
            attribute_failure(true, false, false),
            FailureAttribution::AIModel
        );
        assert_eq!(
            attribute_failure(true, true, true),
            FailureAttribution::Compound
        );
        assert_eq!(
            attribute_failure(true, true, false),
            FailureAttribution::Unknown
        );
    }

    #[test]
    fn test_khs_ai_calculation() {
        let khs = KHSAI::calculate(80, 85, 75, 80);
        assert_eq!(khs.overall, 80);
        assert_eq!(khs.latency_stability, 80);
        assert_eq!(khs.accuracy_stability, 85);
    }

    #[test]
    fn test_khs_ai_interpretation() {
        assert_eq!(interpret_khs_ai(90), KHSAIStatus::Healthy);
        assert_eq!(interpret_khs_ai(70), KHSAIStatus::Monitor);
        assert_eq!(interpret_khs_ai(50), KHSAIStatus::Investigate);
        assert_eq!(interpret_khs_ai(30), KHSAIStatus::Intervene);
    }

    #[test]
    fn test_lemma_required() {
        assert!(lemma_required(ACALemma::L1Temporal));
        assert!(lemma_required(ACALemma::L3Action));
        assert!(lemma_required(ACALemma::L4Harm));
        assert!(!lemma_required(ACALemma::L2Cognition));
        assert!(!lemma_required(ACALemma::L6Rechallenge));
    }
}
