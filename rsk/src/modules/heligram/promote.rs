//! # Microgram → Heligram Promotion
//!
//! Converts a single-stranded microgram into a dual-stranded heligram by:
//! 1. Keeping the microgram's tree as the sense strand
//! 2. Auto-generating an antisense strand that inverts each condition
//! 3. Inferring base pairs from sense output field names
//! 4. Generating a 2×2 resolution matrix (confirmed/contested/absent/default)
//!
//! The antisense strand is the *falsifier* — it applies the negated logic
//! to test whether the sense conclusion holds under inverted conditions.

use crate::modules::decision_engine::{DecisionNode, DecisionTree, Operator, Value};
use crate::modules::heligram::{
    Heligram, HeligramTest, HelixInterface, HelixParams, Resolution, ResolutionRule, Strand,
};
use crate::modules::microgram::{Microgram, MicrogramInterface};
use std::collections::HashMap;

/// Promote a microgram to a heligram.
///
/// The sense strand is the original microgram tree.
/// The antisense strand inverts every condition operator.
/// Resolution rules produce a 2×2 matrix from sense/antisense boolean outputs.
pub fn promote(mg: &Microgram) -> Result<Heligram, String> {
    // 1. Sense strand = original tree verbatim
    let sense = Strand {
        tree: mg.tree.clone(),
    };

    // 2. Build antisense by inverting condition operators + output field names
    let (antisense_tree, field_pairs) = build_antisense(&mg.tree)?;
    let antisense = Strand {
        tree: antisense_tree,
    };

    // 3. Base pairs from field_pairs: sense_field → antisense_field
    let base_pairs: HashMap<String, String> = field_pairs.into_iter().collect();

    // 4. Resolution rules: 2×2 matrix for each boolean sense output
    let resolution = build_resolution(&base_pairs);

    // 5. Interface: major groove = original interface with antisense inputs added
    let interface = build_interface(&mg.interface);

    // 6. Tests: promote microgram tests to heligram tests
    //    Original tests become "confirmed" cases (sense=true, antisense=false)
    let tests = promote_tests(&mg.tests, &base_pairs);

    let name = format!("{}-helix", mg.name);
    let description = format!(
        "Auto-promoted from microgram '{}'. Sense: original logic. Antisense: inverted falsifier.",
        mg.name
    );

    Ok(Heligram {
        name,
        description,
        version: "0.1.0".to_string(),
        heligram_type: "heligram".to_string(),
        helix: HelixParams {
            twist_rate: 3,
            base_pairs,
        },
        sense,
        antisense,
        resolution,
        interface: Some(interface),
        tests,
        primitive_signature: mg.primitive_signature.clone(),
    })
}

/// Invert the operator for the antisense strand.
fn invert_operator(op: &Operator) -> Operator {
    match op {
        Operator::Eq => Operator::Neq,
        Operator::Neq => Operator::Eq,
        Operator::Gt => Operator::Lte,
        Operator::Gte => Operator::Lt,
        Operator::Lt => Operator::Gte,
        Operator::Lte => Operator::Gt,
        Operator::Contains => Operator::NotContains,
        Operator::NotContains => Operator::Contains,
        Operator::IsNull => Operator::IsNotNull,
        Operator::IsNotNull => Operator::IsNull,
        Operator::Matches => Operator::Matches, // no clean inverse for regex
    }
}

/// Rename a sense output field to its antisense counterpart.
fn antisense_field_name(sense_name: &str) -> String {
    // Common semantic inversions
    match sense_name {
        "signal_detected" => "falsified".to_string(),
        "classification" => "null_hypothesis".to_string(),
        "detected" => "refuted".to_string(),
        "confirmed" => "denied".to_string(),
        "positive" => "negative".to_string(),
        "pass" => "fail".to_string(),
        "risk" => "risk_refuted".to_string(),
        "causal" => "acausal".to_string(),
        other => format!("{other}_inverted"),
    }
}

