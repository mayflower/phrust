//! PHP numeric-string classification for runtime conversion.

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

impl NumericString {
    /// Returns true when the string has a PHP numeric prefix.
    #[must_use]
    pub const fn has_numeric_value(self) -> bool {
        self.value.is_some()
    }
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
    use super::{NumericStringKind, NumericStringValue, classify};

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
}
