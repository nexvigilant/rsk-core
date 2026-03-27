//! CLI command handlers.
//!
//! Each handler module corresponds to a top-level command or group of related commands.

pub mod anti_pattern;
pub mod compress;
pub mod epistemic;
pub mod microgram;
pub mod exec;
#[cfg(feature = "forge")]
pub mod forge;
pub mod generate;
pub mod graph;
pub mod heligram;
pub mod guardian;
pub mod hooks;
pub mod json;
pub mod route;
pub mod session;
pub mod simple;
pub mod skills;
pub mod state;
pub mod stats;
pub mod taxonomy;
pub mod telemetry;
pub mod text;
pub mod tov;
pub mod verify;
pub mod yaml;
