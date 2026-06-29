//! Minimal object storage and class metadata for runtime.

mod attribute;
mod class;
mod debug;
mod member;
mod storage;
mod types;

pub use attribute::AttributeEntry;
pub use class::{ClassEntry, ClassFlags, display_class_name, normalize_class_name};
pub use member::{
    ClassConstantEntry, ClassConstantFlags, ClassEnumBackingType, ClassEnumCaseEntry,
    ClassMethodEntry, ClassMethodFlags, ClassPropertyEntry, ClassPropertyFlags, ClassPropertyHooks,
};
pub use storage::{ObjectRef, WeakObjectHandle};
pub use types::RuntimeType;

use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicU64, Ordering},
};

static NEXT_OBJECT_ID: AtomicU64 = AtomicU64::new(1);
static FREE_OBJECT_IDS: OnceLock<Mutex<Vec<u64>>> = OnceLock::new();

pub(crate) fn next_object_id() -> u64 {
    let free_ids = FREE_OBJECT_IDS.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut free_ids) = free_ids.lock()
        && let Some(index) = free_ids
            .iter()
            .enumerate()
            .min_by_key(|(_, id)| **id)
            .map(|(index, _)| index)
    {
        return free_ids.swap_remove(index);
    }
    NEXT_OBJECT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug)]
pub(crate) struct ObjectIdGuard {
    id: u64,
}

impl ObjectIdGuard {
    #[must_use]
    pub(crate) const fn new(id: u64) -> Self {
        Self { id }
    }
}

impl Drop for ObjectIdGuard {
    fn drop(&mut self) {
        let free_ids = FREE_OBJECT_IDS.get_or_init(|| Mutex::new(Vec::new()));
        if let Ok(mut free_ids) = free_ids.lock() {
            free_ids.push(self.id);
        }
    }
}

#[cfg(test)]
mod tests;
