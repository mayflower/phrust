//! Output normalization helpers for runtime differential tests.

use std::path::Path;

/// Normalizes volatile stderr details from PHP runtime execution.
///
/// Rules:
///
/// - Convert CRLF and CR line endings to LF.
/// - Replace the tested fixture path with `{file}`.
/// - Replace the configured PHP binary path with `{php}`.
/// - Replace PHP line suffixes such as `on line 12` with `on line <line>`.
/// - Replace wall-clock looking decimal durations with `<duration>`.
#[must_use]
pub fn normalize_runtime_stderr(stderr: &str, file: &Path, php_bin: Option<&Path>) -> String {
    let mut normalized = stderr.replace("\r\n", "\n").replace('\r', "\n");
    normalized = replace_path(&normalized, file, "{file}");
    if let Some(php_bin) = php_bin {
        normalized = replace_path(&normalized, php_bin, "{php}");
    }
    normalized = normalize_line_numbers(&normalized);
    normalize_decimal_durations(&normalized)
}

fn replace_path(input: &str, path: &Path, replacement: &str) -> String {
    let raw = path.to_string_lossy();
    let mut normalized = input.replace(raw.as_ref(), replacement);
    if let Ok(canonical) = path.canonicalize() {
        normalized = normalized.replace(canonical.to_string_lossy().as_ref(), replacement);
    }
    normalized
}

fn normalize_line_numbers(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut remaining = input;
    while let Some(index) = remaining.find(" on line ") {
        let (before, after_prefix) = remaining.split_at(index + " on line ".len());
        out.push_str(before);
        let digits = after_prefix.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            remaining = after_prefix;
        } else {
            out.push_str("<line>");
            remaining = &after_prefix[digits..];
        }
    }
    out.push_str(remaining);
    out
}

fn normalize_decimal_durations(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let digit_count = bytes[index..]
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digit_count > 0
            && bytes.get(index + digit_count) == Some(&b'.')
            && bytes
                .get(index + digit_count + 1)
                .is_some_and(u8::is_ascii_digit)
        {
            let frac_count = bytes[index + digit_count + 1..]
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            out.push_str("<duration>");
            index += digit_count + 1 + frac_count;
        } else {
            out.push(bytes[index] as char);
            index += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::normalize_runtime_stderr;
    use std::path::Path;

    #[test]
    fn runtime_normalizes_paths_lines_and_durations() {
        let stderr =
            "/tmp/php fixtures/runtime/invalid/runtime-error.php on line 12\r\nelapsed 0.1234";
        let normalized = normalize_runtime_stderr(
            stderr,
            Path::new("fixtures/runtime/invalid/runtime-error.php"),
            Some(Path::new("/tmp/php")),
        );
        assert_eq!(
            normalized,
            "{php} {file} on line <line>\nelapsed <duration>"
        );
    }
}
