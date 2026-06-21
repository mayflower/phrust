//! Opaque ordered PHP array storage for Phase 4.

use crate::{PhpString, Value};

/// PHP array key after MVP key normalization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArrayKey {
    /// Integer array key.
    Int(i64),
    /// String array key.
    String(PhpString),
}

impl ArrayKey {
    /// Converts a runtime value into an MVP PHP array key.
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
            Value::Array(_) | Value::Object(_) | Value::Callable(_) | Value::Reference(_) => None,
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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PhpArray {
    entries: Vec<ArrayEntry>,
    next_append_key: i64,
}

impl PhpArray {
    /// Creates an empty array.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_append_key: 0,
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
        self.entries.len()
    }

    /// Returns true when no entries are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Inserts or overwrites a key. Existing-key overwrites preserve insertion
    /// order.
    pub fn insert(&mut self, key: ArrayKey, value: Value) -> Option<Value> {
        self.bump_append_key(&key);
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.key == key) {
            return Some(std::mem::replace(&mut entry.value, value));
        }
        self.entries.push(ArrayEntry { key, value });
        None
    }

    /// Appends with the next integer key.
    pub fn append(&mut self, value: Value) -> ArrayKey {
        let key = ArrayKey::Int(self.next_append_key);
        self.next_append_key = self.next_append_key.saturating_add(1);
        self.entries.push(ArrayEntry {
            key: key.clone(),
            value,
        });
        key
    }

    /// Returns a value by normalized key.
    #[must_use]
    pub fn get(&self, key: &ArrayKey) -> Option<&Value> {
        self.entries
            .iter()
            .find(|entry| &entry.key == key)
            .map(ArrayEntry::value)
    }

    /// Returns a mutable value by normalized key without exposing storage.
    pub fn get_mut(&mut self, key: &ArrayKey) -> Option<&mut Value> {
        self.entries
            .iter_mut()
            .find(|entry| &entry.key == key)
            .map(|entry| &mut entry.value)
    }

    /// Removes a value by normalized key.
    pub fn remove(&mut self, key: &ArrayKey) -> Option<Value> {
        self.entries
            .iter()
            .position(|entry| &entry.key == key)
            .map(|index| self.entries.remove(index).value)
    }

    /// Iterates in insertion order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&ArrayKey, &Value)> {
        self.entries
            .iter()
            .map(|entry| (entry.key(), entry.value()))
    }

    /// Returns packed elements only when the keys are exactly `0..len`.
    #[must_use]
    pub fn packed_elements(&self) -> Option<Vec<&Value>> {
        let mut elements = Vec::with_capacity(self.entries.len());
        for (index, entry) in self.entries.iter().enumerate() {
            if entry.key != ArrayKey::Int(index as i64) {
                return None;
            }
            elements.push(&entry.value);
        }
        Some(elements)
    }

    fn bump_append_key(&mut self, key: &ArrayKey) {
        if let ArrayKey::Int(value) = key
            && *value >= self.next_append_key
        {
            self.next_append_key = value.saturating_add(1);
        }
    }
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
    fn array_remove_and_get_mut_do_not_expose_storage() {
        let mut array = PhpArray::from_packed(vec![Value::Int(1), Value::Int(2)]);
        *array.get_mut(&ArrayKey::Int(1)).expect("entry") = Value::Int(5);

        assert_eq!(array.get(&ArrayKey::Int(1)), Some(&Value::Int(5)));
        assert_eq!(array.remove(&ArrayKey::Int(0)), Some(Value::Int(1)));
        assert_eq!(array.len(), 1);
        assert_eq!(array.get(&ArrayKey::Int(0)), None);
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
    }

    #[test]
    fn array_packed_facade_detects_contiguous_integer_keys() {
        let packed = PhpArray::from_packed(vec![Value::Int(1), Value::Int(2)]);
        assert_eq!(
            packed
                .packed_elements()
                .expect("packed")
                .into_iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec![Value::Int(1), Value::Int(2)]
        );

        let mut mixed = packed;
        mixed.remove(&ArrayKey::Int(0));
        assert!(mixed.packed_elements().is_none());
    }
}
