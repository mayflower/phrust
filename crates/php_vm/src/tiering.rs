//! Request-local tiering policy and stats for performance adaptive execution.

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
    /// Compile immediately when JIT execution is enabled; intended for tests.
    pub jit_eager: bool,
    /// Maximum native compile time budget for one request, in microseconds.
    /// `u64::MAX` means no practical budget limit.
    pub jit_max_compile_us: u64,
    /// Maximum number of functions that may be compiled in one request.
    /// `u64::MAX` means no practical budget limit.
    pub jit_max_functions: u64,
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
            jit_eager: false,
            jit_max_compile_us: u64::MAX,
            jit_max_functions: u64::MAX,
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
    pub jit_cold_entries: u64,
    pub jit_eager_candidates: u64,
    pub jit_threshold_candidates: u64,
    pub jit_blacklist_rejections: u64,
    pub jit_compile_budget_rejections: u64,
    pub jit_compile_budget_used_us: u64,
    pub jit_compiled_functions: u64,
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
                "  \"tiering_disabled_entries\": {},\n",
                "  \"jit_cold_entries\": {},\n",
                "  \"jit_eager_candidates\": {},\n",
                "  \"jit_threshold_candidates\": {},\n",
                "  \"jit_blacklist_rejections\": {},\n",
                "  \"jit_compile_budget_rejections\": {},\n",
                "  \"jit_compile_budget_used_us\": {},\n",
                "  \"jit_compiled_functions\": {}\n",
                "}}\n"
            ),
            self.function_entry_count,
            self.loop_backedge_count,
            self.ic_stability_score,
            self.guard_failure_score,
            self.tier0_interpreter_entries,
            self.tier1_quickened_entries,
            self.tier2_jit_candidates,
            self.tiering_disabled_entries,
            self.jit_cold_entries,
            self.jit_eager_candidates,
            self.jit_threshold_candidates,
            self.jit_blacklist_rejections,
            self.jit_compile_budget_rejections,
            self.jit_compile_budget_used_us,
            self.jit_compiled_functions
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

        let jit_enabled = matches!(jit, JitMode::Cranelift);
        let hot_by_entry = hotness.entries >= self.options.function_entry_threshold;
        let hot_by_backedge = hotness.backedges >= self.options.loop_backedge_threshold;
        let guards_stable = self.stats.guard_failure_score < self.options.guard_failure_threshold;
        if jit_enabled
            && guards_stable
            && (self.options.jit_eager || hot_by_entry || hot_by_backedge)
        {
            self.stats.tier2_jit_candidates = self.stats.tier2_jit_candidates.saturating_add(1);
            if self.options.jit_eager {
                self.stats.jit_eager_candidates = self.stats.jit_eager_candidates.saturating_add(1);
            } else {
                self.stats.jit_threshold_candidates =
                    self.stats.jit_threshold_candidates.saturating_add(1);
            }
            ExecutionTier::Jit
        } else if quickening.enabled() && (hot_by_entry || hot_by_backedge) && guards_stable {
            self.stats.tier1_quickened_entries =
                self.stats.tier1_quickened_entries.saturating_add(1);
            ExecutionTier::Quickened
        } else {
            if jit_enabled && guards_stable && !self.options.jit_eager {
                self.stats.jit_cold_entries = self.stats.jit_cold_entries.saturating_add(1);
            }
            self.stats.tier0_interpreter_entries =
                self.stats.tier0_interpreter_entries.saturating_add(1);
            ExecutionTier::Interpreter
        }
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

    pub fn record_jit_blacklist_rejection(&mut self) {
        self.stats.jit_blacklist_rejections = self.stats.jit_blacklist_rejections.saturating_add(1);
    }

    pub fn record_jit_compile_budget_rejection(&mut self) {
        self.stats.jit_compile_budget_rejections =
            self.stats.jit_compile_budget_rejections.saturating_add(1);
    }

    pub fn record_jit_compiled_function(&mut self, compile_time_nanos: u64) {
        self.stats.jit_compiled_functions = self.stats.jit_compiled_functions.saturating_add(1);
        let micros = compile_time_nanos.saturating_add(999) / 1_000;
        self.stats.jit_compile_budget_used_us =
            self.stats.jit_compile_budget_used_us.saturating_add(micros);
    }

    #[must_use]
    pub fn jit_compile_budget_used_us(&self) -> u64 {
        self.stats.jit_compile_budget_used_us
    }

    #[must_use]
    pub fn jit_compiled_functions(&self) -> u64 {
        self.stats.jit_compiled_functions
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
            state.record_function_entry(FunctionId::new(1), QuickeningMode::On, JitMode::Cranelift),
            ExecutionTier::Interpreter
        );
        assert_eq!(state.stats().tiering_disabled_entries, 1);
        assert_eq!(state.stats().tier2_jit_candidates, 0);
    }

    #[test]
    fn eager_policy_promotes_first_entry_for_tests() {
        let mut state = TieringState::new(TieringOptions {
            jit_eager: true,
            function_entry_threshold: 10,
            ..TieringOptions::default()
        });

        assert_eq!(
            state.record_function_entry(
                FunctionId::new(1),
                QuickeningMode::Off,
                JitMode::Cranelift
            ),
            ExecutionTier::Jit
        );
        assert_eq!(state.stats().jit_eager_candidates, 1);
        assert_eq!(state.stats().jit_cold_entries, 0);
    }

    #[test]
    fn cold_jit_entry_stays_interpreter_until_threshold() {
        let mut state = TieringState::new(TieringOptions {
            function_entry_threshold: 2,
            ..TieringOptions::default()
        });

        assert_eq!(
            state.record_function_entry(
                FunctionId::new(1),
                QuickeningMode::Off,
                JitMode::Cranelift
            ),
            ExecutionTier::Interpreter
        );
        assert_eq!(
            state.record_function_entry(
                FunctionId::new(1),
                QuickeningMode::Off,
                JitMode::Cranelift
            ),
            ExecutionTier::Jit
        );
        assert_eq!(state.stats().jit_cold_entries, 1);
        assert_eq!(state.stats().jit_threshold_candidates, 1);
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
            state.record_function_entry(FunctionId::new(1), QuickeningMode::On, JitMode::Cranelift),
            ExecutionTier::Interpreter
        );
        assert_eq!(state.stats().guard_failure_score, 2);
        assert_eq!(state.stats().tier2_jit_candidates, 0);
    }
}
