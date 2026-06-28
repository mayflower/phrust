use php_source::byte_kernel::{find_any2, find_any3, find_byte};

/// Result of scanning a quoted string from scripting mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StringScan {
    /// The string can be emitted as `T_CONSTANT_ENCAPSED_STRING`.
    Constant { len: usize, terminated: bool },
    /// The string contains interpolation and belongs to a later encapsed mode.
    Interpolated,
}

/// Scans single-quoted strings and non-interpolated double-quoted strings.
pub(crate) fn scan_constant_encapsed_string(source: &str, start: usize) -> Option<StringScan> {
    let bytes = source.as_bytes();
    let (quote_start, prefix_len) = match (bytes.get(start), bytes.get(start + 1)) {
        (Some(b'b' | b'B'), Some(b'\'' | b'"')) => (start + 1, 1),
        _ => (start, 0),
    };
    match bytes.get(quote_start) {
        Some(b'\'') => Some(add_string_prefix_len(
            scan_single_quoted(bytes, quote_start),
            prefix_len,
        )),
        Some(b'"') => match scan_double_quoted(bytes, quote_start) {
            StringScan::Constant { len, terminated } => Some(StringScan::Constant {
                len: len + prefix_len,
                terminated,
            }),
            StringScan::Interpolated => {
                if prefix_len == 0 {
                    Some(StringScan::Interpolated)
                } else {
                    None
                }
            }
        },
        _ => None,
    }
}

fn add_string_prefix_len(scan: StringScan, prefix_len: usize) -> StringScan {
    match scan {
        StringScan::Constant { len, terminated } => StringScan::Constant {
            len: len + prefix_len,
            terminated,
        },
        StringScan::Interpolated => StringScan::Interpolated,
    }
}

fn scan_single_quoted(bytes: &[u8], start: usize) -> StringScan {
    let mut offset = start + 1;
    while offset < bytes.len() {
        let Some(relative) = find_any2(&bytes[offset..], b'\\', b'\'') else {
            break;
        };
        offset += relative;

        match bytes[offset] {
            b'\\' if matches!(bytes.get(offset + 1), Some(b'\\' | b'\'')) => {
                offset += 2;
            }
            b'\'' => {
                return StringScan::Constant {
                    len: offset + 1 - start,
                    terminated: true,
                };
            }
            _ => offset += 1,
        }
    }

    StringScan::Constant {
        len: bytes.len() - start,
        terminated: false,
    }
}

fn scan_double_quoted(bytes: &[u8], start: usize) -> StringScan {
    let mut offset = start + 1;
    while offset < bytes.len() {
        let next_escape_quote_or_dollar = find_any3(&bytes[offset..], b'\\', b'"', b'$');
        let next_open_brace = find_byte(&bytes[offset..], b'{');
        let Some(relative) = nearest_found(next_escape_quote_or_dollar, next_open_brace) else {
            break;
        };
        offset += relative;

        match bytes[offset] {
            b'\\' => {
                offset += 1;
                if offset < bytes.len() {
                    offset += 1;
                }
            }
            b'"' => {
                return StringScan::Constant {
                    len: offset + 1 - start,
                    terminated: true,
                };
            }
            b'$' if starts_interpolation_after_dollar(bytes, offset + 1) => {
                return StringScan::Interpolated;
            }
            b'{' if bytes.get(offset + 1) == Some(&b'$') => return StringScan::Interpolated,
            _ => offset += 1,
        }
    }

    StringScan::Constant {
        len: bytes.len() - start,
        terminated: false,
    }
}

fn nearest_found(first: Option<usize>, second: Option<usize>) -> Option<usize> {
    match (first, second) {
        (Some(first), Some(second)) => Some(first.min(second)),
        (Some(first), None) => Some(first),
        (None, Some(second)) => Some(second),
        (None, None) => None,
    }
}

fn starts_interpolation_after_dollar(bytes: &[u8], offset: usize) -> bool {
    matches!(bytes.get(offset), Some(b'{' | b'_'))
        || bytes
            .get(offset)
            .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte >= 0x80)
}

#[cfg(test)]
mod tests {
    use super::{StringScan, scan_constant_encapsed_string};

    #[test]
    fn single_quoted_strings_handle_php_escapes() {
        assert_eq!(
            scan_constant_encapsed_string("'it\\'s'", 0),
            Some(StringScan::Constant {
                len: 7,
                terminated: true
            })
        );
        assert_eq!(
            scan_constant_encapsed_string("'\\\\'", 0),
            Some(StringScan::Constant {
                len: 4,
                terminated: true
            })
        );
    }

    #[test]
    fn double_quoted_strings_without_interpolation_are_constant() {
        assert_eq!(
            scan_constant_encapsed_string("\"\\\\n\"", 0),
            Some(StringScan::Constant {
                len: 5,
                terminated: true
            })
        );
    }

    #[test]
    fn binary_prefixed_strings_are_constant_string_tokens() {
        assert_eq!(
            scan_constant_encapsed_string("b\"binary\"", 0),
            Some(StringScan::Constant {
                len: 9,
                terminated: true
            })
        );
        assert_eq!(
            scan_constant_encapsed_string("B'bin'", 0),
            Some(StringScan::Constant {
                len: 6,
                terminated: true
            })
        );
    }

    #[test]
    fn interpolated_double_quoted_strings_are_deferred() {
        assert_eq!(
            scan_constant_encapsed_string("\"$x\"", 0),
            Some(StringScan::Interpolated)
        );
        assert_eq!(
            scan_constant_encapsed_string("\"{$x}\"", 0),
            Some(StringScan::Interpolated)
        );
    }

    #[test]
    fn unterminated_strings_report_length_to_eof() {
        assert_eq!(
            scan_constant_encapsed_string("'abc", 0),
            Some(StringScan::Constant {
                len: 4,
                terminated: false
            })
        );
    }
}
