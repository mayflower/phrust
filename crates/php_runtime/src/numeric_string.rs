//! PHP numeric-string classification for runtime conversion.

use crate::PhpString;
use std::cell::RefCell;
use std::collections::HashMap;

const CACHE_LIMIT: usize = 4096;

thread_local! {
    static CLASSIFICATION_CACHE: RefCell<HashMap<NumericStringCacheKey, NumericString>> =
        RefCell::new(HashMap::new());
    static CLASSIFICATION_STATS: RefCell<NumericStringCacheStats> =
        RefCell::new(NumericStringCacheStats::default());
}

/// Numeric value parsed from a PHP string.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NumericStringValue {
    /// Integer payload.
    Int(i64),
    /// Floating-point payload.
    Float(f64),
}

impl NumericStringValue {
    /// Returns the value as an `f64`.
    #[must_use]
    pub const fn as_f64(self) -> f64 {
        match self {
            Self::Int(value) => value as f64,
            Self::Float(value) => value,
        }
    }

    /// Returns the integer truncation used by explicit casts.
    #[must_use]
    pub const fn to_i64(self) -> i64 {
        match self {
            Self::Int(value) => value,
            Self::Float(value) => value as i64,
        }
    }

    /// Returns true when this value is represented as a float.
    #[must_use]
    pub const fn is_float(self) -> bool {
        matches!(self, Self::Float(_))
    }
}

/// PHP numeric-string class in the Phase 5 conversion subset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumericStringKind {
    /// The whole trimmed string is an integer numeric string.
    IntString,
    /// The whole trimmed string is a floating-point numeric string.
    FloatString,
    /// The string starts with a numeric prefix followed by non-whitespace.
    LeadingNumeric,
    /// The string does not start with a numeric prefix.
    NonNumeric,
}

/// Classification result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumericString {
    /// Numeric-string class.
    pub kind: NumericStringKind,
    /// Parsed value for full or leading numeric strings.
    pub value: Option<NumericStringValue>,
}

/// Numeric-string cache stats collected by the VM when counters are enabled.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NumericStringCacheStats {
    /// Cache hits.
    pub hits: u64,
    /// Cache misses.
    pub misses: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct NumericStringCacheKey {
    storage_id: usize,
    len: usize,
    fingerprint: u64,
}

impl NumericString {
    /// Returns true when the string has a PHP numeric prefix.
    #[must_use]
    pub const fn has_numeric_value(self) -> bool {
        self.value.is_some()
    }
}

/// Classifies a PHP string through a conservative request-local cache.
///
/// The key includes storage identity, byte length, and a stable byte
/// fingerprint. That keeps COW and in-place test mutations safe: changed bytes
/// cannot reuse an old classification even when the backing allocation is the
/// same.
#[must_use]
pub fn classify_php_string(value: &PhpString) -> NumericString {
    let key = NumericStringCacheKey {
        storage_id: value.storage_id(),
        len: value.len(),
        fingerprint: fingerprint(value.as_bytes()),
    };
    if let Some(classified) = CLASSIFICATION_CACHE.with(|cache| cache.borrow().get(&key).copied()) {
        CLASSIFICATION_STATS.with(|stats| stats.borrow_mut().hits += 1);
        return classified;
    }
    let classified = classify(value.as_bytes());
    CLASSIFICATION_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, classified);
    });
    CLASSIFICATION_STATS.with(|stats| stats.borrow_mut().misses += 1);
    classified
}

/// Clears cache contents and hit/miss stats for deterministic VM executions.
pub fn reset_cache_and_stats() {
    CLASSIFICATION_CACHE.with(|cache| cache.borrow_mut().clear());
    reset_cache_stats();
}

/// Clears only numeric-string cache hit/miss stats.
pub fn reset_cache_stats() {
    CLASSIFICATION_STATS.with(|stats| *stats.borrow_mut() = NumericStringCacheStats::default());
}

/// Returns and clears numeric-string cache hit/miss stats.
#[must_use]
pub fn take_cache_stats() -> NumericStringCacheStats {
    CLASSIFICATION_STATS.with(|stats| {
        let mut stats = stats.borrow_mut();
        let current = *stats;
        *stats = NumericStringCacheStats::default();
        current
    })
}

/// Classifies a byte string using the Phase 5 PHP numeric-string subset.
#[must_use]
pub fn classify(bytes: &[u8]) -> NumericString {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let bytes = &bytes[start..];
    if bytes.is_empty() {
        return non_numeric();
    }
    let Some(prefix) = numeric_prefix_len(bytes) else {
        return non_numeric();
    };
    let value = parse_numeric_prefix(&bytes[..prefix]);
    let Some(value) = value else {
        return non_numeric();
    };
    let trailing = &bytes[prefix..];
    if trailing.iter().all(u8::is_ascii_whitespace) {
        let kind = if value.is_float() {
            NumericStringKind::FloatString
        } else {
            NumericStringKind::IntString
        };
        return NumericString {
            kind,
            value: Some(value),
        };
    }
    NumericString {
        kind: NumericStringKind::LeadingNumeric,
        value: Some(value),
    }
}

