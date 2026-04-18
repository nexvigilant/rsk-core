//! CLI handler for `rsk train` — records a verdict-chain execution as a
//! learning pattern for compound growth over repeated runs.
//!
//! Emits a structured JSON learning record to stdout with the schema:
//!   (chain_hash, inputs_json, verdict, outcome, timestamp, compound_growth)
//!
//! Persistence is the caller's responsibility (e.g., pipe to `jq` + sqlite3,
//! or to the `brain_artifact_save` / `implicit_set` MCP tools). This keeps
//! the rsk binary db-free and composable.
//!
//! The chain_hash is sha256(chain_name || "|" || inputs_json), giving a
//! stable identity for (chain, input) pairs. Re-training the same pair
//! should increment compound_growth in the persistence layer.

use chrono::Utc;
use rsk::modules::crypto::sha256_hash;
use serde_json::{Value, json};

/// Handle the `rsk train` subcommand.
///
/// # Arguments
/// * `from_chain` — the chain name or existing chain_hash to train from
/// * `input` — optional input JSON the chain was run with
/// * `verdict` — optional verdict string the chain produced
/// * `outcome` — ground-truth label: "success", "failure", "partial", "unknown"
pub fn handle_train(from_chain: &str, input: Option<&str>, verdict: Option<&str>, outcome: &str) {
    let inputs_json: Value = input
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(Value::Null);

    // Stable hash over (chain_name, input) — allows dedup + compound accumulation.
    let inputs_str = serde_json::to_string(&inputs_json).unwrap_or_else(|_| "null".to_string());
    let hash_input = format!("{from_chain}|{inputs_str}");
    let chain_hash = sha256_hash(&hash_input).hex;

    let timestamp = Utc::now().to_rfc3339();

    let record = json!({
        "pattern_type": "verdict_chain_learning",
        "chain_hash": chain_hash,
        "chain_name": from_chain,
        "inputs_json": inputs_json,
        "verdict": verdict.unwrap_or(""),
        "outcome": outcome,
        "timestamp": timestamp,
        "compound_growth": 1,
    });

    println!("{record}");
}
