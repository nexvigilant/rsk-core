//! Anti-Pattern Detection Module
//!
//! Pattern matching and scoring engine for detecting code/process anti-patterns.
//! Replaces the Python `detector.py` with native Rust performance.
//!
//! ## Features
//!
//! - Symptom matching against features (structural, behavioral, textual, metric)
//! - Confidence scoring for pattern matches
//! - Severity assessment based on context
//! - Remediation recommendations
//!
//! ## Example
//!
//! ```rust,ignore
//! use rsk::modules::anti_pattern::{detect_anti_patterns, DetectionConfig};
//! use std::collections::HashMap;
//!
//! let mut features = HashMap::new();
//! features.insert("method_count".to_string(), 25.0);
//! features.insert("line_count".to_string(), 500.0);
//!
//! let mut context = HashMap::new();
//! context.insert("is_critical_path".to_string(), true);
//!
//! let result = detect_anti_patterns(&features, &context, &DetectionConfig::default());
//! println!("Health: {:?}, Detections: {}", result.overall_health, result.detections.len());
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Type of symptom to match
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymptomType {
    Structural, // Code/design structure metrics
    Behavioral, // Process patterns
    Textual,    // Keyword matching
    Metric,     // Numeric thresholds
}

impl Default for SymptomType {
    fn default() -> Self {
        SymptomType::Metric
    }
}

/// Severity of detected anti-pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
    Blocker = 5,
}

impl Severity {
    pub fn from_level(level: u8) -> Self {
        match level {
            1 => Severity::Low,
            2 => Severity::Medium,
            3 => Severity::High,
            4 => Severity::Critical,
            _ => Severity::Blocker,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Severity::Low => "LOW",
            Severity::Medium => "MEDIUM",
            Severity::High => "HIGH",
            Severity::Critical => "CRITICAL",
            Severity::Blocker => "BLOCKER",
        }
    }

    pub fn level(&self) -> u8 {
        *self as u8
    }
}

/// Overall health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverallHealth {
    Clean,
    Ok,
    NeedsAttention,
    Critical,
}

impl OverallHealth {
    pub fn label(&self) -> &'static str {
        match self {
            OverallHealth::Clean => "CLEAN",
            OverallHealth::Ok => "OK",
            OverallHealth::NeedsAttention => "NEEDS_ATTENTION",
            OverallHealth::Critical => "CRITICAL",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SYMPTOM AND PATTERN DEFINITIONS
// ═══════════════════════════════════════════════════════════════════════════

/// A symptom of an anti-pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symptom {
    #[serde(rename = "type", default)]
    pub symptom_type: SymptomType,
    #[serde(default)]
    pub pattern: String,
    #[serde(default)]
    pub description: String,
    pub threshold: Option<f64>,
    #[serde(default = "default_direction")]
    pub direction: String, // "exceeds" or "below"
    #[serde(default)]
    pub keywords: Vec<String>,
    pub metric: Option<String>,
}

fn default_direction() -> String {
    "exceeds".to_string()
}

impl Default for Symptom {
    fn default() -> Self {
        Symptom {
            symptom_type: SymptomType::Metric,
            pattern: String::new(),
            description: String::new(),
            threshold: None,
            direction: "exceeds".to_string(),
            keywords: vec![],
            metric: None,
        }
    }
}

/// An anti-pattern definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiPattern {
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default = "default_base_severity")]
    pub base_severity: u8,
    #[serde(default)]
    pub definition: String,
    #[serde(default)]
    pub symptoms: Vec<Symptom>,
    #[serde(default)]
    pub root_causes: Vec<String>,
    #[serde(default)]
    pub prevention: Vec<String>,
    #[serde(default)]
    pub remediation: Vec<String>,
    #[serde(default)]
    pub related_patterns: Vec<String>,
}

fn default_base_severity() -> u8 {
    2
}

/// Result of matching a symptom
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymptomMatch {
    pub description: String,
    pub pattern: String,
    pub evidence: String,
    pub confidence: f64,
}

/// A detected anti-pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub pattern_name: String,
    pub category: String,
    pub confidence: f64,
    pub match_rate: f64,
    pub symptom_count: String,
    pub severity: String,
    pub symptom_matches: Vec<SymptomMatch>,
    pub root_causes: Vec<String>,
    pub remediation: Vec<String>,
}

