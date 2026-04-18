//! Signal Detection for Guardian-AV
//!
//! Implements signal detection and pattern analysis from IAIRs,
//! using ToV signal equation S = U × R × T.

use super::iair::{IAIR, IncidentCategory};
use crate::tov::{ACACase, case_propagation_factor};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// SIGNAL AGGREGATION
// ============================================================================

/// Signal strength for a pattern
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signal {
    /// Signal identifier
    pub id: String,
    /// Category being tracked
    pub category: IncidentCategory,
    /// Domain if domain-specific
    pub domain: Option<String>,
    /// Number of incidents in window
    pub incident_count: usize,
    /// Total severity score
    pub total_severity: f64,
    /// Average context risk score
    pub avg_context_risk: f64,
    /// Time window (days)
    pub window_days: u32,
    /// Signal strength (computed)
    pub signal_strength: f64,
    /// Whether this is an actionable signal
    pub actionable: bool,
    /// Trend direction
    pub trend: SignalTrend,
}

/// Signal trend direction
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalTrend {
    Increasing,
    Stable,
    Decreasing,
    Insufficient,
}

/// Signal detector for analyzing IAIR patterns
#[derive(Clone, Debug)]
pub struct SignalDetector {
    /// Minimum incidents to form a signal
    pub min_incidents: usize,
    /// Default time window in days
    pub default_window_days: u32,
    /// Signal strength threshold for actionable
    pub actionable_threshold: f64,
}

impl Default for SignalDetector {
    fn default() -> Self {
        Self {
            min_incidents: 3,
            default_window_days: 30,
            actionable_threshold: 0.5,
        }
    }
}

