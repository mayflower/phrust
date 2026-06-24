//! Local include/require loader for the runtime VM MVP.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Result of loading one include target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedInclude {
    /// Canonical path used for once tracking and source maps.
    pub canonical_path: PathBuf,
    /// PHP source text.
    pub source: String,
}

/// Metadata fingerprint used to validate cached include-path resolutions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncludePathFileFingerprint {
    pub len: u64,
    pub modified_unix_nanos: Option<u128>,
    pub readonly: bool,
}

/// Result of resolving one include target without loading its contents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedIncludePath {
    /// Canonical path used for once tracking and source maps.
    pub canonical_path: PathBuf,
    /// File metadata fingerprint used to invalidate stale path resolutions.
    pub fingerprint: IncludePathFileFingerprint,
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
        self.load_with_include_path(including_file, path, &[], None)
    }

    /// Loads a file using PHP-style include_path candidates for relative paths,
    /// then applies the same allowed-root check as `load`.
    pub fn load_with_include_path(
        &self,
        including_file: Option<&Path>,
        path: &str,
        include_path: &[PathBuf],
        cwd: Option<&Path>,
    ) -> Result<LoadedInclude, String> {
        let resolved = self.resolve_with_include_path(including_file, path, include_path, cwd)?;
        self.load_resolved(resolved.canonical_path)
    }

    /// Resolves a file using PHP-style include_path candidates without reading
    /// or executing file contents.
    pub fn resolve_with_include_path(
        &self,
        including_file: Option<&Path>,
        path: &str,
        include_path: &[PathBuf],
        cwd: Option<&Path>,
    ) -> Result<ResolvedIncludePath, String> {
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
        let mut candidates = Vec::new();
        if raw.is_absolute() {
            candidates.push(raw.to_path_buf());
        } else {
            let base = including_file.and_then(Path::parent);
            for entry in include_path {
                candidates.push(resolve_include_path_entry(base, cwd, entry).join(raw));
            }
            if let Some(parent) = base {
                candidates.push(parent.join(raw));
            }
            if let Some(cwd) = cwd {
                candidates.push(cwd.join(raw));
            }
            candidates.push(raw.to_path_buf());
        }
        let mut last_error = None;
        let mut canonical = None;
        for candidate in candidates {
            match fs::canonicalize(&candidate) {
                Ok(path) => {
                    canonical = Some(path);
                    break;
                }
                Err(error) => {
                    last_error = Some(format!(
                        "E_PHP_VM_INCLUDE_MISSING: {}: {error}",
                        candidate.display()
                    ));
                }
            }
        }
        let canonical = canonical.ok_or_else(|| {
            last_error.unwrap_or_else(|| format!("E_PHP_VM_INCLUDE_MISSING: {path}: not found"))
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
        let fingerprint = include_path_file_fingerprint(&canonical)?;
        Ok(ResolvedIncludePath {
            canonical_path: canonical,
            fingerprint,
        })
    }

    /// Loads a previously resolved canonical include path, rechecking that the
    /// path remains inside an allowed root.
    pub fn load_resolved(&self, canonical: PathBuf) -> Result<LoadedInclude, String> {
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

pub fn include_path_file_fingerprint(path: &Path) -> Result<IncludePathFileFingerprint, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("E_PHP_VM_INCLUDE_METADATA: {}: {error}", path.display()))?;
    let modified_unix_nanos = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos());
    Ok(IncludePathFileFingerprint {
        len: metadata.len(),
        modified_unix_nanos,
        readonly: metadata.permissions().readonly(),
    })
}

fn resolve_include_path_entry(base: Option<&Path>, cwd: Option<&Path>, entry: &Path) -> PathBuf {
    if entry.is_absolute() {
        return entry.to_path_buf();
    }
    if entry == Path::new(".") {
        if let Some(base) = base {
            return base.to_path_buf();
        }
        if let Some(cwd) = cwd {
            return cwd.to_path_buf();
        }
    }
    if let Some(cwd) = cwd {
        return cwd.join(entry);
    }
    if let Some(base) = base {
        return base.join(entry);
    }
    entry.to_path_buf()
}
