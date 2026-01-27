//! Logic Prelude - Core Types for Curry-Howard Proofs
//!
//! This module provides the foundational type definitions that implement
//! the Curry-Howard correspondence, allowing logical propositions to be
//! represented as Rust types and proofs as programs.
//!
//! # The Correspondence
//!
//! | Logic | Rust |
//! |-------|------|
//! | True (top) | `()` |
//! | False (bottom) | `Void` |
//! | P AND Q | `And<P, Q>` |
//! | P OR Q | `Or<P, Q>` |
//! | P -> Q | `fn(P) -> Q` |
//! | NOT P | `fn(P) -> Void` |

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

// ============================================================================
// FALSITY (bottom) - The empty/never type
// ============================================================================

/// Represents logical falsity (bottom).
///
/// `Void` has no constructors, therefore no inhabitants exist.
/// A function returning `Void` can never actually return.
///
/// # Properties
/// - Uninhabited: No value of type `Void` can ever be constructed
/// - Ex falso quodlibet: From `Void`, any proposition follows
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Void {}

impl Void {
    /// Ex falso quodlibet: from falsity, anything follows.
    ///
    /// Since we can never actually have a `Void` value, this function
    /// can claim to return any type - it will never be executed.
    ///
    /// In logic: bottom -> P (for any P)
    #[inline]
    pub fn absurd<T>(self) -> T {
        match self {}
    }
}

// ============================================================================
// TRUTH (top) - The unit type
// ============================================================================

/// Type alias for logical truth.
///
/// The unit type `()` has exactly one inhabitant: `()`.
/// This corresponds to a proposition that is trivially provable.
pub type Truth = ();

/// Construct the trivial proof of truth.
///
/// In logic: top is always provable.
#[inline]
pub const fn trivial() -> Truth {}

// ============================================================================
// CONJUNCTION (AND) - Product types
// ============================================================================

/// Represents logical conjunction (P AND Q).
///
/// A proof of `And<P, Q>` requires both a proof of `P` and a proof of `Q`.
///
/// # Logical Properties
/// - Introduction: P, Q |- P AND Q
/// - Left Elimination: P AND Q |- P
/// - Right Elimination: P AND Q |- Q
/// - Commutativity: P AND Q <-> Q AND P
/// - Associativity: (P AND Q) AND R <-> P AND (Q AND R)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct And<P, Q> {
    pub left: P,
    pub right: Q,
}

impl<P, Q> And<P, Q> {
    /// Conjunction introduction: from P and Q, derive P AND Q.
    ///
    /// In logic: P, Q |- P AND Q
    #[inline]
    pub fn intro(p: P, q: Q) -> Self {
        And { left: p, right: q }
    }

    /// Left elimination: from P AND Q, derive P.
    ///
    /// In logic: P AND Q |- P
    #[inline]
    pub fn elim_left(self) -> P {
        self.left
    }

    /// Right elimination: from P AND Q, derive Q.
    ///
    /// In logic: P AND Q |- Q
    #[inline]
    pub fn elim_right(self) -> Q {
        self.right
    }

    /// Commutativity: P AND Q -> Q AND P.
    #[inline]
    pub fn commute(self) -> And<Q, P> {
        And {
            left: self.right,
            right: self.left,
        }
    }

    /// Map over both components.
    #[inline]
    pub fn bimap<P2, Q2>(self, f: impl FnOnce(P) -> P2, g: impl FnOnce(Q) -> Q2) -> And<P2, Q2> {
        And {
            left: f(self.left),
            right: g(self.right),
        }
    }
}

/// Convert a tuple to And (conjunction).
#[inline]
pub fn and_from_tuple<P, Q>(tuple: (P, Q)) -> And<P, Q> {
    And {
        left: tuple.0,
        right: tuple.1,
    }
}

/// Convert And (conjunction) to a tuple.
#[inline]
pub fn and_to_tuple<P, Q>(and: And<P, Q>) -> (P, Q) {
    (and.left, and.right)
}

// ============================================================================
// DISJUNCTION (OR) - Sum types
// ============================================================================

/// Represents logical disjunction (P OR Q).
///
/// A proof of `Or<P, Q>` requires either a proof of `P` OR a proof of `Q`,
/// together with a tag indicating which alternative is proven.
///
/// # Logical Properties
/// - Left Introduction: P |- P OR Q
/// - Right Introduction: Q |- P OR Q
/// - Elimination: P OR Q, P -> R, Q -> R |- R
/// - Commutativity: P OR Q <-> Q OR P
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Or<P, Q> {
    /// Proof of the left disjunct.
    Left(P),
    /// Proof of the right disjunct.
    Right(Q),
}