impl SignalDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect signals from a collection of IAIRs
    pub fn detect_signals(&self, iairs: &[IAIR], window_days: Option<u32>) -> Vec<Signal> {
        let window = window_days.unwrap_or(self.default_window_days);
        let cutoff = Utc::now() - Duration::days(i64::from(window));

        // Filter to recent incidents
        let recent: Vec<_> = iairs
            .iter()
            .filter(|i| i.block_a.incident_timestamp > cutoff)
            .collect();

        // Group by category
        let mut by_category: HashMap<IncidentCategory, Vec<&IAIR>> = HashMap::new();
        for iair in &recent {
            by_category
                .entry(iair.block_d.incident_category)
                .or_default()
                .push(iair);
        }

        // Generate signals for each category
        let mut signals = Vec::new();
        for (category, incidents) in by_category {
            if incidents.len() >= self.min_incidents {
                let signal = self.compute_signal(category, None, &incidents, window);
                signals.push(signal);
            }
        }

        // Also check for domain-specific signals
        let mut by_domain: HashMap<(IncidentCategory, String), Vec<&IAIR>> = HashMap::new();
        for iair in &recent {
            let key = (iair.block_d.incident_category, iair.block_c.domain.clone());
            by_domain.entry(key).or_default().push(iair);
        }

        for ((category, domain), incidents) in by_domain {
            if incidents.len() >= self.min_incidents {
                let signal = self.compute_signal(category, Some(domain), &incidents, window);
                if signal.signal_strength
                    > signals
                        .iter()
                        .find(|s| s.category == category && s.domain.is_none())
                        .map(|s| s.signal_strength)
                        .unwrap_or(0.0)
                {
                    signals.push(signal);
                }
            }
        }

        // Sort by signal strength
        signals.sort_by(|a, b| {
            b.signal_strength
                .partial_cmp(&a.signal_strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        signals
    }

    fn compute_signal(
        &self,
        category: IncidentCategory,
        domain: Option<String>,
        incidents: &[&IAIR],
        window_days: u32,
    ) -> Signal {
        let count = incidents.len();
        let total_severity: f64 = incidents.iter().map(|i| i.block_e.severity).sum();
        #[allow(clippy::as_conversions)] // usize→f64 precision loss acceptable for incident counts
        let count_f = count as f64;
        let avg_context_risk: f64 = incidents
            .iter()
            .map(|i| i.block_g.context_risk_score)
            .sum::<f64>()
            / count_f;

        // Signal strength formula: count * avg_severity * avg_risk / window_normalization
        let avg_severity = total_severity / count_f;
        let signal_strength =
            (count_f * avg_severity * avg_context_risk) / (f64::from(window_days) / 30.0);

        let actionable = signal_strength >= self.actionable_threshold;

        Signal {
            id: format!(
                "SIG-{}-{}",
                category.code(),
                domain.as_deref().unwrap_or("ALL")
            ),
            category,
            domain,
            incident_count: count,
            total_severity,
            avg_context_risk,
            window_days,
            signal_strength,
            actionable,
            trend: SignalTrend::Insufficient, // Would need historical data to compute
        }
    }

    /// Check for drift indicators
    pub fn detect_drift(&self, iairs: &[IAIR], baseline_period_days: u32) -> DriftAnalysis {
        let now = Utc::now();
        let recent_cutoff = now - Duration::days(i64::from(baseline_period_days) / 2);
        let baseline_cutoff = now - Duration::days(i64::from(baseline_period_days));

        let recent: Vec<_> = iairs
            .iter()
            .filter(|i| i.block_a.incident_timestamp > recent_cutoff)
            .collect();

        let baseline: Vec<_> = iairs
            .iter()
            .filter(|i| {
                i.block_a.incident_timestamp > baseline_cutoff
                    && i.block_a.incident_timestamp <= recent_cutoff
            })
            .collect();

        let period_f = f64::from(baseline_period_days);
        #[allow(clippy::as_conversions)] // usize→f64 precision loss acceptable for incident counts
        let recent_rate = recent.len() as f64 / (period_f / 2.0 / 30.0);
        #[allow(clippy::as_conversions)] // usize→f64 precision loss acceptable for incident counts
        let baseline_rate = baseline.len() as f64 / (period_f / 2.0 / 30.0);

        let drift_ratio = if baseline_rate > 0.0 {
            recent_rate / baseline_rate
        } else if recent_rate > 0.0 {
            f64::INFINITY
        } else {
            1.0
        };

        let drift_detected = !(0.5..=1.5).contains(&drift_ratio);

        DriftAnalysis {
            baseline_incident_rate: baseline_rate,
            recent_incident_rate: recent_rate,
            drift_ratio,
            drift_detected,
            direction: if drift_ratio > 1.0 {
                DriftDirection::Increasing
            } else if drift_ratio < 1.0 {
                DriftDirection::Decreasing
            } else {
                DriftDirection::Stable
            },
        }
    }
}

/// Drift analysis result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DriftAnalysis {
    pub baseline_incident_rate: f64,
    pub recent_incident_rate: f64,
    pub drift_ratio: f64,
    pub drift_detected: bool,
    pub direction: DriftDirection,
}

/// Drift direction
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriftDirection {
    Increasing,
    Stable,
    Decreasing,
}

// ============================================================================
// SIGNAL PROPAGATION (ToV Integration)
// ============================================================================

/// Calculate signal propagation factor based on ACA case
pub fn signal_propagation_factor(case: ACACase) -> f64 {
    case_propagation_factor(case)
}

/// Aggregate signals with propagation weights
pub fn aggregate_signals_with_propagation(_signals: &[Signal], iairs: &[IAIR]) -> f64 {
    let mut total_weighted = 0.0;
    let mut total_weight = 0.0;

    for iair in iairs {
        let propagation = signal_propagation_factor(iair.block_f.logic_engine_case);
        if propagation > 0.0 {
            total_weighted += iair.block_g.context_risk_score * propagation;
            total_weight += propagation;
        }
    }

    if total_weight > 0.0 {
        total_weighted / total_weight
    } else {
        0.0
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::guardian::iair::{
        CheckabilityLevel, ExpertiseLevel, IAIRBuilder, OutcomeType, OutputTreatment, StakesLevel,
    };

    fn create_test_iair(category: IncidentCategory, severity: f64, domain: &str) -> IAIR {
        IAIRBuilder::new()
            .session_id("test")
            .model("Claude", "4.5")
            .context(
                ExpertiseLevel::Low,
                StakesLevel::High,
                CheckabilityLevel::Low,
            )
            .domain(domain)
            .output_treatment(OutputTreatment::DirectUse)
            .incident(category)
            .outcome(OutcomeType::SignificantError, severity)
            .build_minimal()
            .unwrap()
    }

    #[test]
    fn test_signal_detector_default() {
        let detector = SignalDetector::default();
        assert_eq!(detector.min_incidents, 3);
        assert_eq!(detector.default_window_days, 30);
    }

    #[test]
    fn test_detect_signals_empty() {
        let detector = SignalDetector::default();
        let signals = detector.detect_signals(&[], None);
        assert!(signals.is_empty());
    }

    #[test]
    fn test_detect_signals_below_threshold() {
        let detector = SignalDetector::default();
        let iairs = vec![
            create_test_iair(IncidentCategory::Confabulation, 0.5, "legal"),
            create_test_iair(IncidentCategory::Confabulation, 0.4, "legal"),
        ];
        // Only 2 incidents, below min_incidents of 3
        let signals = detector.detect_signals(&iairs, None);
        assert!(signals.is_empty());
    }

    #[test]
    fn test_detect_signals_above_threshold() {
        let detector = SignalDetector::default();
        let iairs = vec![
            create_test_iair(IncidentCategory::Confabulation, 0.5, "legal"),
            create_test_iair(IncidentCategory::Confabulation, 0.4, "legal"),
            create_test_iair(IncidentCategory::Confabulation, 0.6, "legal"),
        ];
        let signals = detector.detect_signals(&iairs, None);
        assert!(!signals.is_empty());
        assert_eq!(signals[0].category, IncidentCategory::Confabulation);
        assert_eq!(signals[0].incident_count, 3);
    }

    #[test]
    fn test_signal_propagation_factor() {
        assert_eq!(signal_propagation_factor(ACACase::CaseI), 1.0);
        assert_eq!(signal_propagation_factor(ACACase::CaseII), 0.0);
        assert_eq!(signal_propagation_factor(ACACase::CaseIII), 0.5);
    }
}
