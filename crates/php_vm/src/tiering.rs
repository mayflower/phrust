//! Request-local tiering policy and stats for Phase 7 adaptive execution.

use std::collections::BTreeMap;

use php_ir::ids::{BlockId, FunctionId};

use crate::{InlineCacheObservation, JitMode, QuickeningMode, QuickeningObservation};

/// Runtime tier selected by the policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionTier {
    /// Baseline interpreter.
    Interpreter,
    /// Quickened interpreter.
    Quickened,
    /// Experimental feature-gated JIT.
    Jit,
}

/// Configurable tiering thresholds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TieringOptions {
    /// Disable all adaptive tiering when false.
    pub enabled: bool,
    /// Collect request-local tiering stats.
    pub collect_stats: bool,
    /// Function entries required before the policy considers Tier 1.
    pub function_entry_threshold: u64,
    /// Loop backedges required before the policy considers Tier 1.
    pub loop_backedge_threshold: u64,
    /// IC hit score required before the policy considers a site stable.
    pub ic_stability_threshold: i64,
    /// Guard failures after which a site is treated as unstable.
    pub guard_failure_threshold: u64,
}

impl Default for TieringOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            collect_stats: false,
            function_entry_threshold: 8,
            loop_backedge_threshold: 8,
            ic_stability_threshold: 4,
            guard_failure_threshold: 2,
        }
    }
}

/// Visible request-local tiering stats.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TieringStats {
    pub function_entry_count: u64,
    pub loop_backedge_count: u64,
    pub ic_stability_score: i64,
    pub guard_failure_score: u64,
    pub tier0_interpreter_entries: u64,
    pub tier1_quickened_entries: u64,
    pub tier2_jit_candidates: u64,
    pub tiering_disabled_entries: u64,
}

impl TieringStats {
    #[must_use]
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"schema_version\": 1,\n",
                "  \"function_entry_count\": {},\n",
                "  \"loop_backedge_count\": {},\n",
                "  \"ic_stability_score\": {},\n",
                "  \"guard_failure_score\": {},\n",
                "  \"tier0_interpreter_entries\": {},\n",
                "  \"tier1_quickened_entries\": {},\n",
                "  \"tier2_jit_candidates\": {},\n",
                "  \"tiering_disabled_entries\": {}\n",
                "}}\n"
            ),
            self.function_entry_count,
            self.loop_backedge_count,
            self.ic_stability_score,
            self.guard_failure_score,
            self.tier0_interpreter_entries,
            self.tier1_quickened_entries,
            self.tier2_jit_candidates,
            self.tiering_disabled_entries
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct FunctionHotness {
    entries: u64,
    backedges: u64,
}

/// Request-local tiering state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TieringState {
    options: TieringOptions,
    stats: TieringStats,
    functions: BTreeMap<u32, FunctionHotness>,
}