impl<P, Q> Or<P, Q> {
    /// Left introduction: from P, derive P OR Q.
    ///
    /// In logic: P |- P OR Q
    #[inline]
    pub fn intro_left(p: P) -> Self {
        Or::Left(p)
    }

    /// Right introduction: from Q, derive P OR Q.
    ///
    /// In logic: Q |- P OR Q
    #[inline]
    pub fn intro_right(q: Q) -> Self {
        Or::Right(q)
    }

    /// Disjunction elimination (case analysis).
    ///
    /// If P -> R and Q -> R, then P OR Q -> R.
    ///
    /// In logic: P OR Q, P -> R, Q -> R |- R
    #[inline]
    pub fn elim<R>(self, left_case: impl FnOnce(P) -> R, right_case: impl FnOnce(Q) -> R) -> R {
        match self {
            Or::Left(p) => left_case(p),
            Or::Right(q) => right_case(q),
        }
    }

    /// Commutativity: P OR Q -> Q OR P.
    #[inline]
    pub fn commute(self) -> Or<Q, P> {
        match self {
            Or::Left(p) => Or::Right(p),
            Or::Right(q) => Or::Left(q),
        }
    }

    /// Map over both alternatives.
    #[inline]
    pub fn bimap<P2, Q2>(self, f: impl FnOnce(P) -> P2, g: impl FnOnce(Q) -> Q2) -> Or<P2, Q2> {
        match self {
            Or::Left(p) => Or::Left(f(p)),
            Or::Right(q) => Or::Right(g(q)),
        }
    }
}

// ============================================================================
// NEGATION (NOT) - Function to Void
// ============================================================================

/// Type alias for logical negation (function pointer form).
///
/// NOT P is defined as P -> bottom (if P, then contradiction).
/// Use this type for parameters; for return types that capture variables,
/// use `impl FnOnce(P) -> Void` instead.
pub type Not<P> = fn(P) -> Void;

/// Double negation introduction: P -> NOT NOT P.
///
/// This is always valid, even in intuitionistic logic.
/// Given a proof of P, we can refute any refutation of P.
///
/// In logic: P |- NOT NOT P
#[inline]
pub fn double_neg_intro<P>(p: P) -> impl FnOnce(Not<P>) -> Void {
    move |not_p: Not<P>| not_p(p)
}

/// Contradiction introduction: P, NOT P -> bottom.
///
/// In logic: P, NOT P |- bottom
#[inline]
pub fn contradiction<P>(p: P, not_p: Not<P>) -> Void {
    not_p(p)
}

/// Ex falso quodlibet (standalone function): bottom -> P.
///
/// From a contradiction, anything follows.
#[inline]
pub fn ex_falso<P>(void: Void) -> P {
    void.absurd()
}

// ============================================================================
// BICONDITIONAL (IFF) - Pair of implications
// ============================================================================

/// Represents logical biconditional (P IFF Q).
///
/// Equivalent to (P -> Q) AND (Q -> P).
pub struct Iff<P, Q> {
    forward: Box<dyn Fn(P) -> Q>,
    backward: Box<dyn Fn(Q) -> P>,
}

impl<P: 'static, Q: 'static> Iff<P, Q> {
    /// Construct a biconditional from both implications.
    pub fn new(forward: impl Fn(P) -> Q + 'static, backward: impl Fn(Q) -> P + 'static) -> Self {
        Iff {
            forward: Box::new(forward),
            backward: Box::new(backward),
        }
    }

    /// Apply the forward implication: P -> Q.
    #[inline]
    pub fn forward(&self, p: P) -> Q {
        (self.forward)(p)
    }

    /// Apply the backward implication: Q -> P.
    #[inline]
    pub fn backward(&self, q: Q) -> P {
        (self.backward)(q)
    }
}

// ============================================================================
// EXISTENTIAL QUANTIFICATION (EXISTS)
// ============================================================================

/// Represents existential quantification (EXISTS x. P(x)).
///
/// A proof requires providing a witness `x` and a proof that `P(x)` holds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exists<Witness, Property> {
    /// The witness that satisfies the property.
    pub witness: Witness,
    /// The proof that the property holds for the witness.
    pub proof: Property,
}

