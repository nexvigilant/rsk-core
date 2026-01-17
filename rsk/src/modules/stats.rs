//! Statistical Inference Module
//!
//! Pure Rust implementation of statistical tests with epistemic interpretation.
//! Replaces the Python `stats_calc.py` with native Rust performance.
//!
//! ## Supported Tests
//!
//! | Test | Function | Input |
//! |------|----------|-------|
//! | Chi-square independence | `chi_square_test()` | 2x2 contingency table |
//! | Welch's t-test | `t_test_independent()` | Two sample groups |
//! | One-sample proportion | `proportion_test()` | Successes, n, null hypothesis |
//! | Pearson correlation | `correlation_test()` | X and Y vectors |
//!
//! ## Example
//!
//! ```rust,ignore
//! use rsk::modules::stats::{chi_square_test, ChiSquareInput};
//!
//! let input = ChiSquareInput { a: 47, b: 12000, c: 23, d: 45000 };
//! let result = chi_square_test(&input);
//! println!("p-value: {}, interpretation: {}", result.p_value, result.interpretation);
//! ```

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use statrs::distribution::{Continuous, ContinuousCDF, Normal};

// ═══════════════════════════════════════════════════════════════════════════
// DISTRIBUTION FUNCTIONS (Utilizing statrs for robustness)
// ═══════════════════════════════════════════════════════════════════════════

/// Standard normal CDF using statrs
fn normal_cdf(x: f64) -> f64 {
    let n = Normal::new(0.0, 1.0).unwrap();
    n.cdf(x)
}

/// Standard normal PDF using statrs
fn normal_pdf(x: f64) -> f64 {
    let n = Normal::new(0.0, 1.0).unwrap();
    n.pdf(x)
}

/// Student's t-distribution CDF
fn t_cdf(t: f64, df: f64) -> f64 {
    use statrs::distribution::StudentsT;
    if df <= 0.0 {
        return if t < 0.0 { 0.0 } else { 1.0 };
    }
    let dist = StudentsT::new(0.0, 1.0, df).unwrap();
    dist.cdf(t)
}

/// Incomplete beta function approximation using continued fraction
fn incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    if x == 0.0 {
        return 0.0;
    }
    if x == 1.0 {
        return 1.0;
    }

    // Use continued fraction (Lentz's algorithm)
    let lbeta = ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b);
    let front = (x.ln() * a + (1.0 - x).ln() * b - lbeta).exp() / a;

    // Continued fraction
    let eps = 1e-10;
    let max_iter = 200;

    let mut cf = 1.0;
    let mut c = 1.0;
    let mut d = 0.0;

    for m in 1..=max_iter {
        let m_f = m as f64;

        // Even step
        let numerator = if m == 1 {
            1.0
        } else {
            let m1 = (m - 1) as f64;
            (m1 * (b - m1) * x) / ((a + 2.0 * m1 - 1.0) * (a + 2.0 * m1))
        };

        d = 1.0 + numerator * d;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        d = 1.0 / d;

        c = 1.0 + numerator / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }

        cf *= c * d;

        // Odd step
        let numerator = -((a + m_f) * (a + b + m_f) * x) / ((a + 2.0 * m_f) * (a + 2.0 * m_f + 1.0));

        d = 1.0 + numerator * d;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        d = 1.0 / d;

        c = 1.0 + numerator / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }

        let delta = c * d;
        cf *= delta;

        if (delta - 1.0).abs() < eps {
            break;
        }
    }

    front * cf
}

/// Log gamma function (Stirling's approximation for larger values)
fn ln_gamma(x: f64) -> f64 {
    if x <= 0.0 {
        return f64::INFINITY;
    }

    // For small x, use reflection formula or direct computation
    if x < 0.5 {
        return PI.ln() - (PI * x).sin().ln() - ln_gamma(1.0 - x);
    }

    // Lanczos approximation coefficients
    let g = 7.0;
    let c = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    ];

    let x = x - 1.0;
    let mut ag = c[0];
    for i in 1..c.len() {
        ag += c[i] / (x + i as f64);
    }

    let t = x + g + 0.5;
    0.5 * (2.0 * PI).ln() + (x + 0.5) * t.ln() - t + ag.ln()
}

