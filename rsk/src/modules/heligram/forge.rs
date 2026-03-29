//! # Heligram Forge
//!
//! Domain-aware heligram generation that goes beyond mechanical operator
//! inversion (`promote`). The forge analyzes a microgram's primitive
//! signature and domain to generate contextually appropriate:
//!
//! 1. **Antisense strands** with domain-specific confounders
//! 2. **Confidence-weighted resolution** with calibrated levels
//! 3. **Four-quadrant test coverage** (confirmed/contested/absent/default)
//!
//! ## Forge vs Promote
//!
//! | Aspect | `promote` | `forge` |
//! |--------|-----------|---------|
//! | Antisense | Inverted operators | Domain confounders |
//! | Resolution | Binary 2×2 | Confidence-weighted |
//! | Tests | Promoted from microgram | Four-quadrant generated |
//! | Domain | Agnostic | Signature-aware |

use crate::modules::decision_engine::{DecisionNode, DecisionTree, Operator, Value};
use crate::modules::heligram::{
    Heligram, HelixInterface, HelixParams, HeligramTest, Resolution, ResolutionRule, Strand,
};
use crate::modules::microgram::{Microgram, MicrogramInterface, PrimitiveSignature};
use std::collections::HashMap;

/// Domain classification derived from primitive signature analysis.
#[derive(Debug, Clone, PartialEq)]
enum Domain {
    /// PRR/ROR/IC signal detection (κ-dominant, ∂-heavy)
    SignalDetection,
    /// Naranjo/WHO-UMC causality (→-dominant)
    CausalityAssessment,
    /// ICH E2A seriousness (σ-dominant, ∂-heavy)
    SeriousnessClassification,
    /// Case routing, workflow (σ-dominant, μ-heavy)
    WorkflowRouting,
    /// Flywheel, system health (ς-dominant)
    SystemHealth,
    /// Generic — no recognized domain pattern
    Generic,
}

/// Confounder: a named challenge to a sense strand conclusion.
#[derive(Debug, Clone)]
struct Confounder {
    /// Variable name in the antisense tree
    name: String,
    /// What this confounder tests
    #[allow(dead_code)]
    description: String,
    /// Condition: variable, operator, threshold
    variable: String,
    operator: Operator,
    threshold: Value,
    /// Weight in confidence calculation (0.0–1.0)
    weight: f64,
}

/// Forge a heligram from a microgram with domain-aware antisense generation.
///
/// Unlike `promote` which mechanically inverts operators, `forge` analyzes
/// the microgram's primitive signature and domain to produce contextually
/// appropriate falsification logic.
pub fn forge(mg: &Microgram) -> Result<Heligram, String> {
    // 1. Classify domain from primitive signature
    let domain = classify_domain(mg);

    // 2. Sense strand = original tree verbatim
    let sense = Strand {
        tree: mg.tree.clone(),
    };

    // 3. Generate domain-specific confounders
    let confounders = generate_confounders(&domain, mg);

    // 4. Build antisense tree from confounders
    let (antisense_tree, field_pairs) = build_confounder_tree(&confounders, &mg.tree, &domain);
    let antisense = Strand {
        tree: antisense_tree,
    };

    // 5. Base pairs
    let base_pairs: HashMap<String, String> = field_pairs.into_iter().collect();

    // 6. Confidence-weighted resolution
    let resolution = build_confidence_resolution(&base_pairs, &confounders, &mg.tree);

    // 7. Interface: merge original + confounder inputs
    let interface = build_forge_interface(&mg.interface, &confounders);

    // 8. Four-quadrant test generation
    let tests = generate_quadrant_tests(mg, &confounders, &base_pairs, &domain);

    let name = format!("{}-helix", mg.name);
    let description = format!(
        "Forged from microgram '{}'. Domain: {:?}. {} confounders. Confidence-weighted resolution.",
        mg.name,
        domain,
        confounders.len()
    );

    Ok(Heligram {
        name,
        description,
        version: "0.1.0".to_string(),
        heligram_type: "heligram".to_string(),
        helix: HelixParams {
            twist_rate: twist_rate_for_domain(&domain),
            base_pairs,
        },
        sense,
        antisense,
        resolution,
        interface: Some(interface),
        tests,
        primitive_signature: forge_signature(mg, &domain, &confounders),
    })
}

