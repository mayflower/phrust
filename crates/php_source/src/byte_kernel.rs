//! Safe byte-oriented scanning helpers.
//!
//! These helpers are intentionally small facades around scalar reference logic
//! and well-maintained optimized byte-search routines. Public APIs stay safe and
//! byte-oriented so source, lexer, and runtime string callers can share the same
//! behavior without introducing UTF-8 assumptions.

/// Finds the first occurrence of `needle`.
#[must_use]
pub fn find_byte(bytes: &[u8], needle: u8) -> Option<usize> {
    memchr::memchr(needle, bytes)
}

/// Scalar reference implementation for [`find_byte`].
#[must_use]
pub fn find_byte_scalar(bytes: &[u8], needle: u8) -> Option<usize> {
    bytes.iter().position(|byte| *byte == needle)
}

/// Finds the first occurrence of either byte.
#[must_use]
pub fn find_any2(bytes: &[u8], first: u8, second: u8) -> Option<usize> {
    memchr::memchr2(first, second, bytes)
}

/// Scalar reference implementation for [`find_any2`].
#[must_use]
pub fn find_any2_scalar(bytes: &[u8], first: u8, second: u8) -> Option<usize> {
    bytes
        .iter()
        .position(|byte| *byte == first || *byte == second)
}

/// Finds the first occurrence of any of three bytes.
#[must_use]
pub fn find_any3(bytes: &[u8], first: u8, second: u8, third: u8) -> Option<usize> {
    memchr::memchr3(first, second, third, bytes)
}

/// Scalar reference implementation for [`find_any3`].
#[must_use]
pub fn find_any3_scalar(bytes: &[u8], first: u8, second: u8, third: u8) -> Option<usize> {
    bytes
        .iter()
        .position(|byte| *byte == first || *byte == second || *byte == third)
}

/// Counts PHP source line breaks.
///
/// `\n`, `\r\n`, and standalone `\r` each count as one line break. This
/// matches [`crate::LineIndex`] and keeps byte offsets as the source of truth.
#[must_use]
pub fn count_newlines(bytes: &[u8]) -> usize {
    let mut count = 0;
    let mut offset = 0;

    while offset < bytes.len() {
        let Some(relative) = memchr::memchr2(b'\n', b'\r', &bytes[offset..]) else {
            break;
        };
        offset += relative;
        count += 1;

        if bytes[offset] == b'\r' && bytes.get(offset + 1) == Some(&b'\n') {
            offset += 2;
        } else {
            offset += 1;
        }
    }

    count
}

/// Scalar reference implementation for [`count_newlines`].
#[must_use]
pub fn count_newlines_scalar(bytes: &[u8]) -> usize {
    let mut count = 0;
    let mut offset = 0;

    while offset < bytes.len() {
        match bytes[offset] {
            b'\n' => {
                count += 1;
                offset += 1;
            }
            b'\r' if bytes.get(offset + 1) == Some(&b'\n') => {
                count += 1;
                offset += 2;
            }
            b'\r' => {
                count += 1;
                offset += 1;
            }
            _ => {
                offset += 1;
            }
        }
    }

    count
}

/// Returns true when every byte is ASCII.
#[must_use]
pub fn is_all_ascii(bytes: &[u8]) -> bool {
    bytes.is_ascii()
}

/// Scalar reference implementation for [`is_all_ascii`].
#[must_use]
pub fn is_all_ascii_scalar(bytes: &[u8]) -> bool {
    bytes.iter().all(u8::is_ascii)
}

/// Returns true for ASCII identifier-start bytes.
///
/// This is ASCII-only by design. PHP lexer call sites must keep handling
/// non-ASCII identifier bytes separately.
#[must_use]
pub const fn is_ascii_identifier_start(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphabetic()
}

/// Returns true for ASCII identifier-continuation bytes.
///
/// This is ASCII-only by design. PHP lexer call sites must keep handling
/// non-ASCII identifier bytes separately.
#[must_use]
pub const fn is_ascii_identifier_continue(byte: u8) -> bool {
    is_ascii_identifier_start(byte) || byte.is_ascii_digit()
}

/// Returns the length of the initial ASCII identifier-continuation chunk.
#[must_use]
pub fn ascii_identifier_continue_chunk_len(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .position(|byte| !is_ascii_identifier_continue(*byte))
        .unwrap_or(bytes.len())
}

/// Scalar reference implementation for [`ascii_identifier_continue_chunk_len`].
#[must_use]
pub fn ascii_identifier_continue_chunk_len_scalar(bytes: &[u8]) -> usize {
    let mut len = 0;
    while len < bytes.len() && is_ascii_identifier_continue(bytes[len]) {
        len += 1;
    }
    len
}

/// Converts ASCII lowercase bytes to uppercase in place.
///
/// Non-ASCII and non-lowercase bytes are left unchanged.
pub fn ascii_uppercase_in_place(bytes: &mut [u8]) {
    for byte in bytes {
        byte.make_ascii_uppercase();
    }
}

/// Converts ASCII uppercase bytes to lowercase in place.
///
/// Non-ASCII and non-uppercase bytes are left unchanged.
pub fn ascii_lowercase_in_place(bytes: &mut [u8]) {
    for byte in bytes {
        byte.make_ascii_lowercase();
    }
}

/// Returns an ASCII-uppercase copy.
#[must_use]
pub fn ascii_uppercase_copy(bytes: &[u8]) -> Vec<u8> {
    let mut copy = bytes.to_vec();
    ascii_uppercase_in_place(&mut copy);
    copy
}

/// Returns an ASCII-lowercase copy.
#[must_use]
pub fn ascii_lowercase_copy(bytes: &[u8]) -> Vec<u8> {
    let mut copy = bytes.to_vec();
    ascii_lowercase_in_place(&mut copy);
    copy
}

