//! Local include/require resolution, caching, compilation port, and metadata.
//!
//! Source ownership is intentionally one-way:
//! `source/diagnostics -> resolver/compiler -> resolution/compiled caches -> cache facade`.
//! The concrete frontend/lowering/optimizer implementation lives in
//! `php_executor`; no include module may import those crates.

mod cache;
mod compiled_cache;
mod compiler;
mod diagnostics;
mod metadata;
mod metrics;
mod resolution_cache;
mod resolver;
mod source;

pub use cache::{
    IncludeCache, SERVER_INCLUDE_REVALIDATION_INTERVAL, include_revalidation_interval_from_env,
};
pub use compiler::{CompiledInclude, IncludeCompiler, IncludeCompilerFingerprint};
pub use metadata::{
    ComposerFingerprintTransition, DeploymentRootFingerprint, DeploymentRootMode,
    composer_autoload_map_fingerprint,
};
pub use metrics::IncludeCacheStats;
pub use resolver::{IncludeLoader, ResolvedIncludePath, negative_include_cache_enabled};
pub(crate) use source::resolution_path_targets;
pub use source::{
    IncludeDependency, IncludeDirectoryVersion, IncludePathFileFingerprint, LoadedInclude,
    ValidatedIncludeSource, fnv1a_64, include_directory_version, include_path_file_fingerprint,
};

#[cfg(test)]
mod tests;