/// Build the antisense tree by inverting every condition and renaming return fields.
/// Returns (antisense_tree, Vec<(sense_field, antisense_field)>).
fn build_antisense(
    sense_tree: &DecisionTree,
) -> Result<(DecisionTree, Vec<(String, String)>), String> {
    let mut antisense_nodes: HashMap<String, DecisionNode> = HashMap::new();
    let mut field_pairs: Vec<(String, String)> = Vec::new();
    let mut seen_fields: HashMap<String, String> = HashMap::new();

    for (node_name, node) in &sense_tree.nodes {
        let antisense_name = format!("anti_{node_name}");

        let antisense_node = match node {
            DecisionNode::Condition {
                variable,
                operator,
                value,
                true_next,
                false_next,
            } => {
                // Invert: swap true/false branches AND invert the operator
                // This means: the antisense asks the opposite question
                // and routes to the inverted branch
                DecisionNode::Condition {
                    variable: variable.clone(),
                    operator: invert_operator(operator),
                    value: value.clone(),
                    true_next: format!("anti_{true_next}"),
                    false_next: format!("anti_{false_next}"),
                }
            }
            DecisionNode::Return { value } => {
                // Invert return values: rename fields, flip booleans
                let inverted = invert_return_value(value, &mut seen_fields);

                // Collect field pairs for base_pairs
                if let Value::Object(sense_map) = value
                    && let Value::Object(_anti_map) = &inverted
                {
                    for sense_key in sense_map.keys() {
                        let anti_key = antisense_field_name(sense_key);
                        if !seen_fields.contains_key(sense_key) {
                            seen_fields.insert(sense_key.clone(), anti_key.clone());
                            field_pairs.push((sense_key.clone(), anti_key));
                        }
                    }
                }

                DecisionNode::Return { value: inverted }
            }
            // Action, LlmFallback, Intrinsic — pass through with renamed nexts
            DecisionNode::Action {
                action,
                target,
                value,
                next,
            } => DecisionNode::Action {
                action: action.clone(),
                target: target.clone(),
                value: value.clone(),
                next: next.as_ref().map(|n| format!("anti_{n}")),
            },
            DecisionNode::LlmFallback { prompt, schema } => DecisionNode::LlmFallback {
                prompt: prompt.clone(),
                schema: schema.clone(),
            },
            DecisionNode::Intrinsic {
                function,
                input_variable,
                output_variable,
                next,
            } => DecisionNode::Intrinsic {
                function: function.clone(),
                input_variable: input_variable.clone(),
                output_variable: output_variable.clone(),
                next: format!("anti_{next}"),
            },
        };

        antisense_nodes.insert(antisense_name, antisense_node);
    }

    let antisense_tree = DecisionTree {
        start: format!("anti_{}", sense_tree.start),
        nodes: antisense_nodes,
    };

    Ok((antisense_tree, field_pairs))
}

/// Rename return value fields to antisense names WITHOUT flipping values.
/// The operator inversion already routes to the opposite branch —
/// flipping booleans too would be a double-negation.
fn invert_return_value(value: &Value, seen: &mut HashMap<String, String>) -> Value {
    match value {
        Value::Object(map) => {
            let mut inverted = HashMap::new();
            for (key, val) in map {
                let anti_key = seen
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| antisense_field_name(key));
                // Keep value as-is — the path inversion provides the negation
                inverted.insert(anti_key, val.clone());
            }
            Value::Object(inverted)
        }
        other => other.clone(),
    }
}

