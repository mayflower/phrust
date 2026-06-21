//! MVP scalar conversion and comparison helpers for Phase 4 execution.

use crate::{PhpString, Value};
use std::cmp::Ordering;

/// Numeric scalar produced by PHP-style scalar conversion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NumericValue {
    /// Integer value.
    Int(i64),
    /// Floating-point value.
    Float(f64),
}

impl NumericValue {
    /// Returns the value as an `f64`.
    #[must_use]
    pub const fn as_f64(self) -> f64 {
        match self {
            Self::Int(value) => value as f64,
            Self::Float(value) => value,
        }
    }

    /// Returns true when this value is represented as a float.
    #[must_use]
    pub const fn is_float(self) -> bool {
        matches!(self, Self::Float(_))
    }
}

/// Converts a scalar to PHP truthiness for the documented MVP subset.
pub fn to_bool(value: &Value) -> Result<bool, String> {
    match value {
        Value::Null => Ok(false),
        Value::Bool(value) => Ok(*value),
        Value::Int(value) => Ok(*value != 0),
        Value::Float(value) => {
            let value = value.to_f64();
            Ok(value != 0.0 && !value.is_nan())
        }
        Value::String(value) => Ok(!value.is_empty() && value.as_bytes() != b"0"),
        Value::Uninitialized => Err("cannot convert uninitialized value to bool".to_owned()),
        Value::Array(_) => Err("array truthiness is not implemented".to_owned()),
        Value::Object(_) => Err("object truthiness is not implemented".to_owned()),
        Value::Callable(_) => Err("callable truthiness is not implemented".to_owned()),
        Value::Reference(_) => Err("reference truthiness is not implemented".to_owned()),
    }
}

/// Converts a scalar to PHP string bytes for the documented MVP subset.
pub fn to_string(value: &Value) -> Result<PhpString, String> {
    match value {
        Value::Null | Value::Bool(false) => Ok(PhpString::from_bytes(Vec::new())),
        Value::Bool(true) => Ok(PhpString::from_test_str("1")),
        Value::Int(value) => Ok(PhpString::from_test_str(&value.to_string())),
        Value::Float(value) => Ok(PhpString::from_test_str(&value.to_string())),
        Value::String(value) => Ok(value.clone()),
        Value::Uninitialized => Err("cannot convert uninitialized value to string".to_owned()),
        Value::Array(_) => Err("array to string conversion is not implemented".to_owned()),
        Value::Object(_) => Err("object to string conversion is not implemented".to_owned()),
        Value::Callable(_) => Err("callable to string conversion is not implemented".to_owned()),
        Value::Reference(_) => Err("reference to string conversion is not implemented".to_owned()),
    }
}

/// Converts a scalar to a PHP numeric value for the documented MVP subset.
///
/// Known gap: this intentionally handles only plain decimal integer and float
/// strings. PHP's full numeric-string grammar, warnings, leading numeric
/// substrings, INF/NAN spelling, and locale-independent edge cases are deferred.
pub fn to_number(value: &Value) -> Result<NumericValue, String> {
    match value {
        Value::Null | Value::Bool(false) => Ok(NumericValue::Int(0)),
        Value::Bool(true) => Ok(NumericValue::Int(1)),
        Value::Int(value) => Ok(NumericValue::Int(*value)),
        Value::Float(value) => Ok(NumericValue::Float(value.to_f64())),
        Value::String(value) => parse_plain_numeric_string(value),
        Value::Uninitialized => Err("cannot convert uninitialized value to number".to_owned()),
        Value::Array(_) => Err("array to number conversion is not implemented".to_owned()),
        Value::Object(_) => Err("object to number conversion is not implemented".to_owned()),
        Value::Callable(_) => Err("callable to number conversion is not implemented".to_owned()),
        Value::Reference(_) => Err("reference to number conversion is not implemented".to_owned()),
    }
}

/// Strict identity for scalar MVP values.
pub fn identical(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Int(left), Value::Int(right)) => left == right,
        (Value::Float(left), Value::Float(right)) => left == right,
        (Value::String(left), Value::String(right)) => left == right,
        _ => false,
    }
}

/// Loose equality for safe scalar MVP cases.
pub fn equal(left: &Value, right: &Value) -> Result<bool, String> {
    Ok(compare(left, right)? == Ordering::Equal)
}

