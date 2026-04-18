use super::compose::{input_variables, output_fields};
use super::load_all;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

/// Ecosystem catalog entry
#[derive(Debug, Clone, Serialize)]
pub struct CatalogEntry {
    pub name: String,
    pub description: String,
    pub version: String,
    pub nodes: usize,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    /// Typed inputs (field_name -> type_name) — declared or inferred
    pub typed_inputs: HashMap<String, String>,
    /// Typed outputs (field_name -> type_name) — declared or inferred
    pub typed_outputs: HashMap<String, String>,
    /// Field aliases: alternative name → canonical name
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aliases: HashMap<String, String>,
    /// Whether this entry has a declared interface (vs inferred)
    pub has_interface: bool,
    pub test_count: usize,
    pub tests_pass: bool,
}

/// Full ecosystem catalog
#[derive(Debug, Clone, Serialize)]
pub struct Catalog {
    pub entries: Vec<CatalogEntry>,
    pub total_micrograms: usize,
    pub total_tests: usize,
    pub all_pass: bool,
    /// Reachability matrix: for each pair (A, B), can A's output feed B's input?
    pub connections: Vec<(String, String)>,
}

/// Build a full catalog of the microgram ecosystem
pub fn catalog(dir: &Path) -> Result<Catalog, String> {
    let all = load_all(dir)?;

    let mut entries = Vec::with_capacity(all.len());
    let mut total_tests = 0;
    let mut all_pass = true;

    for mg in &all {
        let typed_in = mg.typed_inputs();
        let typed_out = mg.typed_outputs();
        let inputs: Vec<String> = if mg.interface.is_some() {
            typed_in.keys().cloned().collect()
        } else {
            input_variables(mg)
        };
        let outputs: Vec<String> = if mg.interface.is_some() {
            typed_out.keys().cloned().collect()
        } else {
            output_fields(mg)
        };
        let test_result = mg.test();
        let tests_pass = test_result.failed == 0;
        if !tests_pass {
            all_pass = false;
        }
        total_tests += test_result.total;

        let aliases = mg
            .interface
            .as_ref()
            .map(|iface| iface.aliases.clone())
            .unwrap_or_default();

        entries.push(CatalogEntry {
            name: mg.name.clone(),
            description: mg.description.clone(),
            version: mg.version.clone(),
            nodes: mg.tree.nodes.len(),
            has_interface: mg.interface.is_some(),
            typed_inputs: typed_in,
            typed_outputs: typed_out,
            aliases,
            inputs,
            outputs,
            test_count: test_result.total,
            tests_pass,
        });
    }

    // Build connection graph: A → B if any output of A matches any input of B (with alias resolution)
    let mut connections = Vec::new();
    for a in &entries {
        for b in &entries {
            if a.name == b.name {
                continue;
            }
            let can_feed = a.outputs.iter().any(|o| {
                // Direct match
                if b.inputs.contains(o) {
                    return true;
                }
                // B declares alias: output name → canonical input in B
                if b.aliases
                    .get(o.as_str())
                    .is_some_and(|canonical| b.inputs.iter().any(|i| i == canonical))
                {
                    return true;
                }
                // A declares alias: alias of output matches B's input
                for (alias, canonical) in &a.aliases {
                    if canonical == o && b.inputs.iter().any(|i| i == alias) {
                        return true;
                    }
                }
                false
            });
            if can_feed {
                connections.push((a.name.clone(), b.name.clone()));
            }
        }
    }

    Ok(Catalog {
        total_micrograms: entries.len(),
        total_tests,
        all_pass,
        entries,
        connections,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// ALIAS CHECK — validate alias declarations across the ecosystem
// ═══════════════════════════════════════════════════════════════════════════

/// An alias conflict: same alias maps to different canonicals in different micrograms
#[derive(Debug, Clone, Serialize)]
pub struct AliasConflict {
    pub alias: String,
    pub canonicals: Vec<(String, String)>, // (microgram_name, canonical_name)
}

/// A suggestion for a new alias based on output→input field name similarity
#[derive(Debug, Clone, Serialize)]
pub struct AliasSuggestion {
    pub from: String,   // source microgram
    pub to: String,     // target microgram
    pub source: String, // output field in source
    pub target: String, // input field in target
}

/// Result of alias validation across an ecosystem
#[derive(Debug, Clone, Serialize)]
pub struct AliasCheckResult {
    pub total_aliases: usize,
    pub conflicts: Vec<AliasConflict>,
    pub unused: Vec<(String, String, String)>, // (microgram, alias, canonical)
    pub suggested: Vec<AliasSuggestion>,
}

/// Validate alias declarations across all micrograms in a directory.
/// Checks for conflicts (same alias → different canonicals) and unused aliases,
/// and suggests new aliases for near-miss field name pairs.
pub fn alias_check(dir: &Path) -> Result<AliasCheckResult, String> {
    let all = load_all(dir)?;

    // Collect all aliases: alias_name → [(microgram_name, canonical_name)]
    let mut alias_map: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut total_aliases = 0;
    let mut unused = Vec::new();

    for mg in &all {
        let Some(iface) = &mg.interface else { continue };
        for (alias, canonical) in &iface.aliases {
            total_aliases += 1;
            alias_map
                .entry(alias.clone())
                .or_default()
                .push((mg.name.clone(), canonical.clone()));

            // Check if alias is actually used: canonical should exist in inputs or outputs
            let in_inputs = iface.inputs.contains_key(canonical);
            let in_outputs = iface.outputs.contains_key(canonical);
            if !in_inputs && !in_outputs {
                unused.push((mg.name.clone(), alias.clone(), canonical.clone()));
            }
        }
    }

    // Find conflicts: same alias name maps to different canonical names
    let conflicts: Vec<AliasConflict> = alias_map
        .into_iter()
        .filter(|(_, entries)| {
            if entries.len() <= 1 {
                return false;
            }
            // Conflict if different canonical targets
            let first_canon = &entries[0].1;
            entries.iter().any(|(_, c)| c != first_canon)
        })
        .map(|(alias, canonicals)| AliasConflict { alias, canonicals })
        .collect();

    // Suggest aliases: find output→input pairs across micrograms that don't
    // directly match but are close (share a common substring)
    let mut suggested = Vec::new();
    let entries: Vec<_> = all
        .iter()
        .map(|mg| (mg.name.clone(), output_fields(mg), input_variables(mg)))
        .collect();

    for (a_name, a_outputs, _) in &entries {
        for (b_name, _, b_inputs) in &entries {
            if a_name == b_name {
                continue;
            }
            for out_field in a_outputs {
                for in_field in b_inputs {
                    if out_field == in_field {
                        continue;
                    } // already matches directly
                    if !fields_likely_alias(out_field, in_field) {
                        continue;
                    }
                    suggested.push(AliasSuggestion {
                        from: a_name.clone(),
                        to: b_name.clone(),
                        source: out_field.clone(),
                        target: in_field.clone(),
                    });
                }
            }
        }
    }

    Ok(AliasCheckResult {
        total_aliases,
        conflicts,
        unused,
        suggested,
    })
}

/// Heuristic: are two field names likely aliases for the same concept?
/// Filters out noise by requiring meaningful structural similarity.
fn fields_likely_alias(a: &str, b: &str) -> bool {
    // Skip very short field names — "n" ⊂ "incidence" is not a real signal
    if a.len() < 4 || b.len() < 4 {
        return false;
    }
    // Containment: one name fully contains the other (e.g., valid_icsr contains valid)
    if a.contains(b) || b.contains(a) {
        return true;
    }
    // Normalized comparison: strip common PV prefixes/suffixes
    let normalize = |s: &str| -> String {
        s.replace("_icsr", "")
            .replace("_flag", "")
            .replace("_status", "")
            .replace("_pct", "")
            .replace("is_", "")
            .replace("has_", "")
    };
    let na = normalize(a);
    let nb = normalize(b);
    if !na.is_empty() && !nb.is_empty() && na == nb {
        return true;
    }
    // Levenshtein distance: catch typos and minor variations
    // Only for names of similar length (within 3 chars)
    let len_diff = a.len().abs_diff(b.len());
    if len_diff <= 3 {
        let dist = levenshtein(a, b);
        let max_len = a.len().max(b.len());
        // Normalized distance < 0.3 suggests same concept
        #[allow(clippy::as_conversions)] // usize→f64 for ratio
        let normalized_dist = dist as f64 / max_len as f64;
        if max_len > 0 && normalized_dist < 0.3 {
            return true;
        }
    }
    false
}

/// Simple Levenshtein distance (edit distance between two strings)
fn levenshtein(a: &str, b: &str) -> usize {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let a_len = a_bytes.len();
    let b_len = b_bytes.len();

    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0usize; b_len + 1];

    for i in 1..=a_len {
        curr[0] = i;
        for j in 1..=b_len {
            let cost = if a_bytes[i - 1] == b_bytes[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_len]
}
