use std::path::{Path, PathBuf};

/// Parser fixture category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParserFixtureKind {
    /// Expected to be accepted by the reference parser.
    Valid,
    /// Expected to be rejected by the reference parser.
    Invalid,
    /// Expected to exercise parser recovery.
    Recovery,
    /// PHP 8.5-specific syntax fixture.
    Php85,
    /// Any other fixture group.
    Other,
}

/// Parser fixture metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserFixture {
    /// Fixture path.
    pub path: PathBuf,
    /// Fixture kind inferred from its path.
    pub kind: ParserFixtureKind,
}

impl ParserFixture {
    /// Creates fixture metadata.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        let kind = infer_kind(&path);
        Self { path, kind }
    }
}

/// Semantic fixture category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticFixtureKind {
    /// Expected to be accepted by the reference and Rust frontend.
    Valid,
    /// Expected to be rejected by the reference and Rust frontend.
    Invalid,
    /// Accepted by PHP lint but rejected by the semantic frontend.
    SemanticOnlyReject,
    /// Explicit reference reject that is not yet modeled by the semantic frontend.
    KnownGap,
    /// Any other semantic fixture group.
    Other,
}

/// Semantic fixture metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticFixture {
    /// Fixture path.
    pub path: PathBuf,
    /// Fixture kind inferred from its path and known fixture naming.
    pub kind: SemanticFixtureKind,
}

impl SemanticFixture {
    /// Creates semantic fixture metadata.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        let kind = infer_semantic_kind(&path);
        Self { path, kind }
    }
}

fn infer_kind(path: &Path) -> ParserFixtureKind {
    let path = path.to_string_lossy();
    if path.contains("/valid/") {
        ParserFixtureKind::Valid
    } else if path.contains("/invalid/") {
        ParserFixtureKind::Invalid
    } else if path.contains("/recovery/") {
        ParserFixtureKind::Recovery
    } else if path.contains("/php85/") {
        ParserFixtureKind::Php85
    } else {
        ParserFixtureKind::Other
    }
}

fn infer_semantic_kind(path: &Path) -> SemanticFixtureKind {
    let path = path.to_string_lossy();
    if path.contains("goto-invalid-known-gap.php") || path.contains("promotion-invalid.php") {
        SemanticFixtureKind::KnownGap
    } else if path.contains("duplicate-class-invalid.php") {
        SemanticFixtureKind::SemanticOnlyReject
    } else if path.contains("/valid/") {
        SemanticFixtureKind::Valid
    } else if path.contains("/invalid/") || path.contains("-invalid.php") {
        SemanticFixtureKind::Invalid
    } else {
        SemanticFixtureKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::{ParserFixture, ParserFixtureKind, SemanticFixture, SemanticFixtureKind};
    use std::path::PathBuf;

    #[test]
    fn infers_fixture_kind_from_path() {
        let fixture = ParserFixture::new(PathBuf::from("fixtures/parser/invalid/missing.php"));
        assert_eq!(fixture.kind, ParserFixtureKind::Invalid);
    }

    #[test]
    fn infers_semantic_fixture_kind_from_path() {
        let invalid = SemanticFixture::new(PathBuf::from(
            "fixtures/semantic/functions/duplicate-param-invalid.php",
        ));
        assert_eq!(invalid.kind, SemanticFixtureKind::Invalid);

        let semantic_only = SemanticFixture::new(PathBuf::from(
            "fixtures/semantic/declarations/duplicate-class-invalid.php",
        ));
        assert_eq!(semantic_only.kind, SemanticFixtureKind::SemanticOnlyReject);

        let known_gap = SemanticFixture::new(PathBuf::from(
            "fixtures/semantic/control_flow/goto-invalid-known-gap.php",
        ));
        assert_eq!(known_gap.kind, SemanticFixtureKind::KnownGap);
    }
}