/// Chi-square CDF using incomplete gamma function
fn chi_square_cdf(x: f64, df: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    incomplete_gamma(df / 2.0, x / 2.0)
}

/// Regularized incomplete gamma function P(a, x)
fn incomplete_gamma(a: f64, x: f64) -> f64 {
    if x < 0.0 || a <= 0.0 {
        return 0.0;
    }

    if x < a + 1.0 {
        // Use series expansion
        gamma_series(a, x)
    } else {
        // Use continued fraction
        1.0 - gamma_cf(a, x)
    }
}

/// Incomplete gamma by series expansion
fn gamma_series(a: f64, x: f64) -> f64 {
    let eps = 1e-10;
    let max_iter = 100;

    let mut sum = 1.0 / a;
    let mut term = sum;

    for n in 1..max_iter {
        term *= x / (a + n as f64);
        sum += term;
        if term.abs() < sum.abs() * eps {
            break;
        }
    }

    sum * (-x + a * x.ln() - ln_gamma(a)).exp()
}

/// Incomplete gamma by continued fraction
fn gamma_cf(a: f64, x: f64) -> f64 {
    let eps = 1e-10;
    let max_iter = 100;

    let mut b = x + 1.0 - a;
    let mut c = 1.0 / 1e-30;
    let mut d = 1.0 / b;
    let mut h = d;

    for i in 1..=max_iter {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        c = b + an / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < eps {
            break;
        }
    }

    (-x + a * x.ln() - ln_gamma(a)).exp() * h
}

// ═══════════════════════════════════════════════════════════════════════════
// EPISTEMIC INTERPRETATION
// ═══════════════════════════════════════════════════════════════════════════

/// Epistemic confidence level (L1 = highest certainty, L6 = lowest)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpistemicLevel {
    L1, // Established fact
    L2, // Strong evidence
    L3, // Moderate evidence
    L4, // Weak evidence
    L5, // Insufficient evidence
    L6, // Unknown/unmeasurable
}

impl EpistemicLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            EpistemicLevel::L1 => "L1",
            EpistemicLevel::L2 => "L2",
            EpistemicLevel::L3 => "L3",
            EpistemicLevel::L4 => "L4",
            EpistemicLevel::L5 => "L5",
            EpistemicLevel::L6 => "L6",
        }
    }
}

/// Convert p-value to epistemic interpretation
fn interpret_p_value(p: f64) -> (EpistemicLevel, String) {
    if p < 0.001 {
        (
            EpistemicLevel::L2,
            "very strong statistical evidence against null hypothesis".to_string(),
        )
    } else if p < 0.01 {
        (
            EpistemicLevel::L2,
            "strong statistical evidence against null hypothesis".to_string(),
        )
    } else if p < 0.05 {
        (
            EpistemicLevel::L3,
            "moderate statistical evidence against null hypothesis".to_string(),
        )
    } else if p < 0.10 {
        (
            EpistemicLevel::L4,
            "weak statistical evidence; results are suggestive but not conclusive".to_string(),
        )
    } else {
        (
            EpistemicLevel::L5,
            "insufficient statistical evidence to reject null hypothesis".to_string(),
        )
    }
}