#[cfg(test)]
mod tests {
    use super::{
        ascii_identifier_continue_chunk_len, ascii_identifier_continue_chunk_len_scalar,
        ascii_lowercase_copy, ascii_lowercase_in_place, ascii_uppercase_copy,
        ascii_uppercase_in_place, count_newlines, count_newlines_scalar, find_any2,
        find_any2_scalar, find_any3, find_any3_scalar, find_byte, find_byte_scalar, is_all_ascii,
        is_all_ascii_scalar, is_ascii_identifier_continue, is_ascii_identifier_start,
    };

    fn corpus() -> Vec<Vec<u8>> {
        let mut cases = vec![
            Vec::new(),
            b"a".to_vec(),
            b"\n".to_vec(),
            b"\r".to_vec(),
            b"\r\n".to_vec(),
            b"\xff".to_vec(),
            b"abc_def123".to_vec(),
            b"abc-def".to_vec(),
            b"\xff\xfeabc\n\r\nz\r".to_vec(),
            (0u8..=255).collect(),
        ];

        for len in [
            2usize, 3, 7, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1024, 4096,
        ] {
            let generated = (0..len)
                .map(|index| {
                    let value = ((index * 37) + (len * 11)) % 251;
                    match index % 97 {
                        0 => b'\n',
                        1 => b'\r',
                        2 => b'_',
                        3 => b'A',
                        4 => b'9',
                        _ => value as u8,
                    }
                })
                .collect();
            cases.push(generated);
        }

        for len in [1usize, 2, 8, 16, 32, 64, 128] {
            for needle in [0u8, b'a', b'\n', b'\r', 0xff] {
                let mut start = vec![b'x'; len];
                start[0] = needle;
                cases.push(start);

                let mut middle = vec![b'x'; len];
                middle[len / 2] = needle;
                cases.push(middle);

                let mut end = vec![b'x'; len];
                end[len - 1] = needle;
                cases.push(end);
            }
        }

        cases
    }

    #[test]
    fn byte_search_matches_scalar_reference() {
        for bytes in corpus() {
            for needle in [0u8, b'a', b'_', b'\n', b'\r', b'9', 0x80, 0xff] {
                assert_eq!(find_byte(&bytes, needle), find_byte_scalar(&bytes, needle));
            }
        }
    }

    #[test]
    fn multi_byte_search_matches_scalar_reference() {
        let pairs = [(b'\n', b'\r'), (b'a', b'z'), (0, 0xff), (b'_', b'-')];
        let triples = [
            (b'\n', b'\r', b';'),
            (b'a', b'z', b'_'),
            (0, 0x80, 0xff),
            (b'0', b'9', b'.'),
        ];

        for bytes in corpus() {
            for (first, second) in pairs {
                assert_eq!(
                    find_any2(&bytes, first, second),
                    find_any2_scalar(&bytes, first, second)
                );
            }

            for (first, second, third) in triples {
                assert_eq!(
                    find_any3(&bytes, first, second, third),
                    find_any3_scalar(&bytes, first, second, third)
                );
            }
        }
    }

    #[test]
    fn newline_count_matches_source_line_break_rules() {
        for bytes in corpus() {
            assert_eq!(count_newlines(&bytes), count_newlines_scalar(&bytes));
        }

        assert_eq!(count_newlines(b"a\nb"), 1);
        assert_eq!(count_newlines(b"a\r\nb"), 1);
        assert_eq!(count_newlines(b"a\rb"), 1);
        assert_eq!(count_newlines(b"\r\n\n\r"), 3);
    }

    #[test]
    fn ascii_detection_matches_scalar_reference() {
        for bytes in corpus() {
            assert_eq!(is_all_ascii(&bytes), is_all_ascii_scalar(&bytes));
        }

        assert!(is_all_ascii(b"abc_123"));
        assert!(!is_all_ascii(b"abc\xff"));
    }

    #[test]
    fn ascii_identifier_helpers_are_ascii_only() {
        assert!(is_ascii_identifier_start(b'_'));
        assert!(is_ascii_identifier_start(b'A'));
        assert!(is_ascii_identifier_start(b'z'));
        assert!(!is_ascii_identifier_start(b'9'));
        assert!(!is_ascii_identifier_start(0x80));

        assert!(is_ascii_identifier_continue(b'9'));
        assert!(is_ascii_identifier_continue(b'_'));
        assert!(!is_ascii_identifier_continue(b'-'));
        assert!(!is_ascii_identifier_continue(0xff));

        for bytes in corpus() {
            assert_eq!(
                ascii_identifier_continue_chunk_len(&bytes),
                ascii_identifier_continue_chunk_len_scalar(&bytes)
            );
        }

        assert_eq!(ascii_identifier_continue_chunk_len(b"abc_123-z"), 7);
        assert_eq!(ascii_identifier_continue_chunk_len(b"\xffabc"), 0);
    }

    #[test]
    fn ascii_case_helpers_match_standard_byte_semantics() {
        for bytes in corpus() {
            let expected_upper = bytes.to_ascii_uppercase();
            let expected_lower = bytes.to_ascii_lowercase();

            assert_eq!(ascii_uppercase_copy(&bytes), expected_upper);
            assert_eq!(ascii_lowercase_copy(&bytes), expected_lower);

            let mut upper = bytes.clone();
            ascii_uppercase_in_place(&mut upper);
            assert_eq!(upper, expected_upper);

            let mut lower = bytes;
            ascii_lowercase_in_place(&mut lower);
            assert_eq!(lower, expected_lower);
        }
    }
}
