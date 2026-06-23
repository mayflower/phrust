//! Request-local quickening side table for Phase 7 adaptive execution.

use std::collections::BTreeMap;

use php_ir::ids::{BlockId, FunctionId, InstrId};

use crate::fallback::{DEQUICKEN_AFTER_GUARD_MISSES, FallbackProtocolStats};

const SPECIALIZE_AFTER_EXECUTIONS: u64 = 8;

/// Quickening runtime mode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum QuickeningMode {
    /// Do not create or update quickening state.
    #[default]
    Off,
    /// Maintain request-local quickening metadata without changing semantics.
    On,
}

impl QuickeningMode {
    #[must_use]
    pub const fn enabled(self) -> bool {
        matches!(self, Self::On)
    }
}

/// Adaptive state for one instruction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuickeningState {
    Cold,
    Warming,
    Specialized,
    Megamorphic,
    Disabled,
}

/// Concrete quickening specialization installed for one instruction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuickeningSpecialization {
    AddIntInt,
    ConcatStringString,
    PackedArrayIntKey,
}

/// Result of observing one instruction dispatch.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QuickeningObservation {
    pub attempt: bool,
    pub specialized: bool,
    pub guard_hit: bool,
    pub guard_miss: bool,
    pub guard_failure: bool,
    pub fallback_call: bool,
    pub dequickened: bool,
    pub megamorphic: bool,
    pub disabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct QuickeningKey {
    function: u32,
    block: u32,
    instruction: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QuickeningEntry {
    state: QuickeningState,
    executions: u64,
    specialization: Option<QuickeningSpecialization>,
    stats: FallbackProtocolStats,
}

impl Default for QuickeningEntry {
    fn default() -> Self {
        Self {
            state: QuickeningState::Cold,
            executions: 0,
            specialization: None,
            stats: FallbackProtocolStats::default(),
        }
    }
}

/// Per-request quickening metadata table.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct QuickeningTable {
    entries: BTreeMap<QuickeningKey, QuickeningEntry>,
}

impl QuickeningTable {
    /// Observes one instruction dispatch and updates metadata only.
    pub fn observe(
        &mut self,
        function: FunctionId,
        block: BlockId,
        instruction: InstrId,
    ) -> QuickeningObservation {
        let key = quickening_key(function, block, instruction);
        let entry = self.entries.entry(key).or_default();
        entry.executions = entry.executions.saturating_add(1);
        match entry.state {
            QuickeningState::Cold => {
                entry.state = QuickeningState::Warming;
                QuickeningObservation {
                    attempt: true,
                    ..QuickeningObservation::default()
                }
            }
            QuickeningState::Warming => {
                if entry.executions >= SPECIALIZE_AFTER_EXECUTIONS {
                    entry.state = QuickeningState::Specialized;
                    QuickeningObservation {
                        attempt: true,
                        specialized: true,
                        ..QuickeningObservation::default()
                    }
                } else {
                    QuickeningObservation {
                        attempt: true,
                        ..QuickeningObservation::default()
                    }
                }
            }
            QuickeningState::Specialized | QuickeningState::Megamorphic => QuickeningObservation {
                attempt: true,
                ..QuickeningObservation::default()
            },
            QuickeningState::Disabled => QuickeningObservation::default(),
        }
    }

    /// Returns the specialization currently installed for one instruction.
    #[must_use]
    pub fn specialization(
        &self,
        function: FunctionId,
        block: BlockId,
        instruction: InstrId,
    ) -> Option<QuickeningSpecialization> {
        self.entries
            .get(&quickening_key(function, block, instruction))
            .and_then(|entry| entry.specialization)
    }

    /// Returns the adaptive state for one instruction.
    #[must_use]
    pub fn state(
        &self,
        function: FunctionId,
        block: BlockId,
        instruction: InstrId,
    ) -> Option<QuickeningState> {
        self.entries
            .get(&quickening_key(function, block, instruction))
            .map(|entry| entry.state)
    }

    /// Applies the shared guard/fallback protocol for an installed
    /// specialization.
    pub fn record_specialized_guard(
        &mut self,
        function: FunctionId,
        block: BlockId,
        instruction: InstrId,
        hit: bool,
    ) -> QuickeningObservation {
        let key = quickening_key(function, block, instruction);
        let Some(entry) = self.entries.get_mut(&key) else {
            return QuickeningObservation::default();
        };
        if hit {
            let event = entry.stats.record_guard_hit();
            return QuickeningObservation {
                guard_hit: event.guard_hit,
                ..QuickeningObservation::default()
            };
        }

        let fallback = entry.stats.record_guard_fallback();
        let mut dequickened = false;
        let mut megamorphic = false;
        if entry.state == QuickeningState::Specialized
            && entry.stats.guard_failures >= DEQUICKEN_AFTER_GUARD_MISSES
        {
            let event = entry.stats.record_dequicken();
            entry.state = QuickeningState::Megamorphic;
            entry.specialization = None;
            dequickened = event.dequickened;
            megamorphic = event.megamorphic;
        }

        QuickeningObservation {
            guard_miss: fallback.guard_miss,
            guard_failure: fallback.guard_failure,
            fallback_call: fallback.fallback_call,
            dequickened,
            megamorphic,
            ..QuickeningObservation::default()
        }
    }

    /// Installs the ADD_INT_INT specialization after the generic instruction is hot.
    pub fn observe_add_int_int_candidate(
        &mut self,
        function: FunctionId,
        block: BlockId,
        instruction: InstrId,
    ) -> QuickeningObservation {
        let entry = self
            .entries
            .entry(quickening_key(function, block, instruction))
            .or_default();
        if entry.state == QuickeningState::Specialized && entry.specialization.is_none() {
            entry.specialization = Some(QuickeningSpecialization::AddIntInt);
            return QuickeningObservation {
                specialized: true,
                ..QuickeningObservation::default()
            };
        }
        QuickeningObservation::default()
    }

