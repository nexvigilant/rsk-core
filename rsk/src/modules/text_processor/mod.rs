//! # Text Processor Module
//!
//! Text processing utilities and SKILL.md parsing support.
//!
//! ## Architecture
//!
//! This module is composed of atomic sub-modules:
//!
//! | Module | Responsibility |
//! |--------|---------------|
//! | `generic` | General text utilities (tokenize, normalize, slugify, etc.) |
//! | `skill_metadata` | SkillFrontmatter and SkillSection parsing |
//! | `machine_spec` | SMST parsing, validation, and scoring |
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rsk::modules::text_processor::{tokenize, parse_frontmatter, extract_smst};
//!
//! // Tokenize text
//! let tokens = tokenize("Hello world!");
//!
//! // Parse SKILL.md frontmatter
//! let fm = parse_frontmatter(content);
//!
//! // Extract complete SMST with scoring
//! let smst = extract_smst(content);
//! ```

pub mod generic;
pub mod machine_spec;
pub mod skill_metadata;

// Re-export generic text utilities
pub use generic::{
    CompressionAnalysis, NormalizeResult, TokenizeResult, WordFrequencyResult,
    analyze_compressibility, extract_ngrams, normalize, slugify, tokenize, truncate,
    word_frequency,
};

// Re-export skill metadata types
pub use skill_metadata::{SkillFrontmatter, SkillSection, parse_frontmatter};

// Re-export machine spec types and functions
pub use machine_spec::{
    ParsingResult, SkillMachineSpec, SmstResult, SmstScore, calculate_smst_score, extract_smst,
    parse_skill_md, validate_diamond_spec,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_reexports() {
        // Verify all public types are accessible through mod.rs
        let _ = TokenizeResult {
            tokens: vec![],
            count: 0,
            unique_count: 0,
        };
        let _ = SkillFrontmatter::default();
        let _ = SkillMachineSpec::default();
    }
}