/// Interpret effect size using Cohen's conventions
fn interpret_effect_size(d: f64, metric: &str) -> String {
    let abs_d = d.abs();

    match metric {
        "cohens_d" | "phi" => {
            if abs_d < 0.2 {
                "negligible effect".to_string()
            } else if abs_d < 0.5 {
                "small effect".to_string()
            } else if abs_d < 0.8 {
                "medium effect".to_string()
            } else {
                "large effect".to_string()
            }
        }
        "odds_ratio" | "risk_ratio" => {
            if (0.9..=1.1).contains(&d) {
                "negligible association".to_string()
            } else if (0.67..=1.5).contains(&d) {
                "small association".to_string()
            } else if (0.5..=2.0).contains(&d) {
                "moderate association".to_string()
            } else {
                "strong association".to_string()
            }
        }
        "correlation" => {
            if abs_d < 0.1 {
                "negligible".to_string()
            } else if abs_d < 0.3 {
                "weak".to_string()
            } else if abs_d < 0.5 {
                "moderate".to_string()
            } else if abs_d < 0.7 {
                "strong".to_string()
            } else {
                "very strong".to_string()
            }
        }
        _ => "effect size calculated".to_string(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ASSUMPTION CHECKS
// ═══════════════════════════════════════════════════════════════════════════

/// Result of an assumption check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssumptionCheck {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub severity: String, // "info", "warning", "error"
}

// ═══════════════════════════════════════════════════════════════════════════
// STATISTICAL RESULT
// ═══════════════════════════════════════════════════════════════════════════

/// Complete statistical result with epistemic interpretation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalResult {
    pub test_name: String,
    pub test_statistic: f64,
    pub p_value: f64,
    pub effect_size: Option<f64>,
    pub ci_lower: Option<f64>,
    pub ci_upper: Option<f64>,
    pub ci_level: f64,
    pub degrees_of_freedom: Option<f64>,
    pub sample_size: Option<usize>,
    pub assumptions: Vec<AssumptionCheck>,
    pub epistemic_level: String,
    pub interpretation: String,
    pub raw_output: serde_json::Value,
}

// ═══════════════════════════════════════════════════════════════════════════
// CHI-SQUARE TEST
// ═══════════════════════════════════════════════════════════════════════════

/// Input for chi-square test (2x2 contingency table)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChiSquareInput {
    pub a: i64, // exposed + event
    pub b: i64, // exposed + no event
    pub c: i64, // not exposed + event
    pub d: i64, // not exposed + no event
}