impl<W, P> Exists<W, P> {
    /// Existential introduction: provide a witness and proof.
    ///
    /// In logic: P(a) |- EXISTS x. P(x)
    #[inline]
    pub fn intro(witness: W, proof: P) -> Self {
        Exists { witness, proof }
    }

    /// Existential elimination: use the witness in a context.
    ///
    /// In logic: EXISTS x. P(x), (FORALL x. P(x) -> R) |- R
    #[inline]
    pub fn elim<R>(self, consumer: impl FnOnce(W, P) -> R) -> R {
        consumer(self.witness, self.proof)
    }

    /// Map over the witness.
    #[inline]
    pub fn map_witness<W2>(self, f: impl FnOnce(W) -> W2) -> Exists<W2, P> {
        Exists {
            witness: f(self.witness),
            proof: self.proof,
        }
    }
}

// ============================================================================
// PROOF MARKERS - For propositions without computational content
// ============================================================================

/// A zero-cost proof marker for type-level propositions.
///
/// Use when the proposition has no computational content but you need
/// to carry proof evidence through the type system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proof<P>(PhantomData<P>);

impl<P> Proof<P> {
    /// Create a proof marker.
    ///
    /// Only call this when you have actually established the truth of P
    /// through other means (construction, axiom, etc.).
    #[inline]
    pub const fn qed() -> Self {
        Proof(PhantomData)
    }
}

impl<P> Default for Proof<P> {
    fn default() -> Self {
        Proof::qed()
    }
}

// ============================================================================
// STANDARD INFERENCE RULES
// ============================================================================

/// Modus ponens: P, P -> Q |- Q.
///
/// Given a proof of P and a proof that P implies Q, derive Q.
#[inline]
pub fn modus_ponens<P, Q>(premise: P, implication: impl FnOnce(P) -> Q) -> Q {
    implication(premise)
}

/// Hypothetical syllogism: (P -> Q), (Q -> R) |- (P -> R).
///
/// Chain two implications together.
#[inline]
pub fn hypothetical_syllogism<P, Q, R>(
    pq: impl Fn(P) -> Q,
    qr: impl Fn(Q) -> R,
) -> impl Fn(P) -> R {
    move |p| qr(pq(p))
}

/// Modus tollens: NOT Q, P -> Q |- NOT P.
///
/// If Q is false and P implies Q, then P must be false.
#[inline]
pub fn modus_tollens<P, Q>(
    not_q: Not<Q>,
    p_implies_q: impl FnOnce(P) -> Q,
) -> impl FnOnce(P) -> Void {
    move |p: P| not_q(p_implies_q(p))
}

/// Disjunctive syllogism: P OR Q, NOT P |- Q.
///
/// If P or Q holds and P is false, then Q must be true.
#[inline]
pub fn disjunctive_syllogism<P, Q>(p_or_q: Or<P, Q>, not_p: Not<P>) -> Q {
    match p_or_q {
        Or::Left(p) => not_p(p).absurd(),
        Or::Right(q) => q,
    }
}

/// Contraposition: (P -> Q), NOT Q |- NOT P.
///
/// Given an implication and the negation of its consequent,
/// derive the negation of its antecedent.
#[inline]
pub fn contraposition<P, Q>(
    p_implies_q: impl FnOnce(P) -> Q,
    not_q: Not<Q>,
) -> impl FnOnce(P) -> Void {
    move |p: P| not_q(p_implies_q(p))
}

// ============================================================================
// DE MORGAN'S LAWS (Intuitionistically valid directions only)
// ============================================================================

/// De Morgan: NOT(P OR Q), P |- bottom and NOT(P OR Q), Q |- bottom.
///
/// This direction is intuitionistically valid.
#[inline]
pub fn de_morgan_nor_left<P, Q>(not_p_or_q: Not<Or<P, Q>>, p: P) -> Void {
    not_p_or_q(Or::Left(p))
}

#[inline]
pub fn de_morgan_nor_right<P, Q>(not_p_or_q: Not<Or<P, Q>>, q: Q) -> Void {
    not_p_or_q(Or::Right(q))
}

