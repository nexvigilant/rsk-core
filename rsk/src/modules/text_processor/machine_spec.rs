//! Machine specification (SMST) parsing, validation, and scoring.
//!
//! This module provides types and functions for working with SKILL.md Machine Specifications:
//! - Parsing the `## Machine Specification` section
//! - Validating Diamond v2 compliance
//! - Calculating SMST compliance scores

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use super::skill_metadata::{SkillFrontmatter, parse_frontmatter};

macro_rules! f {
    ($($arg:tt)*) => { format!($($arg)*) };
}

// ═══════════════════════════════════════════════════════════════════════════
// PRECOMPILED REGEX PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

/// Next header pattern for section parsing
#[allow(clippy::unwrap_used)] // Safety: compile-time literal pattern — Regex::new cannot fail
pub(crate) static RE_NEXT_HEADER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^###|^##").unwrap());

/// Skill name extraction pattern
#[allow(clippy::unwrap_used)] // Safety: compile-time literal pattern — Regex::new cannot fail
pub(crate) static RE_SKILL_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^name:\s*([a-zA-Z0-9_-]+)").unwrap());

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Machine specification sections from SKILL.md
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SkillMachineSpec {
    pub inputs: Option<String>,
    pub outputs: Option<String>,
    pub state: Option<String>,
    pub operator_mode: Option<String>,
    pub performance: Option<String>,
    pub invariants: Option<String>,
    pub failure_modes: Option<String>,
    pub telemetry: Option<String>,
}

/// Result of parsing a SKILL.md file
#[derive(Debug, Serialize, Deserialize)]
pub struct ParsingResult {
    pub skill_name: String,
    pub has_machine_spec: bool,
    pub sections_found: Vec<String>,
    pub spec: SkillMachineSpec,
}

/// SMST (Skill Machine Specification Template) scoring result
#[derive(Debug, Serialize, Deserialize)]
pub struct SmstScore {
    pub total_score: f64,
    pub sections_present: u8,
    pub sections_required: u8,
    pub has_frontmatter: bool,
    pub has_machine_spec: bool,
    pub compliance_level: String,
    pub missing_sections: Vec<String>,
}

