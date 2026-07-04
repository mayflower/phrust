//! Exact-case fast paths for hot array builtins.
//!
//! Like `string_intrinsics`, each helper is the single semantic
//! implementation for its exact common case and shares the generic
//! builtins' bounds math, so the VM hook and the registry path cannot
//! diverge.

use super::core::slice_bounds;
use crate::PhpArray;

/// `array_slice($array, $offset, $length)` over values-only packed storage
/// without preserved keys: the result is the repacked value range.
///
/// Returns `None` for non-packed storage so the caller falls back to the
/// generic entries-snapshot path.
pub fn array_slice_packed(array: &PhpArray, offset: i64, length: Option<i64>) -> Option<PhpArray> {
    let values = array.packed_values_fast()?;
    let (start, end) = slice_bounds(array.len(), offset, length);
    Some(PhpArray::from_packed(
        values.skip(start).take(end - start).cloned().collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::super::core::{array_entries, array_from_entries_reindex_ints, slice_entries};
    use super::*;
    use crate::Value;

    #[test]
    fn packed_slice_matches_generic_entries_path() {
        let array = PhpArray::from_packed((0..7).map(Value::Int).collect());
        for offset in [-9_i64, -7, -3, -1, 0, 1, 3, 6, 7, 9] {
            for length in [
                None,
                Some(-9_i64),
                Some(-3),
                Some(-1),
                Some(0),
                Some(2),
                Some(20),
            ] {
                let generic = array_from_entries_reindex_ints(slice_entries(
                    array_entries(&array),
                    offset,
                    length,
                ));
                let fast = array_slice_packed(&array, offset, length)
                    .expect("packed storage takes the fast path");
                assert_eq!(
                    Value::Array(fast),
                    Value::Array(generic),
                    "offset={offset} length={length:?}"
                );
            }
        }
    }

    #[test]
    fn non_packed_storage_falls_back() {
        let mut array = PhpArray::new();
        array.insert(
            crate::ArrayKey::String(crate::PhpString::from_test_str("k")),
            Value::Int(1),
        );
        assert!(array_slice_packed(&array, 0, None).is_none());
    }
}
