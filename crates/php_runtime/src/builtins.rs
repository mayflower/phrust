//! Deterministic internal builtin registry for the Phase 4 VM.

use crate::{ArrayKey, OutputBuffer, Value, to_string};

/// Source location passed to internal builtins.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeSourceSpan {
    /// Optional source file path.
    pub file: Option<String>,
    /// Start byte offset.
    pub start: u32,
    /// End byte offset.
    pub end: u32,
}

/// Mutable runtime services available to internal builtins.
pub struct BuiltinContext<'a> {
    output: &'a mut OutputBuffer,
}

impl<'a> BuiltinContext<'a> {
    /// Creates a runtime context backed by the VM output buffer.
    #[must_use]
    pub fn new(output: &'a mut OutputBuffer) -> Self {
        Self { output }
    }

    /// Returns the output buffer.
    pub fn output(&mut self) -> &mut OutputBuffer {
        self.output
    }
}

/// Result returned by an internal builtin.
pub type BuiltinResult = Result<Value, BuiltinError>;

/// Internal builtin function signature.
pub type InternalFunction =
    fn(&mut BuiltinContext<'_>, Vec<Value>, RuntimeSourceSpan) -> BuiltinResult;

/// Runtime error reported by a builtin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuiltinError {
    diagnostic_id: &'static str,
    message: String,
}

impl BuiltinError {
    /// Creates a builtin error with a stable diagnostic ID.
    #[must_use]
    pub fn new(diagnostic_id: &'static str, message: impl Into<String>) -> Self {
        Self {
            diagnostic_id,
            message: message.into(),
        }
    }

    /// Stable diagnostic ID.
    #[must_use]
    pub const fn diagnostic_id(&self) -> &'static str {
        self.diagnostic_id
    }

    /// Human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Combines ID and message for VM runtime errors.
    #[must_use]
    pub fn display_message(&self) -> String {
        format!("{}: {}", self.diagnostic_id, self.message)
    }
}

/// Registered builtin entry.
#[derive(Clone, Copy, Debug)]
pub struct BuiltinEntry {
    name: &'static str,
    function: InternalFunction,
    compatibility: BuiltinCompatibility,
}

impl BuiltinEntry {
    /// Builtin name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Internal function pointer.
    #[must_use]
    pub const fn function(self) -> InternalFunction {
        self.function
    }

    /// Compatibility classification.
    #[must_use]
    pub const fn compatibility(self) -> BuiltinCompatibility {
        self.compatibility
    }
}

/// Whether a builtin is PHP-compatible or only for local fixtures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinCompatibility {
    /// PHP-compatible MVP builtin.
    Php,
    /// Internal test helper, not exposed as a PHP standard builtin.
    InternalTestHelper,
}

/// Deterministic builtin registry.
#[derive(Clone, Copy, Debug, Default)]
pub struct BuiltinRegistry;

impl BuiltinRegistry {
    /// Creates a builtin registry view.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Returns entries in stable sorted order.
    #[must_use]
    pub const fn entries(self) -> &'static [BuiltinEntry] {
        BUILTINS
    }

    /// Looks up a builtin by normalized name.
    #[must_use]
    pub fn get(self, name: &str) -> Option<BuiltinEntry> {
        BUILTINS.iter().copied().find(|entry| entry.name == name)
    }

    /// Returns true when a normalized name is registered.
    #[must_use]
    pub fn contains(self, name: &str) -> bool {
        self.get(name).is_some()
    }
}

const BUILTINS: &[BuiltinEntry] = &[
    BuiltinEntry {
        name: "gettype",
        function: builtin_gettype,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_array",
        function: builtin_is_array,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_bool",
        function: builtin_is_bool,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_int",
        function: builtin_is_int,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_null",
        function: builtin_is_null,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_string",
        function: builtin_is_string,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "print",
        function: builtin_print,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "serialize",
        function: builtin_serialize_gap,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strlen",
        function: builtin_strlen,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strtoupper",
        function: builtin_strtoupper,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "trim",
        function: builtin_trim,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "unserialize",
        function: builtin_unserialize_gap,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "var_dump",
        function: builtin_var_dump,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "var_export",
        function: builtin_var_export_gap,
        compatibility: BuiltinCompatibility::Php,
    },
];

fn expect_arity(name: &str, args: &[Value], expected: usize) -> Result<(), BuiltinError> {
    if args.len() == expected {
        return Ok(());
    }
    Err(BuiltinError::new(
        "E_PHP_RUNTIME_BUILTIN_ARITY",
        format!("builtin {name} expects exactly {expected} argument(s)"),
    ))
}