fn fingerprint(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn non_numeric() -> NumericString {
    NumericString {
        kind: NumericStringKind::NonNumeric,
        value: None,
    }
}

fn numeric_prefix_len(bytes: &[u8]) -> Option<usize> {
    let mut index = usize::from(matches!(bytes.first(), Some(b'+') | Some(b'-')));
    let mut digits = 0usize;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        digits += 1;
        index += 1;
    }
    let mut has_fraction = false;
    if bytes.get(index) == Some(&b'.') {
        has_fraction = true;
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            digits += 1;
            index += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if matches!(bytes.get(index), Some(b'e') | Some(b'E')) {
        let exponent_marker = index;
        index += 1;
        if matches!(bytes.get(index), Some(b'+') | Some(b'-')) {
            index += 1;
        }
        let exponent_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == exponent_start {
            return Some(exponent_marker);
        }
    }
    if has_fraction || matches!(bytes.get(index), Some(b'e') | Some(b'E')) {
        return Some(index);
    }
    Some(index)
}

fn parse_numeric_prefix(bytes: &[u8]) -> Option<NumericStringValue> {
    let text = std::str::from_utf8(bytes).ok()?;
    let is_float = bytes.iter().any(|byte| matches!(byte, b'.' | b'e' | b'E'));
    if is_float {
        return text.parse::<f64>().ok().map(NumericStringValue::Float);
    }
    text.parse::<i64>()
        .map(NumericStringValue::Int)
        .or_else(|_| text.parse::<f64>().map(NumericStringValue::Float))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::{
        NumericStringKind, NumericStringValue, classify, classify_php_string,
        reset_cache_and_stats, take_cache_stats,
    };
    use crate::PhpString;

    #[test]
    fn numeric_string_classifies_full_int_float_and_whitespace() {
        assert_eq!(classify(b"0").kind, NumericStringKind::IntString);
        assert_eq!(classify(b"0").value, Some(NumericStringValue::Int(0)));
        assert_eq!(classify(b"0.0").kind, NumericStringKind::FloatString);
        assert_eq!(classify(b"0.0").value, Some(NumericStringValue::Float(0.0)));
        assert_eq!(classify(b" 42\t").kind, NumericStringKind::IntString);
        assert_eq!(classify(b" 42\t").value, Some(NumericStringValue::Int(42)));
    }

    #[test]
    fn numeric_string_classifies_leading_and_non_numeric() {
        assert_eq!(classify(b"42abc").kind, NumericStringKind::LeadingNumeric);
        assert_eq!(classify(b"42abc").value, Some(NumericStringValue::Int(42)));
        assert_eq!(classify(b"").kind, NumericStringKind::NonNumeric);
        assert_eq!(classify(b"abc").kind, NumericStringKind::NonNumeric);
    }

    #[test]
    fn numeric_string_cache_records_hits_misses_and_overflow() {
        reset_cache_and_stats();
        let value = PhpString::from("9223372036854775808");

        let first = classify_php_string(&value);
        let second = classify_php_string(&value);

        assert_eq!(first, second);
        assert_eq!(first.kind, NumericStringKind::FloatString);
        assert!(matches!(first.value, Some(NumericStringValue::Float(_))));
        let stats = take_cache_stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 1);
    }

    #[test]
    fn numeric_string_cache_separates_whitespace_and_non_numeric_cases() {
        reset_cache_and_stats();
        let int_with_space = PhpString::from(" 42\t");
        let leading = PhpString::from("42abc");
        let non_numeric = PhpString::from("abc");

        assert_eq!(
            classify_php_string(&int_with_space).kind,
            NumericStringKind::IntString
        );
        assert_eq!(
            classify_php_string(&leading).kind,
            NumericStringKind::LeadingNumeric
        );
        assert_eq!(
            classify_php_string(&non_numeric).kind,
            NumericStringKind::NonNumeric
        );
        assert_eq!(
            classify_php_string(&non_numeric).kind,
            NumericStringKind::NonNumeric
        );
        let stats = take_cache_stats();
        assert_eq!(stats.misses, 3);
        assert_eq!(stats.hits, 1);
    }

    #[test]
    fn numeric_string_cache_does_not_reuse_after_cow_or_in_place_mutation() {
        reset_cache_and_stats();
        let original = PhpString::from("12");
        let mut shared = original.clone();

        assert_eq!(
            classify_php_string(&original).kind,
            NumericStringKind::IntString
        );
        shared.bytes_mut()[0] = b'x';
        assert_eq!(
            classify_php_string(&shared).kind,
            NumericStringKind::NonNumeric
        );

        let mut unique = PhpString::from("34");
        assert_eq!(
            classify_php_string(&unique).kind,
            NumericStringKind::IntString
        );
        unique.bytes_mut()[0] = b'y';
        assert_eq!(
            classify_php_string(&unique).kind,
            NumericStringKind::NonNumeric
        );

        let stats = take_cache_stats();
        assert_eq!(stats.misses, 4);
        assert_eq!(stats.hits, 0);
    }
}
