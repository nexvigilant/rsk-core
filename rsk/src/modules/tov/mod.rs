//! Theory of Vigilance (ToV) - Shared Rust Primitives
//!
//! This module provides the core Rust implementation of the Theory of Vigilance,
//! a universal mathematical framework for predicting and preventing harm in
//! complex systems.
//!
//! ## Modules
//!
//! - **logic_prelude** - Core Curry-Howard types (Void, And, Or, Exists, Not)
//! - **type_level** - Compile-time constraint verification
//! - **attenuation** - Attenuation Theorem (T10.2) implementation
//! - **vigilance** - Core types and classification (harm types, conservation laws, ACA)
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rsk::tov::*;
//!
//! // Classify a harm event
//! let event = CharacterizedHarmEvent {
//!     characteristics: HarmCharacteristics {
//!         multiplicity: Multiplicity::Single,
//!         temporal: Temporal::Acute,
//!         determinism: Determinism::Deterministic,
//!     },
//! };
//! let harm_type = classify_harm(event);
//! assert_eq!(harm_type, HarmType::Acute);
//!
//! // Calculate attenuation
//! let probs = vec![
//!     PropagationProbability::new(0.5),
//!     PropagationProbability::new(0.3),
//! ];
//! let result = analyze_attenuation(&probs);
//! assert!(result.attenuation_verified);
//! ```
//!
//! ## CLI Commands
//!
//! ```text
//! rsk tov classify --mult single --temp acute --det deterministic
//! rsk tov attenuation --probs 0.5,0.3,0.2
//! rsk tov aca --correctness wrong --response followed --outcome harm
//! rsk tov khs --latency 80 --accuracy 85 --resource 75 --drift 80
//! ```

pub mod attenuation;
pub mod logic_prelude;
pub mod type_level;
pub mod vigilance;

// Re-export core types for convenience
pub use attenuation::{
    AttenuationResult, PropagationProbability, ProtectiveDepthRecommendations, analyze_attenuation,
    attenuation_rate, harm_probability, harm_probability_exponential, protective_depth,
    protective_depth_recommendations, verify_attenuation,
};

pub use logic_prelude::{And, Exists, Not, Or, Proof, Truth, Void};

pub use type_level::{
    BoundedProbability, ElementCount, HasElementCount, IsValidLevel, NonRecurrenceThreshold,
    StandardElementCount, ValidatedDomainIndex, ValidatedHarmTypeIndex, ValidatedLawIndex,
    ValidatedLevel, ValidatedRarity,
};

pub use vigilance::{
    ACACase, ACACausalityCategory, ACALemma, AlgorithmCorrectness, ArchitectureRelationship,
    CharacterizedHarmEvent, ClinicalOutcome, ClinicianResponse, ConservationLaw, Determinism,
    Domain, FailureAttribution, HarmCharacteristics, HarmType, KHSAI, KHSAIStatus, LawType,
    Multiplicity, Temporal, architecture_adjacency, attribute_failure, case_propagation_factor,
    categorize_aca_score, classify_harm, determine_aca_case, harm_law_connection,
    harm_type_characteristics, interpret_khs_ai, lemma_points, lemma_required,
};