fn builtin_strlen(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("strlen", &args, 1)?;
    match args.into_iter().next().expect("checked arity") {
        Value::String(value) => Ok(Value::Int(value.len() as i64)),
        other => Err(type_error("strlen", "string", &other)),
    }
}

fn builtin_strtoupper(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("strtoupper", &args, 1)?;
    match args.into_iter().next().expect("checked arity") {
        Value::String(value) => {
            let upper = value
                .as_bytes()
                .iter()
                .map(u8::to_ascii_uppercase)
                .collect::<Vec<_>>();
            Ok(Value::string(upper))
        }
        other => Err(type_error("strtoupper", "string", &other)),
    }
}

fn builtin_trim(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("trim", &args, 1)?;
    match args.into_iter().next().expect("checked arity") {
        Value::String(value) => {
            let bytes = value.as_bytes();
            let start = bytes
                .iter()
                .position(|byte| !byte.is_ascii_whitespace())
                .unwrap_or(bytes.len());
            let end = bytes
                .iter()
                .rposition(|byte| !byte.is_ascii_whitespace())
                .map_or(start, |index| index + 1);
            Ok(Value::string(bytes[start..end].to_vec()))
        }
        other => Err(type_error("trim", "string", &other)),
    }
}

fn builtin_print(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("print", &args, 1)?;
    let value = args.into_iter().next().expect("checked arity");
    let string = to_string(&value).map_err(|message| {
        BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_TYPE",
            format!("builtin print could not convert value: {message}"),
        )
    })?;
    context.output().write_php_string(&string);
    Ok(Value::Int(1))
}

fn builtin_gettype(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("gettype", &args, 1)?;
    Ok(Value::string(php_gettype(
        &args.into_iter().next().expect("checked arity"),
    )))
}

fn builtin_is_int(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_int", &args, 1)?;
    Ok(Value::Bool(matches!(args.first(), Some(Value::Int(_)))))
}

fn builtin_is_string(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_string", &args, 1)?;
    Ok(Value::Bool(matches!(args.first(), Some(Value::String(_)))))
}

fn builtin_is_bool(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_bool", &args, 1)?;
    Ok(Value::Bool(matches!(args.first(), Some(Value::Bool(_)))))
}

fn builtin_is_null(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_null", &args, 1)?;
    Ok(Value::Bool(matches!(args.first(), Some(Value::Null))))
}

fn builtin_is_array(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_array", &args, 1)?;
    Ok(Value::Bool(matches!(args.first(), Some(Value::Array(_)))))
}

fn builtin_var_dump(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    for value in &args {
        write_var_dump_value(context.output(), value, 0);
    }
    Ok(Value::Null)
}

fn builtin_serialize_gap(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("serialize", &args, 1)?;
    Err(serialization_gap("serialize"))
}

fn builtin_unserialize_gap(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("unserialize", &args, 1)?;
    Err(serialization_gap("unserialize"))
}

fn builtin_var_export_gap(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin var_export expects one or two argument(s)",
        ));
    }
    Err(serialization_gap("var_export"))
}

fn serialization_gap(name: &str) -> BuiltinError {
    BuiltinError::new(
        "E_PHP_RUNTIME_SERIALIZATION_PHASE6_GAP",
        format!("{name} and serialization magic hooks are deferred to Phase 6"),
    )
}

fn type_error(name: &str, expected: &str, actual: &Value) -> BuiltinError {
    BuiltinError::new(
        "E_PHP_RUNTIME_BUILTIN_TYPE",
        format!(
            "builtin {name} expects {expected}, got {}",
            runtime_type_name(actual)
        ),
    )
}

fn php_gettype(value: &Value) -> &'static str {
    match value {
        Value::Null => "NULL",
        Value::Bool(_) => "boolean",
        Value::Int(_) => "integer",
        Value::Float(_) => "double",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) | Value::Fiber(_) | Value::Generator(_) => "object",
        Value::Callable(_) => "object",
        Value::Reference(_) => "reference",
        Value::Uninitialized => "NULL",
    }
}

fn runtime_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) | Value::Fiber(_) | Value::Generator(_) => "object",
        Value::Callable(_) => "callable",
        Value::Reference(_) => "reference",
        Value::Uninitialized => "uninitialized",
    }
}

