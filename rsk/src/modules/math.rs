use serde::{Deserialize, Serialize};
use uom::si::f64::*;
use uom::si::length::meter;
use uom::si::mass::kilogram;
use uom::si::time::second;

#[derive(Serialize, Deserialize, Debug)]
pub struct MomentumResult {
    pub value: f64,
    pub unit: String,
}

/// Calculate momentum using type-safe uom units
pub fn calculate_momentum(mass_kg: f64, distance_m: f64, time_s: f64) -> MomentumResult {
    let m = Mass::new::<kilogram>(mass_kg);
    let d = Length::new::<meter>(distance_m);
    let t = Time::new::<second>(time_s);

    let velocity = d / t;
    let momentum = m * velocity;

    MomentumResult {
        value: momentum.value,
        unit: "kg·m/s".to_string(),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VarianceResult {
    pub absolute: f64,
    pub percentage: f64,
}

pub fn calculate_variance(actual: f64, target: f64) -> VarianceResult {
    let absolute = actual - target;
    let percentage = if target == 0.0 {
        if actual == 0.0 { 0.0 } else { f64::INFINITY }
    } else {
        (absolute / target) * 100.0
    };

    VarianceResult {
        absolute: (absolute * 100.0).round() / 100.0,
        percentage: (percentage * 100.0).round() / 100.0,
    }
}

/// Result of a prime check
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct PrimeResult {
    pub is_prime: bool,
    pub number: i64,
    pub reason: String,
}

/// Check if a number is prime
pub fn is_prime(n: i64) -> PrimeResult {
    if n <= 1 {
        return PrimeResult {
            is_prime: false,
            number: n,
            reason: "Numbers less than or equal to 1 are not prime".to_string(),
        };
    }

    if n == 2 {
        return PrimeResult {
            is_prime: true,
            number: 2,
            reason: "2 is the only even prime number".to_string(),
        };
    }

    if n % 2 == 0 {
        return PrimeResult {
            is_prime: false,
            number: n,
            reason: format!("{n} is even and greater than 2"),
        };
    }

    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)] // i64→f64 precision loss acceptable for sqrt bound; f64→i64 truncation intentional
    let limit = (n as f64).sqrt() as i64;
    for i in (3..=limit).step_by(2) {
        if n % i == 0 {
            return PrimeResult {
                is_prime: false,
                number: n,
                reason: format!("{n} is divisible by {i}"),
            };
        }
    }

    PrimeResult {
        is_prime: true,
        number: n,
        reason: format!("{n} has no divisors other than 1 and itself"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // POSITIVE: Happy path tests
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_positive_variance() {
        let result = calculate_variance(120.0, 100.0);
        assert_eq!(result.absolute, 20.0);
        assert_eq!(result.percentage, 20.0);
    }

    #[test]
    fn test_negative_variance() {
        let result = calculate_variance(80.0, 100.0);
        assert_eq!(result.absolute, -20.0);
        assert_eq!(result.percentage, -20.0);
    }

    #[test]
    fn test_no_variance() {
        let result = calculate_variance(100.0, 100.0);
        assert_eq!(result.absolute, 0.0);
        assert_eq!(result.percentage, 0.0);
    }

    #[test]
    fn test_fractional_values() {
        let result = calculate_variance(85.5, 100.0);
        assert_eq!(result.absolute, -14.5);
        assert_eq!(result.percentage, -14.5);
    }

    // ═══════════════════════════════════════════════════════════════
    // EDGE CASES: Boundary conditions
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_zero_target() {
        // Division by zero edge case
        let result = calculate_variance(50.0, 0.0);
        assert_eq!(result.absolute, 50.0);
        assert!(result.percentage.is_infinite());
    }

    #[test]
    fn test_zero_actual() {
        let result = calculate_variance(0.0, 100.0);
        assert_eq!(result.absolute, -100.0);
        assert_eq!(result.percentage, -100.0);
    }

    #[test]
    fn test_both_zero() {
        let result = calculate_variance(0.0, 0.0);
        assert_eq!(result.absolute, 0.0);
        assert_eq!(result.percentage, 0.0);
    }

    #[test]
    fn test_very_small_values() {
        let result = calculate_variance(0.001, 0.002);
        assert_eq!(result.absolute, -0.0); // Rounded
        assert_eq!(result.percentage, -50.0);
    }

    #[test]
    fn test_rounding_precision() {
        // Verify rounding to 2 decimal places
        let result = calculate_variance(100.456, 100.0);
        assert_eq!(result.absolute, 0.46); // Rounded from 0.456
    }

    // ═══════════════════════════════════════════════════════════════
    // STRESS: Large values
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_large_values() {
        let result = calculate_variance(1_000_000.0, 500_000.0);
        assert_eq!(result.absolute, 500_000.0);
        assert_eq!(result.percentage, 100.0);
    }

    #[test]
    fn test_very_large_percentage() {
        let result = calculate_variance(1000.0, 1.0);
        assert_eq!(result.absolute, 999.0);
        assert_eq!(result.percentage, 99900.0); // 999x increase
    }

    // ═══════════════════════════════════════════════════════════════
    // NEGATIVE: Error conditions (handled gracefully)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_negative_inputs() {
        let result = calculate_variance(-50.0, -100.0);
        assert_eq!(result.absolute, 50.0);
        assert_eq!(result.percentage, -50.0);
    }

    #[test]
    fn test_infinity_handling() {
        let result = calculate_variance(f64::INFINITY, 100.0);
        assert!(result.absolute.is_infinite());
        assert!(result.percentage.is_infinite());
    }

    #[test]
    fn test_nan_handling() {
        let result = calculate_variance(f64::NAN, 100.0);
        assert!(result.absolute.is_nan());
        assert!(result.percentage.is_nan());
    }

    // ═══════════════════════════════════════════════════════════════
    // PRIME CHECKER TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_is_prime_basic() {
        assert!(is_prime(2).is_prime);
        assert!(is_prime(3).is_prime);
        assert!(is_prime(5).is_prime);
        assert!(is_prime(7).is_prime);
        assert!(is_prime(11).is_prime);
        assert!(is_prime(13).is_prime);
        assert!(is_prime(17).is_prime);
        assert!(is_prime(19).is_prime);
        assert!(is_prime(23).is_prime);
    }

    #[test]
    fn test_is_prime_non_primes() {
        assert!(!is_prime(4).is_prime);
        assert!(!is_prime(6).is_prime);
        assert!(!is_prime(8).is_prime);
        assert!(!is_prime(9).is_prime);
        assert!(!is_prime(10).is_prime);
        assert!(!is_prime(15).is_prime);
        assert!(!is_prime(21).is_prime);
        assert!(!is_prime(25).is_prime);
    }

    #[test]
    fn test_is_prime_edge_cases() {
        assert!(!is_prime(0).is_prime);
        assert!(!is_prime(1).is_prime);
        assert!(!is_prime(-7).is_prime);
    }

    #[test]
    fn test_is_prime_large() {
        // 7919 is the 1000th prime
        assert!(is_prime(7919).is_prime);
        // 104729 is the 10000th prime
        assert!(is_prime(104729).is_prime);
    }

    // ═══════════════════════════════════════════════════════════════
    // SCIENTIFIC UNIT TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_calculate_momentum() {
        // mass: 10kg, dist: 100m, time: 10s => velocity: 10m/s => momentum: 100kg·m/s
        let result = calculate_momentum(10.0, 100.0, 10.0);
        assert_eq!(result.value, 100.0);
        assert_eq!(result.unit, "kg·m/s");
    }
}
