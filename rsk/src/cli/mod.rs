//! CLI module for RSK (Rust Skill Kernel)
//!
//! This module provides the command-line interface implementation:
//!
//! | Module | Responsibility |
//! |--------|---------------|
//! | `actions` | Command action enums (GraphAction, TextAction, etc.) |
//! | `utils` | Shared utilities (load_graph, JSON output, path resolution) |
//! | `handlers` | Command handler implementations |

pub mod actions;
pub mod handlers;
pub mod utils;

// Re-export actions for use in main.rs
pub use actions::*;