/// Classify domain from primitive signature and tree variable names.
fn classify_domain(mg: &Microgram) -> Domain {
    let vars = collect_variables(&mg.tree);
    let sig = mg.primitive_signature.as_ref();

    // Check dominant primitive if available
    if let Some(s) = sig {
        match s.dominant.as_str() {
            "κ" | "∂" if has_any(&vars, &["prr", "ror", "ic025", "chi_sq", "ebgm"]) => {
                return Domain::SignalDetection;
            }
            "→" if has_any(&vars, &["naranjo", "causality", "who_umc"]) => {
                return Domain::CausalityAssessment;
            }
            "σ" if has_any(&vars, &["seriousness", "serious", "death", "hospitalization"]) => {
                return Domain::SeriousnessClassification;
            }
            "ς" if has_any(&vars, &["health", "velocity", "momentum", "elastic"]) => {
                return Domain::SystemHealth;
            }
            _ => {}
        }
    }

    // Fallback: variable name heuristics
    if has_any(&vars, &["prr", "ror", "ic025", "chi_sq"]) {
        Domain::SignalDetection
    } else if has_any(&vars, &["naranjo_score", "causality", "dechallenge", "rechallenge"]) {
        Domain::CausalityAssessment
    } else if has_any(&vars, &["seriousness", "death", "hospitalization", "disability"]) {
        Domain::SeriousnessClassification
    } else if has_any(&vars, &["workflow", "route", "action", "case_type"]) {
        Domain::WorkflowRouting
    } else if has_any(&vars, &["health", "velocity", "rim", "momentum"]) {
        Domain::SystemHealth
    } else {
        Domain::Generic
    }
}

/// Generate domain-specific confounders.
fn generate_confounders(domain: &Domain, mg: &Microgram) -> Vec<Confounder> {
    match domain {
        Domain::SignalDetection => vec![
            Confounder {
                name: "insufficient_n".to_string(),
                description: "Too few reports for reliable disproportionality".to_string(),
                variable: "case_count".to_string(),
                operator: Operator::Lt,
                threshold: Value::Int(3),
                weight: 0.4,
            },
            Confounder {
                name: "notoriety_bias".to_string(),
                description: "Media-driven reporting inflation".to_string(),
                variable: "notoriety_bias".to_string(),
                operator: Operator::Eq,
                threshold: Value::Bool(true),
                weight: 0.3,
            },
            Confounder {
                name: "weber_effect".to_string(),
                description: "Reporting declines after initial market years".to_string(),
                variable: "years_on_market".to_string(),
                operator: Operator::Gt,
                threshold: Value::Int(5),
                weight: 0.2,
            },
            Confounder {
                name: "channeling_bias".to_string(),
                description: "Drug prescribed to sicker patients".to_string(),
                variable: "channeling_bias".to_string(),
                operator: Operator::Eq,
                threshold: Value::Bool(true),
                weight: 0.1,
            },
        ],
        Domain::CausalityAssessment => vec![
            Confounder {
                name: "alternative_cause".to_string(),
                description: "Plausible alternative explanation exists".to_string(),
                variable: "alternative_causes".to_string(),
                operator: Operator::Eq,
                threshold: Value::String("likely".to_string()),
                weight: 0.5,
            },
            Confounder {
                name: "temporal_implausible".to_string(),
                description: "Time to onset inconsistent with mechanism".to_string(),
                variable: "temporal_plausible".to_string(),
                operator: Operator::Eq,
                threshold: Value::Bool(false),
                weight: 0.3,
            },
            Confounder {
                name: "no_dechallenge".to_string(),
                description: "No improvement after drug withdrawal".to_string(),
                variable: "dechallenge_positive".to_string(),
                operator: Operator::Eq,
                threshold: Value::Bool(false),
                weight: 0.2,
            },
        ],
        Domain::SeriousnessClassification => vec![
            Confounder {
                name: "incomplete_info".to_string(),
                description: "Missing follow-up or outcome data".to_string(),
                variable: "follow_up_complete".to_string(),
                operator: Operator::Eq,
                threshold: Value::Bool(false),
                weight: 0.4,
            },
            Confounder {
                name: "comorbidity_confound".to_string(),
                description: "Pre-existing condition explains outcome".to_string(),
                variable: "comorbidity_explains".to_string(),
                operator: Operator::Eq,
                threshold: Value::Bool(true),
                weight: 0.4,
            },
            Confounder {
                name: "coding_error".to_string(),
                description: "MedDRA coding may overstate seriousness".to_string(),
                variable: "coding_verified".to_string(),
                operator: Operator::Eq,
                threshold: Value::Bool(false),
                weight: 0.2,
            },
        ],
        Domain::SystemHealth => vec![
            Confounder {
                name: "measurement_stale".to_string(),
                description: "Measurement older than 1 hour".to_string(),
                variable: "measurement_age_minutes".to_string(),
                operator: Operator::Gt,
                threshold: Value::Int(60),
                weight: 0.5,
            },
            Confounder {
                name: "single_source".to_string(),
                description: "Health derived from only one probe".to_string(),
                variable: "probe_count".to_string(),
                operator: Operator::Lt,
                threshold: Value::Int(2),
                weight: 0.5,
            },
        ],
        Domain::WorkflowRouting | Domain::Generic => {
            // Generic: invert the dominant condition as a basic confounder
            let vars = collect_variables(&mg.tree);
            if let Some(first_var) = vars.first() {
                vec![Confounder {
                    name: "negation".to_string(),
                    description: format!("Negation of primary condition on {first_var}"),
                    variable: first_var.clone(),
                    operator: invert_first_operator(&mg.tree),
                    threshold: first_threshold(&mg.tree),
                    weight: 1.0,
                }]
            } else {
                vec![]
            }
        }
    }
}

