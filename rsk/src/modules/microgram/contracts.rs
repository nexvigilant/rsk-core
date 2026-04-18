use super::load_all;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

/// (name, typed_inputs, typed_outputs, aliases) — used for alias-aware contract validation
type TypedEntry = (
    String,
    HashMap<String, String>,
    HashMap<String, String>,
    HashMap<String, String>,
);

/// A type compatibility violation between connected micrograms
#[derive(Debug, Clone, Serialize)]
pub struct ContractViolation {
    pub from: String,
    pub to: String,
    pub field: String,
    pub from_type: String,
    pub to_type: String,
    pub message: String,
}

/// Result of validating all contracts across an ecosystem
#[derive(Debug, Clone, Serialize)]
pub struct ContractValidation {
    pub total_connections: usize,
    pub valid: usize,
    pub violations: Vec<ContractViolation>,
}

/// Validate type compatibility across all microgram connections in a directory.
/// For each connection (A → B), check that A's output types match B's input types.
/// Alias-aware: resolves field aliases before type comparison.
pub fn validate_contracts(dir: &Path) -> Result<ContractValidation, String> {
    let all = load_all(dir)?;

    // Build typed maps and aliases for each microgram
    let typed: Vec<TypedEntry> = all
        .iter()
        .map(|mg| {
            let aliases = mg
                .interface
                .as_ref()
                .map(|iface| iface.aliases.clone())
                .unwrap_or_default();
            (
                mg.name.clone(),
                mg.typed_inputs(),
                mg.typed_outputs(),
                aliases,
            )
        })
        .collect();

    // For each pair, check connections
    let mut violations = Vec::new();
    let mut total_connections = 0;

    for (a_name, _a_inputs, a_outputs, a_aliases) in &typed {
        for (b_name, b_inputs, _b_outputs, b_aliases) in &typed {
            if a_name == b_name {
                continue;
            }
            // Check if A can feed B: direct match or alias-resolved match
            // Build pairs of (a_output_field, b_input_field) considering aliases
            let b_input_names: Vec<String> = b_inputs.keys().cloned().collect();
            let shared: Vec<(String, String)> = a_outputs
                .keys()
                .filter_map(|out_field| {
                    // Direct match
                    if b_inputs.contains_key(out_field) {
                        return Some((out_field.clone(), out_field.clone()));
                    }
                    // B declares alias: output name → canonical input in B
                    if let Some(canonical) = b_aliases
                        .get(out_field.as_str())
                        .filter(|c| b_inputs.contains_key(c.as_str()))
                    {
                        return Some((out_field.clone(), canonical.clone()));
                    }
                    // A declares alias: alias of output matches B's input
                    for (alias, canonical) in a_aliases {
                        if canonical == out_field && b_input_names.iter().any(|i| i == alias) {
                            return Some((out_field.clone(), alias.clone()));
                        }
                    }
                    None
                })
                .collect();
            if shared.is_empty() {
                continue;
            }
            total_connections += 1;

            // Check type compatibility for each shared field pair
            for (a_field, b_field) in &shared {
                let a_type = &a_outputs[a_field];
                if let Some(b_type) = b_inputs
                    .get(b_field)
                    .filter(|bt| !types_compatible(a_type, bt))
                {
                    violations.push(ContractViolation {
                        from: a_name.clone(),
                        to: b_name.clone(),
                        field: if a_field == b_field {
                            a_field.clone()
                        } else {
                            format!("{a_field} (alias → {b_field})")
                        },
                        from_type: a_type.clone(),
                        to_type: b_type.clone(),
                        message: format!(
                            "{a_name} outputs '{a_field}' as {a_type} but {b_name} expects {b_field} as {b_type}"
                        ),
                    });
                }
            }
        }
    }

    let valid = total_connections - violations.len().min(total_connections);

    Ok(ContractValidation {
        total_connections,
        valid,
        violations,
    })
}

/// Normalize type aliases to canonical forms used by the decision engine.
fn canonical_type(t: &str) -> &str {
    match t {
        "boolean" => "bool",
        "integer" => "int",
        "number" => "float",
        _ => t,
    }
}

/// Check if two types are compatible for chaining.
/// Aliases: boolean ↔ bool, integer ↔ int, number ↔ float.
/// Numeric coercion: int ↔ float. `any` accepts everything.
fn types_compatible(output_type: &str, input_type: &str) -> bool {
    let out = canonical_type(output_type);
    let inp = canonical_type(input_type);
    if out == inp {
        return true;
    }
    // numeric coercion: int ↔ float
    if (out == "int" || out == "float") && (inp == "int" || inp == "float") {
        return true;
    }
    // `any` accepts everything
    if inp == "any" {
        return true;
    }
    // string accepts anything (loose)
    if inp == "string" {
        return true;
    }
    false
}
