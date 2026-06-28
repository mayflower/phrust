//! Request-local runtime layout and allocation counters.

use std::cell::RefCell;

/// Runtime value/layout counters collected by the VM when counters are enabled.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuntimeLayoutStats {
    /// Runtime `Value` clones observed during execution.
    pub value_clones: u64,
    /// PHP byte-string backing allocations.
    pub string_allocations: u64,
    /// PHP array handle clones sharing copy-on-write storage.
    pub array_handle_clones: u64,
    /// Copy-on-write storage separations for runtime containers.
    pub cow_separations: u64,
    /// Reference cells created for PHP references/aliases.
    pub reference_cell_creations: u64,
    /// Runtime object storage allocations.
    pub object_allocations: u64,
}

thread_local! {
    static LAYOUT_STATS: RefCell<RuntimeLayoutStats> =
        RefCell::new(RuntimeLayoutStats::default());
}

pub(crate) fn record_value_clone() {
    LAYOUT_STATS.with(|stats| stats.borrow_mut().value_clones += 1);
}

pub(crate) fn record_string_allocation() {
    LAYOUT_STATS.with(|stats| stats.borrow_mut().string_allocations += 1);
}

pub(crate) fn record_array_handle_clone() {
    LAYOUT_STATS.with(|stats| stats.borrow_mut().array_handle_clones += 1);
}

pub(crate) fn record_cow_separation() {
    LAYOUT_STATS.with(|stats| stats.borrow_mut().cow_separations += 1);
}

pub(crate) fn record_reference_cell_creation() {
    LAYOUT_STATS.with(|stats| stats.borrow_mut().reference_cell_creations += 1);
}

pub(crate) fn record_object_allocation() {
    LAYOUT_STATS.with(|stats| stats.borrow_mut().object_allocations += 1);
}

/// Clears layout counters for deterministic VM executions.
pub fn reset_layout_stats() {
    LAYOUT_STATS.with(|stats| *stats.borrow_mut() = RuntimeLayoutStats::default());
}

/// Returns and clears layout counters.
#[must_use]
pub fn take_layout_stats() -> RuntimeLayoutStats {
    LAYOUT_STATS.with(|stats| {
        let mut stats = stats.borrow_mut();
        let current = *stats;
        *stats = RuntimeLayoutStats::default();
        current
    })
}

#[cfg(test)]
mod tests {
    use crate::{PhpArray, PhpString, ReferenceCell, Value, layout_stats};

    #[test]
    fn layout_stats_record_safe_runtime_events() {
        layout_stats::reset_layout_stats();

        let string = PhpString::from("abc");
        let _string_clone = string.clone();
        let array = PhpArray::from_packed(vec![Value::Int(1), Value::Int(2)]);
        let mut array_clone = array.clone();
        array_clone.append(Value::Int(3));
        let _cell = ReferenceCell::new(Value::String(string));
        let _value_clone = Value::Array(array).clone();

        let stats = layout_stats::take_layout_stats();
        assert!(stats.value_clones >= 1, "{stats:?}");
        assert!(stats.string_allocations >= 1, "{stats:?}");
        assert!(stats.array_handle_clones >= 2, "{stats:?}");
        assert!(stats.cow_separations >= 1, "{stats:?}");
        assert_eq!(stats.reference_cell_creations, 1);
    }
}
