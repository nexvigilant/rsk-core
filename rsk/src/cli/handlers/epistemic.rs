//! CLI handler for epistemic rigor validation.

use crate::cli::actions::EpistemicAction;
use rsk::modules::epistemic;
use serde_json::json;

/// Handle the epistemic subcommands.
pub fn handle_epistemic(action: &EpistemicAction) {
    match action {
        EpistemicAction::Validate { claim } => {
            let result = epistemic::validate_claim(claim);
            println!("{}", json!(result));
        }
        EpistemicAction::Batch { claims } => {
            let claim_list: Vec<&str> = claims.split("|||").collect();
            let results = epistemic::validate_claims(&claim_list);
            println!(
                "{}",
                json!({"status": "success", "results": results, "total": results.len()})
            );
        }
        EpistemicAction::Suggestions => {
            let suggestions = epistemic::get_hedging_suggestions();
            let mapped: Vec<serde_json::Value> = suggestions
                .iter()
                .map(|(word, alts)| json!({"word": word, "alternatives": alts}))
                .collect();
            println!("{}", json!({"status": "success", "suggestions": mapped}));
        }
    }
}