impl TieringState {
    #[must_use]
    pub fn new(options: TieringOptions) -> Self {
        Self {
            options,
            stats: TieringStats::default(),
            functions: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn stats(&self) -> TieringStats {
        self.stats.clone()
    }

    pub fn record_function_entry(
        &mut self,
        function: FunctionId,
        quickening: QuickeningMode,
        jit: JitMode,
    ) -> ExecutionTier {
        if !self.options.enabled {
            self.stats.tiering_disabled_entries =
                self.stats.tiering_disabled_entries.saturating_add(1);
            return ExecutionTier::Interpreter;
        }

        self.stats.function_entry_count = self.stats.function_entry_count.saturating_add(1);
        let hotness = self.functions.entry(function.raw()).or_default();
        hotness.entries = hotness.entries.saturating_add(1);

        let tier = if jit == JitMode::On
            && hotness.entries >= self.options.function_entry_threshold
            && self.stats.guard_failure_score < self.options.guard_failure_threshold
        {
            self.stats.tier2_jit_candidates = self.stats.tier2_jit_candidates.saturating_add(1);
            ExecutionTier::Jit
        } else if quickening.enabled()
            && (hotness.entries >= self.options.function_entry_threshold
                || hotness.backedges >= self.options.loop_backedge_threshold)
            && self.stats.guard_failure_score < self.options.guard_failure_threshold
        {
            self.stats.tier1_quickened_entries =
                self.stats.tier1_quickened_entries.saturating_add(1);
            ExecutionTier::Quickened
        } else {
            self.stats.tier0_interpreter_entries =
                self.stats.tier0_interpreter_entries.saturating_add(1);
            ExecutionTier::Interpreter
        };
        tier
    }

    pub fn record_loop_backedge(
        &mut self,
        function: FunctionId,
        current: BlockId,
        target: BlockId,
    ) {
        if !self.options.enabled || target.raw() > current.raw() {
            return;
        }
        self.stats.loop_backedge_count = self.stats.loop_backedge_count.saturating_add(1);
        let hotness = self.functions.entry(function.raw()).or_default();
        hotness.backedges = hotness.backedges.saturating_add(1);
    }

    pub fn record_quickening(&mut self, observation: QuickeningObservation) {
        if !self.options.enabled {
            return;
        }
        if observation.guard_hit || observation.specialized {
            self.stats.ic_stability_score = self.stats.ic_stability_score.saturating_add(1);
        }
        if observation.guard_failure {
            self.stats.guard_failure_score = self.stats.guard_failure_score.saturating_add(1);
        }
    }

    pub fn record_inline_cache(&mut self, observation: InlineCacheObservation) {
        if !self.options.enabled {
            return;
        }
        if observation.hit {
            self.stats.ic_stability_score = self.stats.ic_stability_score.saturating_add(1);
        }
        if observation.guard_failure {
            self.stats.guard_failure_score = self.stats.guard_failure_score.saturating_add(1);
        }
    }
}

impl Default for TieringState {
    fn default() -> Self {
        Self::new(TieringOptions::default())
    }
}

#[cfg(test)]
mod tests {
    use super::{ExecutionTier, TieringOptions, TieringState};
    use crate::{InlineCacheObservation, JitMode, QuickeningMode, QuickeningObservation};
    use php_ir::ids::{BlockId, FunctionId};

    #[test]
    fn policy_promotes_quickening_after_entry_threshold() {
        let mut state = TieringState::new(TieringOptions {
            function_entry_threshold: 2,
            ..TieringOptions::default()
        });

        assert_eq!(
            state.record_function_entry(FunctionId::new(1), QuickeningMode::On, JitMode::Off),
            ExecutionTier::Interpreter
        );
        assert_eq!(
            state.record_function_entry(FunctionId::new(1), QuickeningMode::On, JitMode::Off),
            ExecutionTier::Quickened
        );
        assert_eq!(state.stats().tier1_quickened_entries, 1);
    }

    #[test]
    fn disabled_policy_stays_interpreter() {
        let mut state = TieringState::new(TieringOptions {
            enabled: false,
            function_entry_threshold: 1,
            ..TieringOptions::default()
        });

        assert_eq!(
            state.record_function_entry(FunctionId::new(1), QuickeningMode::On, JitMode::On),
            ExecutionTier::Interpreter
        );
        assert_eq!(state.stats().tiering_disabled_entries, 1);
        assert_eq!(state.stats().tier2_jit_candidates, 0);
    }

    #[test]
    fn backedge_hotness_is_counted() {
        let mut state = TieringState::new(TieringOptions::default());

        state.record_loop_backedge(FunctionId::new(1), BlockId::new(3), BlockId::new(1));
        state.record_loop_backedge(FunctionId::new(1), BlockId::new(1), BlockId::new(3));

        assert_eq!(state.stats().loop_backedge_count, 1);
    }

    #[test]
    fn megamorphic_guard_failures_stay_interpreter() {
        let mut state = TieringState::new(TieringOptions {
            function_entry_threshold: 1,
            guard_failure_threshold: 2,
            ..TieringOptions::default()
        });

        state.record_quickening(QuickeningObservation {
            attempt: true,
            specialized: false,
            guard_hit: false,
            guard_miss: true,
            guard_failure: true,
            fallback_call: true,
            dequickened: false,
            megamorphic: false,
            disabled: false,
        });
        state.record_inline_cache(InlineCacheObservation {
            candidate: true,
            slot_allocated: true,
            kind: None,
            hit: false,
            miss: true,
            guard_failure: true,
            invalidation: false,
            fallback_call: true,
            monomorphic: false,
            polymorphic: false,
            megamorphic: true,
            disabled: false,
        });

        assert_eq!(
            state.record_function_entry(FunctionId::new(1), QuickeningMode::On, JitMode::On),
            ExecutionTier::Interpreter
        );
        assert_eq!(state.stats().guard_failure_score, 2);
        assert_eq!(state.stats().tier2_jit_candidates, 0);
    }
}
