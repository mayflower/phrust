//! Scalar conversion and comparison helpers for Phase 4/5 execution.

use crate::{
    PhpString, Value,
    numeric_string::{NumericStringKind, NumericStringValue, classify_php_string},
};
use std::cell::Cell;
use std::cmp::Ordering;

const DEFAULT_FLOAT_STRING_PRECISION: i32 = 14;

thread_local! {
    static FLOAT_STRING_PRECISION: Cell<i32> = const { Cell::new(DEFAULT_FLOAT_STRING_PRECISION) };
}

/// Numeric scalar produced by PHP-style scalar conversion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NumericValue {
    /// Integer value.
    Int(i64),
    /// Floating-point value.
    Float(f64),
}

/// Resets request-local float-to-string precision to PHP's default.
pub fn reset_float_string_precision() {
    set_float_string_precision(DEFAULT_FLOAT_STRING_PRECISION);
}

/// Sets request-local float-to-string precision for INI-driven VM execution.
pub fn set_float_string_precision(precision: i32) {
    FLOAT_STRING_PRECISION.with(|cell| cell.set(precision.clamp(0, 17)));
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
        Value::Array(array) => Ok(!array.is_empty()),
        Value::Object(_) | Value::Resource(_) | Value::Fiber(_) | Value::Generator(_) => Ok(true),
        Value::Callable(_) => Err("callable truthiness is not implemented".to_owned()),
        Value::Reference(cell) => to_bool(&cell.get()),
    }
}

/// Converts a scalar to PHP string bytes for the documented MVP subset.
pub fn to_string(value: &Value) -> Result<PhpString, String> {
    match value {
        Value::Null | Value::Bool(false) => Ok(PhpString::from_bytes(Vec::new())),
        Value::Bool(true) => Ok(PhpString::from_test_str("1")),
        Value::Int(value) => Ok(PhpString::from_test_str(&value.to_string())),
        Value::Float(value) => Ok(PhpString::from_test_str(&float_to_php_string(
            value.to_f64(),
        ))),
        Value::String(value) => Ok(value.clone()),
        Value::Uninitialized => Err("cannot convert uninitialized value to string".to_owned()),
        Value::Array(_) => Err(
            "E_PHP_RUNTIME_ARRAY_TO_STRING_GAP: array to string conversion is not implemented"
                .to_owned(),
        ),
        Value::Object(_) | Value::Fiber(_) | Value::Generator(_) => Err(
            "E_PHP_RUNTIME_OBJECT_TO_STRING_GAP: object __toString conversion is not implemented"
                .to_owned(),
        ),
        Value::Resource(resource) => Ok(PhpString::from_test_str(&format!(
            "Resource id #{}",
            resource.id().get()
        ))),
        Value::Callable(_) => Err("callable to string conversion is not implemented".to_owned()),
        Value::Reference(cell) => to_string(&cell.get()),
    }
}

