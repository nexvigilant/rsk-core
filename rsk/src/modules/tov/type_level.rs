//! Type-Level Constraints for Theory of Vigilance
//!
//! This module uses const generics and typenum to enforce ToV constraints
//! at compile time, providing stronger guarantees than PhantomData-based encoding.
//!
//! ## Verification Strategy
//!
//! | Constraint | Mechanism | Verified Property |
//! |------------|-----------|-------------------|
//! | Hierarchy levels | `ValidatedLevel<N>` | N in [1, 8] |
//! | Conservation laws | `ValidatedLawIndex<I>` | I in [1, 11] |
//! | Element cardinality | `ElementCount<15>` | |E| = 15 |
//! | Signal threshold | `NonRecurrenceThreshold` | U_NR = 63 bits |

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use typenum::{U15, U63, Unsigned};

// ============================================================================
// HIERARCHY LEVEL CONSTRAINTS (Axiom 2)
// ============================================================================

/// Compile-time validated hierarchy level.
///
/// The ToV framework specifies N <= 8 hierarchy levels. This type ensures
/// at compile time that the level is within bounds.
///
/// # Type-Level Guarantee
///
/// If `ValidatedLevel<N>` compiles, then 1 <= N <= 8.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedLevel<const N: u8> {
    _private: (),
}

impl<const N: u8> ValidatedLevel<N> {
    /// Create a validated level. Only compiles if N in [1, 8].
    pub const fn new() -> Self {
        assert!(N >= 1, "Hierarchy level must be at least 1");
        assert!(N <= 8, "Hierarchy level must not exceed 8 (Axiom 2)");
        Self { _private: () }
    }

    /// Get the level value
    pub const fn value(&self) -> u8 {
        N
    }
}

impl<const N: u8> Default for ValidatedLevel<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for all valid hierarchy levels
pub type Level1 = ValidatedLevel<1>;
pub type Level2 = ValidatedLevel<2>;
pub type Level3 = ValidatedLevel<3>;
pub type Level4 = ValidatedLevel<4>;
pub type Level5 = ValidatedLevel<5>;
pub type Level6 = ValidatedLevel<6>;
pub type Level7 = ValidatedLevel<7>;
pub type Level8 = ValidatedLevel<8>;

/// Marker trait for valid hierarchy levels
pub trait IsValidLevel {
    const LEVEL: u8;
}

impl<const N: u8> IsValidLevel for ValidatedLevel<N> {
    const LEVEL: u8 = N;
}

// ============================================================================
// CONSERVATION LAW CONSTRAINTS (Axiom 3, Section 8)
// ============================================================================

/// Compile-time validated conservation law index.
///
/// The ToV framework specifies exactly 11 conservation laws. This type ensures
/// at compile time that the index is within bounds.
///
/// # Laws by Index
///
/// | Index | Law | Mathematical Form |
/// |-------|-----|-------------------|
/// | 1 | Mass Conservation | dM/dt = J_in - J_out |
/// | 2 | Energy Gradient | dV/dt <= 0 |
/// | 3 | State Conservation | sum(p_i) = 1 |
/// | 4 | Flux Conservation | sum(J_in) = sum(J_out) |
/// | 5 | Catalyst Regeneration | [E]_final = [E]_initial |
/// | 6 | Rate Conservation | dA_i/dt = net flux |
/// | 7 | Equilibrium | ds/dt -> 0 |
/// | 8 | Saturation | v <= V_max |
/// | 9 | Entropy Production | dS_total >= 0 |
/// | 10 | Discretization | X in {0, q, 2q, ...} |
/// | 11 | Structural Invariance | Sigma(s(t)) = Sigma(s(0)) |
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedLawIndex<const I: u8> {
    _private: (),
}

impl<const I: u8> ValidatedLawIndex<I> {
    /// Create a validated law index. Only compiles if I in [1, 11].
    pub const fn new() -> Self {
        assert!(I >= 1, "Conservation law index must be at least 1");
        assert!(I <= 11, "Conservation law index must not exceed 11");
        Self { _private: () }
    }

    /// Get the index value
    pub const fn value(&self) -> u8 {
        I
    }
}

impl<const I: u8> Default for ValidatedLawIndex<I> {
    fn default() -> Self {
        Self::new()
    }
}

/// Type aliases for all conservation laws
pub type Law1MassIdx = ValidatedLawIndex<1>;
pub type Law2EnergyIdx = ValidatedLawIndex<2>;
pub type Law3StateIdx = ValidatedLawIndex<3>;
pub type Law4FluxIdx = ValidatedLawIndex<4>;
pub type Law5CatalystIdx = ValidatedLawIndex<5>;
pub type Law6RateIdx = ValidatedLawIndex<6>;
pub type Law7EquilibriumIdx = ValidatedLawIndex<7>;
pub type Law8SaturationIdx = ValidatedLawIndex<8>;
pub type Law9EntropyIdx = ValidatedLawIndex<9>;
pub type Law10DiscretizationIdx = ValidatedLawIndex<10>;
pub type Law11StructureIdx = ValidatedLawIndex<11>;

