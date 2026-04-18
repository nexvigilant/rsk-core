//! Cached microgram index.
//!
//! Motivation: `load_all(dir)` walks the filesystem, parses every YAML, and sorts
//! the result — work that's fine once per process but wasteful when a long-lived
//! consumer (MCP server, bench harness) calls it per request.
//!
//! `MicrogramIndex` performs the scan once, indexes the result by `name` inside an
//! `HashMap<String, Arc<Microgram>>`, and exposes O(1) name lookup with cheap
//! `Arc` cloning instead of whole-struct cloning.
//!
//! # Lifecycle
//!
//! The index is immutable after construction. Callers that need to pick up
//! on-disk changes should rebuild (`MicrogramIndex::load`) — cache invalidation
//! is out of scope.

use super::{Microgram, load_all, load_all_collect};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A parse error encountered while loading the index.
#[derive(Debug, Clone)]
pub struct LoadError {
    pub path: PathBuf,
    pub message: String,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

impl std::error::Error for LoadError {}

/// Cached, name-indexed view of a microgram directory.
///
/// Holds `Arc<Microgram>` internally so downstream chain APIs can share ownership
/// without deep-cloning the decision tree on each lookup.
#[derive(Debug, Clone)]
pub struct MicrogramIndex {
    /// Directory this index was loaded from.
    dir: PathBuf,
    /// name → microgram. Cheap to clone via Arc.
    by_name: HashMap<String, Arc<Microgram>>,
    /// Micrograms that failed to parse. Callers can inspect to fail loudly.
    load_errors: Vec<LoadError>,
}

impl MicrogramIndex {
    /// Build an index by scanning `dir` recursively for YAML files.
    ///
    /// Parse errors are collected into `load_errors()` rather than short-circuiting;
    /// callers choose whether to treat them as fatal. Duplicate microgram names
    /// overwrite earlier entries (deterministic order: later wins by filesystem
    /// walk order after sort).
    pub fn load(dir: &Path) -> Result<Self, String> {
        let (loaded, errors) = load_all_collect(dir)?;
        let mut by_name = HashMap::with_capacity(loaded.len());
        for mg in loaded {
            by_name.insert(mg.name.clone(), Arc::new(mg));
        }
        let load_errors = errors
            .into_iter()
            .map(|(path, message)| LoadError { path, message })
            .collect();
        Ok(Self {
            dir: dir.to_path_buf(),
            by_name,
            load_errors,
        })
    }

    /// Convenience: ignore parse errors, same as the previous `load_all` behaviour.
    pub fn load_lossy(dir: &Path) -> Result<Self, String> {
        // Use the existing eprintln-and-continue loader as a fallback.
        let all = load_all(dir)?;
        let mut by_name = HashMap::with_capacity(all.len());
        for mg in all {
            by_name.insert(mg.name.clone(), Arc::new(mg));
        }
        Ok(Self {
            dir: dir.to_path_buf(),
            by_name,
            load_errors: Vec::new(),
        })
    }

    /// Strict loader: returns an error if any YAML file in `dir` failed to parse.
    ///
    /// CI gates and batch operations should prefer this over `load` / `load_lossy`
    /// so that a corrupt microgram doesn't silently reduce the fleet. The error
    /// message enumerates every failing path.
    pub fn load_strict(dir: &Path) -> Result<Self, String> {
        let this = Self::load(dir)?;
        if !this.load_errors.is_empty() {
            let summary = this
                .load_errors
                .iter()
                .map(|e| format!("  {e}"))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(format!(
                "{} microgram file(s) failed to parse:\n{summary}",
                this.load_errors.len()
            ));
        }
        Ok(this)
    }

    /// Build from an already-loaded `Vec`. Useful for tests and callers that
    /// already hold an owned list.
    pub fn from_vec(dir: &Path, micrograms: Vec<Microgram>) -> Self {
        let mut by_name = HashMap::with_capacity(micrograms.len());
        for mg in micrograms {
            by_name.insert(mg.name.clone(), Arc::new(mg));
        }
        Self {
            dir: dir.to_path_buf(),
            by_name,
            load_errors: Vec::new(),
        }
    }

    /// Directory this index was loaded from.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Number of micrograms in the index.
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// True if the index has no entries.
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    /// O(1) lookup by microgram name. Returns cloned `Arc` (cheap — reference bump).
    pub fn get(&self, name: &str) -> Option<Arc<Microgram>> {
        self.by_name.get(name).cloned()
    }

