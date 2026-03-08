//! Guardian-AV algorithmovigilance handler.

use crate::cli::actions::GuardianAction;
use rsk::guardian::iair::{CheckabilityLevel, ExpertiseLevel, OutputTreatment, StakesLevel};
use rsk::guardian::{
    ContextRiskParams, IAIRBuilder, IncidentCategory, OutcomeType, calculate_risk,
    recommend_minimization,
};
use serde_json::json;

/// Handle guardian subcommands.
pub fn handle_guardian(action: &GuardianAction) {
    match action {
        GuardianAction::Risk {
            stakes,
            expertise,
            checkability,
            output,
        } => {
            let stakes_level = match stakes.to_lowercase().as_str() {
                "low" => StakesLevel::Low,
                "moderate" => StakesLevel::Moderate,
                "high" => StakesLevel::High,
                "critical" => StakesLevel::Critical,
                _ => {
                    eprintln!("Error: stakes must be low, moderate, high, or critical");
                    std::process::exit(1);
                }
            };
            let expertise_level = match expertise.to_lowercase().as_str() {
                "low" => ExpertiseLevel::Low,
                "moderate" => ExpertiseLevel::Moderate,
                "high" => ExpertiseLevel::High,
                "unknown" => ExpertiseLevel::Unknown,
                _ => {
                    eprintln!("Error: expertise must be low, moderate, high, or unknown");
                    std::process::exit(1);
                }
            };
            let checkability_level = match checkability.to_lowercase().as_str() {
                "low" => CheckabilityLevel::Low,
                "moderate" => CheckabilityLevel::Moderate,
                "high" => CheckabilityLevel::High,
                "unfalsifiable" => CheckabilityLevel::Unfalsifiable,
                _ => {
                    eprintln!("Error: checkability must be low, moderate, high, or unfalsifiable");
                    std::process::exit(1);
                }
            };
            let output_treatment = match output.to_lowercase().as_str() {
                "draft" => OutputTreatment::Draft,
                "reviewed" => OutputTreatment::Reviewed,
                "direct_use" => OutputTreatment::DirectUse,
                "published" => OutputTreatment::Published,
                _ => {
                    eprintln!("Error: output must be draft, reviewed, direct_use, or published");
                    std::process::exit(1);
                }
            };

            let params = ContextRiskParams {
                stakes: stakes_level,
                expertise: expertise_level,
                checkability: checkability_level,
                output_treatment,
            };
            let result = calculate_risk(&params);
            println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
        }
        GuardianAction::Report {
            category,
            domain,
            stakes,
            severity,
        } => {
            let Some(cat) = IncidentCategory::from_code(category) else {
                eprintln!(
                    "Error: unknown category code '{category}'. Use 'rsk guardian categories' to see valid codes."
                );
                std::process::exit(1);
            };
            let stakes_level = match stakes.to_lowercase().as_str() {
                "low" => StakesLevel::Low,
                "moderate" => StakesLevel::Moderate,
                "high" => StakesLevel::High,
                "critical" => StakesLevel::Critical,
                _ => StakesLevel::Moderate,
            };

            let iair = match IAIRBuilder::new()
                .session_id("cli-generated")
                .model("Claude", "unknown")
                .context(
                    ExpertiseLevel::Unknown,
                    stakes_level,
                    CheckabilityLevel::Moderate,
                )
                .domain(domain.clone())
                .incident(cat)
                .outcome(OutcomeType::NearMiss, *severity)
                .build_minimal()
            {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error building IAIR report: {e}");
                    std::process::exit(1);
                }
            };

            println!("{}", serde_json::to_string_pretty(&iair).unwrap_or_default());
        }
        GuardianAction::Categories => {
            let categories = [
                (
                    "CL-CONFAB",
                    "Confabulation",
                    "Confident, detailed, incorrect output",
                ),
                (
                    "CL-MOTREASON",
                    "Motivated Reasoning",
                    "Apparent rigor, wrong conclusion",
                ),
                (
                    "CL-VULNCODE",
                    "Vulnerable Code",
                    "Security flaw in generated code",
                ),
                (
                    "CL-MANIP",
                    "Manipulation",
                    "Persuasion without user awareness",
                ),
                (
                    "CL-FALSESYNTH",
                    "False Synthesis",
                    "Imposed coherence on contradictions",
                ),
                ("CL-APOPH", "Apophenia", "False pattern detection"),
                (
                    "CL-BADFOLLOW",
                    "Bad Follow",
                    "Harmful instruction following",
                ),
                (
                    "CL-ERRORPROP",
                    "Error Propagation",
                    "Early error compounded",
                ),
                (
                    "CL-OVERCONF",
                    "Overconfidence",
                    "Certainty exceeded accuracy",
                ),
                (
                    "CL-HALLUCITE",
                    "Hallucinated Citation",
                    "Non-existent source cited",
                ),
            ];
            let output: Vec<_> = categories
                .iter()
                .map(|(code, name, desc)| {
                    json!({
                        "code": code,
                        "name": name,
                        "description": desc,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
        }
        GuardianAction::Minimize { risk, incidents } => {
            let level = recommend_minimization(*risk, *incidents);
            println!(
                "{}",
                json!({
                    "risk_score": risk,
                    "incident_count": incidents,
                    "recommended_level": format!("{:?}", level),
                    "description": level.description(),
                    "effect": format!("{:?}", level.signal_effect()),
                })
            );
        }
    }
}
