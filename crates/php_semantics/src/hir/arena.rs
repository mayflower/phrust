//! Small typed arena used by the Semantic frontend HIR skeleton.

use crate::hir::ids::HirId;
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

/// Append-only arena keyed by a typed ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Arena<T, Id> {
    items: Vec<T>,
    _id: PhantomData<fn() -> Id>,
}

impl<T, Id> Arena<T, Id> {
    /// Creates an empty arena.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            _id: PhantomData,
        }
    }

    /// Returns the number of allocated items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true when the arena is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterates over raw arena values in allocation order.
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }
}

impl<T, Id> Default for Arena<T, Id> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, Id> Arena<T, Id>
where
    Id: HirId,
{
    /// Allocates a value and returns its typed ID.
    pub fn alloc(&mut self, value: T) -> Id {
        let id = Id::from_usize(self.items.len());
        self.items.push(value);
        id
    }

    /// Returns the value for a typed ID.
    #[must_use]
    pub fn get(&self, id: Id) -> Option<&T> {
        self.items.get(id.to_usize())
    }

    /// Returns the mutable value for a typed ID.
    #[must_use]
    pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        self.items.get_mut(id.to_usize())
    }

    /// Iterates over IDs and values in allocation order.
    pub fn iter(&self) -> impl Iterator<Item = (Id, &T)> {
        self.items
            .iter()
            .enumerate()
            .map(|(index, value)| (Id::from_usize(index), value))
    }
}

impl<T, Id> Index<Id> for Arena<T, Id>
where
    Id: HirId,
{
    type Output = T;

    fn index(&self, index: Id) -> &Self::Output {
        &self.items[index.to_usize()]
    }
}

impl<T, Id> IndexMut<Id> for Arena<T, Id>
where
    Id: HirId,
{
    fn index_mut(&mut self, index: Id) -> &mut Self::Output {
        &mut self.items[index.to_usize()]
    }
}
