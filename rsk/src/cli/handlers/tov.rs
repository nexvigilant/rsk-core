//! Theory of Vigilance (ToV) handler.

use crate::cli::actions::TovAction;
use rsk::tov::{
    AlgorithmCorrectness, CharacterizedHarmEvent, ClinicalOutcome, ClinicianResponse,
    ConservationLaw, Determinism, HarmCharacteristics, HarmType, KHSAI, Multiplicity,
    PropagationProbability, Temporal, analyze_attenuation, classify_harm, determine_aca_case,
    harm_type_characteristics, interpret_khs_ai, protective_depth,
};
use serde_json::json;

/// Handle ToV subcommands.
pub fn handle_tov(action: &TovAction) {
    match action {
        TovAction::Classify { mult, temp, det } => {
            let multiplicity = match mult.to_lowercase().as_str() {
                "single" => Multiplicity::Single,
                "multiple" => Multiplicity::Multiple,
                _ => {
                    eprintln!("Error: mult must be 'single' or 'multiple'");
                    std::process::exit(1);
                }
            };
            let temporal = match temp.to_lowercase().as_str() {
                "acute" => Temporal::Acute,
                "chronic" => Temporal::Chronic,
                _ => {
                    eprintln!("Error: temp must be 'acute' or 'chronic'");
                    std::process::exit(1);
                }
            };
            let determinism = match det.to_lowercase().as_str() {
                "deterministic" => Determinism::Deterministic,
                "stochastic" => Determinism::Stochastic,
                _ => {
                    eprintln!("Error: det must be 'deterministic' or 'stochastic'");
                    std::process::exit(1);
                }
            };

            let event = CharacterizedHarmEvent {
                characteristics: HarmCharacteristics {
                    multiplicity,
                    temporal,
                    determinism,
                },
            };
            let harm_type = classify_harm(event);
            println!(
                "{}",
                json!({
                    "harm_type": format!("{:?}", harm_type),
                    "multiplicity": format!("{:?}", multiplicity),
                    "temporal": format!("{:?}", temporal),
                    "determinism": format!("{:?}", determinism),
                })
            );
        }
        TovAction::Attenuation { probs } => {
            let probabilities: Result<Vec<PropagationProbability>, _> = probs
                .split(',')
                .map(|s| {
                    s.trim()
                        .parse::<f64>()
                        .map_err(|e| e.to_string())
                        .and_then(|v| {
                            if v > 0.0 && v < 1.0 {
                                Ok(PropagationProbability::new(v))
                            } else {
                                Err("Probability must be in (0, 1)".to_string())
                            }
                        })
                })
                .collect();

            match probabilities {
                Ok(probs) => {
                    let result = analyze_attenuation(&probs);
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                }
                Err(e) => {
                    eprintln!("Error parsing probabilities: {}", e);
                    std::process::exit(1);
                }
            }
        }
        TovAction::ProtectiveDepth { target, alpha } => {
            if *target <= 0.0 || *target >= 1.0 {
                eprintln!("Error: target must be in (0, 1)");
                std::process::exit(1);
            }
            if *alpha <= 0.0 {
                eprintln!("Error: alpha must be positive");
                std::process::exit(1);
            }
            let depth = protective_depth(*target, *alpha);
            println!(
                "{}",
                json!({
                    "target_probability": target,
                    "attenuation_rate": alpha,
                    "protective_depth": depth,
                })
            );
        }
        TovAction::Aca {
            correctness,
            response,
            outcome,
        } => {
            let alg_correctness = match correctness.to_lowercase().as_str() {
                "correct" => AlgorithmCorrectness::Correct,
                "wrong" => AlgorithmCorrectness::Wrong,
                _ => {
                    eprintln!("Error: correctness must be 'correct' or 'wrong'");
                    std::process::exit(1);
                }
            };
            let clin_response = match response.to_lowercase().as_str() {
                "followed" => ClinicianResponse::Followed,
                "overrode" => ClinicianResponse::Overrode,
                _ => {
                    eprintln!("Error: response must be 'followed' or 'overrode'");
                    std::process::exit(1);
                }
            };
            let clin_outcome = match outcome.to_lowercase().as_str() {
                "good" => ClinicalOutcome::Good,
                "harm" => ClinicalOutcome::Harm,
                _ => {
                    eprintln!("Error: outcome must be 'good' or 'harm'");
                    std::process::exit(1);
                }
            };

            let case = determine_aca_case(alg_correctness, clin_response, clin_outcome);
            let propagation = rsk::tov::case_propagation_factor(case);
            println!(
                "{}",
                json!({
                    "case": format!("{:?}", case),
                    "propagation_factor": propagation,
                    "description": match case {
                        rsk::tov::ACACase::CaseI => "Incident - algorithm wrong, followed, harm occurred",
                        rsk::tov::ACACase::CaseII => "Exculpated - algorithm correct, overridden, harm occurred",
                        rsk::tov::ACACase::CaseIII => "Signal - algorithm wrong, overridden (near-miss)",
                        rsk::tov::ACACase::CaseIV => "Baseline - algorithm correct, followed, good outcome",
                    },
                })
            );
        }
        TovAction::Khs {
            latency,
            accuracy,
            resource,
            drift,
        } => {
            let khs = KHSAI::calculate(*latency, *accuracy, *resource, *drift);
            let status = interpret_khs_ai(khs.overall);
            println!(
                "{}",
                json!({
                    "overall": khs.overall,
                    "status": format!("{:?}", status),
                    "latency_stability": khs.latency_stability,
                    "accuracy_stability": khs.accuracy_stability,
                    "resource_efficiency": khs.resource_efficiency,
                    "drift_score": khs.drift_score,
                })
            );
        }
        TovAction::HarmTypes => {
            let types = [
                HarmType::Acute,
                HarmType::Cumulative,
                HarmType::OffTarget,
                HarmType::Cascade,
                HarmType::Idiosyncratic,
                HarmType::Saturation,
                HarmType::Interaction,
                HarmType::Population,
            ];
            let output: Vec<_> = types
                .iter()
                .map(|t| {
                    let chars = harm_type_characteristics(*t);
                    json!({
                        "type": format!("{:?}", t),
                        "multiplicity": format!("{:?}", chars.multiplicity),
                        "temporal": format!("{:?}", chars.temporal),
                        "determinism": format!("{:?}", chars.determinism),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        TovAction::ConservationLaws => {
            let laws = [
                ConservationLaw::Mass,
                ConservationLaw::Energy,
                ConservationLaw::State,
                ConservationLaw::Flux,
                ConservationLaw::Catalyst,
                ConservationLaw::Rate,
                ConservationLaw::Equilibrium,
                ConservationLaw::Saturation,
                ConservationLaw::Entropy,
                ConservationLaw::Discretization,
                ConservationLaw::Structure,
            ];
            let output: Vec<_> = laws
                .iter()
                .map(|l| {
                    json!({
                        "index": l.index(),
                        "name": format!("{:?}", l),
                        "type": format!("{:?}", l.law_type()),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
    }
}