/// Complete detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub artifact: String,
    pub categories_checked: Vec<String>,
    pub patterns_checked: usize,
    pub detections_count: usize,
    pub clean: bool,
    pub overall_health: String,
    pub severity_breakdown: HashMap<String, usize>,
    pub detections: Vec<Detection>,
}

/// Configuration for detection
#[derive(Debug, Clone)]
pub struct DetectionConfig {
    pub threshold: f64,
    pub artifact_name: String,
    pub categories: Option<Vec<String>>,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        DetectionConfig {
            threshold: 0.3,
            artifact_name: "artifact".to_string(),
            categories: None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SYMPTOM MATCHING
// ═══════════════════════════════════════════════════════════════════════════

/// Match a structural symptom (code/design metrics)
fn match_structural_symptom(
    symptom: &Symptom,
    features: &HashMap<String, f64>,
) -> Option<SymptomMatch> {
    let metric = symptom.metric.as_ref().unwrap_or(&symptom.pattern);
    let threshold = symptom.threshold?;

    let value = features.get(metric)?;
    let direction = &symptom.direction;

    let (matched, confidence) = if direction == "exceeds" {
        let m = *value > threshold;
        let c = if m {
            (value / threshold).min(1.0)
        } else {
            0.0
        };
        (m, c)
    } else {
        let m = *value < threshold;
        let c = if m {
            (threshold / value.max(0.1)).min(1.0)
        } else {
            0.0
        };
        (m, c)
    };

    if matched {
        Some(SymptomMatch {
            description: symptom.description.clone(),
            pattern: symptom.pattern.clone(),
            evidence: format!("{}={} ({} threshold {})", metric, value, direction, threshold),
            confidence,
        })
    } else {
        None
    }
}

/// Match a behavioral symptom (process patterns)
fn match_behavioral_symptom(
    symptom: &Symptom,
    features: &HashMap<String, f64>,
    context: &HashMap<String, bool>,
) -> Option<SymptomMatch> {
    let pattern = &symptom.pattern;
    let threshold = symptom.threshold.unwrap_or(1.0);

    match pattern.as_str() {
        "spec_without_implementation" => {
            let spec_count = features.get("spec_count").copied().unwrap_or(0.0);
            let impl_count = features.get("impl_count").copied().unwrap_or(0.0);

            if impl_count == 0.0 && spec_count > 0.0 {
                Some(SymptomMatch {
                    description: symptom.description.clone(),
                    pattern: symptom.pattern.clone(),
                    evidence: format!("Spec:Impl ratio = {}:0 (infinite)", spec_count),
                    confidence: (spec_count / threshold / 2.0).min(1.0),
                })
            } else if impl_count > 0.0 {
                let ratio = spec_count / impl_count;
                if ratio > threshold {
                    Some(SymptomMatch {
                        description: symptom.description.clone(),
                        pattern: symptom.pattern.clone(),
                        evidence: format!("Spec:Impl ratio = {:.1}:1", ratio),
                        confidence: (ratio / threshold / 2.0).min(1.0),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }
        "meta_vs_actual" => {
            let meta_count = features.get("meta_count").copied().unwrap_or(0.0);
            let actual_count = features.get("actual_count").copied().unwrap_or(0.0);

            if actual_count == 0.0 && meta_count > 0.0 {
                Some(SymptomMatch {
                    description: symptom.description.clone(),
                    pattern: symptom.pattern.clone(),
                    evidence: format!("Meta:Actual ratio = {}:0", meta_count),
                    confidence: 0.9,
                })
            } else if actual_count > 0.0 {
                let ratio = meta_count / actual_count;
                if ratio > threshold {
                    Some(SymptomMatch {
                        description: symptom.description.clone(),
                        pattern: symptom.pattern.clone(),
                        evidence: format!("Meta:Actual ratio = {:.1}:1", ratio),
                        confidence: (ratio / threshold / 2.0).min(1.0),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }
        "recurrence_detected" => {
            let recurrence_count = features.get("recurrence_count").copied().unwrap_or(0.0);
            if recurrence_count >= threshold {
                Some(SymptomMatch {
                    description: symptom.description.clone(),
                    pattern: symptom.pattern.clone(),
                    evidence: format!("Recurrence count = {}", recurrence_count),
                    confidence: (recurrence_count / threshold).min(1.0),
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Match a textual symptom (keyword matching)
fn match_textual_symptom(
    symptom: &Symptom,
    features: &HashMap<String, String>,
) -> Option<SymptomMatch> {
    if symptom.keywords.is_empty() {
        return None;
    }

    let text_content = features
        .get("text_content")
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let found_keywords: Vec<&String> = symptom
        .keywords
        .iter()
        .filter(|kw| text_content.contains(&kw.to_lowercase()))
        .collect();

    if !found_keywords.is_empty() {
        let match_ratio = found_keywords.len() as f64 / symptom.keywords.len() as f64;
        Some(SymptomMatch {
            description: symptom.description.clone(),
            pattern: symptom.pattern.clone(),
            evidence: format!(
                "Found keywords: {}",
                found_keywords
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            confidence: match_ratio,
        })
    } else {
        None
    }
}

/// Match a metric symptom (numeric thresholds)
fn match_metric_symptom(
    symptom: &Symptom,
    features: &HashMap<String, f64>,
) -> Option<SymptomMatch> {
    let metric = symptom.metric.as_ref().unwrap_or(&symptom.pattern);
    let threshold = symptom.threshold?;
    let value = features.get(metric)?;
    let direction = &symptom.direction;

    let matched = if direction == "exceeds" {
        *value > threshold
    } else {
        *value < threshold
    };

    if matched {
        Some(SymptomMatch {
            description: symptom.description.clone(),
            pattern: symptom.pattern.clone(),
            evidence: format!("{}={} ({} {})", metric, value, direction, threshold),
            confidence: ((value - threshold).abs() / threshold + 0.5).min(1.0),
        })
    } else {
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SEVERITY ASSESSMENT
// ═══════════════════════════════════════════════════════════════════════════

/// Assess severity of a detection based on context
fn assess_severity(
    base_severity: u8,
    match_rate: f64,
    context: &HashMap<String, bool>,
) -> Severity {
    let mut level = base_severity;

    if context.get("is_critical_path").copied().unwrap_or(false) {
        level = level.saturating_add(1).min(5);
    }

    if context.get("is_user_facing").copied().unwrap_or(false) {
        level = level.saturating_add(1).min(5);
    }

    if match_rate > 0.8 {
        level = level.saturating_add(1).min(5);
    }

    Severity::from_level(level)
}

/// Determine overall health from detections
fn determine_overall_health(detections: &[Detection]) -> OverallHealth {
    if detections.is_empty() {
        return OverallHealth::Clean;
    }

    let max_severity = detections
        .iter()
        .filter_map(|d| {
            match d.severity.as_str() {
                "BLOCKER" => Some(5u8),
                "CRITICAL" => Some(4),
                "HIGH" => Some(3),
                "MEDIUM" => Some(2),
                "LOW" => Some(1),
                _ => None,
            }
        })
        .max()
        .unwrap_or(1);

    if max_severity >= 4 {
        OverallHealth::Critical
    } else if max_severity >= 3 {
        OverallHealth::NeedsAttention
    } else {
        OverallHealth::Ok
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MAIN DETECTION FUNCTION
// ═══════════════════════════════════════════════════════════════════════════

/// Features that can be passed to detection
#[derive(Debug, Clone, Default)]
pub struct Features {
    pub numeric: HashMap<String, f64>,
    pub text: HashMap<String, String>,
}

/// Detect anti-patterns in an artifact
pub fn detect_anti_patterns(
    features: &Features,
    context: &HashMap<String, bool>,
    patterns: &[AntiPattern],
    config: &DetectionConfig,
) -> DetectionResult {
    let mut detections = Vec::new();
    let mut categories_checked = std::collections::HashSet::new();

    for pattern in patterns {
        // Filter by category if specified
        if let Some(ref cats) = config.categories {
            if !cats.contains(&pattern.category) {
                continue;
            }
        }

        categories_checked.insert(pattern.category.clone());
        let mut symptom_matches = Vec::new();

        for symptom in &pattern.symptoms {
            let maybe_match = match symptom.symptom_type {
                SymptomType::Structural => {
                    match_structural_symptom(symptom, &features.numeric)
                }
                SymptomType::Behavioral => {
                    match_behavioral_symptom(symptom, &features.numeric, context)
                }
                SymptomType::Textual => match_textual_symptom(symptom, &features.text),
                SymptomType::Metric => match_metric_symptom(symptom, &features.numeric),
            };

            if let Some(m) = maybe_match {
                symptom_matches.push(m);
            }
        }

        if !symptom_matches.is_empty() && !pattern.symptoms.is_empty() {
            let match_rate = symptom_matches.len() as f64 / pattern.symptoms.len() as f64;
            let avg_confidence: f64 =
                symptom_matches.iter().map(|m| m.confidence).sum::<f64>()
                    / symptom_matches.len() as f64;
            let overall_confidence = match_rate * avg_confidence;

            if overall_confidence > config.threshold {
                let severity = assess_severity(pattern.base_severity, match_rate, context);

                detections.push(Detection {
                    pattern_name: pattern.name.clone(),
                    category: pattern.category.clone(),
                    confidence: (overall_confidence * 100.0).round() / 100.0,
                    match_rate: (match_rate * 100.0).round() / 100.0,
                    symptom_count: format!(
                        "{}/{}",
                        symptom_matches.len(),
                        pattern.symptoms.len()
                    ),
                    severity: severity.label().to_string(),
                    symptom_matches,
                    root_causes: pattern.root_causes.clone(),
                    remediation: pattern.remediation.clone(),
                });
            }
        }
    }

    // Sort by confidence
    detections.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Calculate severity breakdown
    let mut severity_breakdown = HashMap::new();
    for d in &detections {
        *severity_breakdown.entry(d.severity.clone()).or_insert(0) += 1;
    }

    let overall_health = determine_overall_health(&detections);

    DetectionResult {
        artifact: config.artifact_name.clone(),
        categories_checked: categories_checked.into_iter().collect(),
        patterns_checked: patterns.len(),
        detections_count: detections.len(),
        clean: detections.is_empty(),
        overall_health: overall_health.label().to_string(),
        severity_breakdown,
        detections,
    }
}

/// Create a common anti-pattern for testing/demo purposes
pub fn create_god_object_pattern() -> AntiPattern {
    AntiPattern {
        name: "God Object".to_string(),
        category: "code".to_string(),
        base_severity: 3,
        definition: "A class that knows too much or does too much".to_string(),
        symptoms: vec![
            Symptom {
                symptom_type: SymptomType::Metric,
                pattern: "method_count".to_string(),
                description: "Class has too many methods".to_string(),
                threshold: Some(20.0),
                direction: "exceeds".to_string(),
                metric: Some("method_count".to_string()),
                ..Default::default()
            },
            Symptom {
                symptom_type: SymptomType::Metric,
                pattern: "line_count".to_string(),
                description: "Class is too large".to_string(),
                threshold: Some(500.0),
                direction: "exceeds".to_string(),
                metric: Some("line_count".to_string()),
                ..Default::default()
            },
            Symptom {
                symptom_type: SymptomType::Metric,
                pattern: "dependency_count".to_string(),
                description: "Class has too many dependencies".to_string(),
                threshold: Some(10.0),
                direction: "exceeds".to_string(),
                metric: Some("dependency_count".to_string()),
                ..Default::default()
            },
        ],
        root_causes: vec![
            "Lack of proper decomposition".to_string(),
            "Feature creep without refactoring".to_string(),
        ],
        prevention: vec!["Apply Single Responsibility Principle".to_string()],
        remediation: vec![
            "Extract classes for distinct responsibilities".to_string(),
            "Apply facade pattern for coordination".to_string(),
        ],
        related_patterns: vec!["Feature Envy".to_string(), "Long Method".to_string()],
    }
}

/// Create a paper constructs anti-pattern
pub fn create_paper_constructs_pattern() -> AntiPattern {
    AntiPattern {
        name: "Paper Constructs".to_string(),
        category: "process".to_string(),
        base_severity: 2,
        definition: "Specifications that never get implemented".to_string(),
        symptoms: vec![Symptom {
            symptom_type: SymptomType::Behavioral,
            pattern: "spec_without_implementation".to_string(),
            description: "Spec to implementation ratio is too high".to_string(),
            threshold: Some(3.0),
            ..Default::default()
        }],
        root_causes: vec![
            "Analysis paralysis".to_string(),
            "Specification obsession".to_string(),
        ],
        prevention: vec!["Implement while specifying".to_string()],
        remediation: vec![
            "Prioritize implementation".to_string(),
            "Use timeboxed spikes".to_string(),
        ],
        related_patterns: vec!["Analysis Paralysis".to_string()],
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_levels() {
        assert_eq!(Severity::Low.level(), 1);
        assert_eq!(Severity::Medium.level(), 2);
        assert_eq!(Severity::High.level(), 3);
        assert_eq!(Severity::Critical.level(), 4);
        assert_eq!(Severity::Blocker.level(), 5);
    }

    #[test]
    fn test_severity_from_level() {
        assert_eq!(Severity::from_level(1), Severity::Low);
        assert_eq!(Severity::from_level(3), Severity::High);
        assert_eq!(Severity::from_level(6), Severity::Blocker); // clamps to max
    }

    #[test]
    fn test_overall_health_clean() {
        let result = determine_overall_health(&[]);
        assert_eq!(result, OverallHealth::Clean);
    }

    #[test]
    fn test_overall_health_critical() {
        let detections = vec![Detection {
            pattern_name: "test".to_string(),
            category: "code".to_string(),
            confidence: 0.9,
            match_rate: 1.0,
            symptom_count: "1/1".to_string(),
            severity: "CRITICAL".to_string(),
            symptom_matches: vec![],
            root_causes: vec![],
            remediation: vec![],
        }];
        assert_eq!(determine_overall_health(&detections), OverallHealth::Critical);
    }

    #[test]
    fn test_detect_god_object() {
        let mut features = Features::default();
        features.numeric.insert("method_count".to_string(), 25.0);
        features.numeric.insert("line_count".to_string(), 600.0);

        let context = HashMap::new();
        let patterns = vec![create_god_object_pattern()];
        let config = DetectionConfig::default();

        let result = detect_anti_patterns(&features, &context, &patterns, &config);

        assert!(!result.clean);
        assert_eq!(result.detections.len(), 1);
        assert_eq!(result.detections[0].pattern_name, "God Object");
    }

    #[test]
    fn test_detect_clean() {
        let mut features = Features::default();
        features.numeric.insert("method_count".to_string(), 5.0);
        features.numeric.insert("line_count".to_string(), 100.0);

        let context = HashMap::new();
        let patterns = vec![create_god_object_pattern()];
        let config = DetectionConfig::default();

        let result = detect_anti_patterns(&features, &context, &patterns, &config);

        assert!(result.clean);
        assert_eq!(result.overall_health, "CLEAN");
    }

    #[test]
    fn test_severity_adjustment() {
        // Base severity 2 + critical path + match_rate > 0.8 = 4
        let mut context = HashMap::new();
        context.insert("is_critical_path".to_string(), true);

        let severity = assess_severity(2, 0.9, &context);
        assert_eq!(severity, Severity::Critical);
    }

    #[test]
    fn test_paper_constructs_detection() {
        let mut features = Features::default();
        features.numeric.insert("spec_count".to_string(), 10.0);
        features.numeric.insert("impl_count".to_string(), 0.0);

        let context = HashMap::new();
        let patterns = vec![create_paper_constructs_pattern()];
        let config = DetectionConfig::default();

        let result = detect_anti_patterns(&features, &context, &patterns, &config);

        assert!(!result.clean);
        assert_eq!(result.detections[0].pattern_name, "Paper Constructs");
    }

    #[test]
    fn test_textual_symptom_matching() {
        let symptom = Symptom {
            symptom_type: SymptomType::Textual,
            pattern: "tech_debt_keywords".to_string(),
            description: "Contains technical debt markers".to_string(),
            keywords: vec![
                "TODO".to_string(),
                "FIXME".to_string(),
                "HACK".to_string(),
            ],
            ..Default::default()
        };

        let mut text_features = HashMap::new();
        text_features.insert(
            "text_content".to_string(),
            "// TODO: Fix this later\n// HACK: Temporary workaround".to_string(),
        );

        let result = match_textual_symptom(&symptom, &text_features);
        assert!(result.is_some());
        let m = result.unwrap();
        assert!(m.evidence.contains("TODO"));
        assert!(m.evidence.contains("HACK"));
    }

    #[test]
    fn test_category_filtering() {
        let patterns = vec![
            create_god_object_pattern(),
            create_paper_constructs_pattern(),
        ];

        let features = Features::default();
        let context = HashMap::new();

        let config = DetectionConfig {
            categories: Some(vec!["process".to_string()]),
            ..Default::default()
        };

        let result = detect_anti_patterns(&features, &context, &patterns, &config);

        // Should only check process category
        assert!(result.categories_checked.contains(&"process".to_string()));
        assert!(!result.categories_checked.contains(&"code".to_string()));
    }
}