fn float_to_php_string(value: f64) -> String {
    if value.is_nan() {
        "NAN".to_owned()
    } else if value.is_infinite() {
        if value.is_sign_negative() {
            "-INF".to_owned()
        } else {
            "INF".to_owned()
        }
    } else if FLOAT_STRING_PRECISION.with(Cell::get) == 0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

/// Converts a value to an integer using explicit PHP cast rules in the subset.
pub fn to_int(value: &Value) -> Result<i64, String> {
    match value {
        Value::Null | Value::Bool(false) => Ok(0),
        Value::Bool(true) => Ok(1),
        Value::Int(value) => Ok(*value),
        Value::Float(value) => Ok(value.to_f64() as i64),
        Value::String(value) => Ok(classify_php_string(value)
            .value
            .map_or(0, NumericStringValue::to_i64)),
        Value::Uninitialized => Err("cannot convert uninitialized value to int".to_owned()),
        Value::Array(array) => Ok(if array.is_empty() { 0 } else { 1 }),
        Value::Object(_) | Value::Fiber(_) | Value::Generator(_) => {
            Err("E_PHP_RUNTIME_OBJECT_NUMERIC_CONVERSION_GAP: object to int conversion is not implemented".to_owned())
        }
        Value::Resource(resource) => Ok(resource.id().get() as i64),
        Value::Callable(_) => Err("callable to int conversion is not implemented".to_owned()),
        Value::Reference(cell) => to_int(&cell.get()),
    }
}

/// Converts a value to a float using explicit PHP cast rules in the subset.
pub fn to_float(value: &Value) -> Result<f64, String> {
    match value {
        Value::Null | Value::Bool(false) => Ok(0.0),
        Value::Bool(true) => Ok(1.0),
        Value::Int(value) => Ok(*value as f64),
        Value::Float(value) => Ok(value.to_f64()),
        Value::String(value) => Ok(classify_php_string(value)
            .value
            .map_or(0.0, NumericStringValue::as_f64)),
        Value::Uninitialized => Err("cannot convert uninitialized value to float".to_owned()),
        Value::Array(array) => Ok(if array.is_empty() { 0.0 } else { 1.0 }),
        Value::Object(_) | Value::Fiber(_) | Value::Generator(_) => {
            Err("E_PHP_RUNTIME_OBJECT_NUMERIC_CONVERSION_GAP: object to float conversion is not implemented".to_owned())
        }
        Value::Resource(resource) => Ok(resource.id().get() as f64),
        Value::Callable(_) => Err("callable to float conversion is not implemented".to_owned()),
        Value::Reference(cell) => to_float(&cell.get()),
    }
}

/// Converts a value to a PHP numeric value for arithmetic operators.
pub fn to_number(value: &Value) -> Result<NumericValue, String> {
    match value {
        Value::Null | Value::Bool(false) => Ok(NumericValue::Int(0)),
        Value::Bool(true) => Ok(NumericValue::Int(1)),
        Value::Int(value) => Ok(NumericValue::Int(*value)),
        Value::Float(value) => Ok(NumericValue::Float(value.to_f64())),
        Value::String(value) => arithmetic_numeric_string(value),
        Value::Uninitialized => Err("cannot convert uninitialized value to number".to_owned()),
        Value::Array(_) => Err("array to number conversion is not implemented".to_owned()),
        Value::Object(_) | Value::Fiber(_) | Value::Generator(_) => {
            Err("E_PHP_RUNTIME_OBJECT_NUMERIC_CONVERSION_GAP: object to number conversion is not implemented".to_owned())
        }
        Value::Resource(resource) => Ok(NumericValue::Int(resource.id().get() as i64)),
        Value::Callable(_) => Err("callable to number conversion is not implemented".to_owned()),
        Value::Reference(cell) => to_number(&cell.get()),
    }
}

/// Strict identity for Phase 5 runtime values.
pub fn identical(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Int(left), Value::Int(right)) => left == right,
        (Value::Float(left), Value::Float(right)) => left == right,
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Array(left), Value::Array(right)) => arrays_identical(left, right),
        (Value::Object(left), Value::Object(right)) => left.id() == right.id(),
        (Value::Resource(left), Value::Resource(right)) => left.id() == right.id(),
        (Value::Reference(left), Value::Reference(right)) if left.ptr_eq(right) => true,
        (Value::Reference(left), right) => identical(&left.get(), right),
        (left, Value::Reference(right)) => identical(left, &right.get()),
        _ => false,
    }
}

/// Loose equality for Phase 5 comparison cases.
pub fn equal(left: &Value, right: &Value) -> Result<bool, String> {
    match (left, right) {
        (Value::Array(left), Value::Array(right)) => arrays_equal(left, right),
        (Value::Array(_), _) | (_, Value::Array(_)) => Ok(false),
        (Value::Object(left), Value::Object(right)) => objects_equal(left, right),
        (Value::Object(_), _) | (_, Value::Object(_)) => Ok(false),
        (Value::Resource(left), Value::Resource(right)) => Ok(left.id() == right.id()),
        (Value::Resource(_), _) | (_, Value::Resource(_)) => Ok(false),
        (Value::Reference(left), Value::Reference(right)) if left.ptr_eq(right) => Ok(true),
        (Value::Reference(left), right) => equal(&left.get(), right),
        (left, Value::Reference(right)) => equal(left, &right.get()),
        _ => Ok(compare(left, right)? == Ordering::Equal),
    }
}