fn write_var_dump_value(output: &mut OutputBuffer, value: &Value, indent: usize) {
    match value {
        Value::Null | Value::Uninitialized => output.write_test_str("NULL\n"),
        Value::Bool(true) => output.write_test_str("bool(true)\n"),
        Value::Bool(false) => output.write_test_str("bool(false)\n"),
        Value::Int(value) => output.write_test_str(&format!("int({value})\n")),
        Value::Float(value) => output.write_test_str(&format!("float({value})\n")),
        Value::String(value) => output.write_test_str(&format!(
            "string({}) \"{}\"\n",
            value.len(),
            value.to_string_lossy()
        )),
        Value::Array(array) => {
            output.write_test_str(&format!("array({}) {{\n", array.len()));
            for (key, element) in array.iter() {
                write_indent(output, indent + 2);
                match key {
                    ArrayKey::Int(index) => output.write_test_str(&format!("[{index}]=>\n")),
                    ArrayKey::String(key) => {
                        output.write_test_str(&format!("[\"{}\"]=>\n", key.to_string_lossy()))
                    }
                }
                write_indent(output, indent + 2);
                write_var_dump_value(output, element, indent + 2);
            }
            write_indent(output, indent);
            output.write_test_str("}\n");
        }
        Value::Object(object) => {
            output.write_test_str(&format!("object({})\n", object.class_name()))
        }
        Value::Fiber(_) => output.write_test_str("object(Fiber)\n"),
        Value::Generator(_) => output.write_test_str("object(Generator)\n"),
        Value::Callable(_) => output.write_test_str("object(Closure)#0 (0) {\n}\n"),
        Value::Reference(_) => output.write_test_str("reference(<placeholder>)\n"),
    }
}

fn write_indent(output: &mut OutputBuffer, spaces: usize) {
    output.write_bytes(vec![b' '; spaces]);
}

#[cfg(test)]
mod tests {
    use super::{BuiltinCompatibility, BuiltinContext, BuiltinRegistry, RuntimeSourceSpan};
    use crate::{OutputBuffer, Value};

    fn call(name: &str, args: Vec<Value>, output: &mut OutputBuffer) -> Value {
        let entry = BuiltinRegistry::new().get(name).expect("builtin exists");
        let mut context = BuiltinContext::new(output);
        (entry.function())(&mut context, args, RuntimeSourceSpan::default()).expect("builtin ok")
    }

    #[test]
    fn builtins_registry_is_sorted_and_classified() {
        let registry = BuiltinRegistry::new();
        let names = registry
            .entries()
            .iter()
            .map(|entry| entry.name())
            .collect::<Vec<_>>();
        let mut sorted = names.clone();
        sorted.sort_unstable();

        assert_eq!(names, sorted);
        assert!(registry.contains("print"));
        assert!(registry.contains("strlen"));
        assert!(
            registry
                .entries()
                .iter()
                .all(|entry| entry.compatibility() == BuiltinCompatibility::Php)
        );
    }

    #[test]
    fn builtins_cover_scalar_type_queries_and_print() {
        let mut output = OutputBuffer::new();

        assert_eq!(
            call("gettype", vec![Value::Int(7)], &mut output),
            Value::string("integer")
        );
        assert_eq!(
            call("is_int", vec![Value::Int(7)], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("is_string", vec![Value::string("x")], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("is_bool", vec![Value::Bool(false)], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("is_null", vec![Value::Null], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("is_array", vec![Value::packed_array(vec![])], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("print", vec![Value::string("p")], &mut output),
            Value::Int(1)
        );
        assert_eq!(output.to_string_lossy(), "p");
    }

    #[test]
    fn builtins_var_dump_is_stable_for_scalars_and_arrays() {
        let mut output = OutputBuffer::new();
        let result = call(
            "var_dump",
            vec![
                Value::Null,
                Value::Bool(true),
                Value::Int(7),
                Value::string("hi"),
                Value::packed_array(vec![Value::Int(1), Value::string("x")]),
            ],
            &mut output,
        );

        assert_eq!(result, Value::Null);
        assert_eq!(
            output.to_string_lossy(),
            "NULL\nbool(true)\nint(7)\nstring(2) \"hi\"\narray(2) {\n  [0]=>\n  int(1)\n  [1]=>\n  string(1) \"x\"\n}\n"
        );
    }

    #[test]
    fn serialization_builtins_report_phase6_gap() {
        for (name, args) in [
            ("serialize", vec![Value::Int(1)]),
            ("unserialize", vec![Value::string("i:1;")]),
            ("var_export", vec![Value::Int(1)]),
        ] {
            let entry = BuiltinRegistry::new().get(name).expect("builtin exists");
            let mut output = OutputBuffer::new();
            let mut context = BuiltinContext::new(&mut output);
            let error = (entry.function())(&mut context, args, RuntimeSourceSpan::default())
                .expect_err("serialization is a Phase 6 gap");
            assert_eq!(
                error.diagnostic_id(),
                "E_PHP_RUNTIME_SERIALIZATION_PHASE6_GAP"
            );
        }
    }
}