    /// True if a microgram with this name exists.
    pub fn contains(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// All microgram names, in sorted order.
    pub fn names(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.by_name.keys().map(String::as_str).collect();
        keys.sort_unstable();
        keys
    }

    /// Iterate over all entries. Order is not guaranteed.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Arc<Microgram>)> {
        self.by_name.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// All micrograms, returned as cheap `Arc` clones, in sorted-by-name order.
    pub fn all(&self) -> Vec<Arc<Microgram>> {
        let mut names: Vec<&String> = self.by_name.keys().collect();
        names.sort_unstable();
        names
            .into_iter()
            .map(|n| Arc::clone(&self.by_name[n]))
            .collect()
    }

    /// Parse errors encountered while loading. Empty when all files parsed.
    pub fn load_errors(&self) -> &[LoadError] {
        &self.load_errors
    }

    /// Resolve a list of names to `Arc<Microgram>`s, returning the first missing
    /// name as an error. Used by chain APIs to convert `&[&str]` → `Vec<Arc<Microgram>>`
    /// with a single hash lookup per name instead of `load_all` + linear find.
    pub fn resolve<'a, I>(&self, names: I) -> Result<Vec<Arc<Microgram>>, String>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut out = Vec::new();
        for name in names {
            match self.by_name.get(name) {
                Some(mg) => out.push(Arc::clone(mg)),
                None => {
                    return Err(format!(
                        "Microgram '{name}' not found in {}",
                        self.dir.display()
                    ));
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    const SAMPLE: &str = r#"
name: is-positive
description: "test"
tree:
  start: root
  nodes:
    root:
      type: return
      value:
        answer: yes
tests: []
"#;

    fn write_sample(dir: &Path, name: &str) {
        fs::write(dir.join(format!("{name}.yaml")), SAMPLE).unwrap();
    }

    #[test]
    fn index_load_round_trip() {
        let tmp = TempDir::new().unwrap();
        write_sample(tmp.path(), "alpha");
        write_sample(tmp.path(), "beta");
        let idx = MicrogramIndex::load(tmp.path()).unwrap();
        // Both yaml files declare `name: is-positive` — later load wins.
        // Test the API shape, not the overwrite semantics.
        assert!(idx.len() >= 1);
        assert!(idx.contains("is-positive"));
        assert!(idx.get("is-positive").is_some());
    }

    #[test]
    fn resolve_reports_missing() {
        let tmp = TempDir::new().unwrap();
        write_sample(tmp.path(), "only");
        let idx = MicrogramIndex::load(tmp.path()).unwrap();
        let err = idx.resolve(["is-positive", "does-not-exist"]).unwrap_err();
        assert!(err.contains("does-not-exist"));
    }

    #[test]
    fn from_vec_skips_filesystem() {
        let dir = Path::new("/tmp");
        let idx = MicrogramIndex::from_vec(dir, Vec::new());
        assert!(idx.is_empty());
        assert_eq!(idx.dir(), dir);
        assert!(idx.load_errors().is_empty());
    }

    #[test]
    fn resolve_preserves_order() {
        let tmp = TempDir::new().unwrap();
        write_sample(tmp.path(), "one");
        let idx = MicrogramIndex::load(tmp.path()).unwrap();
        let order = idx.resolve(["is-positive"]).unwrap();
        assert_eq!(order.len(), 1);
        assert_eq!(order[0].name, "is-positive");
    }

    #[test]
    fn get_returns_arc() {
        let tmp = TempDir::new().unwrap();
        write_sample(tmp.path(), "one");
        let idx = MicrogramIndex::load(tmp.path()).unwrap();
        let a = idx.get("is-positive").unwrap();
        let b = idx.get("is-positive").unwrap();
        // Both Arcs point at the same allocation.
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn load_collects_parse_errors_instead_of_swallowing() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("good.yaml"), SAMPLE).unwrap();
        // Intentional garbage — not a valid microgram.
        fs::write(tmp.path().join("corrupt.yaml"), "this: is: not: yaml:").unwrap();

        let idx = MicrogramIndex::load(tmp.path()).unwrap();
        // At least one entry loaded successfully.
        assert!(!idx.is_empty());
        // And at least one error surfaced — it did NOT silently vanish.
        assert!(!idx.load_errors().is_empty());
        let msg = idx.load_errors()[0].to_string();
        assert!(
            msg.contains("corrupt.yaml"),
            "error should name the bad file: {msg}"
        );
    }

    #[test]
    fn load_strict_fails_on_any_parse_error() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("good.yaml"), SAMPLE).unwrap();
        fs::write(tmp.path().join("corrupt.yaml"), "not: [valid").unwrap();

        let err = MicrogramIndex::load_strict(tmp.path()).unwrap_err();
        assert!(
            err.contains("corrupt.yaml"),
            "strict loader must name the bad file: {err}"
        );
        assert!(err.contains("failed to parse"));
    }
}