/// Loose comparison for Phase 5 comparison cases.
pub fn compare(left: &Value, right: &Value) -> Result<Ordering, String> {
    match (left, right) {
        (Value::Reference(left), Value::Reference(right)) if left.ptr_eq(right) => {
            return Ok(Ordering::Equal);
        }
        (Value::Reference(left), right) => return compare(&left.get(), right),
        (left, Value::Reference(right)) => return compare(left, &right.get()),
        (Value::String(left), Value::String(right)) => {
            if let (Some(left), Some(right)) = (
                comparison_numeric_string(left),
                comparison_numeric_string(right),
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
            compare_number_and_string(left, right)
        }
        (Value::String(left), Value::String(right)) => Ok(left.as_bytes().cmp(right.as_bytes())),
        (Value::Array(left), Value::Array(right)) => arrays_compare(left, right),
        (Value::Array(_), _) => Ok(Ordering::Greater),
        (_, Value::Array(_)) => Ok(Ordering::Less),
        (Value::Object(left), Value::Object(right)) => objects_compare(left, right),
        (Value::Object(_), _) => Ok(Ordering::Greater),
        (_, Value::Object(_)) => Ok(Ordering::Less),
        (Value::Resource(left), Value::Resource(right)) => Ok(left.id().cmp(&right.id())),
        (Value::Resource(_), _) => Ok(Ordering::Greater),
        (_, Value::Resource(_)) => Ok(Ordering::Less),
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

fn compare_number_and_string(left: &Value, right: &Value) -> Result<Ordering, String> {
    match (left, right) {
        (Value::String(string), number) => {
            if let Some(string) = comparison_numeric_string(string) {
                return compare_numbers(string, to_number(number)?);
            }
            if is_nan_number(number) {
                return Ok(Ordering::Greater);
            }
            Ok(string.as_bytes().cmp(to_string(number)?.as_bytes()))
        }
        (number, Value::String(string)) => {
            if let Some(string) = comparison_numeric_string(string) {
                return compare_numbers(to_number(number)?, string);
            }
            if is_nan_number(number) {
                return Ok(Ordering::Less);
            }
            Ok(to_string(number)?.as_bytes().cmp(string.as_bytes()))
        }
        _ => unreachable!("compare_number_and_string requires one string and one number"),
    }
}

fn is_nan_number(value: &Value) -> bool {
    matches!(value, Value::Float(value) if value.to_f64().is_nan())
}

fn comparison_numeric_string(value: &PhpString) -> Option<NumericValue> {
    let classified = classify_php_string(value);
    match (classified.kind, classified.value) {
        (
            NumericStringKind::IntString | NumericStringKind::FloatString,
            Some(NumericStringValue::Int(value)),
        ) => Some(NumericValue::Int(value)),
        (
            NumericStringKind::IntString | NumericStringKind::FloatString,
            Some(NumericStringValue::Float(value)),
        ) => Some(NumericValue::Float(value)),
        _ => None,
    }
}

fn arithmetic_numeric_string(value: &PhpString) -> Result<NumericValue, String> {
    let classified = classify_php_string(value);
    match (classified.kind, classified.value) {
        (
            NumericStringKind::IntString
            | NumericStringKind::FloatString
            | NumericStringKind::LeadingNumeric,
            Some(NumericStringValue::Int(value)),
        ) => Ok(NumericValue::Int(value)),
        (
            NumericStringKind::IntString
            | NumericStringKind::FloatString
            | NumericStringKind::LeadingNumeric,
            Some(NumericStringValue::Float(value)),
        ) => Ok(NumericValue::Float(value)),
        _ => Err(
            "E_PHP_RUNTIME_NON_NUMERIC_STRING: non-numeric string cannot be used as a number"
                .to_owned(),
        ),
    }
}

fn arrays_identical(left: &crate::PhpArray, right: &crate::PhpArray) -> bool {
    left.len() == right.len()
        && left.iter().zip(right.iter()).all(
            |((left_key, left_value), (right_key, right_value))| {
                left_key == right_key && identical(left_value, right_value)
            },
        )
}

fn arrays_equal(left: &crate::PhpArray, right: &crate::PhpArray) -> Result<bool, String> {
    if left.len() != right.len() {
        return Ok(false);
    }
    for (left_key, left_value) in left.iter() {
        let Some(right_value) = right.get(left_key) else {
            return Ok(false);
        };
        if !equal(left_value, right_value)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn arrays_compare(left: &crate::PhpArray, right: &crate::PhpArray) -> Result<Ordering, String> {
    match left.len().cmp(&right.len()) {
        Ordering::Equal => {}
        ordering => return Ok(ordering),
    }
    for (left_key, left_value) in left.iter() {
        let Some(right_value) = right.get(left_key) else {
            return Ok(Ordering::Greater);
        };
        let ordering = compare(left_value, right_value)?;
        if ordering != Ordering::Equal {
            return Ok(ordering);
        }
    }
    Ok(Ordering::Equal)
}

fn objects_equal(left: &crate::ObjectRef, right: &crate::ObjectRef) -> Result<bool, String> {
    if left.id() == right.id() {
        return Ok(true);
    }
    if left.class_name() != right.class_name() {
        return Ok(false);
    }
    let left_properties = left.properties_snapshot();
    let right_properties = right.properties_snapshot();
    if left_properties.len() != right_properties.len() {
        return Ok(false);
    }
    for ((left_name, left_value), (right_name, right_value)) in
        left_properties.iter().zip(right_properties.iter())
    {
        if left_name != right_name || !equal(left_value, right_value)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn objects_compare(left: &crate::ObjectRef, right: &crate::ObjectRef) -> Result<Ordering, String> {
    if objects_equal(left, right)? {
        return Ok(Ordering::Equal);
    }
    match left.class_name().cmp(&right.class_name()) {
        Ordering::Equal => {}
        ordering => return Ok(ordering),
    }
    Ok(left.id().cmp(&right.id()))
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
        Value::Object(_) | Value::Fiber(_) | Value::Generator(_) => "object",
        Value::Resource(_) => "resource",
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
        assert!(!to_bool(&Value::packed_array(Vec::new())).unwrap());
        assert!(to_bool(&Value::packed_array(vec![Value::Int(1)])).unwrap());
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
        assert_eq!(
            to_number(&Value::string(b" 42".to_vec())).unwrap(),
            NumericValue::Int(42)
        );
        assert_eq!(
            to_number(&Value::string(b"12abc".to_vec())).unwrap(),
            NumericValue::Int(12)
        );
        assert!(to_number(&Value::string(b"abc".to_vec())).is_err());
    }

    #[test]
    fn numeric_casts_handle_non_numeric_strings_and_arrays() {
        reset_float_string_precision();
        assert_eq!(to_int(&Value::string(b"".to_vec())).unwrap(), 0);
        assert_eq!(to_int(&Value::string(b"42abc".to_vec())).unwrap(), 42);
        assert_eq!(to_int(&Value::string(b"abc".to_vec())).unwrap(), 0);
        assert_eq!(to_float(&Value::string(b"0.5x".to_vec())).unwrap(), 0.5);
        assert_eq!(to_float(&Value::packed_array(Vec::new())).unwrap(), 0.0);
        assert_eq!(
            to_int(&Value::packed_array(vec![Value::Int(1)])).unwrap(),
            1
        );
        assert_eq!(
            to_string(&Value::float(f64::INFINITY)).unwrap().as_bytes(),
            b"INF"
        );
        assert_eq!(
            to_string(&Value::float(f64::NEG_INFINITY))
                .unwrap()
                .as_bytes(),
            b"-INF"
        );
        assert_eq!(
            to_string(&Value::float(f64::NAN)).unwrap().as_bytes(),
            b"NAN"
        );
        assert_eq!(to_string(&Value::float(1.75)).unwrap().as_bytes(), b"1.75");
        set_float_string_precision(0);
        assert_eq!(to_string(&Value::float(1.75)).unwrap().as_bytes(), b"2");
        reset_float_string_precision();
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

    #[test]
    fn compare_uses_php8_numeric_string_rules_without_arithmetic_errors() {
        assert!(equal(&Value::Int(42), &Value::string(b" 42".to_vec())).unwrap());
        assert!(!equal(&Value::Int(42), &Value::string(b"42abc".to_vec())).unwrap());
        assert!(!equal(&Value::Int(0), &Value::string(b"foo".to_vec())).unwrap());
        assert_eq!(
            compare(&Value::Int(0), &Value::string(b"foo".to_vec())).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            compare(&Value::string(b"foo".to_vec()), &Value::Int(0)).unwrap(),
            Ordering::Greater
        );
        assert!(
            equal(
                &Value::string(b"0e123".to_vec()),
                &Value::string(b"0".to_vec())
            )
            .unwrap()
        );
        assert_eq!(
            compare(
                &Value::string(b"42abc".to_vec()),
                &Value::string(b"42".to_vec())
            )
            .unwrap(),
            Ordering::Greater
        );
        assert!(
            equal(
                &Value::float(f64::INFINITY),
                &Value::string(b"INF".to_vec())
            )
            .unwrap()
        );
        assert!(
            equal(
                &Value::float(f64::NEG_INFINITY),
                &Value::string(b"-INF".to_vec())
            )
            .unwrap()
        );
        assert!(!equal(&Value::float(f64::NAN), &Value::string(b"NAN".to_vec())).unwrap());
    }

    #[test]
    fn compare_arrays_distinguishes_loose_and_strict_identity() {
        let first = {
            let mut array = crate::PhpArray::new();
            array.insert(crate::ArrayKey::Int(0), Value::string(b"1".to_vec()));
            array.insert(
                crate::ArrayKey::String(crate::PhpString::from("name")),
                Value::Int(2),
            );
            Value::Array(array)
        };
        let reordered = {
            let mut array = crate::PhpArray::new();
            array.insert(
                crate::ArrayKey::String(crate::PhpString::from("name")),
                Value::Int(2),
            );
            array.insert(crate::ArrayKey::Int(0), Value::Int(1));
            Value::Array(array)
        };

        assert!(equal(&first, &reordered).unwrap());
        assert!(!identical(&first, &reordered));
    }

    #[test]
    fn compare_references_and_objects_use_value_and_handle_identity() {
        let cell = crate::ReferenceCell::new(Value::Int(1));
        let alias = Value::Reference(cell.clone());
        let same_alias = Value::Reference(cell);
        let other_reference = Value::Reference(crate::ReferenceCell::new(Value::Int(1)));

        assert!(identical(&alias, &same_alias));
        assert!(equal(&alias, &other_reference).unwrap());
        assert!(identical(&alias, &other_reference));

        let class = crate::ClassEntry {
            name: "Box".to_owned(),
            parent: None,
            interfaces: Vec::new(),
            methods: Vec::new(),
            properties: vec![crate::ClassPropertyEntry {
                name: "value".to_owned(),
                default: Value::Int(1),
                type_: None,
                flags: crate::ClassPropertyFlags::default(),
                hooks: crate::ClassPropertyHooks::default(),
                attributes: Vec::new(),
            }],
            constants: Vec::new(),
            enum_cases: Vec::new(),
            attributes: Vec::new(),
            enum_backing_type: None,
            constructor_id: None,
            flags: crate::ClassFlags::default(),
        };
        let one = crate::ObjectRef::new(&class);
        let same_handle = one.clone();
        let clone = one.clone_shallow();

        assert!(identical(
            &Value::Object(one.clone()),
            &Value::Object(same_handle)
        ));
        assert!(!identical(
            &Value::Object(one.clone()),
            &Value::Object(clone.clone())
        ));
        assert!(equal(&Value::Object(one), &Value::Object(clone)).unwrap());
    }
}