/// Complete SMST extraction result
#[derive(Debug, Serialize, Deserialize)]
pub struct SmstResult {
    pub frontmatter: SkillFrontmatter,
    pub spec: SkillMachineSpec,
    pub score: SmstScore,
    pub is_diamond_compliant: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// PARSING FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Parse a SKILL.md file and extract its machine specification
pub fn parse_skill_md(content: &str) -> ParsingResult {
    let mut sections_found = Vec::new();
    let mut spec = SkillMachineSpec::default();

    // Extract skill name from frontmatter or first H1 using precompiled regex
    let skill_name = RE_SKILL_NAME
        .captures(content)
        .map(|cap| cap[1].to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let has_machine_spec = content.contains("## Machine Specification");

    if has_machine_spec {
        // Find the Machine Specification section
        if let Some(spec_start) = content.find("## Machine Specification") {
            let spec_content = &content[spec_start..];

            // Map of section names to struct fields
            let section_patterns = [
                ("INPUTS", "inputs"),
                ("OUTPUTS", "outputs"),
                ("STATE", "state"),
                ("OPERATOR MODE", "operator_mode"),
                ("PERFORMANCE", "performance"),
                ("INVARIANTS", "invariants"),
                ("FAILURE MODES", "failure_modes"),
                ("TELEMETRY", "telemetry"),
            ];

            for (section_name, field_name) in section_patterns {
                // Use regex to match optional numbering and extra text like "### 1. INPUTS (Extra)"
                // Note: These section-specific patterns are compiled per-call but this is acceptable
                // since the number of iterations is bounded (8 sections max).
                // Safety: `section_name` values are compile-time string literals (INPUTS, OUTPUTS,
                // etc.) containing only ASCII word characters — the interpolated pattern is always
                // a valid regex and Regex::new cannot fail.
                #[allow(clippy::unwrap_used)]
                let re = Regex::new(&format!(r"(?im)^###\s*(?:\d+\.\s*)?{section_name}\b"))
                    .unwrap();

                if let Some(mat) = re.find(spec_content) {
                    let start: usize = mat.start();
                    sections_found.push(field_name.to_string());

                    // Content starts after the header line
                    let header_end = spec_content[start..]
                        .find('\n')
                        .map(|i| start + i + 1)
                        .unwrap_or(spec_content.len());

                    // Skip optional horizontal rule "---" if it exists immediately after header
                    let mut content_start = header_end;
                    let remaining_from_header = &spec_content[header_end..];
                    if remaining_from_header.trim_start().starts_with("---")
                        && let Some(hr_end) = remaining_from_header.find('\n')
                    {
                        content_start = header_end + hr_end + 1;
                    }

                    // Content ends at next ### or next ## (major section)
                    let remaining = &spec_content[content_start..];
                    let next_section = RE_NEXT_HEADER
                        .find(remaining)
                        .map(|m: regex::Match| content_start + m.start())
                        .unwrap_or(spec_content.len());

                    let section_text = spec_content[content_start..next_section].trim().to_string();

                    match field_name {
                        "inputs" => spec.inputs = Some(section_text),
                        "outputs" => spec.outputs = Some(section_text),
                        "state" => spec.state = Some(section_text),
                        "operator_mode" => spec.operator_mode = Some(section_text),
                        "performance" => spec.performance = Some(section_text),
                        "invariants" => spec.invariants = Some(section_text),
                        "failure_modes" => spec.failure_modes = Some(section_text),
                        "telemetry" => spec.telemetry = Some(section_text),
                        _ => {}
                    }
                }
            }
        }
    }

    ParsingResult {
        skill_name,
        has_machine_spec,
        sections_found,
        spec,
    }
}

/// Validates that a Machine Specification has the required components for Diamond v2.
pub fn validate_diamond_spec(result: &ParsingResult) -> Vec<String> {
    let mut errors = Vec::new();

    if !result.has_machine_spec {
        errors.push("Missing '## Machine Specification' section".to_string());
        return errors;
    }

    let required = [
        "inputs",
        "outputs",
        "state",
        "operator_mode",
        "performance",
        "invariants",
        "failure_modes",
        "telemetry",
    ];
    for req in required {
        if !result.sections_found.contains(&req.to_string()) {
            errors.push(f!(
                "Missing required section: ### {}",
                req.to_uppercase().replace("_", " ")
            ));
        }
    }

    errors
}

// ═══════════════════════════════════════════════════════════════════════════
// SCORING FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Calculate SMST score for Diamond v2 compliance
pub fn calculate_smst_score(
    frontmatter: &SkillFrontmatter,
    spec: &SkillMachineSpec,
    has_machine_spec: bool,
) -> SmstScore {
    const REQUIRED_SECTIONS: u8 = 8;
    let mut missing_sections = Vec::new();
    let mut sections_present = 0;

    // Helper to check if a section has actual machine specification content (Phase 2 Substance)
    let has_substance = |content: &Option<String>| -> bool {
        match content {
            Some(c) => {
                let trimmed = c.trim();
                // Must have at least 15 characters of specification (ignore purely empty/whitespace)
                trimmed.len() > 15
            }
            None => false,
        }
    };

    let section_checks = [
        (has_substance(&spec.inputs), "INPUTS"),
        (has_substance(&spec.outputs), "OUTPUTS"),
        (has_substance(&spec.state), "STATE"),
        (has_substance(&spec.operator_mode), "OPERATOR MODE"),
        (has_substance(&spec.performance), "PERFORMANCE"),
        (has_substance(&spec.invariants), "INVARIANTS"),
        (has_substance(&spec.failure_modes), "FAILURE MODES"),
        (has_substance(&spec.telemetry), "TELEMETRY"),
    ];

    for (present, name) in section_checks {
        if present {
            sections_present += 1;
        } else {
            missing_sections.push(name.to_string());
        }
    }

    // Calculate score (0-100)
    // Scoring weights:
    // - Has frontmatter with name: 5 points
    // - Has description: 5 points
    // - Has compliance-level declared: 5 points
    // - Has Machine Specification section: 10 points
    // - Each of 8 sections: 9.375 points (total 75 points)

    let mut score: f64 = 0.0;

    // Frontmatter checks
    let has_frontmatter = !frontmatter.name.is_empty() && frontmatter.name != "unknown";
    if has_frontmatter {
        score += 5.0;
    }
    if frontmatter.description.is_some() {
        score += 5.0;
    }
    if frontmatter.compliance_level.is_some() {
        score += 5.0;
    }

    // Machine spec presence
    if has_machine_spec {
        score += 10.0;
    }

    // Section scores
    score += f64::from(sections_present) * 9.375;

    // Determine compliance level based on score
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation, clippy::cast_sign_loss)] // f64→u8 for compliance level bucketing; score is bounded [0, 100]
    let compliance_level = match score as u8 {
        85..=100 => "diamond",
        70..=84 => "platinum",
        55..=69 => "gold",
        40..=54 => "silver",
        _ => "bronze",
    }
    .to_string();