/// Build an antisense decision tree from confounders.
/// Returns (tree, field_pairs) where field_pairs maps sense→antisense output names.
fn build_confounder_tree(
    confounders: &[Confounder],
    sense_tree: &DecisionTree,
    domain: &Domain,
) -> (DecisionTree, Vec<(String, String)>) {
    let mut nodes = HashMap::new();
    let mut field_pairs = Vec::new();

    // Domain-specific primary field selection for deterministic pairing.
    // Critical: verify the candidate field actually exists in sense outputs.
    let sense_output_fields = collect_output_fields(sense_tree);
    let sense_bool_fields = collect_bool_output_fields(sense_tree);

    let domain_candidate = match domain {
        Domain::SignalDetection => Some("signal_detected"),
        Domain::SeriousnessClassification => Some("is_serious"),
        _ => None,
    };

    let primary_sense = if let Some(candidate) = domain_candidate {
        if sense_output_fields.iter().any(|f| f == candidate) {
            candidate.to_string()
        } else {
            // Domain candidate not in outputs — fall back to heuristic
            sense_bool_fields.first()
                .or(sense_output_fields.first())
                .cloned()
                .unwrap_or_else(|| "result".to_string())
        }
    } else {
        // Causality/Generic: prefer bool fields, else first field
        sense_bool_fields.first()
            .or(sense_output_fields.first())
            .cloned()
            .unwrap_or_else(|| "result".to_string())
    };

    // Chain confounders: each one gates into the next
    for (i, conf) in confounders.iter().enumerate() {
        let node_name = format!("check_{}", conf.name);
        let true_next = "falsified".to_string();
        let false_next = if i + 1 < confounders.len() {
            format!("check_{}", confounders[i + 1].name)
        } else {
            "not_falsified".to_string()
        };

        nodes.insert(
            node_name,
            DecisionNode::Condition {
                variable: conf.variable.clone(),
                operator: conf.operator.clone(),
                value: Some(conf.threshold.clone()),
                true_next,
                false_next,
            },
        );
    }

    // Terminal: falsified
    let mut falsified_output = HashMap::new();
    falsified_output.insert("falsified".to_string(), Value::Bool(true));
    falsified_output.insert("confounder_count".to_string(), Value::Int(1));
    nodes.insert(
        "falsified".to_string(),
        DecisionNode::Return {
            value: Value::Object(falsified_output),
        },
    );

    // Terminal: not falsified
    let mut clean_output = HashMap::new();
    clean_output.insert("falsified".to_string(), Value::Bool(false));
    clean_output.insert("confounder_count".to_string(), Value::Int(0));
    nodes.insert(
        "not_falsified".to_string(),
        DecisionNode::Return {
            value: Value::Object(clean_output),
        },
    );

    let start = if confounders.is_empty() {
        "not_falsified".to_string()
    } else {
        format!("check_{}", confounders[0].name)
    };

    // Pair the primary sense output field with "falsified"
    field_pairs.push((primary_sense, "falsified".to_string()));

    (DecisionTree { start, nodes }, field_pairs)
}