    /// Installs the CONCAT_STRING_STRING specialization after the instruction is hot.
    pub fn observe_concat_string_string_candidate(
        &mut self,
        function: FunctionId,
        block: BlockId,
        instruction: InstrId,
    ) -> QuickeningObservation {
        let entry = self
            .entries
            .entry(quickening_key(function, block, instruction))
            .or_default();
        if entry.state == QuickeningState::Specialized && entry.specialization.is_none() {
            entry.specialization = Some(QuickeningSpecialization::ConcatStringString);
            return QuickeningObservation {
                specialized: true,
                ..QuickeningObservation::default()
            };
        }
        QuickeningObservation::default()
    }

    /// Installs the PACKED_ARRAY_INT_KEY specialization after the instruction is hot.
    pub fn observe_packed_array_int_key_candidate(
        &mut self,
        function: FunctionId,
        block: BlockId,
        instruction: InstrId,
    ) -> QuickeningObservation {
        let entry = self
            .entries
            .entry(quickening_key(function, block, instruction))
            .or_default();
        if entry.state == QuickeningState::Specialized && entry.specialization.is_none() {
            entry.specialization = Some(QuickeningSpecialization::PackedArrayIntKey);
            return QuickeningObservation {
                specialized: true,
                ..QuickeningObservation::default()
            };
        }
        QuickeningObservation::default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn quickening_key(function: FunctionId, block: BlockId, instruction: InstrId) -> QuickeningKey {
    QuickeningKey {
        function: function.raw(),
        block: block.raw(),
        instruction: instruction.raw(),
    }
}

#[cfg(test)]
mod tests {
    use super::{QuickeningSpecialization, QuickeningState, QuickeningTable};
    use php_ir::ids::{BlockId, FunctionId, InstrId};

    #[test]
    fn quickening_table_warms_then_marks_metadata_specialized() {
        let mut table = QuickeningTable::default();
        let function = FunctionId::new(0);
        let block = BlockId::new(0);
        let instruction = InstrId::new(0);

        for _ in 0..7 {
            let observation = table.observe(function, block, instruction);
            assert!(observation.attempt);
            assert!(!observation.specialized);
        }

        let last = table.observe(function, block, instruction);
        assert!(last.attempt);
        assert!(last.specialized);
        assert_eq!(table.len(), 1);
        let entry = table.entries.values().next().expect("entry");
        assert_eq!(entry.state, QuickeningState::Specialized);
    }

    #[test]
    fn quickening_table_tracks_distinct_instructions() {
        let mut table = QuickeningTable::default();

        table.observe(FunctionId::new(0), BlockId::new(0), InstrId::new(0));
        table.observe(FunctionId::new(0), BlockId::new(0), InstrId::new(1));

        assert_eq!(table.len(), 2);
    }

    #[test]
    fn quickening_table_installs_add_int_int_after_warmup() {
        let mut table = QuickeningTable::default();
        let function = FunctionId::new(0);
        let block = BlockId::new(0);
        let instruction = InstrId::new(0);

        for _ in 0..8 {
            table.observe(function, block, instruction);
        }

        let observation = table.observe_add_int_int_candidate(function, block, instruction);

        assert!(observation.specialized);
        assert_eq!(
            table.specialization(function, block, instruction),
            Some(QuickeningSpecialization::AddIntInt)
        );
    }

    #[test]
    fn quickening_table_installs_concat_string_string_after_warmup() {
        let mut table = QuickeningTable::default();
        let function = FunctionId::new(0);
        let block = BlockId::new(0);
        let instruction = InstrId::new(0);

        for _ in 0..8 {
            table.observe(function, block, instruction);
        }

        let observation =
            table.observe_concat_string_string_candidate(function, block, instruction);

        assert!(observation.specialized);
        assert_eq!(
            table.specialization(function, block, instruction),
            Some(QuickeningSpecialization::ConcatStringString)
        );
    }

    #[test]
    fn quickening_table_installs_packed_array_int_key_after_warmup() {
        let mut table = QuickeningTable::default();
        let function = FunctionId::new(0);
        let block = BlockId::new(0);
        let instruction = InstrId::new(0);

        for _ in 0..8 {
            table.observe(function, block, instruction);
        }

        let observation =
            table.observe_packed_array_int_key_candidate(function, block, instruction);

        assert!(observation.specialized);
        assert_eq!(
            table.specialization(function, block, instruction),
            Some(QuickeningSpecialization::PackedArrayIntKey)
        );
    }

    #[test]
    fn quickening_guard_fallback_dequickens_to_megamorphic() {
        let mut table = QuickeningTable::default();
        let function = FunctionId::new(0);
        let block = BlockId::new(0);
        let instruction = InstrId::new(0);

        for _ in 0..8 {
            table.observe(function, block, instruction);
        }
        table.observe_add_int_int_candidate(function, block, instruction);

        let first = table.record_specialized_guard(function, block, instruction, false);
        assert!(first.guard_miss);
        assert!(first.guard_failure);
        assert!(first.fallback_call);
        assert!(!first.dequickened);

        let second = table.record_specialized_guard(function, block, instruction, false);
        assert!(second.guard_miss);
        assert!(second.guard_failure);
        assert!(second.fallback_call);
        assert!(second.dequickened);
        assert!(second.megamorphic);
        assert_eq!(
            table.state(function, block, instruction),
            Some(QuickeningState::Megamorphic)
        );
        assert_eq!(table.specialization(function, block, instruction), None);
    }
}