/// De Morgan: (NOT P AND NOT Q), (P OR Q) |- bottom.
///
/// This direction is intuitionistically valid.
#[inline]
pub fn de_morgan_nor_converse<P, Q>(
    not_p_and_not_q: And<Not<P>, Not<Q>>,
    p_or_q: Or<P, Q>,
) -> Void {
    match p_or_q {
        Or::Left(p) => (not_p_and_not_q.left)(p),
        Or::Right(q) => (not_p_and_not_q.right)(q),
    }
}

// Note: NOT(P AND Q) -> (NOT P OR NOT Q) is NOT intuitionistically valid!

// ============================================================================
// DISTRIBUTIVITY
// ============================================================================

/// Distribute conjunction over disjunction: P AND (Q OR R) -> (P AND Q) OR (P AND R).
#[inline]
pub fn distribute_and_over_or<P: Clone, Q, R>(
    p_and_qr: And<P, Or<Q, R>>,
) -> Or<And<P, Q>, And<P, R>> {
    let p = p_and_qr.left;
    match p_and_qr.right {
        Or::Left(q) => Or::Left(And::intro(p, q)),
        Or::Right(r) => Or::Right(And::intro(p, r)),
    }
}

/// Distribute disjunction over conjunction: P OR (Q AND R) -> (P OR Q) AND (P OR R).
#[inline]
pub fn distribute_or_over_and<P: Clone, Q, R>(
    p_or_qr: Or<P, And<Q, R>>,
) -> And<Or<P, Q>, Or<P, R>> {
    match p_or_qr {
        Or::Left(p) => And::intro(Or::Left(p.clone()), Or::Left(p)),
        Or::Right(qr) => And::intro(Or::Right(qr.left), Or::Right(qr.right)),
    }
}

// ============================================================================
// ADDITIONAL COMBINATORS
// ============================================================================

/// Identity: P -> P.
#[inline]
pub fn identity<P>(p: P) -> P {
    p
}

/// Constant: P -> Q -> P.
#[inline]
pub fn constant<P: Clone, Q>(p: P) -> impl Fn(Q) -> P {
    move |_q| p.clone()
}

/// Composition: (Q -> R) -> (P -> Q) -> (P -> R).
#[inline]
pub fn compose<P, Q, R>(qr: impl Fn(Q) -> R, pq: impl Fn(P) -> Q) -> impl Fn(P) -> R {
    move |p| qr(pq(p))
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_and_operations() {
        let pq = And::intro(1, "hello");
        assert_eq!(pq.elim_left(), 1);
        assert_eq!(pq.elim_right(), "hello");

        let qp = And::intro(1, "hello").commute();
        assert_eq!(qp.left, "hello");
        assert_eq!(qp.right, 1);
    }

    #[test]
    fn test_or_operations() {
        let p_or_q: Or<i32, &str> = Or::intro_left(42);
        assert!(matches!(p_or_q, Or::Left(42)));

        let result = p_or_q.elim(|n| n.to_string(), |s| s.to_string());
        assert_eq!(result, "42");
    }

    #[test]
    fn test_exists_operations() {
        let proof: Exists<u32, ()> = Exists::intro(4, ());
        assert_eq!(proof.witness, 4);

        let result = proof.elim(|w, _| w * 2);
        assert_eq!(result, 8);
    }

    #[test]
    fn test_modus_ponens() {
        let result = modus_ponens(5, |x| x * 2);
        assert_eq!(result, 10);
    }

    #[test]
    fn test_hypothetical_syllogism() {
        let pq = |x: i32| x.to_string();
        let qr = |s: String| s.len();
        let pr = hypothetical_syllogism(pq, qr);
        assert_eq!(pr(123), 3);
    }

    #[test]
    fn test_disjunctive_syllogism() {
        // We can't easily test this without constructing Void,
        // but we can verify the type signature compiles
        fn _type_check() {
            fn _not_p(_: i32) -> Void {
                unreachable!()
            }
            let _: &str = disjunctive_syllogism(Or::<i32, &str>::Right("result"), _not_p);
        }
    }

    #[test]
    fn test_distributivity() {
        let and_or = And::intro(1, Or::<i32, i32>::Left(2));
        let result = distribute_and_over_or(and_or);
        assert!(matches!(result, Or::Left(And { left: 1, right: 2 })));
    }

    #[test]
    fn test_tuple_conversions() {
        let tuple = (1, "hello");
        let and = and_from_tuple(tuple);
        assert_eq!(and.left, 1);
        assert_eq!(and.right, "hello");

        let back = and_to_tuple(and);
        assert_eq!(back, (1, "hello"));
    }
}