/// Build confidence-weighted resolution rules.
fn build_confidence_resolution(
    base_pairs: &HashMap<String, String>,
    confounders: &[Confounder],
    sense_tree: &DecisionTree,
) -> Resolution {
    let total_weight: f64 = confounders.iter().map(|c| c.weight).sum();
    let max_confidence = if total_weight > 0.0 {
        (1.0 - total_weight * 0.3).max(0.5) // Confidence diminishes with confounder weight
    } else {
        0.9
    };

    let (sense_key, _) = base_pairs.iter().next()
        .map(|(k, v)| (k.clone(), v.clone()))
        .unwrap_or_else(|| ("result".to_string(), "falsified".to_string()));

    let mut rules = Vec::new();

    // Resolution strategy: match on `falsified` (always boolean from antisense),
    // and `sense_key` for boolean outputs. For string outputs, sense polarity
    // is inferred from falsified alone — the sense strand always produces output
    // when given valid input.
    // Check if sense_key is actually boolean in the sense tree's return nodes
    let bool_output_fields = collect_bool_output_fields(sense_tree);
    let sense_is_bool = bool_output_fields.contains(&sense_key);

    if sense_is_bool {
        // Boolean sense: full 2×2 matrix
        // Q1: Sense true, not falsified → confirmed
        let mut when_confirmed = HashMap::new();
        when_confirmed.insert(sense_key.clone(), Value::Bool(true));
        when_confirmed.insert("falsified".to_string(), Value::Bool(false));
        let mut emit_confirmed = HashMap::new();
        emit_confirmed.insert("verdict".to_string(), Value::String("confirmed".to_string()));
        emit_confirmed.insert("confidence".to_string(), Value::String("high".to_string()));
        emit_confirmed.insert("confidence_score".to_string(), Value::Float(max_confidence));
        rules.push(ResolutionRule { when: Some(when_confirmed), default: None, emit: Some(emit_confirmed) });

        // Q2: Sense true, falsified → contested
        let mut when_contested = HashMap::new();
        when_contested.insert(sense_key.clone(), Value::Bool(true));
        when_contested.insert("falsified".to_string(), Value::Bool(true));
        let mut emit_contested = HashMap::new();
        emit_contested.insert("verdict".to_string(), Value::String("contested".to_string()));
        emit_contested.insert("confidence".to_string(), Value::String("low".to_string()));
        emit_contested.insert("confidence_score".to_string(), Value::Float(max_confidence * 0.4));
        rules.push(ResolutionRule { when: Some(when_contested), default: None, emit: Some(emit_contested) });

        // Q3: Sense false, not falsified → absent
        let mut when_absent = HashMap::new();
        when_absent.insert(sense_key.clone(), Value::Bool(false));
        when_absent.insert("falsified".to_string(), Value::Bool(false));
        let mut emit_absent = HashMap::new();
        emit_absent.insert("verdict".to_string(), Value::String("absent".to_string()));
        emit_absent.insert("confidence".to_string(), Value::String("high".to_string()));
        emit_absent.insert("confidence_score".to_string(), Value::Float(max_confidence));
        rules.push(ResolutionRule { when: Some(when_absent), default: None, emit: Some(emit_absent) });
    } else {
        // String/non-boolean sense: match on `falsified` alone.
        // Sense strand always produces output for valid input → presence = positive.
        // Q1: Not falsified → confirmed
        let mut when_confirmed = HashMap::new();
        when_confirmed.insert("falsified".to_string(), Value::Bool(false));
        let mut emit_confirmed = HashMap::new();
        emit_confirmed.insert("verdict".to_string(), Value::String("confirmed".to_string()));
        emit_confirmed.insert("confidence".to_string(), Value::String("high".to_string()));
        emit_confirmed.insert("confidence_score".to_string(), Value::Float(max_confidence));
        rules.push(ResolutionRule { when: Some(when_confirmed), default: None, emit: Some(emit_confirmed) });

        // Q2: Falsified → contested
        let mut when_contested = HashMap::new();
        when_contested.insert("falsified".to_string(), Value::Bool(true));
        let mut emit_contested = HashMap::new();
        emit_contested.insert("verdict".to_string(), Value::String("contested".to_string()));
        emit_contested.insert("confidence".to_string(), Value::String("low".to_string()));
        emit_contested.insert("confidence_score".to_string(), Value::Float(max_confidence * 0.4));
        rules.push(ResolutionRule { when: Some(when_contested), default: None, emit: Some(emit_contested) });
    }

    // Q4: Default fallback
    let mut default_output = HashMap::new();
    default_output.insert("verdict".to_string(), Value::String("indeterminate".to_string()));
    default_output.insert("confidence".to_string(), Value::String("medium".to_string()));
    default_output.insert("confidence_score".to_string(), Value::Float(0.5));
    rules.push(ResolutionRule {
        when: None,
        default: Some(default_output),
        emit: None,
    });

    Resolution {
        mode: "base_pair".to_string(),
        rules,
    }
}

