//! Phase 5 runtime-semantics fixture metadata.

use serde::{Deserialize, Serialize};

/// Canonical Phase 5 runtime fixture categories.
pub const PHASE5_FIXTURE_CATEGORIES: &[&str] = &[
    "refs",
    "cow",
    "arrays",
    "foreach",
    "functions",
    "closures",
    "callables",
    "objects",
    "traits",
    "enums",
    "magic",
    "properties",
    "property_hooks",
    "clone_with",
    "void_cast",
    "const_expr",
    "generators",
    "fibers",
    "reflection",
    "errors",
    "destructors",
    "gc",
    "include_eval_autoload",
    "globals",
    "real_world",
    "regressions",
    "known_gaps",
];

/// Machine-readable Phase 5 differential summary counters.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Phase5DiffSummary {
    /// Number of fixture comparisons selected.
    pub total: usize,
    /// Number of exact matches or expected runtime failures.
    pub pass: usize,
    /// Number of unexpected mismatches or harness errors.
    pub fail: usize,
    /// Number of comparisons skipped because prerequisites were unavailable.
    pub skip: usize,
    /// Number of explicitly marked known-gap fixtures.
    pub known_gap: usize,
}

/// Returns true when `category` is part of the Phase 5 fixture matrix.
#[must_use]
pub fn is_phase5_category(category: &str) -> bool {
    PHASE5_FIXTURE_CATEGORIES.contains(&category)
}

#[cfg(test)]
mod tests {
    use super::{PHASE5_FIXTURE_CATEGORIES, Phase5DiffSummary, is_phase5_category};

    #[test]
    fn phase5_categories_match_prompt_matrix() {
        assert_eq!(PHASE5_FIXTURE_CATEGORIES.len(), 27);
        assert!(is_phase5_category("refs"));
        assert!(is_phase5_category("errors"));
        assert!(is_phase5_category("destructors"));
        assert!(is_phase5_category("gc"));
        assert!(is_phase5_category("include_eval_autoload"));
        assert!(is_phase5_category("globals"));
        assert!(is_phase5_category("real_world"));
        assert!(is_phase5_category("regressions"));
        assert!(is_phase5_category("known_gaps"));
        assert!(!is_phase5_category("phase4"));
    }

    #[test]
    fn phase5_summary_serializes_machine_readable_counters() {
        let summary = Phase5DiffSummary {
            total: 4,
            pass: 1,
            fail: 1,
            skip: 1,
            known_gap: 1,
        };
        let json = serde_json::to_string(&summary).expect("json");
        assert!(json.contains("\"known_gap\":1"));
    }
}
