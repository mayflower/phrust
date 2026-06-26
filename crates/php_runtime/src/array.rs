//! Opaque ordered PHP array storage for runtime-semantics.

use crate::{PhpString, Value};
use std::rc::{Rc, Weak};

/// PHP array key after runtime-semantics key normalization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArrayKey {
    /// Integer array key.
    Int(i64),
    /// String array key.
    String(PhpString),
}

impl ArrayKey {
    /// Converts a runtime value into a runtime-semantics PHP array key.
    ///
    /// Supported conversions:
    /// - `int` remains an integer key;
    /// - `bool` becomes `0` or `1`;
    /// - `null` becomes the empty-string key;
    /// - `float` truncates toward zero;
    /// - decimal integer strings without a leading plus and without leading
    ///   zeroes become integer keys;
    /// - all other strings remain string keys.
    #[must_use]
    pub fn from_value_mvp(value: &Value) -> Option<Self> {
        match value {
            Value::Int(value) => Some(Self::Int(*value)),
            Value::Bool(false) => Some(Self::Int(0)),
            Value::Bool(true) => Some(Self::Int(1)),
            Value::Null => Some(Self::String(PhpString::from_bytes(Vec::new()))),
            Value::Float(value) => Some(Self::Int(value.to_f64() as i64)),
            Value::String(value) => Some(Self::from_php_string(value.clone())),
            Value::Uninitialized => Some(Self::String(PhpString::from_bytes(Vec::new()))),
            Value::Array(_)
            | Value::Object(_)
            | Value::Resource(_)
            | Value::Fiber(_)
            | Value::Generator(_)
            | Value::Callable(_)
            | Value::Reference(_) => None,
        }
    }

    /// Normalizes a PHP string key in the tested MVP range.
    #[must_use]
    pub fn from_php_string(value: PhpString) -> Self {
        normalize_string_key(&value).map_or(Self::String(value), Self::Int)
    }

    /// Returns the integer key when present.
    #[must_use]
    pub const fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(value) => Some(*value),
            Self::String(_) => None,
        }
    }

    /// Returns the string key when present.
    #[must_use]
    pub const fn as_string(&self) -> Option<&PhpString> {
        match self {
            Self::String(value) => Some(value),
            Self::Int(_) => None,
        }
    }
}

/// One ordered array slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayEntry {
    key: ArrayKey,
    value: Value,
}

impl ArrayEntry {
    /// Array key.
    #[must_use]
    pub const fn key(&self) -> &ArrayKey {
        &self.key
    }

    /// Array value.
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.value
    }
}

/// Ordered PHP array facade.
///
/// The storage is intentionally opaque. Today it is a simple insertion-ordered
/// vector, but callers interact through key/value APIs that can later route to
/// packed or mixed representations without changing the VM boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ArrayStorage {
    entries: Vec<ArrayEntry>,
    next_append_key: Option<i64>,
    packed_len: Option<usize>,
    internal_pointer: Option<usize>,
}

impl Default for ArrayStorage {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            next_append_key: None,
            packed_len: Some(0),
            internal_pointer: None,
        }
    }
}

/// Copy-on-write ordered PHP array facade.
///
/// Cloning a `PhpArray` shares immutable storage. Mutating methods call
/// `separate_for_write` through `storage_mut`, so by-value assignment shares
/// until the first write while true PHP references still write through their
/// owning slot/reference cell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhpArray {
    storage: Rc<ArrayStorage>,
}

impl Default for PhpArray {
    fn default() -> Self {
        Self::new()
    }
}

/// Weak debug handle to array storage for GC tests.
#[derive(Clone, Debug)]
pub struct WeakArrayHandle {
    id: usize,
    storage: Weak<ArrayStorage>,
}

impl WeakArrayHandle {
    /// Returns the process-local debug ID for this handle.
    #[must_use]
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Returns true when the array storage is still alive.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.storage.strong_count() > 0
    }
}