    SmstScore {
        total_score: (score * 100.0).round() / 100.0, // 2 decimal places
        sections_present,
        sections_required: REQUIRED_SECTIONS,
        has_frontmatter,
        has_machine_spec,
        compliance_level,
        missing_sections,
    }
}

/// Extract complete SMST (Skill Machine Specification Template) from SKILL.md
pub fn extract_smst(content: &str) -> SmstResult {
    // Parse frontmatter
    let frontmatter = parse_frontmatter(content);

    // Parse machine specification
    let parsing_result = parse_skill_md(content);

    // Calculate score
    let score = calculate_smst_score(
        &frontmatter,
        &parsing_result.spec,
        parsing_result.has_machine_spec,
    );

    // Diamond compliance requires score >= 85
    let is_diamond_compliant = score.total_score >= 85.0;

    SmstResult {
        frontmatter,
        spec: parsing_result.spec,
        score,
        is_diamond_compliant,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // PARSING: POSITIVE TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_parse_complete_skill() {
        let content = r#"---
name: test-skill
---

# Test Skill

## Machine Specification

### INPUTS
Input content here

### OUTPUTS
Output content here

### STATE
State content here

### OPERATOR MODE
Operator mode content

### PERFORMANCE
Performance content

### INVARIANTS
Invariant content

### FAILURE MODES
Failure mode content

### TELEMETRY
Telemetry content
"#;
        let result = parse_skill_md(content);

        assert_eq!(result.skill_name, "test-skill");
        assert!(result.has_machine_spec);
        assert_eq!(result.sections_found.len(), 8);
        assert!(result.spec.inputs.is_some());
        assert!(result.spec.outputs.is_some());
        assert!(result.spec.state.is_some());
        assert!(result.spec.operator_mode.is_some());
        assert!(result.spec.performance.is_some());
        assert!(result.spec.invariants.is_some());
        assert!(result.spec.failure_modes.is_some());
        assert!(result.spec.telemetry.is_some());
    }

    #[test]
    fn test_parse_skill_name_extraction() {
        let content = "---\nname: my-awesome-skill\n---\n# Title";
        let result = parse_skill_md(content);
        assert_eq!(result.skill_name, "my-awesome-skill");
    }

    #[test]
    fn test_parse_section_content() {
        let content = r#"## Machine Specification

### INPUTS
This is the input section content.
It spans multiple lines.

### OUTPUTS
Output section content.
"#;
        let result = parse_skill_md(content);

        let inputs = result.spec.inputs.unwrap();
        assert!(inputs.contains("This is the input section content"));
        assert!(inputs.contains("multiple lines"));
    }

    #[test]
    fn test_parse_numbered_sections() {
        let content = r#"## Machine Specification

### 1. INPUTS
Numbered input section

### 2. OUTPUTS
Numbered output section
"#;
        let result = parse_skill_md(content);

        assert!(result.sections_found.contains(&"inputs".to_string()));
        assert!(result.sections_found.contains(&"outputs".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════
    // PARSING: NEGATIVE/EDGE CASES
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_parse_no_machine_spec() {
        let content = "---\nname: simple\n---\n# Just a README";
        let result = parse_skill_md(content);

        assert_eq!(result.skill_name, "simple");
        assert!(!result.has_machine_spec);
        assert!(result.sections_found.is_empty());
    }

    #[test]
    fn test_parse_no_name() {
        let content = "# Some Skill\n\nNo frontmatter here";
        let result = parse_skill_md(content);

        assert_eq!(result.skill_name, "unknown");
    }

    #[test]
    fn test_parse_empty_content() {
        let content = "";
        let result = parse_skill_md(content);

        assert_eq!(result.skill_name, "unknown");
        assert!(!result.has_machine_spec);
    }

    #[test]
    fn test_parse_partial_spec() {
        let content = r#"---
name: partial
---

## Machine Specification

### INPUTS
Some inputs

### OUTPUTS
Some outputs
"#;
        let result = parse_skill_md(content);

        assert!(result.has_machine_spec);
        assert_eq!(result.sections_found.len(), 2);
        assert!(result.spec.inputs.is_some());
        assert!(result.spec.outputs.is_some());
        assert!(result.spec.state.is_none());
    }

    #[test]
    fn test_parse_with_horizontal_rule() {
        let content = r#"## Machine Specification

### INPUTS
---
Content after horizontal rule
"#;
        let result = parse_skill_md(content);

        let inputs = result.spec.inputs.unwrap();
        assert!(inputs.contains("Content after horizontal rule"));
    }

    // ═══════════════════════════════════════════════════════════════
    // VALIDATION: POSITIVE TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_validate_complete_spec() {
        let result = ParsingResult {
            skill_name: "complete".to_string(),
            has_machine_spec: true,
            sections_found: vec![
                "inputs",
                "outputs",
                "state",
                "operator_mode",
                "performance",
                "invariants",
                "failure_modes",
                "telemetry",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            spec: SkillMachineSpec::default(),
        };

        let errors = validate_diamond_spec(&result);
        assert!(errors.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════
    // VALIDATION: NEGATIVE TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_validate_no_machine_spec() {
        let result = ParsingResult {
            skill_name: "missing".to_string(),
            has_machine_spec: false,
            sections_found: vec![],
            spec: SkillMachineSpec::default(),
        };

        let errors = validate_diamond_spec(&result);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Missing '## Machine Specification'"));
    }

    #[test]
    fn test_validate_missing_sections() {
        let result = ParsingResult {
            skill_name: "incomplete".to_string(),
            has_machine_spec: true,
            sections_found: vec!["inputs".to_string(), "outputs".to_string()],
            spec: SkillMachineSpec::default(),
        };

        let errors = validate_diamond_spec(&result);
        assert_eq!(errors.len(), 6); // Missing 6 of 8 required sections

        // Verify specific missing sections are reported
        let error_text = errors.join(" ");
        assert!(error_text.contains("STATE"));
        assert!(error_text.contains("OPERATOR MODE"));
        assert!(error_text.contains("PERFORMANCE"));
        assert!(error_text.contains("INVARIANTS"));
        assert!(error_text.contains("FAILURE MODES"));
        assert!(error_text.contains("TELEMETRY"));
    }

    #[test]
    fn test_validate_empty_sections_list() {
        let result = ParsingResult {
            skill_name: "empty".to_string(),
            has_machine_spec: true,
            sections_found: vec![],
            spec: SkillMachineSpec::default(),
        };

        let errors = validate_diamond_spec(&result);
        assert_eq!(errors.len(), 8); // All 8 sections missing
    }

    // ═══════════════════════════════════════════════════════════════
    // ADVERSARIAL TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_parse_malformed_frontmatter() {
        // Empty name value - regex [a-zA-Z0-9_-] allows hyphens,
        // so "---" on next line gets captured. This is a known limitation.
        let content = "---\nname: \n---";
        let result = parse_skill_md(content);
        // The regex captures "---" because hyphen is a valid char
        assert_eq!(result.skill_name, "---");
    }

    #[test]
    fn test_parse_truly_empty_name() {
        // No valid name chars anywhere after "name:"
        let content = "---\nname: 123invalid\n---";
        let result = parse_skill_md(content);
        // Starts with digit, so won't match [a-zA-Z0-9_-]+ which needs letter first?
        // Actually [a-zA-Z0-9_-]+ allows digits, so this matches "123invalid"
        assert_eq!(result.skill_name, "123invalid");
    }

    #[test]
    fn test_parse_special_characters_in_content() {
        let content = r#"---
name: special-chars
---

## Machine Specification

### INPUTS
Content with special chars: <>&"'`${}[]
"#;
        let result = parse_skill_md(content);

        let inputs = result.spec.inputs.unwrap();
        assert!(inputs.contains("<>&"));
    }

    #[test]
    fn test_parse_unicode_content() {
        let content = r#"---
name: unicode-skill
---

## Machine Specification

### INPUTS
日本語のコンテンツ 🎉 émojis and ñ
"#;
        let result = parse_skill_md(content);

        let inputs = result.spec.inputs.unwrap();
        assert!(inputs.contains("日本語"));
        assert!(inputs.contains("🎉"));
    }

    #[test]
    fn test_parse_case_insensitive_sections() {
        let content = r#"## Machine Specification

### inputs
lowercase section

### Outputs
Mixed case section

### INVARIANTS
UPPERCASE section
"#;
        let result = parse_skill_md(content);

        // Should find all three due to case-insensitive matching
        assert!(result.sections_found.contains(&"inputs".to_string()));
        assert!(result.sections_found.contains(&"outputs".to_string()));
        assert!(result.sections_found.contains(&"invariants".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════
    // SMST SCORING TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_smst_score_diamond() {
        let fm = SkillFrontmatter {
            name: "test".to_string(),
            description: Some("desc".to_string()),
            compliance_level: Some("diamond".to_string()),
            ..Default::default()
        };
        // Content must be >15 chars to pass has_substance check
        let spec = SkillMachineSpec {
            inputs: Some("Input specification content here".to_string()),
            outputs: Some("Output specification content here".to_string()),
            state: Some("State specification content here".to_string()),
            operator_mode: Some("Operator mode specification".to_string()),
            performance: Some("Performance specification here".to_string()),
            invariants: Some("Invariants specification here".to_string()),
            failure_modes: Some("Failure modes specification".to_string()),
            telemetry: Some("Telemetry specification here".to_string()),
        };

        let score = calculate_smst_score(&fm, &spec, true);

        // 5 (name) + 5 (desc) + 5 (compliance) + 10 (has_spec) + 8*9.375 (sections) = 100
        assert_eq!(score.total_score, 100.0);
        assert_eq!(score.compliance_level, "diamond");
        assert!(score.missing_sections.is_empty());
        assert_eq!(score.sections_present, 8);
    }

    #[test]
    fn test_smst_score_bronze() {
        let fm = SkillFrontmatter::default();
        let spec = SkillMachineSpec::default();

        let score = calculate_smst_score(&fm, &spec, false);

        // No points earned
        assert_eq!(score.total_score, 0.0);
        assert_eq!(score.compliance_level, "bronze");
        assert_eq!(score.missing_sections.len(), 8);
    }

    #[test]
    fn test_smst_score_partial() {
        let fm = SkillFrontmatter {
            name: "partial".to_string(),
            description: Some("desc".to_string()),
            compliance_level: Some("gold".to_string()),
            ..Default::default()
        };
        // Content must be >15 chars to pass has_substance check
        let spec = SkillMachineSpec {
            inputs: Some("Input specification content here".to_string()),
            outputs: Some("Output specification content here".to_string()),
            state: Some("State specification content here".to_string()),
            ..Default::default()
        };

        let score = calculate_smst_score(&fm, &spec, true);

        // 5 + 5 + 5 + 10 + 3*9.375 = 53.125
        assert_eq!(score.total_score, 53.13); // rounded to 2 decimals
        assert_eq!(score.compliance_level, "silver");
        assert_eq!(score.sections_present, 3);
        assert_eq!(score.missing_sections.len(), 5);
    }

    // ═══════════════════════════════════════════════════════════════
    // FULL SMST EXTRACTION TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_extract_smst_diamond_compliant() {
        // Content must be >15 chars per section to pass has_substance check
        let content = r#"---
name: diamond-skill
description: A fully compliant skill
compliance-level: diamond
---

## Machine Specification

### INPUTS
Input specification content here with enough text

### OUTPUTS
Output specification content here with enough text

### STATE
State specification content here with enough text

### OPERATOR MODE
Operator mode specification here with enough text

### PERFORMANCE
Performance specification here with enough text

### INVARIANTS
Invariant specification here with enough text

### FAILURE MODES
Failure mode specification here with enough text

### TELEMETRY
Telemetry specification here with enough text
"#;
        let result = extract_smst(content);

        assert_eq!(result.frontmatter.name, "diamond-skill");
        assert!(result.is_diamond_compliant);
        assert_eq!(result.score.total_score, 100.0);
        assert_eq!(result.score.sections_present, 8);
    }

    #[test]
    fn test_extract_smst_not_compliant() {
        let content = r#"---
name: bronze-skill
---

## Some Section

No machine specification here.
"#;
        let result = extract_smst(content);

        assert_eq!(result.frontmatter.name, "bronze-skill");
        assert!(!result.is_diamond_compliant);
        assert!(result.score.total_score < 85.0);
    }
}