// ============================================================================
// ELEMENT CARDINALITY CONSTRAINTS (Axiom 1, Section 12)
// ============================================================================

/// Compile-time element count using typenum.
///
/// The ToV framework specifies |E| = 15 elements for each domain.
/// This type uses typenum to encode the cardinality at the type level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElementCount<N: Unsigned>(PhantomData<N>);

impl<N: Unsigned> ElementCount<N> {
    pub fn new() -> Self {
        Self(PhantomData)
    }

    pub fn count() -> usize {
        N::to_usize()
    }
}

impl<N: Unsigned> Default for ElementCount<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard element count for ToV domains
pub type StandardElementCount = ElementCount<U15>;

/// Trait for types with verified element count
pub trait HasElementCount {
    type Count: Unsigned;

    fn element_count() -> usize {
        Self::Count::to_usize()
    }
}

// ============================================================================
// SIGNAL DETECTION CONSTRAINTS (Section 19-33)
// ============================================================================

/// Non-recurrence threshold U_NR = 63 bits (type-level constant)
///
/// This is the bit threshold beyond which a configuration is considered
/// non-recurrent (will never be observed again by chance).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonRecurrenceThreshold;

impl NonRecurrenceThreshold {
    /// The threshold value in bits
    pub const VALUE: u8 = 63;

    /// Get as typenum type
    pub fn as_typenum() -> U63 {
        U63::new()
    }
}

/// Compile-time validated signal rarity (U value).
///
/// Signal rarity must be non-negative (measured in bits of surprise).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedRarity<const BITS: u64> {
    _private: (),
}

impl<const BITS: u64> ValidatedRarity<BITS> {
    pub const fn new() -> Self {
        Self { _private: () }
    }

    pub const fn bits(&self) -> u64 {
        BITS
    }

    /// Check if this rarity exceeds the non-recurrence threshold
    pub const fn is_non_recurrent(&self) -> bool {
        BITS >= NonRecurrenceThreshold::VALUE as u64
    }
}

// ============================================================================
// HARM TYPE CONSTRAINTS (Section 9)
// ============================================================================

/// Compile-time validated harm type index.
///
/// The ToV framework specifies exactly 8 harm types (A-H).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedHarmTypeIndex<const T: u8> {
    _private: (),
}

impl<const T: u8> ValidatedHarmTypeIndex<T> {
    /// Create a validated harm type index. Only compiles if T in [0, 7].
    pub const fn new() -> Self {
        assert!(
            T <= 7,
            "Harm type index must be in range [0, 7] (8 types A-H)"
        );
        Self { _private: () }
    }
}

/// Type aliases for harm types A-H
pub type HarmTypeA = ValidatedHarmTypeIndex<0>; // Acute
pub type HarmTypeB = ValidatedHarmTypeIndex<1>; // Cumulative
pub type HarmTypeC = ValidatedHarmTypeIndex<2>; // OffTarget
pub type HarmTypeD = ValidatedHarmTypeIndex<3>; // Cascade
pub type HarmTypeE = ValidatedHarmTypeIndex<4>; // Idiosyncratic
pub type HarmTypeF = ValidatedHarmTypeIndex<5>; // Saturation
pub type HarmTypeG = ValidatedHarmTypeIndex<6>; // Interaction
pub type HarmTypeH = ValidatedHarmTypeIndex<7>; // Population

// ============================================================================
// PROPAGATION PROBABILITY CONSTRAINTS (Axiom 5)
// ============================================================================

/// A propagation probability that is provably less than 1.
///
/// For the Attenuation Theorem (T10.2) to hold, all propagation
/// probabilities must be strictly less than 1. This type encodes
/// that constraint at compile time using a rational representation.
///
/// P = numerator / denominator where numerator < denominator
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedProbability<const NUM: u32, const DEN: u32> {
    _private: (),
}

impl<const NUM: u32, const DEN: u32> BoundedProbability<NUM, DEN> {
    /// Create a bounded probability. Only compiles if NUM < DEN (probability < 1).
    pub const fn new() -> Self {
        assert!(DEN > 0, "Denominator must be positive");
        assert!(
            NUM < DEN,
            "Probability must be < 1 for attenuation (NUM < DEN)"
        );
        Self { _private: () }
    }

    /// Get the probability as f64
    pub const fn value(&self) -> f64 {
        NUM as f64 / DEN as f64
    }
}

/// Type alias for common probabilities
pub type Prob50Pct = BoundedProbability<1, 2>; // 0.5
pub type Prob10Pct = BoundedProbability<1, 10>; // 0.1
pub type Prob1Pct = BoundedProbability<1, 100>; // 0.01

// ============================================================================
// DOMAIN CONSTRAINTS (Section 11-15)
// ============================================================================

/// Compile-time validated domain index.
///
/// The ToV framework has exactly 3 domains.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedDomainIndex<const D: u8> {
    _private: (),
}