/// Build resolution rules as a 2×2 matrix.
/// For each sense boolean output paired with an antisense boolean output:
/// - sense=true, antisense=false → confirmed (high confidence)
/// - sense=true, antisense=true → contested (low confidence)
/// - sense=false, antisense=false → absent (high confidence)
/// - default → absent (medium confidence)
fn build_resolution(base_pairs: &HashMap<String, String>) -> Resolution {
    let mut rules = Vec::new();

    // Find a boolean-typed pair. Prefer fields with "detected", "signal", "pass", "confirmed"
    // as these are most likely boolean. Fall back to first pair.
    let boolean_hints = [
        "detected",
        "signal",
        "falsified",
        "pass",
        "confirmed",
        "causal",
        "positive",
    ];
    let primary_sense = base_pairs
        .keys()
        .find(|k| boolean_hints.iter().any(|h| k.contains(h)))
        .or_else(|| base_pairs.keys().next());

    if let Some(sense_key) = primary_sense {
        let anti_key = &base_pairs[sense_key];

        // Rule 1: sense=true, antisense=false → confirmed
        let mut when1 = HashMap::new();
        when1.insert(sense_key.clone(), Value::Bool(true));
        when1.insert(anti_key.clone(), Value::Bool(false));
        let mut emit1 = HashMap::new();
        emit1.insert(
            "verdict".to_string(),
            Value::String("confirmed".to_string()),
        );
        emit1.insert("confidence".to_string(), Value::String("high".to_string()));
        rules.push(ResolutionRule {
            when: Some(when1),
            default: None,
            emit: Some(emit1),
        });

        // Rule 2: sense=true, antisense=true → contested
        let mut when2 = HashMap::new();
        when2.insert(sense_key.clone(), Value::Bool(true));
        when2.insert(anti_key.clone(), Value::Bool(true));
        let mut emit2 = HashMap::new();
        emit2.insert(
            "verdict".to_string(),
            Value::String("contested".to_string()),
        );
        emit2.insert("confidence".to_string(), Value::String("low".to_string()));
        rules.push(ResolutionRule {
            when: Some(when2),
            default: None,
            emit: Some(emit2),
        });

        // Rule 3: sense=false, antisense=false → absent
        let mut when3 = HashMap::new();
        when3.insert(sense_key.clone(), Value::Bool(false));
        when3.insert(anti_key.clone(), Value::Bool(false));
        let mut emit3 = HashMap::new();
        emit3.insert("verdict".to_string(), Value::String("absent".to_string()));
        emit3.insert("confidence".to_string(), Value::String("high".to_string()));
        rules.push(ResolutionRule {
            when: Some(when3),
            default: None,
            emit: Some(emit3),
        });
    }

    // Default rule
    let mut default_emit = HashMap::new();
    default_emit.insert("verdict".to_string(), Value::String("absent".to_string()));
    default_emit.insert(
        "confidence".to_string(),
        Value::String("medium".to_string()),
    );
    rules.push(ResolutionRule {
        when: None,
        default: Some(default_emit),
        emit: None,
    });

    Resolution {
        mode: "base_pair".to_string(),
        rules,
    }
}

/// Build the heligram interface from the microgram interface.
fn build_interface(mg_iface: &Option<MicrogramInterface>) -> HelixInterface {
    match mg_iface {
        Some(iface) => HelixInterface {
            major_groove: Some(iface.clone()),
            minor_groove: None,
        },
        None => HelixInterface {
            major_groove: None,
            minor_groove: None,
        },
    }
}

