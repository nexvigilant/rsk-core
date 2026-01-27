//! Taxonomy lookup handler.

use crate::cli::actions::TaxonomyAction;
use rsk::{list_taxonomy, query_taxonomy};

/// Handle taxonomy subcommands.
pub fn handle_taxonomy(action: &TaxonomyAction) {
    match action {
        TaxonomyAction::Query { taxonomy_type, key } => {
            let result = query_taxonomy(taxonomy_type, key);
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        }
        TaxonomyAction::List { taxonomy_type } => {
            let result = list_taxonomy(taxonomy_type);
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        }
        TaxonomyAction::Compliance { level } => {
            let result = query_taxonomy("compliance", level);
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        }
        TaxonomyAction::Smst { component } => {
            let result = query_taxonomy("smst", component);
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        }
        TaxonomyAction::Category { category } => {
            let result = query_taxonomy("category", category);
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        }
    }
}