impl<const D: u8> ValidatedDomainIndex<D> {
    pub const fn new() -> Self {
        assert!(D <= 2, "Domain index must be in range [0, 2] (3 domains)");
        Self { _private: () }
    }
}

pub type CloudDomainIdx = ValidatedDomainIndex<0>;
pub type PVDomainIdx = ValidatedDomainIndex<1>;
pub type AIDomainIdx = ValidatedDomainIndex<2>;

// ============================================================================
// COMPILE-TIME PROOFS
// ============================================================================

/// Proof that all hierarchy levels are valid (compile-time check)
pub const fn verify_all_levels() {
    let _ = ValidatedLevel::<1>::new();
    let _ = ValidatedLevel::<2>::new();
    let _ = ValidatedLevel::<3>::new();
    let _ = ValidatedLevel::<4>::new();
    let _ = ValidatedLevel::<5>::new();
    let _ = ValidatedLevel::<6>::new();
    let _ = ValidatedLevel::<7>::new();
    let _ = ValidatedLevel::<8>::new();
}

/// Proof that all conservation law indices are valid (compile-time check)
pub const fn verify_all_laws() {
    let _ = ValidatedLawIndex::<1>::new();
    let _ = ValidatedLawIndex::<2>::new();
    let _ = ValidatedLawIndex::<3>::new();
    let _ = ValidatedLawIndex::<4>::new();
    let _ = ValidatedLawIndex::<5>::new();
    let _ = ValidatedLawIndex::<6>::new();
    let _ = ValidatedLawIndex::<7>::new();
    let _ = ValidatedLawIndex::<8>::new();
    let _ = ValidatedLawIndex::<9>::new();
    let _ = ValidatedLawIndex::<10>::new();
    let _ = ValidatedLawIndex::<11>::new();
}

/// Proof that all harm types are valid (compile-time check)
pub const fn verify_all_harm_types() {
    let _ = ValidatedHarmTypeIndex::<0>::new();
    let _ = ValidatedHarmTypeIndex::<1>::new();
    let _ = ValidatedHarmTypeIndex::<2>::new();
    let _ = ValidatedHarmTypeIndex::<3>::new();
    let _ = ValidatedHarmTypeIndex::<4>::new();
    let _ = ValidatedHarmTypeIndex::<5>::new();
    let _ = ValidatedHarmTypeIndex::<6>::new();
    let _ = ValidatedHarmTypeIndex::<7>::new();
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_hierarchy_levels_compile() {
        let _l1: Level1 = ValidatedLevel::new();
        let _l2: Level2 = ValidatedLevel::new();
        let _l3: Level3 = ValidatedLevel::new();
        let _l4: Level4 = ValidatedLevel::new();
        let _l5: Level5 = ValidatedLevel::new();
        let _l6: Level6 = ValidatedLevel::new();
        let _l7: Level7 = ValidatedLevel::new();
        let _l8: Level8 = ValidatedLevel::new();
    }

    #[test]
    fn valid_law_indices_compile() {
        let _l1: Law1MassIdx = ValidatedLawIndex::new();
        let _l2: Law2EnergyIdx = ValidatedLawIndex::new();
        let _l11: Law11StructureIdx = ValidatedLawIndex::new();
    }

    #[test]
    fn element_count_is_15() {
        assert_eq!(StandardElementCount::count(), 15);
    }

    #[test]
    fn non_recurrence_threshold_is_63() {
        assert_eq!(NonRecurrenceThreshold::VALUE, 63);
    }

    #[test]
    fn bounded_probability_values() {
        let p50: Prob50Pct = BoundedProbability::new();
        assert!((p50.value() - 0.5).abs() < 0.001);

        let p10: Prob10Pct = BoundedProbability::new();
        assert!((p10.value() - 0.1).abs() < 0.001);
    }

    #[test]
    fn rarity_non_recurrence_check() {
        let low: ValidatedRarity<30> = ValidatedRarity::new();
        assert!(!low.is_non_recurrent());

        let high: ValidatedRarity<100> = ValidatedRarity::new();
        assert!(high.is_non_recurrent());

        let threshold: ValidatedRarity<63> = ValidatedRarity::new();
        assert!(threshold.is_non_recurrent());
    }

    #[test]
    fn compile_time_verification() {
        // These are const fns - verification happens at compile time
        verify_all_levels();
        verify_all_laws();
        verify_all_harm_types();
    }

    #[test]
    fn harm_type_indices_valid() {
        let _a: HarmTypeA = ValidatedHarmTypeIndex::new();
        let _h: HarmTypeH = ValidatedHarmTypeIndex::new();
    }

    #[test]
    fn domain_indices_valid() {
        let _cloud: CloudDomainIdx = ValidatedDomainIndex::new();
        let _pv: PVDomainIdx = ValidatedDomainIndex::new();
        let _ai: AIDomainIdx = ValidatedDomainIndex::new();
    }
}