/// Promote microgram tests to heligram tests.
/// Microgram tests become "confirmed" scenarios — the sense strand should fire
/// and the antisense should NOT falsify (since we only have sense-side inputs).
fn promote_tests(
    mg_tests: &[crate::modules::microgram::MicrogramTest],
    _base_pairs: &HashMap<String, String>,
) -> Vec<HeligramTest> {
    let mut tests = Vec::new();

    for mg_test in mg_tests {
        // Each microgram test input feeds both strands.
        // Since antisense inverts the same conditions on the same input,
        // the resolved output depends on the 2×2 matrix.
        // We can't predict the exact output without running it,
        // so we carry forward the input and let the user validate.
        tests.push(HeligramTest {
            name: Some(format!(
                "promoted: {}",
                mg_test
                    .expect
                    .iter()
                    .map(|(k, v)| format!("{k}={}", v.as_string()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            input: mg_test.input.clone(),
            expect: mg_test.expect.clone(), // Will need manual adjustment
        });
    }

    // Add null safety test
    tests.push(HeligramTest {
        name: Some("null safety — empty input".to_string()),
        input: HashMap::new(),
        expect: {
            let mut e = HashMap::new();
            e.insert("verdict".to_string(), Value::String("absent".to_string()));
            e
        },
    });

    tests
}

/// Serialize a promoted heligram to YAML string.
pub fn to_yaml(heligram: &Heligram) -> Result<String, String> {
    serde_yaml::to_string(heligram).map_err(|e| format!("YAML serialization error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::microgram::Microgram;
    use std::path::Path;

    #[test]
    fn test_invert_operator_roundtrip() {
        // Double inversion should return to original for all invertible ops
        let ops = vec![
            Operator::Eq,
            Operator::Neq,
            Operator::Gt,
            Operator::Gte,
            Operator::Lt,
            Operator::Lte,
            Operator::Contains,
            Operator::NotContains,
            Operator::IsNull,
            Operator::IsNotNull,
        ];
        for op in ops {
            let inverted = invert_operator(&op);
            let double = invert_operator(&inverted);
            assert_eq!(op, double, "Double inversion failed for {op:?}");
        }
    }

    #[test]
    fn test_promote_prr_signal() {
        let mg_yaml = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("micrograms/prr-signal.yaml"),
        )
        .expect("prr-signal.yaml should exist");
        let mg: Microgram = serde_yaml::from_str(&mg_yaml).expect("should parse microgram");

        let heligram = promote(&mg).expect("promotion should succeed");

        assert_eq!(heligram.name, "prr-signal-helix");
        assert_eq!(heligram.heligram_type, "heligram");
        assert_eq!(heligram.helix.twist_rate, 3);
        assert!(
            !heligram.helix.base_pairs.is_empty(),
            "should have base pairs"
        );
        assert!(
            heligram.helix.base_pairs.contains_key("signal_detected"),
            "should pair signal_detected"
        );
        assert_eq!(
            heligram.helix.base_pairs.get("signal_detected"),
            Some(&"falsified".to_string())
        );

        // Antisense should have inverted nodes
        assert!(
            heligram.antisense.tree.start.starts_with("anti_"),
            "antisense start should be prefixed"
        );
        assert!(
            !heligram.antisense.tree.nodes.is_empty(),
            "antisense should have nodes"
        );

        // Resolution should have 4 rules (3 specific + 1 default)
        assert_eq!(heligram.resolution.rules.len(), 4);

        // Should be runnable
        let mut input = HashMap::new();
        input.insert("prr".to_string(), Value::Float(3.5));
        let result = heligram.run(input);
        assert!(result.success);
    }

    #[test]
    fn test_promote_produces_valid_yaml() {
        let mg_yaml = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("micrograms/prr-signal.yaml"),
        )
        .expect("prr-signal.yaml should exist");
        let mg: Microgram = serde_yaml::from_str(&mg_yaml).expect("should parse microgram");

        let heligram = promote(&mg).expect("promotion should succeed");
        let yaml = to_yaml(&heligram).expect("should serialize to YAML");

        // Round-trip: parse the YAML back
        let reparsed = Heligram::parse(&yaml).expect("promoted YAML should re-parse");
        assert_eq!(reparsed.name, heligram.name);
        assert_eq!(
            reparsed.helix.base_pairs.len(),
            heligram.helix.base_pairs.len()
        );
    }

    #[test]
    fn test_antisense_field_naming() {
        assert_eq!(antisense_field_name("signal_detected"), "falsified");
        assert_eq!(antisense_field_name("classification"), "null_hypothesis");
        assert_eq!(
            antisense_field_name("custom_field"),
            "custom_field_inverted"
        );
    }
}