/// Generate four-quadrant test cases.
fn generate_quadrant_tests(
    mg: &Microgram,
    confounders: &[Confounder],
    _base_pairs: &HashMap<String, String>,
    domain: &Domain,
) -> Vec<HeligramTest> {
    let mut tests = Vec::new();

    // Use the first microgram test as a "positive" base case
    let positive_input: HashMap<String, Value> = if let Some(first_test) = mg.tests.first() {
        first_test.input.clone()
    } else {
        HashMap::new()
    };

    // Q1: Confirmed — positive input, no confounders active
    let mut q1_input = positive_input.clone();
    // Ensure confounders are NOT triggered
    for conf in confounders {
        q1_input.insert(conf.variable.clone(), safe_non_triggering_value(&conf.operator, &conf.threshold));
    }
    let mut q1_expect = HashMap::new();
    q1_expect.insert("verdict".to_string(), Value::String("confirmed".to_string()));
    q1_expect.insert("confidence".to_string(), Value::String("high".to_string()));
    tests.push(HeligramTest {
        name: Some("Q1: confirmed — positive sense, no confounders".to_string()),
        input: q1_input,
        expect: q1_expect,
    });

    // Q2: Contested — positive input, first confounder active
    if let Some(first_conf) = confounders.first() {
        let mut q2_input = positive_input.clone();
        q2_input.insert(first_conf.variable.clone(), triggering_value(&first_conf.operator, &first_conf.threshold));
        // Ensure other confounders don't trigger (chain is sequential, first one suffices)
        let mut q2_expect = HashMap::new();
        q2_expect.insert("verdict".to_string(), Value::String("contested".to_string()));
        q2_expect.insert("confidence".to_string(), Value::String("low".to_string()));
        tests.push(HeligramTest {
            name: Some(format!("Q2: contested — positive sense, {} active", first_conf.name)),
            input: q2_input,
            expect: q2_expect,
        });
    }

    // Q3: Absent/default — null/empty input
    // For boolean domains: sense=false, falsified=false → absent
    // For string domains: falsified=false → confirmed (can't distinguish absent)
    let q3_verdict = match domain {
        Domain::SignalDetection | Domain::SeriousnessClassification => "absent",
        _ => "confirmed", // String domains: no falsification = confirmed by default
    };
    let mut q3_expect = HashMap::new();
    q3_expect.insert("verdict".to_string(), Value::String(q3_verdict.to_string()));
    tests.push(HeligramTest {
        name: Some("Q3: default — empty input, no confounders".to_string()),
        input: HashMap::new(),
        expect: q3_expect,
    });

    // Q4: Null safety
    let mut q4_expect = HashMap::new();
    q4_expect.insert("confidence".to_string(), Value::String("high".to_string()));
    tests.push(HeligramTest {
        name: Some("Q4: null safety — graceful degradation".to_string()),
        input: HashMap::new(),
        expect: q4_expect,
    });

    tests
}

