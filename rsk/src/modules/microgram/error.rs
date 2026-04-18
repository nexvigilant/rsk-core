//! Typed errors for the microgram subsystem.
//!
//! Historically this subsystem returned `Result<_, String>` everywhere. The
//! stated project convention (see `CLAUDE.md`) is `thiserror` for library
//! errors, `anyhow` for binaries — but the microgram runtime is the user-facing
//! product of this crate and had no typed error surface.
//!
//! [`MicrogramError`] gives callers something to match on. Existing
//! `String`-returning helpers can migrate incrementally; the `From<String>`
//! impl lets them be adapted with `?` until they're converted at their own
//! pace.

use std::path::PathBuf;
use thiserror::Error;

/// Errors surfaced by the microgram load/resolve/chain pipeline.
#[derive(Debug, Error)]
pub enum MicrogramError {
    /// A filesystem directory couldn't be scanned.
    #[error("cannot read microgram directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// One or more YAML files failed to parse. Strict loaders fail on the first.
    #[error("{count} microgram file(s) failed to parse:\n{details}")]
    ParseFailures { count: usize, details: String },

    /// A chain referenced a microgram name that is not present in the index.
    #[error("microgram '{name}' not found in {dir}")]
    UnknownName { name: String, dir: PathBuf },

    /// An underlying I/O error at some other step (e.g. snapshot write).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// A parse or serialisation error surfaced via `serde_yaml`.
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Catch-all for legacy `Result<_, String>` boundaries being migrated.
    /// Converting a `String` via `?` produces this variant.
    #[error("{0}")]
    Other(String),
}

impl From<String> for MicrogramError {
    fn from(s: String) -> Self {
        MicrogramError::Other(s)
    }
}

impl From<&str> for MicrogramError {
    fn from(s: &str) -> Self {
        MicrogramError::Other(s.to_string())
    }
}

/// Result alias for the microgram subsystem.
pub type MicrogramResultT<T> = Result<T, MicrogramError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_error_converts() {
        let err: MicrogramError = "boom".to_string().into();
        match err {
            MicrogramError::Other(s) => assert_eq!(s, "boom"),
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    #[test]
    fn display_unknown_name() {
        let err = MicrogramError::UnknownName {
            name: "foo".into(),
            dir: PathBuf::from("/tmp/mg"),
        };
        let msg = err.to_string();
        assert!(msg.contains("foo"));
        assert!(msg.contains("/tmp/mg"));
    }

    #[test]
    fn parse_failures_include_count() {
        let err = MicrogramError::ParseFailures {
            count: 3,
            details: "a\nb\nc".into(),
        };
        assert!(err.to_string().contains("3 microgram file(s)"));
    }
}
