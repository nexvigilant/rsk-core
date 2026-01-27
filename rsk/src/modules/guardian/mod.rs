//! Guardian-AV Module - Algorithmovigilance Operational Framework
//!
//! Implements the Guardian-AV 7-module architecture (ToV §57) for monitoring
//! and reporting AI/algorithm-related incidents using the IAIR schema.
//!
//! ## Modules
//!
//! - **iair** - Individual Algorithm Incident Report schema and operations
//! - **signal** - Signal detection and pattern analysis
//! - **risk** - Risk scoring and therapeutic window calculation
//!
//! ## CLI Commands
//!
//! ```text
//! rsk guardian iair create --category CL-CONFAB --stakes high ...
//! rsk guardian iair analyze <report_id>
//! rsk guardian signal detect --domain legal --window 30d
//! rsk guardian risk score --stakes high --expertise low --checkability low
//! ```

pub mod iair;
pub mod risk;
pub mod signal;

pub use iair::*;
pub use risk::*;
pub use signal::*;
