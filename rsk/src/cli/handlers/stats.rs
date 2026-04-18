//! CLI handler for statistical inference operations.

use crate::cli::actions::StatsAction;
use rsk::modules::stats;
use serde_json::json;

/// Handle the stats subcommands.
pub fn handle_stats(action: &StatsAction) {
    match action {
        StatsAction::ChiSquare { a, b, c, d } => {
            let input = stats::ChiSquareInput {
                a: *a,
                b: *b,
                c: *c,
                d: *d,
            };
            let result = stats::chi_square_test(&input);
            println!("{}", json!(result));
        }
        StatsAction::TTest { group1, group2 } => {
            let g1: Result<Vec<f64>, _> =
                group1.split(',').map(|s| s.trim().parse::<f64>()).collect();
            let g2: Result<Vec<f64>, _> =
                group2.split(',').map(|s| s.trim().parse::<f64>()).collect();
            match (g1, g2) {
                (Ok(g1), Ok(g2)) => {
                    let input = stats::TTestInput {
                        group1: g1,
                        group2: g2,
                    };
                    match stats::t_test_independent(&input) {
                        Ok(result) => println!("{}", json!(result)),
                        Err(e) => println!("{}", json!({"status": "error", "message": e})),
                    }
                }
                _ => println!(
                    "{}",
                    json!({"status": "error", "message": "Invalid number format in group values"})
                ),
            }
        }
        StatsAction::Proportion { successes, n, null } => {
            let input = stats::ProportionInput {
                successes: *successes,
                n: *n,
                null: *null,
            };
            match stats::proportion_test(&input) {
                Ok(result) => println!("{}", json!(result)),
                Err(e) => println!("{}", json!({"status": "error", "message": e})),
            }
        }
        StatsAction::Correlation { x, y } => {
            let xv: Result<Vec<f64>, _> = x.split(',').map(|s| s.trim().parse::<f64>()).collect();
            let yv: Result<Vec<f64>, _> = y.split(',').map(|s| s.trim().parse::<f64>()).collect();
            match (xv, yv) {
                (Ok(xv), Ok(yv)) => {
                    let input = stats::CorrelationInput { x: xv, y: yv };
                    match stats::correlation_test(&input) {
                        Ok(result) => println!("{}", json!(result)),
                        Err(e) => println!("{}", json!({"status": "error", "message": e})),
                    }
                }
                _ => println!(
                    "{}",
                    json!({"status": "error", "message": "Invalid number format in data values"})
                ),
            }
        }
    }
}
