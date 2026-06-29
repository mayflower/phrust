//! Date builtin registry slice.

use super::core::*;
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};
use crate::{Value, datetime, to_bool};
use std::time::{SystemTime, UNIX_EPOCH};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new("date", builtin_date, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "date_format",
        builtin_date_format,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("date_diff", builtin_date_diff, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "date_interval_format",
        builtin_date_interval_format,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "date_default_timezone_get",
        builtin_date_default_timezone_get,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "date_default_timezone_set",
        builtin_date_default_timezone_set,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("gmdate", builtin_gmdate, BuiltinCompatibility::Php),
    BuiltinEntry::new("microtime", builtin_microtime, BuiltinCompatibility::Php),
    BuiltinEntry::new("strtotime", builtin_strtotime, BuiltinCompatibility::Php),
    BuiltinEntry::new("hrtime", builtin_hrtime, BuiltinCompatibility::Php),
    BuiltinEntry::new("time", builtin_time, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "timezone_identifiers_list",
        builtin_timezone_identifiers_list,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "timezone_name_get",
        builtin_timezone_name_get,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "timezone_open",
        builtin_timezone_open,
        BuiltinCompatibility::Php,
    ),
];

pub(in crate::builtins::modules) fn builtin_date_default_timezone_get(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("date_default_timezone_get", &args, 0)?;
    Ok(Value::string(context.default_timezone()))
}
pub(in crate::builtins::modules) fn builtin_date(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("date", "one or two argument(s)"));
    }
    let format = string_arg("date", &args[0])?.to_string_lossy();
    let timestamp = args
        .get(1)
        .map(|value| int_arg("date", value))
        .transpose()?
        .unwrap_or_else(datetime::current_timestamp);
    Ok(Value::string(datetime::format_timestamp(
        timestamp,
        context.default_timezone(),
        &format,
    )))
}
pub(in crate::builtins::modules) fn builtin_gmdate(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("gmdate", "one or two argument(s)"));
    }
    let format = string_arg("gmdate", &args[0])?.to_string_lossy();
    let timestamp = args
        .get(1)
        .map(|value| int_arg("gmdate", value))
        .transpose()?
        .unwrap_or_else(datetime::current_timestamp);
    Ok(Value::string(datetime::format_timestamp(
        timestamp, "GMT", &format,
    )))
}
pub(in crate::builtins::modules) fn builtin_time(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("time", &args, 0)?;
    Ok(Value::Int(datetime::current_timestamp()))
}
pub(in crate::builtins::modules) fn builtin_microtime(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error("microtime", "zero or one argument(s)"));
    }
    let as_float = args
        .first()
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("microtime", message))?
        .unwrap_or(false);
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| value_error("microtime", "system time is before UNIX epoch"))?;
    let seconds = elapsed.as_secs();
    let micros = elapsed.subsec_micros();
    if as_float {
        return Ok(Value::float(
            seconds as f64 + f64::from(micros) / 1_000_000.0,
        ));
    }
    Ok(Value::string(format!("0.{micros:06} {seconds}")))
}
pub(in crate::builtins::modules) fn builtin_hrtime(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error("hrtime", "zero or one argument(s)"));
    }
    let as_number = args
        .first()
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("hrtime", message))?
        .unwrap_or(false);
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| value_error("hrtime", "system time is before UNIX epoch"))?;
    let seconds = i64::try_from(elapsed.as_secs())
        .map_err(|_| value_error("hrtime", "timestamp exceeds PHP integer range"))?;
    let nanos = i64::from(elapsed.subsec_nanos());
    if as_number {
        let total = seconds
            .checked_mul(1_000_000_000)
            .and_then(|value| value.checked_add(nanos))
            .ok_or_else(|| value_error("hrtime", "timestamp exceeds PHP integer range"))?;
        return Ok(Value::Int(total));
    }
    Ok(Value::packed_array(vec![
        Value::Int(seconds),
        Value::Int(nanos),
    ]))
}
pub(in crate::builtins::modules) fn builtin_strtotime(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("strtotime", "one or two argument(s)"));
    }
    let text = string_arg("strtotime", &args[0])?.to_string_lossy();
    let base = args
        .get(1)
        .map(|value| int_arg("strtotime", value))
        .transpose()?
        .unwrap_or_else(datetime::current_timestamp);
    Ok(datetime::parse_datetime_text(&text, base).map_or(Value::Bool(false), Value::Int))
}
pub(in crate::builtins::modules) fn builtin_date_format(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("date_format", &args, 2)?;
    let Value::Object(object) = deref_value(&args[0]) else {
        return Err(type_error("date_format", "DateTimeInterface", &args[0]));
    };
    let format = string_arg("date_format", &args[1])?.to_string_lossy();
    let timestamp = datetime::object_timestamp(&object)
        .ok_or_else(|| value_error("date_format", "object is not a DateTimeInterface MVP"))?;
    let timezone = datetime::object_timezone(&object).unwrap_or_else(|| "UTC".to_string());
    Ok(Value::string(datetime::format_timestamp(
        timestamp, &timezone, &format,
    )))
}
pub(in crate::builtins::modules) fn builtin_date_diff(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("date_diff", &args, 2)?;
    let Value::Object(left) = deref_value(&args[0]) else {
        return Err(type_error("date_diff", "DateTimeInterface", &args[0]));
    };
    let Value::Object(right) = deref_value(&args[1]) else {
        return Err(type_error("date_diff", "DateTimeInterface", &args[1]));
    };
    if datetime::object_timestamp(&left).is_none() {
        return Err(value_error(
            "date_diff",
            "first object is not a DateTimeInterface MVP",
        ));
    }
    if datetime::object_timestamp(&right).is_none() {
        return Err(value_error(
            "date_diff",
            "second object is not a DateTimeInterface MVP",
        ));
    }
    Ok(datetime::diff_objects(&left, &right))
}
pub(in crate::builtins::modules) fn builtin_timezone_open(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("timezone_open", &args, 1)?;
    let timezone = string_arg("timezone_open", &args[0])?.to_string_lossy();
    Ok(datetime::datetimezone_object(&timezone).unwrap_or(Value::Bool(false)))
}
pub(in crate::builtins::modules) fn builtin_timezone_name_get(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("timezone_name_get", &args, 1)?;
    let Value::Object(object) = deref_value(&args[0]) else {
        return Err(type_error("timezone_name_get", "DateTimeZone", &args[0]));
    };
    Ok(datetime::object_timezone(&object).map_or(Value::Bool(false), Value::string))
}
pub(in crate::builtins::modules) fn builtin_date_interval_format(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("date_interval_format", &args, 2)?;
    let Value::Object(object) = deref_value(&args[0]) else {
        return Err(type_error("date_interval_format", "DateInterval", &args[0]));
    };
    let seconds = match object.get_property("__seconds") {
        Some(Value::Int(value)) => value,
        _ => {
            return Err(value_error(
                "date_interval_format",
                "object is not a DateInterval MVP",
            ));
        }
    };
    let format = string_arg("date_interval_format", &args[1])?.to_string_lossy();
    Ok(Value::string(datetime::format_interval(seconds, &format)))
}
pub(in crate::builtins::modules) fn builtin_date_default_timezone_set(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("date_default_timezone_set", &args, 1)?;
    let identifier = string_arg("date_default_timezone_set", &args[0])?.to_string_lossy();
    if !datetime::is_valid_timezone(&identifier) {
        return Ok(Value::Bool(false));
    }
    context.set_default_timezone(identifier);
    Ok(Value::Bool(true))
}
pub(in crate::builtins::modules) fn builtin_timezone_identifiers_list(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 2 {
        return Err(arity_error(
            "timezone_identifiers_list",
            "zero to two argument(s)",
        ));
    }
    Ok(Value::packed_array(
        datetime::TIMEZONE_IDENTIFIERS
            .iter()
            .map(|identifier| Value::string(*identifier))
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::{BuiltinContext, RuntimeSourceSpan, builtin_date_diff};
    use crate::{OutputBuffer, Value, datetime};

    #[test]
    fn date_diff_returns_datetimeinterval_for_datetimeinterface_objects() {
        let Value::Object(left) = datetime::datetime_object(1_603_238_400, "UTC") else {
            panic!("expected DateTime object");
        };
        let Value::Object(right) = datetime::datetime_immutable_object(1_603_929_600, "UTC") else {
            panic!("expected DateTimeImmutable object");
        };
        let mut output = OutputBuffer::new();
        let mut context = BuiltinContext::new(&mut output);
        let result = builtin_date_diff(
            &mut context,
            vec![Value::Object(left), Value::Object(right)],
            RuntimeSourceSpan::default(),
        )
        .expect("date_diff succeeds");
        let Value::Object(interval) = result else {
            panic!("expected DateInterval object");
        };

        assert_eq!(interval.get_property("days"), Some(Value::Int(8)));
        assert_eq!(interval.get_property("d"), Some(Value::Int(8)));
        assert_eq!(interval.get_property("invert"), Some(Value::Int(0)));
    }
}
