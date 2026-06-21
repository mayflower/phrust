//! Cache-friendly Phase 3 frontend entry points.

use crate::hir::{HirModule, ModuleId};
use crate::{FrontendDatabase, FrontendResult, SemanticModule, TARGET_PHP_VERSION};
use php_ast::{AstNode, SourceFile};
use php_source::TextRange;
use php_syntax::{Parse, ParseContext, SourceId, parse_source_file_with_context};

/// Stable caller-owned file identity.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FileId(String);

impl FileId {
    /// Creates a file identity from a stable path, URI, or host ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the identity text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Deterministic source hash used by future parse/cache keys.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceHash(u64);

impl SourceHash {
    /// Hashes source bytes with a fixed FNV-1a variant.
    #[must_use]
    pub fn new(source: &str) -> Self {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for byte in source.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Self(hash)
    }

    /// Returns the raw hash value.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Cache key for parse results.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ParseCacheKey {
    file_id: FileId,
    source_hash: SourceHash,
    target_php_version: String,
}

impl ParseCacheKey {
    /// Creates a parse cache key from identity, source, and options.
    #[must_use]
    pub fn new(file_id: FileId, source: &str, options: &FrontendOptions) -> Self {
        Self {
            file_id,
            source_hash: SourceHash::new(source),
            target_php_version: options.target_php_version.clone(),
        }
    }

    /// Returns the file identity.
    #[must_use]
    pub const fn file_id(&self) -> &FileId {
        &self.file_id
    }

    /// Returns the source hash.
    #[must_use]
    pub const fn source_hash(&self) -> SourceHash {
        self.source_hash
    }
}

/// Options that affect semantic analysis results.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticOptions {
    /// Target PHP version string.
    pub target_php_version: String,
    /// Whether reference annotations should be retained in output.
    pub enable_reference_annotations: bool,
    /// Maximum semantic diagnostics retained by future query stages.
    pub max_diagnostics: usize,
    /// Whether known gaps should be treated strictly by callers.
    pub strict_known_gap_mode: bool,
}

impl Default for SemanticOptions {
    fn default() -> Self {
        Self {
            target_php_version: TARGET_PHP_VERSION.to_owned(),
            enable_reference_annotations: false,
            max_diagnostics: 128,
            strict_known_gap_mode: false,
        }
    }
}

/// Public frontend options.
pub type FrontendOptions = SemanticOptions;

/// Parses one source file without global mutable state.
#[must_use]
pub fn parse_file(source: &str, file_id: Option<&FileId>) -> Parse {
    let context = file_id.map_or_else(ParseContext::new, |id| {
        ParseContext::new().with_source_id(SourceId::new(id.as_str()))
    });
    parse_source_file_with_context(source, context)
}

/// Returns the typed AST root view for a parse result.
#[must_use]
pub fn ast_root(parse: &Parse) -> Option<SourceFile<'_>> {
    SourceFile::new(parse.root())
}

/// Allocates a module and records its source map entry.
#[must_use]
pub fn create_module(database: &mut FrontendDatabase, root_kind: String, source: &str) -> ModuleId {
    let module_id = database.add_module(HirModule::new(root_kind, source.len()));
    database
        .source_map_mut()
        .insert(module_id, TextRange::new(0, source.len()));
    module_id
}

/// Collects declarations. The current implementation also drives the fused
/// Phase 3 name-resolution, HIR-lowering, and check passes.
#[must_use]
pub fn collect_declarations(
    root: SourceFile,
    database: &mut FrontendDatabase,
    module_id: ModuleId,
) -> Vec<crate::SemanticDiagnostic> {
    crate::lower::declarations::collect_module_declarations(root, database, module_id)
}

/// Boundary for future standalone name-resolution queries.
#[must_use]
pub fn resolve_names() -> Vec<crate::SemanticDiagnostic> {
    Vec::new()
}

/// Boundary for future standalone HIR-lowering queries.
#[must_use]
pub fn lower_hir() -> Vec<crate::SemanticDiagnostic> {
    Vec::new()
}

/// Boundary for future standalone semantic check queries.
#[must_use]
pub fn run_checks() -> Vec<crate::SemanticDiagnostic> {
    Vec::new()
}

/// Analyzes a PHP source string using query-shaped Phase 3 frontend stages.
#[must_use]
pub fn analyze_file(source: &str, options: &FrontendOptions) -> FrontendResult {
    let file_id = FileId::new("<memory>");
    let parse = parse_file(source, Some(&file_id));
    let root = ast_root(&parse);
    let root_kind = root
        .as_ref()
        .map(|source_file| source_file.kind().name())
        .unwrap_or_else(|| parse.root().kind().name());
    let root_range = parse.root().text_range();
    let mut database = FrontendDatabase::new();
    let module_id = database.add_module(HirModule::new(root_kind.clone(), source.len()));
    database.source_map_mut().insert(module_id, root_range);
    let mut semantic_diagnostics = root.map_or_else(Vec::new, |source_file| {
        collect_declarations(source_file, &mut database, module_id)
    });
    if semantic_diagnostics.len() > options.max_diagnostics {
        semantic_diagnostics.truncate(options.max_diagnostics);
    }

    FrontendResult {
        parser_diagnostics: parse.diagnostics().to_vec(),
        semantic_diagnostics,
        module: SemanticModule {
            module_id,
            root_kind,
            source_bytes: source.len(),
        },
        database,
    }
}
