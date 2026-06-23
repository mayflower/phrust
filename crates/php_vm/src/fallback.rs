//! Shared adaptive guard/fallback protocol for Phase 7 fast paths.

/// Default number of guarded fallback calls before an installed specialization
/// is dequickened into a baseline-only state.
pub const DEQUICKEN_AFTER_GUARD_MISSES: u64 = 2;

/// Default number of guarded fallback calls before an inline-cache slot stops
/// attempting target refreshes for the request.
pub const DISABLE_AFTER_GUARD_MISSES: u64 = 4;

/// Uniform per-entry protocol stats for quickening entries and IC slots.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FallbackProtocolStats {
    pub guard_hits: u64,
    pub guard_misses: u64,
    pub guard_failures: u64,
    pub fallback_calls: u64,
    pub dequickens: u64,
    pub megamorphic_transitions: u64,
    pub disabled_transitions: u64,
}

/// Result of applying one guard/fallback protocol transition.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FallbackProtocolEvent {
    pub guard_hit: bool,
    pub guard_miss: bool,
    pub guard_failure: bool,
    pub fallback_call: bool,
    pub dequickened: bool,
    pub megamorphic: bool,
    pub disabled: bool,
}

impl FallbackProtocolStats {
    /// Records a successful guard. The caller may run the specialized path.
    pub fn record_guard_hit(&mut self) -> FallbackProtocolEvent {
        self.guard_hits = self.guard_hits.saturating_add(1);
        FallbackProtocolEvent {
            guard_hit: true,
            ..FallbackProtocolEvent::default()
        }
    }

    /// Records a guarded fallback. The caller must run the generic path once.
    pub fn record_guard_fallback(&mut self) -> FallbackProtocolEvent {
        self.guard_misses = self.guard_misses.saturating_add(1);
        self.guard_failures = self.guard_failures.saturating_add(1);
        self.fallback_calls = self.fallback_calls.saturating_add(1);
        FallbackProtocolEvent {
            guard_miss: true,
            guard_failure: true,
            fallback_call: true,
            ..FallbackProtocolEvent::default()
        }
    }

    /// Records a cold or empty-slot fallback that is not a guard failure.
    pub fn record_cold_fallback(&mut self) -> FallbackProtocolEvent {
        self.guard_misses = self.guard_misses.saturating_add(1);
        self.fallback_calls = self.fallback_calls.saturating_add(1);
        FallbackProtocolEvent {
            guard_miss: true,
            fallback_call: true,
            ..FallbackProtocolEvent::default()
        }
    }

    /// Marks a quickened entry as dequickened/megamorphic for this request.
    pub fn record_dequicken(&mut self) -> FallbackProtocolEvent {
        self.dequickens = self.dequickens.saturating_add(1);
        self.megamorphic_transitions = self.megamorphic_transitions.saturating_add(1);
        FallbackProtocolEvent {
            dequickened: true,
            megamorphic: true,
            ..FallbackProtocolEvent::default()
        }
    }

    /// Marks an adaptive slot disabled for this request.
    pub fn record_disabled(&mut self) -> FallbackProtocolEvent {
        self.disabled_transitions = self.disabled_transitions.saturating_add(1);
        FallbackProtocolEvent {
            disabled: true,
            ..FallbackProtocolEvent::default()
        }
    }
}

impl FallbackProtocolEvent {
    #[must_use]
    pub const fn merge(self, other: Self) -> Self {
        Self {
            guard_hit: self.guard_hit || other.guard_hit,
            guard_miss: self.guard_miss || other.guard_miss,
            guard_failure: self.guard_failure || other.guard_failure,
            fallback_call: self.fallback_call || other.fallback_call,
            dequickened: self.dequickened || other.dequickened,
            megamorphic: self.megamorphic || other.megamorphic,
            disabled: self.disabled || other.disabled,
        }
    }
}
