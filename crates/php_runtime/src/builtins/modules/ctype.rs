//! ASCII C-locale ctype extension MVP.

use super::core::{arity_error, string_arg};
use crate::Value;
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "ctype_alnum",
        builtin_ctype_alnum,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "ctype_alpha",
        builtin_ctype_alpha,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "ctype_cntrl",
        builtin_ctype_cntrl,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "ctype_digit",
        builtin_ctype_digit,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "ctype_graph",
        builtin_ctype_graph,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "ctype_lower",
        builtin_ctype_lower,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "ctype_print",
        builtin_ctype_print,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "ctype_punct",
        builtin_ctype_punct,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "ctype_space",
        builtin_ctype_space,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "ctype_upper",
        builtin_ctype_upper,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "ctype_xdigit",
        builtin_ctype_xdigit,
        BuiltinCompatibility::Php,
    ),
];

macro_rules! ctype_builtin {
    ($name:ident, $php_name:literal, $predicate:expr) => {
        fn $name(
            _context: &mut BuiltinContext<'_>,
            args: Vec<Value>,
            _span: RuntimeSourceSpan,
        ) -> BuiltinResult {
            if args.len() != 1 {
                return Err(arity_error($php_name, "one argument"));
            }
            let input = string_arg($php_name, &args[0])?;
            let bytes = input.as_bytes();
            Ok(Value::Bool(
                !bytes.is_empty() && bytes.iter().copied().all($predicate),
            ))
        }
    };
}

ctype_builtin!(builtin_ctype_alnum, "ctype_alnum", |byte: u8| byte
    .is_ascii_alphanumeric());
ctype_builtin!(builtin_ctype_alpha, "ctype_alpha", |byte: u8| byte
    .is_ascii_alphabetic());
ctype_builtin!(builtin_ctype_cntrl, "ctype_cntrl", |byte: u8| byte
    .is_ascii_control());
ctype_builtin!(builtin_ctype_digit, "ctype_digit", |byte: u8| byte
    .is_ascii_digit());
ctype_builtin!(builtin_ctype_graph, "ctype_graph", |byte: u8| byte
    .is_ascii_graphic());
ctype_builtin!(builtin_ctype_lower, "ctype_lower", |byte: u8| byte
    .is_ascii_lowercase());
ctype_builtin!(builtin_ctype_print, "ctype_print", |byte: u8| byte
    .is_ascii_graphic()
    || byte == b' ');
ctype_builtin!(builtin_ctype_punct, "ctype_punct", |byte: u8| byte
    .is_ascii_punctuation());
ctype_builtin!(builtin_ctype_space, "ctype_space", |byte: u8| byte
    .is_ascii_whitespace());
ctype_builtin!(builtin_ctype_upper, "ctype_upper", |byte: u8| byte
    .is_ascii_uppercase());
ctype_builtin!(builtin_ctype_xdigit, "ctype_xdigit", |byte: u8| byte
    .is_ascii_hexdigit());
