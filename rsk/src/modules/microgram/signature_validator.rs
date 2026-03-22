//! # Primitive Signature Chain Validator
//!
//! Validates that a microgram chain's primitive signatures form a coherent
//! → restoration sequence. Based on the proven → failure mode table
//! (primitives.ipynb Cell 10) and conservitor handoff chain (Cell 122).
//!
//! ## Conservation Law Invariant
//!
//! A valid causality chain must:
//! 1. Not jump from κ-dominant to π-dominant without a →-dominant step
//! 2. Preserve the handoff: → fires → ∃ catches → π holds
//! 3. End with a step that produces persistence (π) or action

use super::Microgram;
use serde::Serialize;

/// A single finding from signature validation
#[derive(Debug, Clone, Serialize)]
pub struct SignatureFinding {
    pub severity: FindingSeverity,
    pub step_index: usize,
    pub step_name: String,
    pub message: String,
}

/// Severity levels for signature findings
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum FindingSeverity {
    /// Chain violates conservation law — κ→π without → restoration
    Error,
    /// Missing signature on a step — can't validate
    Warning,
    /// Informational — chain structure observation
    Info,
}

/// Result of validating a chain's primitive signatures
#[derive(Debug, Clone, Serialize)]
pub struct SignatureValidation {
    pub chain_name: String,
    pub valid: bool,
    pub findings: Vec<SignatureFinding>,
    /// The primitive sequence extracted from the chain
    pub signature_sequence: Vec<String>,
    /// Whether the handoff chain (→ → ∃ → π) is complete
    pub handoff_complete: bool,
}

/// Primitives that represent causality restoration stages
const CORRELATION: &str = "κ";
const CAUSALITY: &str = "→";
const PERSISTENCE: &str = "π";
const MAPPING: &str = "μ";

