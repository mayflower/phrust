//! Reference and slot scaffolding for Phase 4.
//!
//! The VM should not pass `Rc<RefCell<Value>>` through public APIs. This module
//! keeps the shared storage private behind `ReferenceCell` and keeps local-slot
//! aliasing explicit through `ValueSlot`. A later intrusive refcount/COW model
//! can replace these internals while preserving the slot-oriented API.

use crate::Value;
use std::cell::{Ref, RefCell};
use std::rc::Rc;

/// Shared cell used for the simple local-reference MVP.
#[derive(Clone, Debug)]
pub struct ReferenceCell {
    inner: Rc<RefCell<Value>>,
}

impl ReferenceCell {
    /// Creates a reference cell containing `value`.
    #[must_use]
    pub fn new(value: Value) -> Self {
        Self {
            inner: Rc::new(RefCell::new(value)),
        }
    }

    /// Reads the contained value by cloning it.
    #[must_use]
    pub fn get(&self) -> Value {
        self.inner.borrow().clone()
    }

    /// Borrows the contained value for read-only inspection.
    #[must_use]
    pub fn borrow(&self) -> Ref<'_, Value> {
        self.inner.borrow()
    }

    /// Replaces the contained value.
    pub fn set(&self, value: Value) {
        *self.inner.borrow_mut() = value;
    }

    /// Returns true when both cells point at the same shared storage.
    #[must_use]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for ReferenceCell {}

impl PartialEq for ReferenceCell {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

/// Runtime storage slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueSlot {
    /// Ordinary by-value storage.
    Value(Value),
    /// Alias to a shared reference cell.
    Reference(ReferenceCell),
}

impl ValueSlot {
    /// Creates an ordinary value slot.
    #[must_use]
    pub const fn value(value: Value) -> Self {
        Self::Value(value)
    }

    /// Creates an uninitialized ordinary slot.
    #[must_use]
    pub const fn uninitialized() -> Self {
        Self::Value(Value::Uninitialized)
    }

    /// Reads the effective value. Reference slots dereference their cell.
    #[must_use]
    pub fn read(&self) -> Value {
        match self {
            Self::Value(value) => value.clone(),
            Self::Reference(cell) => cell.get(),
        }
    }

    /// Returns true when the effective value is uninitialized.
    #[must_use]
    pub fn is_uninitialized(&self) -> bool {
        self.read().is_uninitialized()
    }

    /// Writes through the slot. Reference slots update the shared cell.
    pub fn write(&mut self, value: Value) {
        match self {
            Self::Value(slot) => *slot = value,
            Self::Reference(cell) => cell.set(value),
        }
    }

    /// Converts an ordinary slot into a reference cell or returns its existing
    /// cell. This is the only Phase 4 operation that creates local aliases.
    pub fn ensure_reference_cell(&mut self) -> ReferenceCell {
        match self {
            Self::Value(value) => {
                let cell = ReferenceCell::new(value.clone());
                *self = Self::Reference(cell.clone());
                cell
            }
            Self::Reference(cell) => cell.clone(),
        }
    }

    /// Binds this slot to an existing reference cell.
    pub fn bind_reference(&mut self, cell: ReferenceCell) {
        *self = Self::Reference(cell);
    }
}

/// Backwards-compatible exported name for earlier placeholder references.
pub type ReferencePlaceholder = ReferenceCell;

#[cfg(test)]
mod tests {
    use super::{ReferenceCell, ValueSlot};
    use crate::Value;

    #[test]
    fn reference_cell_aliases_updates() {
        let cell = ReferenceCell::new(Value::Int(1));
        let alias = cell.clone();

        alias.set(Value::Int(2));

        assert_eq!(cell.get(), Value::Int(2));
        assert!(cell.ptr_eq(&alias));
    }

    #[test]
    fn value_slot_writes_through_reference_cells() {
        let mut left = ValueSlot::value(Value::Int(1));
        let cell = left.ensure_reference_cell();
        let mut right = ValueSlot::uninitialized();
        right.bind_reference(cell);

        right.write(Value::Int(3));

        assert_eq!(left.read(), Value::Int(3));
        assert_eq!(right.read(), Value::Int(3));
    }
}