/// Chi-square test for independence (2x2 contingency table)
pub fn chi_square_test(input: &ChiSquareInput) -> StatisticalResult {
    let a = input.a as f64;
    let b = input.b as f64;
    let c = input.c as f64;
    let d = input.d as f64;
    let n = a + b + c + d;

    // Calculate expected values
    let row_totals = [a + b, c + d];
    let col_totals = [a + c, b + d];
    let expected = [
        [row_totals[0] * col_totals[0] / n, row_totals[0] * col_totals[1] / n],
        [row_totals[1] * col_totals[0] / n, row_totals[1] * col_totals[1] / n],
    ];

    // Calculate chi-square statistic
    let observed = [[a, b], [c, d]];
    let mut chi2 = 0.0;
    for i in 0..2 {
        for j in 0..2 {
            let diff = observed[i][j] - expected[i][j];
            chi2 += diff * diff / expected[i][j];
        }
    }

    // Degrees of freedom for 2x2 table
    let dof = 1.0;

    // P-value from chi-square distribution
    let p = 1.0 - chi_square_cdf(chi2, dof);

    // Effect size (phi coefficient for 2x2)
    let phi = (chi2 / n).sqrt();

    // Assumption checks
    let min_expected = expected[0][0]
        .min(expected[0][1])
        .min(expected[1][0])
        .min(expected[1][1]);

    let mut assumptions = vec![];
    if min_expected < 5.0 {
        assumptions.push(AssumptionCheck {
            name: "Expected cell count".to_string(),
            passed: false,
            message: format!(
                "Minimum expected value is {:.1} (<5); Fisher's exact test may be more appropriate",
                min_expected
            ),
            severity: "warning".to_string(),
        });
    } else {
        assumptions.push(AssumptionCheck {
            name: "Expected cell count".to_string(),
            passed: true,
            message: format!("All expected values ≥5 (min: {:.1})", min_expected),
            severity: "info".to_string(),
        });
    }

    // Epistemic interpretation
    let (level, interp) = interpret_p_value(p);
    let effect_interp = interpret_effect_size(phi, "phi");
    let full_interpretation = format!("{}. Effect size (φ={:.3}): {}.", interp, phi, effect_interp);

    StatisticalResult {
        test_name: "Chi-square test for independence".to_string(),
        test_statistic: chi2,
        p_value: p,
        effect_size: Some(phi),
        ci_lower: None,
        ci_upper: None,
        ci_level: 0.95,
        degrees_of_freedom: Some(dof),
        sample_size: Some(n as usize),
        assumptions,
        epistemic_level: level.as_str().to_string(),
        interpretation: full_interpretation,
        raw_output: serde_json::json!({
            "chi_square": chi2,
            "phi": phi,
            "expected": expected
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// T-TEST (INDEPENDENT SAMPLES)
// ═══════════════════════════════════════════════════════════════════════════

/// Input for independent samples t-test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTestInput {
    pub group1: Vec<f64>,
    pub group2: Vec<f64>,
}

/// Welch's independent samples t-test
pub fn t_test_independent(input: &TTestInput) -> Result<StatisticalResult, String> {
    let g1 = &input.group1;
    let g2 = &input.group2;

    if g1.is_empty() || g2.is_empty() {
        return Err("Both groups must have at least one value".to_string());
    }

    let n1 = g1.len() as f64;
    let n2 = g2.len() as f64;

    // Calculate means
    let mean1: f64 = g1.iter().sum::<f64>() / n1;
    let mean2: f64 = g2.iter().sum::<f64>() / n2;

    // Calculate variances
    let var1: f64 = g1.iter().map(|x| (x - mean1).powi(2)).sum::<f64>() / (n1 - 1.0);
    let var2: f64 = g2.iter().map(|x| (x - mean2).powi(2)).sum::<f64>() / (n2 - 1.0);

    // Welch's t-statistic
    let se = (var1 / n1 + var2 / n2).sqrt();
    let t_stat = if se > 0.0 {
        (mean1 - mean2) / se
    } else {
        0.0
    };

    // Welch's degrees of freedom
    let dof = if var1 > 0.0 && var2 > 0.0 {
        let num = (var1 / n1 + var2 / n2).powi(2);
        let denom = (var1 / n1).powi(2) / (n1 - 1.0) + (var2 / n2).powi(2) / (n2 - 1.0);
        num / denom
    } else {
        (n1 + n2 - 2.0).max(1.0)
    };

    // Two-tailed p-value
    let p = 2.0 * (1.0 - t_cdf(t_stat.abs(), dof));

    // Cohen's d effect size
    let pooled_std = (((n1 - 1.0) * var1 + (n2 - 1.0) * var2) / (n1 + n2 - 2.0)).sqrt();
    let d = if pooled_std > 0.0 {
        (mean1 - mean2) / pooled_std
    } else {
        0.0
    };

    // Confidence interval for difference
    // Use t critical value at 0.975 for 95% CI
    // Approximate inverse t-distribution
    let t_crit = 1.96 + 2.0 / dof; // Rough approximation
    let ci_lower = (mean1 - mean2) - t_crit * se;
    let ci_upper = (mean1 - mean2) + t_crit * se;

    // Assumption checks
    let mut assumptions = vec![];

    if n1 < 30.0 || n2 < 30.0 {
        assumptions.push(AssumptionCheck {
            name: "Sample size".to_string(),
            passed: n1 >= 10.0 && n2 >= 10.0,
            message: format!(
                "n1={}, n2={}; small samples may violate normality assumption",
                n1 as usize, n2 as usize
            ),
            severity: if n1 < 10.0 || n2 < 10.0 {
                "warning"
            } else {
                "info"
            }
            .to_string(),
        });
    }

    let var_ratio = if var1.min(var2) > 0.0 {
        var1.max(var2) / var1.min(var2)
    } else {
        f64::INFINITY
    };
    if var_ratio > 4.0 {
        assumptions.push(AssumptionCheck {
            name: "Variance homogeneity".to_string(),
            passed: false,
            message: format!("Variance ratio = {:.1}; using Welch's correction", var_ratio),
            severity: "info".to_string(),
        });
    }

    // Epistemic interpretation
    let (level, interp) = interpret_p_value(p);
    let effect_interp = interpret_effect_size(d, "cohens_d");
    let full_interpretation = format!(
        "{}. Effect size (Cohen's d={:.3}): {}.",
        interp, d, effect_interp
    );

    Ok(StatisticalResult {
        test_name: "Welch's independent samples t-test".to_string(),
        test_statistic: t_stat,
        p_value: p,
        effect_size: Some(d),
        ci_lower: Some(ci_lower),
        ci_upper: Some(ci_upper),
        ci_level: 0.95,
        degrees_of_freedom: Some(dof),
        sample_size: Some((n1 + n2) as usize),
        assumptions,
        epistemic_level: level.as_str().to_string(),
        interpretation: full_interpretation,
        raw_output: serde_json::json!({
            "mean1": mean1,
            "mean2": mean2,
            "diff": mean1 - mean2
        }),
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPORTION TEST
// ═══════════════════════════════════════════════════════════════════════════

/// Input for one-sample proportion z-test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProportionInput {
    pub successes: i64,
    pub n: i64,
    pub null: Option<f64>, // Default 0.5
}

/// One-sample proportion z-test
pub fn proportion_test(input: &ProportionInput) -> Result<StatisticalResult, String> {
    let x = input.successes as f64;
    let n = input.n as f64;
    let p0 = input.null.unwrap_or(0.5);

    if n <= 0.0 {
        return Err("Sample size must be positive".to_string());
    }

    // Sample proportion
    let p_hat = x / n;

    // Z-test statistic
    let se = (p0 * (1.0 - p0) / n).sqrt();
    let z = if se > 0.0 { (p_hat - p0) / se } else { 0.0 };

    // Two-tailed p-value
    let p_value = 2.0 * (1.0 - normal_cdf(z.abs()));

    // Wilson score interval (better for proportions)
    let z_crit: f64 = 1.96;
    let denominator = 1.0 + z_crit.powi(2) / n;
    let center = (p_hat + z_crit.powi(2) / (2.0 * n)) / denominator;
    let margin =
        z_crit * (p_hat * (1.0 - p_hat) / n + z_crit.powi(2) / (4.0 * n.powi(2))).sqrt() / denominator;
    let ci_lower = (center - margin).max(0.0);
    let ci_upper = (center + margin).min(1.0);

    // Assumption checks
    let mut assumptions = vec![];
    if n * p0 < 10.0 || n * (1.0 - p0) < 10.0 {
        assumptions.push(AssumptionCheck {
            name: "Normal approximation".to_string(),
            passed: false,
            message: format!(
                "np={:.1}, n(1-p)={:.1}; binomial exact test may be more appropriate",
                n * p0,
                n * (1.0 - p0)
            ),
            severity: "warning".to_string(),
        });
    } else {
        assumptions.push(AssumptionCheck {
            name: "Normal approximation".to_string(),
            passed: true,
            message: format!("np={:.1} ≥10 and n(1-p)={:.1} ≥10", n * p0, n * (1.0 - p0)),
            severity: "info".to_string(),
        });
    }

    // Epistemic interpretation
    let (level, interp) = interpret_p_value(p_value);
    let full_interpretation = format!(
        "Sample proportion {:.3} vs null {}. {}.",
        p_hat, p0, interp
    );

    Ok(StatisticalResult {
        test_name: "One-sample proportion z-test".to_string(),
        test_statistic: z,
        p_value,
        effect_size: Some(p_hat - p0),
        ci_lower: Some(ci_lower),
        ci_upper: Some(ci_upper),
        ci_level: 0.95,
        degrees_of_freedom: None,
        sample_size: Some(n as usize),
        assumptions,
        epistemic_level: level.as_str().to_string(),
        interpretation: full_interpretation,
        raw_output: serde_json::json!({
            "p_hat": p_hat,
            "null": p0,
            "z": z
        }),
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// CORRELATION TEST
// ═══════════════════════════════════════════════════════════════════════════

/// Input for Pearson correlation test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationInput {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

/// Pearson correlation with significance test
pub fn correlation_test(input: &CorrelationInput) -> Result<StatisticalResult, String> {
    let x = &input.x;
    let y = &input.y;

    if x.len() != y.len() {
        return Err("X and Y must have the same length".to_string());
    }
    if x.len() < 3 {
        return Err("Need at least 3 data points for correlation".to_string());
    }

    let n = x.len() as f64;

    // Calculate means
    let mean_x: f64 = x.iter().sum::<f64>() / n;
    let mean_y: f64 = y.iter().sum::<f64>() / n;

    // Calculate Pearson correlation coefficient
    let mut sum_xy = 0.0;
    let mut sum_x2 = 0.0;
    let mut sum_y2 = 0.0;

    for i in 0..x.len() {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        sum_xy += dx * dy;
        sum_x2 += dx * dx;
        sum_y2 += dy * dy;
    }

    let r = if sum_x2 > 0.0 && sum_y2 > 0.0 {
        sum_xy / (sum_x2 * sum_y2).sqrt()
    } else {
        0.0
    };

    // T-statistic for testing r != 0
    let t_stat = if r.abs() < 1.0 {
        r * ((n - 2.0) / (1.0 - r * r)).sqrt()
    } else {
        f64::INFINITY
    };

    let dof = n - 2.0;

    // Two-tailed p-value
    let p_value = 2.0 * (1.0 - t_cdf(t_stat.abs(), dof));

    // Fisher z-transformation for CI
    let z = if r.abs() < 1.0 {
        0.5 * ((1.0 + r) / (1.0 - r)).ln()
    } else {
        0.0
    };
    let se_z = if n > 3.0 {
        1.0 / (n - 3.0).sqrt()
    } else {
        f64::INFINITY
    };
    let z_crit = 1.96;
    let z_lower = z - z_crit * se_z;
    let z_upper = z + z_crit * se_z;
    let ci_lower = ((2.0 * z_lower).exp() - 1.0) / ((2.0 * z_lower).exp() + 1.0);
    let ci_upper = ((2.0 * z_upper).exp() - 1.0) / ((2.0 * z_upper).exp() + 1.0);

    // Assumption checks
    let mut assumptions = vec![];
    if n < 30.0 {
        assumptions.push(AssumptionCheck {
            name: "Sample size".to_string(),
            passed: n >= 10.0,
            message: format!(
                "n={}; consider Spearman if data may be non-normal",
                n as usize
            ),
            severity: if n < 10.0 { "warning" } else { "info" }.to_string(),
        });
    }

    // Interpret correlation strength
    let (level, p_interp) = interpret_p_value(p_value);
    let strength = interpret_effect_size(r, "correlation");
    let direction = if r > 0.0 { "positive" } else { "negative" };
    let full_interpretation = format!(
        "{} {} correlation (r={:.3}). {}.",
        strength, direction, r, p_interp
    );

    Ok(StatisticalResult {
        test_name: "Pearson correlation".to_string(),
        test_statistic: r,
        p_value,
        effect_size: Some(r),
        ci_lower: Some(ci_lower),
        ci_upper: Some(ci_upper),
        ci_level: 0.95,
        degrees_of_freedom: Some(dof),
        sample_size: Some(n as usize),
        assumptions,
        epistemic_level: level.as_str().to_string(),
        interpretation: full_interpretation,
        raw_output: serde_json::json!({
            "r": r,
            "r_squared": r * r
        }),
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // Distribution function tests

    #[test]
    fn test_normal_cdf() {
        // Test standard normal CDF values
        assert!((normal_cdf(0.0) - 0.5).abs() < 0.001);
        assert!((normal_cdf(1.96) - 0.975).abs() < 0.01);
        assert!((normal_cdf(-1.96) - 0.025).abs() < 0.01);
    }

    #[test]
    fn test_chi_square_cdf() {
        // Chi-square with df=1, x=3.84 should give ~0.95
        let result = chi_square_cdf(3.84, 1.0);
        assert!((result - 0.95).abs() < 0.02, "Got {}", result);
    }

    // Chi-square test

    #[test]
    fn test_chi_square_basic() {
        let input = ChiSquareInput {
            a: 47,
            b: 12000,
            c: 23,
            d: 45000,
        };
        let result = chi_square_test(&input);

        assert!(result.test_statistic > 0.0);
        assert!(result.p_value >= 0.0 && result.p_value <= 1.0);
        assert!(result.effect_size.is_some());
    }

    #[test]
    fn test_chi_square_no_effect() {
        // Perfectly balanced table - no association
        let input = ChiSquareInput {
            a: 50,
            b: 50,
            c: 50,
            d: 50,
        };
        let result = chi_square_test(&input);

        assert!(result.test_statistic < 0.001);
        assert!(result.p_value > 0.95);
    }

    // T-test

    #[test]
    fn test_t_test_different_groups() {
        let input = TTestInput {
            group1: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            group2: vec![6.0, 7.0, 8.0, 9.0, 10.0],
        };
        let result = t_test_independent(&input).unwrap();

        assert!(result.test_statistic < 0.0); // g1 mean < g2 mean
        assert!(result.p_value < 0.05); // Significant difference
    }

    #[test]
    fn test_t_test_similar_groups() {
        let input = TTestInput {
            group1: vec![5.0, 5.1, 4.9, 5.0, 5.0],
            group2: vec![5.0, 4.9, 5.1, 5.0, 5.0],
        };
        let result = t_test_independent(&input).unwrap();

        assert!(result.p_value > 0.5); // Not significant
    }

    #[test]
    fn test_t_test_empty_group() {
        let input = TTestInput {
            group1: vec![],
            group2: vec![1.0, 2.0],
        };
        assert!(t_test_independent(&input).is_err());
    }

    // Proportion test

    #[test]
    fn test_proportion_basic() {
        let input = ProportionInput {
            successes: 60,
            n: 100,
            null: Some(0.5),
        };
        let result = proportion_test(&input).unwrap();

        assert!(result.p_value < 0.1); // 60% vs 50% should be somewhat significant
    }

    #[test]
    fn test_proportion_at_null() {
        let input = ProportionInput {
            successes: 50,
            n: 100,
            null: Some(0.5),
        };
        let result = proportion_test(&input).unwrap();

        assert!(result.p_value > 0.9); // 50% vs 50% - no difference
    }

    // Correlation test

    #[test]
    fn test_correlation_positive() {
        let input = CorrelationInput {
            x: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            y: vec![2.0, 4.0, 6.0, 8.0, 10.0],
        };
        let result = correlation_test(&input).unwrap();

        assert!((result.test_statistic - 1.0).abs() < 0.001); // Perfect correlation
    }

    #[test]
    fn test_correlation_negative() {
        let input = CorrelationInput {
            x: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            y: vec![10.0, 8.0, 6.0, 4.0, 2.0],
        };
        let result = correlation_test(&input).unwrap();

        assert!((result.test_statistic + 1.0).abs() < 0.001); // Perfect negative
    }

    #[test]
    fn test_correlation_mismatched_lengths() {
        let input = CorrelationInput {
            x: vec![1.0, 2.0],
            y: vec![1.0, 2.0, 3.0],
        };
        assert!(correlation_test(&input).is_err());
    }

    // Epistemic interpretation tests

    #[test]
    fn test_interpret_p_value_levels() {
        let (l1, _) = interpret_p_value(0.0001);
        assert_eq!(l1, EpistemicLevel::L2);

        let (l2, _) = interpret_p_value(0.03);
        assert_eq!(l2, EpistemicLevel::L3);

        let (l3, _) = interpret_p_value(0.15);
        assert_eq!(l3, EpistemicLevel::L5);
    }

    #[test]
    fn test_effect_size_interpretation() {
        assert!(interpret_effect_size(0.1, "cohens_d").contains("negligible"));
        assert!(interpret_effect_size(0.3, "cohens_d").contains("small"));
        assert!(interpret_effect_size(0.6, "cohens_d").contains("medium"));
        assert!(interpret_effect_size(1.0, "cohens_d").contains("large"));
    }
}