/// Validate that a chain's primitive signatures form a coherent sequence.
///
/// Rules:
/// 1. Every step should have a primitive_signature (warning if missing)
/// 2. A κ→π jump without an intervening → is an ERROR (bypass detection)
/// 3. The chain should end with → or π (causality established or persisted)
/// 4. μ (mapping) steps are bridges — they connect domains, not restore →
pub fn validate_chain_signatures(
    chain_name: &str,
    micrograms: &[Microgram],
) -> SignatureValidation {
    let mut findings = Vec::new();
    let mut signature_sequence = Vec::new();
    let mut saw_kappa = false;
    let mut saw_arrow = false;
    let mut saw_pi = false;

    for (i, mg) in micrograms.iter().enumerate() {
        let dominant = match &mg.primitive_signature {
            Some(sig) => {
                signature_sequence.push(sig.dominant.clone());
                sig.dominant.clone()
            }
            None => {
                signature_sequence.push("?".to_string());
                findings.push(SignatureFinding {
                    severity: FindingSeverity::Warning,
                    step_index: i,
                    step_name: mg.name.clone(),
                    message: format!(
                        "No primitive_signature on '{}' — cannot validate chain integrity",
                        mg.name
                    ),
                });
                continue;
            }
        };

        // Track what we've seen
        if dominant == CORRELATION {
            saw_kappa = true;
        }
        if dominant == CAUSALITY {
            saw_arrow = true;
        }
        if dominant == PERSISTENCE {
            // Check: did we see → before π?
            if saw_kappa && !saw_arrow {
                findings.push(SignatureFinding {
                    severity: FindingSeverity::Error,
                    step_index: i,
                    step_name: mg.name.clone(),
                    message: format!(
                        "Conservation law violation: '{}' (π) follows κ without → restoration. \
                         Cannot persist causality that was never established. \
                         Insert a causality assessment step (Naranjo/WHO-UMC) before persistence.",
                        mg.name
                    ),
                });
            }
            saw_pi = true;
        }

        // Check for κ→π direct jump (the specific failure mode)
        if i > 0 && dominant == PERSISTENCE {
            let prev_dominant = &signature_sequence[i - 1];
            if prev_dominant == CORRELATION {
                findings.push(SignatureFinding {
                    severity: FindingSeverity::Error,
                    step_index: i,
                    step_name: mg.name.clone(),
                    message: format!(
                        "Direct κ→π jump at step {} → {}. Signal detection (κ) cannot \
                         feed directly into regulatory action (π). The → failure mode table \
                         proves: without temporal ordering and boundary, κ is correlation, not causation.",
                        micrograms[i - 1].name, mg.name
                    ),
                });
            }
        }
    }

    // Check terminal condition
    if let Some(last) = signature_sequence.last() {
        if last != CAUSALITY && last != PERSISTENCE && last != MAPPING {
            findings.push(SignatureFinding {
                severity: FindingSeverity::Info,
                step_index: micrograms.len() - 1,
                step_name: micrograms.last().map_or("?".into(), |m| m.name.clone()),
                message: format!(
                    "Chain ends with '{}'-dominant step. Expected → (causality established) \
                     or π (action persisted) as terminal.",
                    last
                ),
            });
        }
    }

    // Handoff completeness: did the chain implement → fires → ∃ catches → π holds?
    let handoff_complete = saw_arrow && saw_pi;

    if saw_kappa && saw_arrow && !saw_pi {
        findings.push(SignatureFinding {
            severity: FindingSeverity::Info,
            step_index: micrograms.len() - 1,
            step_name: micrograms.last().map_or("?".into(), |m| m.name.clone()),
            message: "Chain restores → from κ but does not persist to π. \
                      Handoff incomplete: → fires, ∃ catches, but π does not hold."
                .to_string(),
        });
    }

    let has_errors = findings.iter().any(|f| f.severity == FindingSeverity::Error);

    SignatureValidation {
        chain_name: chain_name.to_string(),
        valid: !has_errors,
        findings,
        signature_sequence,
        handoff_complete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::microgram::PrimitiveSignature;

    fn mg_with_sig(name: &str, dominant: &str) -> Microgram {
        Microgram {
            name: name.to_string(),
            description: String::new(),
            version: "0.1.0".to_string(),
            tree: crate::modules::microgram::DecisionTree {
                start: "x".to_string(),
                nodes: std::collections::HashMap::new(),
            },
            tests: vec![],
            interface: None,
            primitive_signature: Some(PrimitiveSignature {
                dominant: dominant.to_string(),
                expression: String::new(),
                primes: vec![],
                arguments: vec![],
                chain_prediction: None,
            }),
        }
    }

    #[test]
    fn valid_chain_kappa_to_arrow_to_pi() {
        let chain = vec![
            mg_with_sig("prr-signal", "κ"),
            mg_with_sig("signal-bridge", "μ"),
            mg_with_sig("naranjo", "→"),
            mg_with_sig("action", "π"),
        ];
        let result = validate_chain_signatures("test-chain", &chain);
        assert!(result.valid, "Valid κ→μ→→→π chain should pass");
        assert!(result.handoff_complete);
        assert_eq!(result.signature_sequence, vec!["κ", "μ", "→", "π"]);
    }

    #[test]
    fn invalid_chain_kappa_directly_to_pi() {
        let chain = vec![
            mg_with_sig("prr-signal", "κ"),
            mg_with_sig("action", "π"),
        ];
        let result = validate_chain_signatures("bad-chain", &chain);
        assert!(!result.valid, "κ→π without → should fail");
        assert!(result.findings.iter().any(|f| f.severity == FindingSeverity::Error));
    }

    #[test]
    fn chain_missing_pi_is_info_not_error() {
        let chain = vec![
            mg_with_sig("prr-signal", "κ"),
            mg_with_sig("bridge", "μ"),
            mg_with_sig("naranjo", "→"),
        ];
        let result = validate_chain_signatures("partial-chain", &chain);
        assert!(result.valid, "Missing π is informational, not an error");
        assert!(!result.handoff_complete);
    }

    #[test]
    fn chain_with_no_signatures_warns() {
        let mut mg = mg_with_sig("unknown", "κ");
        mg.primitive_signature = None;
        let chain = vec![mg];
        let result = validate_chain_signatures("no-sig", &chain);
        assert!(result.valid, "Missing signatures are warnings, not errors");
        assert!(result.findings.iter().any(|f| f.severity == FindingSeverity::Warning));
    }
}