/// Loose comparison for safe scalar MVP cases.
pub fn compare(left: &Value, right: &Value) -> Result<Ordering, String> {
    match (left, right) {
        (Value::String(left), Value::String(right)) => {
            if let (Ok(left), Ok(right)) = (
                parse_plain_numeric_string(left),
                parse_plain_numeric_string(right),
            ) {
                return compare_numbers(left, right);
            }
            return Ok(left.as_bytes().cmp(right.as_bytes()));
        }
        (Value::Bool(_), _) | (_, Value::Bool(_)) | (Value::Null, _) | (_, Value::Null) => {
            return Ok(to_bool(left)?.cmp(&to_bool(right)?));
        }
        _ => {}
    }

    match (left, right) {
        (Value::Int(_) | Value::Float(_), Value::Int(_) | Value::Float(_)) => {
            compare_numbers(to_number(left)?, to_number(right)?)
        }
        (Value::String(_), Value::Int(_) | Value::Float(_))
        | (Value::Int(_) | Value::Float(_), Value::String(_)) => {
            compare_numbers(to_number(left)?, to_number(right)?)
        }
        (Value::String(left), Value::String(right)) => Ok(left.as_bytes().cmp(right.as_bytes())),
        _ => Err(format!(
            "loose comparison is not implemented for {} and {}",
            type_name(left),
            type_name(right)
        )),
    }
}

fn compare_numbers(left: NumericValue, right: NumericValue) -> Result<Ordering, String> {
    let Some(ordering) = left.as_f64().partial_cmp(&right.as_f64()) else {
        return Err("cannot compare NaN numeric values".to_owned());
    };
    Ok(ordering)
}

fn parse_plain_numeric_string(value: &PhpString) -> Result<NumericValue, String> {
    let text = std::str::from_utf8(value.as_bytes())
        .map_err(|_| "non-UTF-8 numeric strings are a known gap".to_owned())?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(NumericValue::Int(0));
    }
    if !is_plain_numeric_string(trimmed) {
        return Err(format!(
            "numeric string is outside Phase 4 MVP: {trimmed:?}"
        ));
    }
    if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
        return trimmed
            .parse::<f64>()
            .map(NumericValue::Float)
            .map_err(|error| format!("invalid float numeric string {trimmed:?}: {error}"));
    }
    trimmed
        .parse::<i64>()
        .map(NumericValue::Int)
        .map_err(|error| format!("invalid int numeric string {trimmed:?}: {error}"))
}

fn is_plain_numeric_string(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = usize::from(matches!(bytes.first(), Some(b'+') | Some(b'-')));
    let mut digits = 0usize;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        digits += 1;
        index += 1;
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            digits += 1;
            index += 1;
        }
    }
    if digits == 0 {
        return false;
    }
    if matches!(bytes.get(index), Some(b'e') | Some(b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+') | Some(b'-')) {
            index += 1;
        }
        let exponent_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == exponent_start {
            return false;
        }
    }
    index == bytes.len()
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Uninitialized => "uninitialized",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Callable(_) => "callable",
        Value::Reference(_) => "reference",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_truthiness_matches_scalar_mvp() {
        assert!(!to_bool(&Value::Null).unwrap());
        assert!(!to_bool(&Value::Bool(false)).unwrap());
        assert!(to_bool(&Value::Bool(true)).unwrap());
        assert!(!to_bool(&Value::Int(0)).unwrap());
        assert!(to_bool(&Value::Int(-1)).unwrap());
        assert!(!to_bool(&Value::string(b"".to_vec())).unwrap());
        assert!(!to_bool(&Value::string(b"0".to_vec())).unwrap());
        assert!(to_bool(&Value::string(b"00".to_vec())).unwrap());
    }

    #[test]
    fn convert_scalar_to_string_matches_echo_mvp() {
        assert_eq!(to_string(&Value::Null).unwrap().as_bytes(), b"");
        assert_eq!(to_string(&Value::Bool(false)).unwrap().as_bytes(), b"");
        assert_eq!(to_string(&Value::Bool(true)).unwrap().as_bytes(), b"1");
        assert_eq!(to_string(&Value::Int(42)).unwrap().as_bytes(), b"42");
        assert_eq!(to_string(&Value::float(1.5)).unwrap().as_bytes(), b"1.5");
        assert_eq!(
            to_string(&Value::string(b"x".to_vec())).unwrap().as_bytes(),
            b"x"
        );
    }

    #[test]
    fn convert_scalar_to_number_handles_plain_numeric_strings() {
        assert_eq!(to_number(&Value::Null).unwrap(), NumericValue::Int(0));
        assert_eq!(to_number(&Value::Bool(true)).unwrap(), NumericValue::Int(1));
        assert_eq!(
            to_number(&Value::string(b"12".to_vec())).unwrap(),
            NumericValue::Int(12)
        );
        assert_eq!(
            to_number(&Value::string(b"1.5".to_vec())).unwrap(),
            NumericValue::Float(1.5)
        );
        assert!(to_number(&Value::string(b"12abc".to_vec())).is_err());
    }

    #[test]
    fn convert_comparison_handles_safe_scalar_mvp() {
        assert!(equal(&Value::Int(1), &Value::float(1.0)).unwrap());
        assert!(identical(&Value::Int(1), &Value::Int(1)));
        assert!(!identical(&Value::Int(1), &Value::float(1.0)));
        assert_eq!(
            compare(&Value::string(b"2".to_vec()), &Value::Int(10)).unwrap(),
            Ordering::Less
        );
    }
}
