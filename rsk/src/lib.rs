//! # RSK - Rust Skill Kernel
//!
//! High-performance computation kernel for Claude Code skills.
//!
//! ## Overview
//!
//! RSK provides Rust-native implementations for common skill operations,
//! achieving 10-100x performance improvements over Python equivalents.
//!
//! ## Modules
//!
//! - **math** - Statistical calculations (variance, metrics)
//! - **graph** - DAG operations (topological sort, shortest path, parallel levels)
//! - **`text_processor`** - SKILL.md parsing and SMST extraction
//! - **levenshtein** - Fuzzy string matching (63x faster than Python)
//! - **crypto** - SHA-256 hashing
//! - **`code_generator`** - SMST to code generation (validation rules, tests, stubs)
//! - **`yaml_processor`** - YAML/TOML parsing and validation
//! - **taxonomy** - O(1) compile-time lookup tables via phf (compliance levels, SMST components)
//! - **telemetry** - Unified tracing infrastructure with span-based logging
//! - **hooks** - File organization, validation, and monitoring hooks
//! - **`python_bindings`** - `PyO3` bridge for Python integration (requires `python` feature)
//!
//! ## CLI Commands
//!
//! ```text
//! rsk variance <actual> <target>    Calculate variance
//! rsk graph topsort --input <json>  Topological sort
//! rsk graph levels --input <json>   Parallel execution levels
//! rsk text parse <path>             Parse SKILL.md
//! rsk text smst <path>              Extract SMST with scoring
//! rsk verify <path>                 Diamond v2 validation
//! rsk build <path>                  Build skill artifacts
//! rsk generate rules <path>         Generate validation rules
//! rsk generate tests <path>         Generate test scaffolds
//! rsk generate stub <path>          Generate Rust module stub
//! rsk levenshtein <src> <tgt>       Edit distance
//! rsk fuzzy <query> --candidates    Fuzzy search
//! rsk sha256 hash <input>           Hash string
//! rsk yaml parse <path>             Parse YAML file to JSON
//! rsk yaml parse-stdin              Parse YAML from stdin to JSON
//! rsk yaml toml <path>              Parse TOML to JSON
//! rsk yaml validate <path>          Validate schema
//! rsk yaml decision-tree <path>     Analyze decision tree
//! rsk yaml frontmatter <path>       Parse SKILL.md frontmatter
//! rsk generate logic <path>        Generate decision tree YAML
//! rsk skills scan <path>           Build skill registry
//! rsk skills list                  List discovered skills
//! rsk skills execute <name>        Execute deterministic skill
//! rsk hooks validate <path>        Validate file placement
//! rsk hooks staleness <path>       Check file staleness
//! rsk hooks scan <dir>             Scan for violations
//! rsk hooks policy                 Show policy configuration
//! rsk hooks blindspot <path>       Generate blindspot check
//! ```
//!
//! ## Performance
//!
//! | Operation | Python | Rust | Speedup |
//! |-----------|--------|------|---------|
//! | Levenshtein | 63ms | 1ms | 63x |
//! | SMST Parse | 50ms | 5ms | 10x |
//! | SHA-256 | 10ms | 0.5ms | 20x |
//!
//! ## Example
//!
//! ```rust
//! use rsk::{levenshtein, parse_yaml};
//!
//! // Fuzzy string matching
//! let result = levenshtein("kitten", "sitting");
//! assert_eq!(result.distance, 3);
//!
//! // Parse YAML content
//! let yaml = "name: test\nversion: '1.0'";
//! let parsed = parse_yaml(yaml).unwrap();
//! assert_eq!(parsed.format, "yaml");
//! ```

pub mod modules;

pub use modules::builder::*;
pub use modules::code_generator::*;
pub use modules::compression::*;
pub use modules::crypto::*;
pub use modules::decision_engine::*;
pub use modules::evolution::*;
pub use modules::execution_engine::*;
pub use modules::graph::*;
pub use modules::guardian;
pub use modules::hooks;
pub use modules::intent::*;
pub use modules::levenshtein::*;
pub use modules::math::*;
#[cfg(feature = "python")]
pub use modules::python_bindings::*;
pub use modules::routing_engine::*;
pub use modules::skill_registry::*;
pub use modules::state_manager::*;
pub use modules::state_server::*;
pub use modules::strategy::*;
pub use modules::taxonomy::*;
pub use modules::telemetry::*;
pub use modules::text_processor::*;
pub use modules::tov;
pub use modules::yaml_processor::*;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Check if Python bindings are available
pub const fn has_python_bindings() -> bool {
    cfg!(feature = "python")
}