/// Build interface merging original microgram interface with confounder inputs.
fn build_forge_interface(
    mg_interface: &Option<MicrogramInterface>,
    confounders: &[Confounder],
) -> HelixInterface {
    use crate::modules::microgram::InterfaceField;

    let mut major_inputs: HashMap<String, InterfaceField> = HashMap::new();
    let mut major_outputs: HashMap<String, InterfaceField> = HashMap::new();

    // Copy original interface inputs/outputs
    if let Some(iface) = mg_interface {
        for (k, v) in &iface.inputs {
            major_inputs.insert(k.clone(), v.clone());
        }
        for (k, v) in &iface.outputs {
            major_outputs.insert(k.clone(), v.clone());
        }
    }

    // Add confounder variables as optional inputs
    for conf in confounders {
        if !major_inputs.contains_key(&conf.variable) {
            major_inputs.insert(conf.variable.clone(), InterfaceField {
                field_type: type_name_for_value(&conf.threshold),
                required: false,
            });
        }
    }

    // Add forged outputs
    major_outputs.insert("verdict".to_string(), InterfaceField {
        field_type: "string".to_string(),
        required: false,
    });
    major_outputs.insert("confidence".to_string(), InterfaceField {
        field_type: "string".to_string(),
        required: false,
    });
    major_outputs.insert("confidence_score".to_string(), InterfaceField {
        field_type: "float".to_string(),
        required: false,
    });

    HelixInterface {
        major_groove: Some(MicrogramInterface {
            inputs: major_inputs,
            outputs: major_outputs,
            aliases: HashMap::new(),
        }),
        minor_groove: None,
    }
}

/// Generate a primitive signature for the forged heligram.
fn forge_signature(
    mg: &Microgram,
    domain: &Domain,
    confounders: &[Confounder],
) -> Option<PrimitiveSignature> {
    let domain_str = match domain {
        Domain::SignalDetection => "signal-detection",
        Domain::CausalityAssessment => "causality-assessment",
        Domain::SeriousnessClassification => "seriousness-classification",
        Domain::WorkflowRouting => "workflow-routing",
        Domain::SystemHealth => "system-health",
        Domain::Generic => "generic",
    };

    Some(PrimitiveSignature {
        dominant: "∂".to_string(),
        expression: format!(
            "∂(×(κ_sense, →_antisense)) — {} with {} confounders",
            domain_str,
            confounders.len()
        ),
        primes: vec![
            "∂".to_string(),
            "κ".to_string(),
            "→".to_string(),
            "ς".to_string(),
            "×".to_string(),
        ],
        arguments: mg
            .primitive_signature
            .as_ref()
            .map(|s| s.arguments.clone())
            .unwrap_or_default(),
        chain_prediction: Some(format!(
            "Forged heligram. Domain: {domain_str}. Chains as dual-strand verifier before downstream consumers."
        )),
    })
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Collect all variable names referenced in condition nodes.
fn collect_variables(tree: &DecisionTree) -> Vec<String> {
    let mut vars = Vec::new();
    for node in tree.nodes.values() {
        if let DecisionNode::Condition { variable, .. } = node
            && !vars.contains(variable)
        {
            vars.push(variable.clone());
        }
    }
    vars
}

/// Collect boolean output field names from return nodes (preferred for pairing).
fn collect_bool_output_fields(tree: &DecisionTree) -> Vec<String> {
    let mut fields = Vec::new();
    for node in tree.nodes.values() {
        if let DecisionNode::Return { value: Value::Object(map) } = node {
            for (key, val) in map {
                if matches!(val, Value::Bool(_)) && !fields.contains(key) {
                    fields.push(key.clone());
                }
            }
        }
    }
    fields
}

/// Collect output field names from return nodes.
fn collect_output_fields(tree: &DecisionTree) -> Vec<String> {
    let mut fields = Vec::new();
    for node in tree.nodes.values() {
        if let DecisionNode::Return { value: Value::Object(map) } = node {
            for key in map.keys() {
                if !fields.contains(key) {
                    fields.push(key.clone());
                }
            }
        }
    }
    fields
}

/// Check if any of the target strings appear in the variable list.
fn has_any(vars: &[String], targets: &[&str]) -> bool {
    vars.iter().any(|v| targets.iter().any(|t| v.contains(t)))
}

/// Get the inverted operator of the first condition node.
fn invert_first_operator(tree: &DecisionTree) -> Operator {
    if let Some(node) = tree.nodes.get(&tree.start)
        && let DecisionNode::Condition { operator, .. } = node
    {
        return invert_operator(operator);
    }
    Operator::Eq
}

/// Get the threshold of the first condition node.
fn first_threshold(tree: &DecisionTree) -> Value {
    if let Some(node) = tree.nodes.get(&tree.start)
        && let DecisionNode::Condition { value, .. } = node
    {
        return match value {
            Some(v) => v.clone(),
            None => Value::Null,
        };
    }
    Value::Null
}

/// Invert an operator.
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
        other => other.clone(),
    }
}

