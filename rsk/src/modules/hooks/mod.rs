//! # Hooks Module
//!
//! File organization, validation, and monitoring hooks for Claude Code.
//!
//! ## Features
//!
//! - **File Policy Engine** - Dynamic policy enforcement from YAML configuration
//! - **Blindspot Checker** - Post-write self-review reminders
//! - **Staleness Detection** - Identify stale and orphaned files
//! - **Organization Reports** - Directory health and structure analysis
//!
//! ## CLI Commands
//!
//! ```text
//! rsk hooks validate <path>       Validate file placement against policies
//! rsk hooks staleness <path>      Check if a file is stale
//! rsk hooks categorize <path>     Get file category
//! rsk hooks scan <dir>            Scan directory for violations
//! rsk hooks policy                Show loaded policy configuration
//! rsk hooks blindspot <path>      Generate blindspot check for file type
//! rsk hooks schema-version        Output schema version for compatibility checks
//! ```

/// Schema version for hooks module output format.
/// Increment this when making breaking changes to JSON output structure.
/// Bash scripts check this to ensure compatibility.
pub const SCHEMA_VERSION: u32 = 1;

pub mod blindspot;
pub mod policy;
pub mod scanner;
pub mod staleness;
pub mod validation;

pub use blindspot::*;
pub use policy::*;
pub use scanner::*;
pub use staleness::*;
pub use validation::*;