impl PhpArray {
    /// Creates an empty array.
    #[must_use]
    pub fn new() -> Self {
        Self {
            storage: Rc::new(ArrayStorage {
                entries: Vec::new(),
                next_append_key: None,
                packed_len: Some(0),
                internal_pointer: None,
            }),
        }
    }

    /// Creates a packed array with integer keys starting at zero.
    #[must_use]
    pub fn from_packed(elements: Vec<Value>) -> Self {
        let mut array = Self::new();
        for value in elements {
            array.append(value);
        }
        array
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.storage.entries.len()
    }

    /// Returns true when no entries are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.storage.entries.is_empty()
    }

    /// Returns true when this array shares storage with at least one clone.
    #[must_use]
    pub fn is_shared(&self) -> bool {
        Rc::strong_count(&self.storage) > 1
    }

    /// Returns true when tracked metadata proves the array is exactly
    /// `0..len` in insertion order.
    #[must_use]
    pub fn is_packed_fast(&self) -> bool {
        self.storage.packed_len == Some(self.storage.entries.len())
    }

    /// Returns the packed length when tracked metadata proves packed storage.
    #[must_use]
    pub fn packed_len_fast(&self) -> Option<usize> {
        self.is_packed_fast().then_some(self.storage.entries.len())
    }

    /// Returns a process-local storage identity for GC debug snapshots.
    ///
    /// This is not a PHP-visible handle and must only be used by runtime tests
    /// and diagnostics.
    #[must_use]
    pub fn gc_debug_id(&self) -> usize {
        Rc::as_ptr(&self.storage).cast::<()>() as usize
    }

    /// Returns the current `Rc` strong count for GC debug metadata.
    #[must_use]
    pub fn gc_refcount_estimate(&self) -> usize {
        Rc::strong_count(&self.storage)
    }

    /// Returns a weak debug handle for GC tests.
    #[must_use]
    pub fn weak_handle(&self) -> WeakArrayHandle {
        WeakArrayHandle {
            id: self.gc_debug_id(),
            storage: Rc::downgrade(&self.storage),
        }
    }

    /// Ensures this array has unique storage before mutation.
    pub fn separate_for_write(&mut self) {
        let _ = self.storage_mut();
    }

    /// Inserts or overwrites a key. Existing-key overwrites preserve insertion
    /// order.
    pub fn insert(&mut self, key: ArrayKey, value: Value) -> Option<Value> {
        let storage = self.storage_mut();
        bump_append_key(storage, &key);
        if let Some(entry) = storage.entries.iter_mut().find(|entry| entry.key == key) {
            return Some(std::mem::replace(&mut entry.value, value));
        }
        let old_len = storage.entries.len();
        let remains_packed = storage.packed_len == Some(old_len)
            && matches!(key, ArrayKey::Int(value) if value == old_len as i64);
        storage.entries.push(ArrayEntry { key, value });
        storage.packed_len = remains_packed.then_some(old_len + 1);
        if storage.internal_pointer.is_none() {
            storage.internal_pointer = Some(0);
        }
        None
    }

    /// Appends with the next integer key.
    pub fn append(&mut self, value: Value) -> ArrayKey {
        let storage = self.storage_mut();
        let key = ArrayKey::Int(storage.next_append_key.unwrap_or(0));
        let old_len = storage.entries.len();
        let remains_packed = storage.packed_len == Some(old_len)
            && matches!(key, ArrayKey::Int(value) if value == old_len as i64);
        bump_append_key(storage, &key);
        storage.entries.push(ArrayEntry {
            key: key.clone(),
            value,
        });
        storage.packed_len = remains_packed.then_some(old_len + 1);
        if storage.internal_pointer.is_none() {
            storage.internal_pointer = Some(0);
        }
        key
    }

    /// Returns a value by normalized key.
    #[must_use]
    pub fn get(&self, key: &ArrayKey) -> Option<&Value> {
        self.storage
            .entries
            .iter()
            .find(|entry| &entry.key == key)
            .map(ArrayEntry::value)
    }

    /// Returns a mutable value by normalized key without exposing storage.
    pub fn get_mut(&mut self, key: &ArrayKey) -> Option<&mut Value> {
        self.storage_mut()
            .entries
            .iter_mut()
            .find(|entry| &entry.key == key)
            .map(|entry| &mut entry.value)
    }

    /// Removes a value by normalized key.
    pub fn remove(&mut self, key: &ArrayKey) -> Option<Value> {
        let storage = self.storage_mut();
        storage
            .entries
            .iter()
            .position(|entry| &entry.key == key)
            .map(|index| {
                let was_packed_len = storage.packed_len;
                let value = storage.entries.remove(index).value;
                if let Some(packed_len) = was_packed_len {
                    storage.packed_len =
                        if index + 1 == packed_len && index == storage.entries.len() {
                            Some(storage.entries.len())
                        } else {
                            None
                        };
                }
                adjust_pointer_after_remove(storage, index);
                value
            })
    }

    /// Removes and returns the last element, mirroring PHP's `array_pop`
    /// adjustment of the next auto-index: when the removed key is the most
    /// recent auto-index (`next_append_key - 1`), the next index is decremented
    /// so a following `[]=` reuses it (e.g. popping `-2` from `[-2 => x]` makes
    /// the next append `-2` again).
    pub fn pop(&mut self) -> Option<Value> {
        let last_key = self.storage.entries.last()?.key.clone();
        let previous_next = self.storage.next_append_key;
        let value = self.remove(&last_key);
        if let ArrayKey::Int(key) = last_key
            && previous_next == Some(key.saturating_add(1))
        {
            self.storage_mut().next_append_key = Some(key);
        }
        value
    }

    /// Returns the current internal-pointer value.
    #[must_use]
    pub fn pointer_value(&self) -> Option<Value> {
        self.storage
            .internal_pointer
            .and_then(|index| self.storage.entries.get(index))
            .map(ArrayEntry::value)
            .cloned()
    }

    /// Returns the current internal-pointer key.
    #[must_use]
    pub fn pointer_key(&self) -> Option<ArrayKey> {
        self.storage
            .internal_pointer
            .and_then(|index| self.storage.entries.get(index))
            .map(ArrayEntry::key)
            .cloned()
    }

    /// Moves the internal pointer to the first element.
    pub fn reset_pointer(&mut self) -> Option<Value> {
        let storage = self.storage_mut();
        if storage.entries.is_empty() {
            storage.internal_pointer = None;
            return None;
        }
        storage.internal_pointer = Some(0);
        storage.entries.first().map(ArrayEntry::value).cloned()
    }

    /// Moves the internal pointer to the last element.
    pub fn end_pointer(&mut self) -> Option<Value> {
        let storage = self.storage_mut();
        let last = storage.entries.len().checked_sub(1)?;
        storage.internal_pointer = Some(last);
        storage.entries.get(last).map(ArrayEntry::value).cloned()
    }

    /// Advances the internal pointer by one element.
    pub fn next_pointer(&mut self) -> Option<Value> {
        let storage = self.storage_mut();
        let current = storage.internal_pointer?;
        let next = current.saturating_add(1);
        if next >= storage.entries.len() {
            storage.internal_pointer = None;
            return None;
        }
        storage.internal_pointer = Some(next);
        storage.entries.get(next).map(ArrayEntry::value).cloned()
    }

    /// Moves the internal pointer one element backwards.
    pub fn prev_pointer(&mut self) -> Option<Value> {
        let storage = self.storage_mut();
        let Some(current) = storage.internal_pointer else {
            let last = storage.entries.len().checked_sub(1)?;
            storage.internal_pointer = Some(last);
            return storage.entries.get(last).map(ArrayEntry::value).cloned();
        };
        let Some(previous) = current.checked_sub(1) else {
            storage.internal_pointer = None;
            return None;
        };
        storage.internal_pointer = Some(previous);
        storage
            .entries
            .get(previous)
            .map(ArrayEntry::value)
            .cloned()
    }

    /// Iterates in insertion order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&ArrayKey, &Value)> {
        self.storage
            .entries
            .iter()
            .map(|entry| (entry.key(), entry.value()))
    }

    /// Returns packed elements only when the keys are exactly `0..len`.
    #[must_use]
    pub fn packed_elements(&self) -> Option<Vec<&Value>> {
        if self.is_packed_fast() {
            return Some(self.storage.entries.iter().map(ArrayEntry::value).collect());
        }
        let mut elements = Vec::with_capacity(self.storage.entries.len());
        for (index, entry) in self.storage.entries.iter().enumerate() {
            if entry.key != ArrayKey::Int(index as i64) {
                return None;
            }
            elements.push(&entry.value);
        }
        Some(elements)
    }

    /// Returns one packed element only when the keys are exactly `0..len`.
    #[must_use]
    pub fn packed_element(&self, index: usize) -> Option<&Value> {
        for (entry_index, entry) in self.storage.entries.iter().enumerate() {
            if entry.key != ArrayKey::Int(entry_index as i64) {
                return None;
            }
        }
        self.storage.entries.get(index).map(ArrayEntry::value)
    }

    /// Returns one packed element using only tracked metadata.
    #[must_use]
    pub fn packed_element_fast(&self, index: usize) -> Option<&Value> {
        self.is_packed_fast()
            .then(|| self.storage.entries.get(index).map(ArrayEntry::value))
            .flatten()
    }

    fn storage_mut(&mut self) -> &mut ArrayStorage {
        Rc::make_mut(&mut self.storage)
    }
}