/// Generate a value that DOES trigger the confounder condition.
fn triggering_value(operator: &Operator, threshold: &Value) -> Value {
    match (operator, threshold) {
        (Operator::Lt, Value::Int(n)) => Value::Int(n.saturating_sub(1).max(0)),
        (Operator::Lte, Value::Int(n)) => Value::Int(*n),
        (Operator::Gt, Value::Int(n)) => Value::Int(*n + 1),
        (Operator::Gte, Value::Int(n)) => Value::Int(*n),
        (Operator::Eq, v) => v.clone(),
        // Neq triggers when the value is DIFFERENT from threshold
        (Operator::Neq, Value::Bool(b)) => Value::Bool(!b),
        (Operator::Neq, Value::Int(n)) => Value::Int(*n + 1),
        (Operator::Neq, Value::String(_)) => Value::String("__negated__".to_string()),
        (Operator::Neq, Value::Float(f)) => Value::Float(*f + 1.0),
        _ => threshold.clone(),
    }
}

/// Generate a value that does NOT trigger the confounder condition.
fn safe_non_triggering_value(operator: &Operator, threshold: &Value) -> Value {
    match (operator, threshold) {
        (Operator::Lt, Value::Int(n)) => Value::Int(*n + 10),
        (Operator::Lte, Value::Int(n)) => Value::Int(*n + 10),
        (Operator::Gt, Value::Int(n)) => Value::Int(n.saturating_sub(10).max(0)),
        (Operator::Gte, Value::Int(n)) => Value::Int(n.saturating_sub(10).max(0)),
        (Operator::Eq, Value::Bool(b)) => Value::Bool(!b),
        (Operator::Eq, Value::String(_)) => Value::String("none".to_string()),
        _ => Value::Null,
    }
}

/// Get a type name string for a Value.
fn type_name_for_value(v: &Value) -> String {
    match v {
        Value::Bool(_) => "bool".to_string(),
        Value::Int(_) => "integer".to_string(),
        Value::Float(_) => "float".to_string(),
        Value::String(_) => "string".to_string(),
        _ => "any".to_string(),
    }
}

