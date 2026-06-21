//! Local include/require loader for the Phase 4 VM MVP.

use std::fs;
use std::path::{Path, PathBuf};

/// Result of loading one include target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedInclude {
    /// Canonical path used for once tracking and source maps.
    pub canonical_path: PathBuf,
    /// PHP source text.
    pub source: String,
}

/// Root-constrained local include loader.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncludeLoader {
    allowed_roots: Vec<PathBuf>,
}

impl IncludeLoader {
    /// Creates a loader with canonicalized allowed roots.
    pub fn new(roots: impl IntoIterator<Item = PathBuf>) -> Result<Self, String> {
        let mut allowed_roots = Vec::new();
        for root in roots {
            let canonical = fs::canonicalize(&root)
                .map_err(|error| format!("E_PHP_VM_INCLUDE_ROOT: {}: {error}", root.display()))?;
            if !allowed_roots.contains(&canonical) {
                allowed_roots.push(canonical);
            }
        }
        Ok(Self { allowed_roots })
    }

    /// Creates a loader that permits files under `root`.
    pub fn for_root(root: impl Into<PathBuf>) -> Result<Self, String> {
        Self::new([root.into()])
    }

    /// Returns configured roots.
    #[must_use]
    pub fn allowed_roots(&self) -> &[PathBuf] {
        &self.allowed_roots
    }

    /// Loads a file after resolving it against the including file directory and
    /// checking that the canonical path remains within an allowed root.
    pub fn load(&self, including_file: Option<&Path>, path: &str) -> Result<LoadedInclude, String> {
        if self.allowed_roots.is_empty() {
            return Err(
                "E_PHP_VM_INCLUDE_DISABLED: include loader has no allowed roots".to_owned(),
            );
        }
        if path.contains("://") {
            return Err(format!(
                "E_PHP_VM_INCLUDE_UNSUPPORTED_SCHEME: stream include `{path}` is not supported"
            ));
        }
        let raw = Path::new(path);
        let candidate = if raw.is_absolute() {
            raw.to_path_buf()
        } else if let Some(parent) = including_file.and_then(Path::parent) {
            parent.join(raw)
        } else {
            raw.to_path_buf()
        };
        let canonical = fs::canonicalize(&candidate).map_err(|error| {
            format!("E_PHP_VM_INCLUDE_MISSING: {}: {error}", candidate.display())
        })?;
        if !self
            .allowed_roots
            .iter()
            .any(|root| canonical.starts_with(root))
        {
            return Err(format!(
                "E_PHP_VM_INCLUDE_OUTSIDE_ROOT: {} is outside allowed include roots",
                canonical.display()
            ));
        }
        let source = fs::read_to_string(&canonical)
            .map_err(|error| format!("E_PHP_VM_INCLUDE_READ: {}: {error}", canonical.display()))?;
        Ok(LoadedInclude {
            canonical_path: canonical,
            source,
        })
    }
}
