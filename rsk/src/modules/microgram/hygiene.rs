//! Application 3: Vocabulary Hygiene ($crate pattern)
//!
//! Static analysis of field name alignment between chained micrograms.
//! Detects vocabulary mismatches where producer output fields don't overlap
//! with consumer input fields — the microgram equivalent of macro hygiene
//! violations where identifiers from one scope are invisible in another.
//!
//! Unlike `contracts.rs` (which checks ALL pairwise combinations across
//! a directory), this module checks a SPECIFIC ordered chain and reports
//! per-step coverage: what fraction of each consumer's inputs are satisfied
//! by all prior steps' outputs.

use super::Microgram;
use crate::modules::decision_engine::Value;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// A single field gap at a chain boundary
#[derive(Debug, Clone, Serialize)]
pub struct FieldGap {
    /// Index of the consumer step in the chain
    pub consumer_index: usize,
    /// Name of the consumer microgram
    pub consumer_name: String,
    /// The input field the consumer expects but no prior step provides
    pub field: String,
    /// The declared type of the missing field
    pub field_type: String,
    /// Whether the field is declared required
    pub required: bool,
}

/// A vocabulary overlap measurement at one chain boundary
#[derive(Debug, Clone, Serialize)]
pub struct BoundaryReport {
    /// Index of the consumer step
    pub consumer_index: usize,
    /// Name of the consumer microgram
    pub consumer_name: String,
    /// Total input fields the consumer declares
    pub consumer_inputs: usize,
    /// Fields satisfied by prior steps' outputs (direct or via alias)
    pub satisfied: usize,
    /// Fields NOT provided by any prior step
    pub gaps: Vec<FieldGap>,
    /// Coverage ratio: satisfied / consumer_inputs (0.0 to 1.0)
    pub coverage: f64,
}

/// Full hygiene report for an ordered chain
#[derive(Debug, Clone, Serialize)]
pub struct HygieneReport {
    /// Whether all required fields are satisfied at every boundary
    pub clean: bool,
    /// Per-boundary reports
    pub boundaries: Vec<BoundaryReport>,
    /// Total gaps across all boundaries
    pub total_gaps: usize,
    /// Total REQUIRED field gaps (these are the ones that matter for safety)
    pub required_gaps: usize,
    /// Overall coverage ratio across all boundaries
    pub overall_coverage: f64,
}

/// Check vocabulary hygiene for an ordered chain of micrograms.
///
/// For each step N > 0, determines which of its declared input fields
/// are satisfied by the accumulated output fields of steps 0..N-1
/// (plus the initial input). Alias remapping is applied at each boundary,
/// matching the runtime behavior of `apply_aliases()`.
pub fn check_chain_hygiene(
    micrograms: &[Microgram],
    initial_input: &HashMap<String, Value>,
) -> HygieneReport {
    if micrograms.is_empty() {
        return HygieneReport {
            clean: true,
            boundaries: Vec::new(),
            total_gaps: 0,
            required_gaps: 0,
            overall_coverage: 1.0,
        };
    }

    // Track accumulated available fields (what prior steps provide)
    let mut available: HashSet<String> = initial_input.keys().cloned().collect();

    // Add step 0's output fields to available set
    if let Some(ref iface) = micrograms[0].interface {
        for field in iface.outputs.keys() {
            available.insert(field.clone());
        }
    }

    let mut boundaries = Vec::new();
    let mut total_gaps = 0;
    let mut required_gaps = 0;
    let mut total_inputs = 0;
    let mut total_satisfied = 0;

    for (i, mg) in micrograms.iter().enumerate().skip(1) {
        let Some(ref iface) = mg.interface else {
            // No interface declared — can't check hygiene, skip
            continue;
        };

        if iface.inputs.is_empty() {
            continue;
        }

        // Simulate alias resolution matching runtime apply_aliases() exactly:
        // if available has the alias name but NOT the canonical name,
        // the canonical name becomes effectively available.
        // This is one-directional: alias → canonical only.
        let mut effective_available = available.clone();

        for (alias, canonical) in &iface.aliases {
            if available.contains(alias) && !available.contains(canonical) {
                effective_available.insert(canonical.clone());
            }
        }

        let mut gaps = Vec::new();
        let mut satisfied = 0;

        for (field_name, field_spec) in &iface.inputs {
            if effective_available.contains(field_name) {
                satisfied += 1;
            } else {
                let gap = FieldGap {
                    consumer_index: i,
                    consumer_name: mg.name.clone(),
                    field: field_name.clone(),
                    field_type: field_spec.field_type.clone(),
                    required: field_spec.required,
                };
                if field_spec.required {
                    required_gaps += 1;
                }
                gaps.push(gap);
            }
        }

        let consumer_inputs = iface.inputs.len();
        let coverage = if consumer_inputs > 0 {
            #[allow(clippy::as_conversions)] // usize→f64 for coverage ratio
            let cov = satisfied as f64 / consumer_inputs as f64;
            cov
        } else {
            1.0
        };

        total_gaps += gaps.len();
        total_inputs += consumer_inputs;
        total_satisfied += satisfied;

        boundaries.push(BoundaryReport {
            consumer_index: i,
            consumer_name: mg.name.clone(),
            consumer_inputs,
            satisfied,
            gaps,
            coverage,
        });

        // Add this step's outputs to available set for the next step
        for field in iface.outputs.keys() {
            available.insert(field.clone());
        }
    }

    let overall_coverage = if total_inputs > 0 {
        #[allow(clippy::as_conversions)] // usize→f64 for coverage ratio
        let cov = total_satisfied as f64 / total_inputs as f64;
        cov
    } else {
        1.0
    };

    HygieneReport {
        clean: required_gaps == 0,
        boundaries,
        total_gaps,
        required_gaps,
        overall_coverage,
    }
}

/// Load micrograms by name and check hygiene
pub fn check_chain_hygiene_by_names(
    dir: &std::path::Path,
    names: &[&str],
    initial_input: &HashMap<String, Value>,
) -> Result<HygieneReport, String> {
    let all = super::load_all(dir)?;
    let mut ordered = Vec::with_capacity(names.len());

    for name in names {
        match all.iter().find(|mg| mg.name == *name) {
            Some(mg) => ordered.push(mg.clone()),
            None => return Err(format!("Microgram '{}' not found in {}", name, dir.display())),
        }
    }

    Ok(check_chain_hygiene(&ordered, initial_input))
}
