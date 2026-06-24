use regex::Regex;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpectationKind {
    Expect,
    ExpectF,
    ExpectRegex,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchOutcome {
    pub matched: bool,
    pub diff: Option<ExpectationDiff>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectationDiff {
    pub kind: ExpectationKind,
    pub message: String,
    pub first_mismatch: Option<usize>,
    pub expected_excerpt: String,
    pub actual_excerpt: String,
}

pub fn match_expectation(kind: ExpectationKind, expected: &str, actual: &str) -> MatchOutcome {
    let matched = match kind {
        ExpectationKind::Expect => expected == actual,
        ExpectationKind::ExpectF => match expectf_to_regex(expected) {
            Ok(regex) => regex.is_match(actual),
            Err(error) => {
                return MatchOutcome {
                    matched: false,
                    diff: Some(ExpectationDiff {
                        kind,
                        message: error,
                        first_mismatch: None,
                        expected_excerpt: excerpt(expected, 0),
                        actual_excerpt: excerpt(actual, 0),
                    }),
                };
            }
        },
        ExpectationKind::ExpectRegex => match anchored_regex(expected) {
            Ok(regex) => regex.is_match(actual),
            Err(error) => {
                return MatchOutcome {
                    matched: false,
                    diff: Some(ExpectationDiff {
                        kind,
                        message: format!("invalid EXPECTREGEX pattern: {error}"),
                        first_mismatch: None,
                        expected_excerpt: excerpt(expected, 0),
                        actual_excerpt: excerpt(actual, 0),
                    }),
                };
            }
        },
    };
    if matched {
        MatchOutcome {
            matched: true,
            diff: None,
        }
    } else {
        let mismatch = first_mismatch(expected, actual);
        MatchOutcome {
            matched: false,
            diff: Some(ExpectationDiff {
                kind,
                message: "output did not match expectation".to_string(),
                first_mismatch: mismatch,
                expected_excerpt: excerpt(expected, mismatch.unwrap_or(0)),
                actual_excerpt: excerpt(actual, mismatch.unwrap_or(0)),
            }),
        }
    }
}

pub fn expectf_to_regex(pattern: &str) -> Result<Regex, String> {
    let mut out = String::from("(?s)\\A");
    let mut index = 0usize;
    while index < pattern.len() {
        let rest = &pattern[index..];
        if let Some(regex) = expectf_placeholder(rest) {
            out.push_str(regex.pattern);
            index += regex.width;
        } else {
            let ch = rest
                .chars()
                .next()
                .ok_or_else(|| "invalid UTF-8 boundary in EXPECTF".to_string())?;
            out.push_str(&regex::escape(&ch.to_string()));
            index += ch.len_utf8();
        }
    }
    out.push_str("\\z");
    Regex::new(&out).map_err(|error| format!("invalid EXPECTF pattern: {error}"))
}

fn anchored_regex(pattern: &str) -> Result<Regex, regex::Error> {
    Regex::new(&format!("(?s)\\A(?:{pattern})\\z"))
}

struct Placeholder {
    pattern: &'static str,
    width: usize,
}

fn expectf_placeholder(rest: &str) -> Option<Placeholder> {
    if rest.starts_with("%unicode|string%") {
        return Some(Placeholder {
            pattern: "(?:unicode|string)",
            width: "%unicode|string%".len(),
        });
    }
    let mut chars = rest.chars();
    if chars.next()? != '%' {
        return None;
    }
    let placeholder = chars.next()?;
    let pattern = match placeholder {
        '%' => "%",
        's' => "[^\\r\\n]+",
        'S' => "\\S+",
        'd' => "\\d+",
        'i' => "[+-]?\\d+",
        'f' => "[+-]?(?:\\d+\\.\\d*|\\d*\\.\\d+|\\d+)(?:[Ee][+-]?\\d+)?",
        'x' => "[0-9A-Fa-f]+",
        'w' => "\\s*",
        'a' => ".+",
        'A' => ".*",
        _ => return None,
    };
    Some(Placeholder { pattern, width: 2 })
}

fn first_mismatch(expected: &str, actual: &str) -> Option<usize> {
    let expected_bytes = expected.as_bytes();
    let actual_bytes = actual.as_bytes();
    let len = expected_bytes.len().min(actual_bytes.len());
    for index in 0..len {
        if expected_bytes[index] != actual_bytes[index] {
            return Some(index);
        }
    }
    if expected_bytes.len() == actual_bytes.len() {
        None
    } else {
        Some(len)
    }
}

fn excerpt(value: &str, index: usize) -> String {
    let start = index.saturating_sub(24);
    let end = (index + 80).min(value.len());
    let start = previous_char_boundary(value, start);
    let end = next_char_boundary(value, end);
    value[start..end].replace('\n', "\\n").replace('\r', "\\r")
}

fn previous_char_boundary(value: &str, mut index: usize) -> usize {
    while index > 0 && !value.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn next_char_boundary(value: &str, mut index: usize) -> usize {
    while index < value.len() && !value.is_char_boundary(index) {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_expect_matches_byte_for_byte() {
        assert!(match_expectation(ExpectationKind::Expect, "a\n", "a\n").matched);
        let outcome = match_expectation(ExpectationKind::Expect, "a\n", "a\r\n");
        assert!(!outcome.matched);
        assert_eq!(outcome.diff.unwrap().first_mismatch, Some(1));
    }

    #[test]
    fn expectf_supports_common_placeholders() {
        let expected = "int(%d) signed(%i) float(%f) hex(%x) ws%wtext %s %S %unicode|string%";
        let actual = "int(12) signed(-3) float(1.5e+2) hex(ff) ws \n\ttext hello WORD string";

        assert!(match_expectation(ExpectationKind::ExpectF, expected, actual).matched);
    }

    #[test]
    fn expectf_supports_any_placeholders() {
        assert!(match_expectation(ExpectationKind::ExpectF, "a%Aend", "a\nmiddle\nend").matched);
        assert!(match_expectation(ExpectationKind::ExpectF, "a%aend", "a\nx\nend").matched);
        assert!(!match_expectation(ExpectationKind::ExpectF, "a%aend", "aend").matched);
    }

    #[test]
    fn expectregex_is_anchored() {
        assert!(match_expectation(ExpectationKind::ExpectRegex, "a.+c", "abc").matched);
        assert!(!match_expectation(ExpectationKind::ExpectRegex, "a.+c", "zabc").matched);
    }

    #[test]
    fn invalid_regex_returns_structured_diff() {
        let outcome = match_expectation(ExpectationKind::ExpectRegex, "(", "");

        assert!(!outcome.matched);
        assert!(
            outcome
                .diff
                .unwrap()
                .message
                .contains("invalid EXPECTREGEX")
        );
    }
}