/// Domain-appropriate twist rate (structural review interval in chains).
fn twist_rate_for_domain(domain: &Domain) -> u32 {
    match domain {
        Domain::SignalDetection => 3,       // Review every 3 chain steps
        Domain::CausalityAssessment => 2,   // Tighter review for causality
        Domain::SeriousnessClassification => 2,
        Domain::WorkflowRouting => 5,       // Looser for routing
        Domain::SystemHealth => 3,
        Domain::Generic => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_prr_signal() -> Microgram {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("micrograms")
            .join("prr-signal.yaml");
        Microgram::load(&path).unwrap_or_else(|e| panic!("Failed to load prr-signal: {e}"))
    }

    fn load_naranjo_quick() -> Microgram {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("micrograms")
            .join("naranjo-quick.yaml");
        Microgram::load(&path).unwrap_or_else(|e| panic!("Failed to load naranjo-quick: {e}"))
    }

    #[test]
    fn test_classify_signal_detection() {
        let mg = load_prr_signal();
        let domain = classify_domain(&mg);
        assert_eq!(domain, Domain::SignalDetection);
    }

    #[test]
    fn test_classify_causality() {
        let mg = load_naranjo_quick();
        let domain = classify_domain(&mg);
        // naranjo-quick uses naranjo_score variable
        assert!(
            domain == Domain::CausalityAssessment || domain == Domain::Generic,
            "Expected CausalityAssessment or Generic, got {domain:?}"
        );
    }

    #[test]
    fn test_forge_prr_signal() {
        let mg = load_prr_signal();
        let heligram = forge(&mg).unwrap_or_else(|e| panic!("Forge failed: {e}"));

        assert_eq!(heligram.name, "prr-signal-helix");
        assert_eq!(heligram.heligram_type, "heligram");
        assert!(!heligram.helix.base_pairs.is_empty());
        assert!(!heligram.resolution.rules.is_empty());
        assert!(!heligram.tests.is_empty());

        // Should have domain-specific confounders
        assert!(
            heligram.description.contains("SignalDetection"),
            "Description should mention domain: {}",
            heligram.description
        );
    }

    #[test]
    fn test_forge_produces_four_quadrant_tests() {
        let mg = load_prr_signal();
        let heligram = forge(&mg).unwrap_or_else(|e| panic!("Forge failed: {e}"));

        // Should have at least 4 tests (one per quadrant)
        assert!(
            heligram.tests.len() >= 4,
            "Expected >= 4 quadrant tests, got {}",
            heligram.tests.len()
        );

        let names: Vec<_> = heligram.tests.iter().filter_map(|t| t.name.as_deref()).collect();
        assert!(names.iter().any(|n| n.contains("Q1")), "Missing Q1 test");
        assert!(names.iter().any(|n| n.contains("Q2")), "Missing Q2 test");
        assert!(names.iter().any(|n| n.contains("Q3")), "Missing Q3 test");
        assert!(names.iter().any(|n| n.contains("Q4")), "Missing Q4 test");
    }

    #[test]
    fn test_forged_heligram_runs() {
        let mg = load_prr_signal();
        let heligram = forge(&mg).unwrap_or_else(|e| panic!("Forge failed: {e}"));

        // Run with a confirmed signal case
        let mut input = HashMap::new();
        input.insert("prr".to_string(), Value::Float(3.5));
        input.insert("case_count".to_string(), Value::Int(50));
        input.insert("notoriety_bias".to_string(), Value::Bool(false));
        input.insert("years_on_market".to_string(), Value::Int(2));
        input.insert("channeling_bias".to_string(), Value::Bool(false));

        let result = heligram.run(input);
        assert!(result.success);

        // Should produce a verdict
        assert!(
            result.resolved_output.contains_key("verdict"),
            "Missing verdict in output: {:?}",
            result.resolved_output
        );
    }

    #[test]
    fn test_forge_confidence_weighting() {
        let mg = load_prr_signal();
        let heligram = forge(&mg).unwrap_or_else(|e| panic!("Forge failed: {e}"));

        // Confirmed case: high confidence
        let mut clean = HashMap::new();
        clean.insert("prr".to_string(), Value::Float(3.5));
        clean.insert("case_count".to_string(), Value::Int(50));
        clean.insert("notoriety_bias".to_string(), Value::Bool(false));
        clean.insert("years_on_market".to_string(), Value::Int(2));
        clean.insert("channeling_bias".to_string(), Value::Bool(false));
        let clean_result = heligram.run(clean);

        // Contested case: low confidence (insufficient n triggers confounder)
        let mut contested = HashMap::new();
        contested.insert("prr".to_string(), Value::Float(3.5));
        contested.insert("case_count".to_string(), Value::Int(2)); // triggers insufficient_n
        let contested_result = heligram.run(contested);

        let clean_conf = clean_result.resolved_output.get("confidence");
        let contested_conf = contested_result.resolved_output.get("confidence");

        assert_eq!(clean_conf, Some(&Value::String("high".to_string())));
        assert_eq!(contested_conf, Some(&Value::String("low".to_string())));
    }
}
