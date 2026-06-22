//! Runtime global-symbol table storage for CLI execution.

use crate::{ArrayKey, PhpArray, PhpString, ReferenceCell, Value};
use std::collections::BTreeMap;

/// Shared storage for PHP global variables and superglobals.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GlobalSymbolTable {
    slots: BTreeMap<String, ReferenceCell>,
}

impl GlobalSymbolTable {
    /// Creates an empty global symbol table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns an existing slot or creates it with `default`.
    pub fn ensure_slot(&mut self, name: impl Into<String>, default: Value) -> ReferenceCell {
        let name = name.into();
        self.slots
            .entry(name)
            .or_insert_with(|| ReferenceCell::new(default))
            .clone()
    }

    /// Returns an existing slot without creating it.
    #[must_use]
    pub fn get_slot(&self, name: &str) -> Option<ReferenceCell> {
        self.slots.get(name).cloned()
    }

    /// Writes through a global slot, creating it if necessary.
    pub fn set(&mut self, name: impl Into<String>, value: Value) {
        let slot = self.ensure_slot(name, Value::Uninitialized);
        slot.set(value);
    }

    /// Reads an effective global value.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Value> {
        self.slots.get(name).map(ReferenceCell::get)
    }

    /// Returns a `$GLOBALS` view with live reference entries.
    #[must_use]
    pub fn globals_array(&self) -> PhpArray {
        let mut array = PhpArray::new();
        for (name, slot) in &self.slots {
            if slot.get().is_uninitialized() {
                continue;
            }
            array.insert(
                ArrayKey::String(PhpString::from_test_str(name)),
                Value::Reference(slot.clone()),
            );
        }
        array
    }
}

#[cfg(test)]
mod tests {
    use super::GlobalSymbolTable;
    use crate::{ArrayKey, PhpString, Value};

    #[test]
    fn globals_table_reuses_slots_for_global_aliases() {
        let mut globals = GlobalSymbolTable::new();
        let first = globals.ensure_slot("x", Value::Int(1));
        let second = globals.ensure_slot("x", Value::Null);

        assert!(first.ptr_eq(&second));
        second.set(Value::Int(2));
        assert_eq!(globals.get("x"), Some(Value::Int(2)));
    }

    #[test]
    fn globals_table_exposes_reference_entries() {
        let mut globals = GlobalSymbolTable::new();
        let cell = globals.ensure_slot("x", Value::Int(1));
        let array = globals.globals_array();
        let key = ArrayKey::String(PhpString::from_test_str("x"));
        let Some(Value::Reference(entry)) = array.get(&key) else {
            panic!("expected $GLOBALS entry to be a reference");
        };

        entry.set(Value::Int(3));
        assert_eq!(cell.get(), Value::Int(3));
        assert_eq!(globals.get("x"), Some(Value::Int(3)));
    }
}