fn bump_append_key(storage: &mut ArrayStorage, key: &ArrayKey) {
    if let ArrayKey::Int(value) = key {
        let next = value.saturating_add(1);
        if storage.next_append_key.is_none_or(|current| next > current) {
            storage.next_append_key = Some(next);
        }
    }
}

fn adjust_pointer_after_remove(storage: &mut ArrayStorage, removed_index: usize) {
    let Some(pointer) = storage.internal_pointer else {
        return;
    };
    storage.internal_pointer = if storage.entries.is_empty() {
        None
    } else if pointer > removed_index {
        Some(pointer - 1)
    } else if pointer >= storage.entries.len() {
        None
    } else {
        Some(pointer)
    };
}

fn normalize_string_key(value: &PhpString) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let (negative, digits) = if let Some(rest) = bytes.strip_prefix(b"-") {
        (true, rest)
    } else {
        (false, bytes)
    };
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    if digits.len() > 1 && digits[0] == b'0' {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let value = text.parse::<i64>().ok()?;
    if negative && value == 0 {
        return None;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::{ArrayKey, PhpArray};
    use crate::{PhpString, Value};

    #[test]
    fn array_preserves_insertion_order_and_overwrite_position() {
        let mut array = PhpArray::new();
        array.insert(ArrayKey::String(PhpString::from("a")), Value::Int(1));
        array.insert(ArrayKey::Int(4), Value::Int(2));
        array.insert(ArrayKey::String(PhpString::from("a")), Value::Int(3));

        let entries = array
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            entries,
            vec![
                (ArrayKey::String(PhpString::from("a")), Value::Int(3)),
                (ArrayKey::Int(4), Value::Int(2)),
            ]
        );
    }

    #[test]
    fn array_append_key_tracks_largest_integer_key() {
        let mut array = PhpArray::new();
        assert_eq!(array.append(Value::Int(1)), ArrayKey::Int(0));
        array.insert(ArrayKey::Int(7), Value::Int(2));
        assert_eq!(array.append(Value::Int(3)), ArrayKey::Int(8));
        array.insert(ArrayKey::Int(4), Value::Int(4));
        assert_eq!(array.append(Value::Int(5)), ArrayKey::Int(9));
    }

    #[test]
    fn array_append_key_tracks_negative_integer_keys() {
        let mut array = PhpArray::new();
        array.insert(ArrayKey::Int(-5), Value::Int(1));
        assert_eq!(array.append(Value::Int(2)), ArrayKey::Int(-4));

        let mut array = PhpArray::new();
        array.insert(ArrayKey::Int(-1), Value::Int(1));
        assert_eq!(array.append(Value::Int(2)), ArrayKey::Int(0));

        array.insert(ArrayKey::Int(-10), Value::Int(3));
        assert_eq!(array.append(Value::Int(4)), ArrayKey::Int(1));
    }

    #[test]
    fn array_remove_and_get_mut_do_not_expose_storage() {
        let mut array = PhpArray::from_packed(vec![Value::Int(1), Value::Int(2)]);
        *array.get_mut(&ArrayKey::Int(1)).expect("entry") = Value::Int(5);

        assert_eq!(array.get(&ArrayKey::Int(1)), Some(&Value::Int(5)));
        assert_eq!(array.remove(&ArrayKey::Int(0)), Some(Value::Int(1)));
        assert_eq!(array.len(), 1);
        assert_eq!(array.get(&ArrayKey::Int(0)), None);
    }

    #[test]
    fn foreach_snapshot_keys_keep_insertion_order_after_mutation() {
        let mut array = PhpArray::from_packed(vec![Value::Int(1), Value::Int(2)]);
        let keys = array.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>();

        array.remove(&ArrayKey::Int(0));
        array.append(Value::Int(3));

        assert_eq!(keys, vec![ArrayKey::Int(0), ArrayKey::Int(1)]);
        assert_eq!(
            array.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>(),
            vec![ArrayKey::Int(1), ArrayKey::Int(2)]
        );
    }

    #[test]
    fn foreach_dynamic_key_reads_include_appended_entries() {
        let mut array = PhpArray::from_packed(vec![Value::Int(1), Value::Int(2)]);
        let first_keys = array.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>();
        assert_eq!(first_keys, vec![ArrayKey::Int(0), ArrayKey::Int(1)]);

        array.append(Value::Int(3));
        let second_keys = array.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>();
        assert_eq!(
            second_keys,
            vec![ArrayKey::Int(0), ArrayKey::Int(1), ArrayKey::Int(2)]
        );
    }

    #[test]
    fn cow_array_assignment_shares_until_write() {
        let original = PhpArray::from_packed(vec![Value::Int(1)]);
        let mut copy = original.clone();

        assert!(original.is_shared());
        assert!(copy.is_shared());

        copy.append(Value::Int(2));

        assert_eq!(
            original.packed_elements().expect("packed original").len(),
            1
        );
        assert_eq!(copy.packed_elements().expect("packed copy").len(), 2);
        assert_eq!(original.get(&ArrayKey::Int(1)), None);
        assert_eq!(copy.get(&ArrayKey::Int(1)), Some(&Value::Int(2)));
        assert!(!copy.is_shared());
    }

    #[test]
    fn array_key_conversion_covers_mvp_value_types() {
        assert_eq!(
            ArrayKey::from_value_mvp(&Value::Int(4)),
            Some(ArrayKey::Int(4))
        );
        assert_eq!(
            ArrayKey::from_value_mvp(&Value::Bool(true)),
            Some(ArrayKey::Int(1))
        );
        assert_eq!(
            ArrayKey::from_value_mvp(&Value::Null),
            Some(ArrayKey::String(PhpString::from("")))
        );
        assert_eq!(
            ArrayKey::from_value_mvp(&Value::float(4.9)),
            Some(ArrayKey::Int(4))
        );
        assert_eq!(
            ArrayKey::from_value_mvp(&Value::String(PhpString::from("42"))),
            Some(ArrayKey::Int(42))
        );
        assert_eq!(
            ArrayKey::from_value_mvp(&Value::String(PhpString::from("042"))),
            Some(ArrayKey::String(PhpString::from("042")))
        );
        assert_eq!(
            ArrayKey::from_value_mvp(&Value::String(PhpString::from("+42"))),
            Some(ArrayKey::String(PhpString::from("+42")))
        );
        assert_eq!(
            ArrayKey::from_value_mvp(&Value::String(PhpString::from("-42"))),
            Some(ArrayKey::Int(-42))
        );
        assert_eq!(
            ArrayKey::from_value_mvp(&Value::String(PhpString::from("-0"))),
            Some(ArrayKey::String(PhpString::from("-0")))
        );
        assert_eq!(
            ArrayKey::from_value_mvp(&Value::String(PhpString::from("9223372036854775808"))),
            Some(ArrayKey::String(PhpString::from("9223372036854775808")))
        );
        assert_eq!(
            ArrayKey::from_php_string(PhpString::from(" 42")),
            ArrayKey::String(PhpString::from(" 42"))
        );
        assert_eq!(
            ArrayKey::from_php_string(PhpString::from("1.0")),
            ArrayKey::String(PhpString::from("1.0"))
        );
    }

    #[test]
    fn array_packed_facade_detects_contiguous_integer_keys() {
        let packed = PhpArray::from_packed(vec![Value::Int(1), Value::Int(2)]);
        assert!(packed.is_packed_fast());
        assert_eq!(packed.packed_len_fast(), Some(2));
        assert_eq!(packed.packed_element_fast(1), Some(&Value::Int(2)));
        assert_eq!(packed.packed_element_fast(2), None);
        assert_eq!(
            packed
                .packed_elements()
                .expect("packed")
                .into_iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec![Value::Int(1), Value::Int(2)]
        );
        assert_eq!(packed.packed_element(1), Some(&Value::Int(2)));
        assert_eq!(packed.packed_element(2), None);

        let mut mixed = packed;
        mixed.remove(&ArrayKey::Int(0));
        assert!(!mixed.is_packed_fast());
        assert!(mixed.packed_elements().is_none());
        assert_eq!(mixed.packed_element(0), None);
        assert_eq!(mixed.packed_element_fast(0), None);
    }

    #[test]
    fn packed_metadata_stays_fast_for_sequential_append_and_overwrite() {
        let mut array = PhpArray::new();
        array.append(Value::Int(1));
        array.append(Value::Int(2));
        array.insert(ArrayKey::Int(1), Value::Int(5));

        assert!(array.is_packed_fast());
        assert_eq!(array.packed_len_fast(), Some(2));
        assert_eq!(array.packed_element_fast(1), Some(&Value::Int(5)));
    }

    #[test]
    fn packed_metadata_transitions_for_non_sequential_int_key() {
        let mut array = PhpArray::from_packed(vec![Value::Int(1), Value::Int(2)]);
        array.insert(ArrayKey::Int(4), Value::Int(5));

        assert!(!array.is_packed_fast());
        assert!(array.packed_elements().is_none());
        assert_eq!(array.packed_element_fast(1), None);
    }

    #[test]
    fn packed_metadata_transitions_for_string_key() {
        let mut array = PhpArray::from_packed(vec![Value::Int(1), Value::Int(2)]);
        array.insert(ArrayKey::String(PhpString::from("x")), Value::Int(5));

        assert!(!array.is_packed_fast());
        assert!(array.packed_elements().is_none());
    }

    #[test]
    fn packed_metadata_tracks_unset_holes_and_append_after_last_unset() {
        let mut hole = PhpArray::from_packed(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        hole.remove(&ArrayKey::Int(1));
        assert!(!hole.is_packed_fast());
        assert!(hole.packed_elements().is_none());

        let mut tail = PhpArray::from_packed(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        tail.remove(&ArrayKey::Int(2));
        assert!(tail.is_packed_fast());
        assert_eq!(tail.packed_len_fast(), Some(2));
        assert_eq!(tail.append(Value::Int(4)), ArrayKey::Int(3));
        assert!(!tail.is_packed_fast());
        assert!(tail.packed_elements().is_none());
    }

    #[test]
    fn packed_metadata_allows_reference_elements_without_cow_shortcuts() {
        let mut array = PhpArray::from_packed(vec![Value::Int(1)]);
        let cell = crate::ReferenceCell::new(Value::Int(2));
        array.append(Value::Reference(cell.clone()));

        assert!(array.is_packed_fast());
        assert_eq!(array.packed_len_fast(), Some(2));
        cell.set(Value::Int(7));
        assert_eq!(array.packed_element_fast(1), Some(&Value::Reference(cell)));
    }
}
