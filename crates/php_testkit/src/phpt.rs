//! Minimal PHPT metadata parser for selected Phase 4 smoke tests.
//!
//! This is intentionally not a replacement for PHP's `run-tests.php`. It
//! supports only the small section and `EXPECTF` subset needed by
//! `fixtures/phpt_smoke`.

use std::collections::BTreeMap;

const SUPPORTED_SECTIONS: &[&str] = &["TEST", "FILE", "EXPECT", "EXPECTF", "SKIPIF", "INI"];

/// Parsed PHPT sections keyed by section name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhptFile {
    sections: BTreeMap<String, String>,
}

/// Expected output form for a supported PHPT file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhptExpectation<'a> {
    Exact(&'a str),
    Format(&'a str),
}

/// Runner disposition derived from PHPT sections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhptDisposition {
    Run,
    Skip(String),
    KnownGap(String),
}

impl PhptFile {
    /// Parses a small PHPT document into named sections.
    #[must_use]
    pub fn parse(input: &str) -> Self {
        let mut sections = BTreeMap::new();
        let mut current_name: Option<String> = None;
        let mut current_body = String::new();

        for line in input.lines() {
            if line.starts_with("--") && line.ends_with("--") && line.len() > 4 {
                if let Some(name) = current_name.replace(line.trim_matches('-').to_owned()) {
                    sections.insert(name, current_body.trim_end_matches('\n').to_owned());
                    current_body.clear();
                }
            } else if current_name.is_some() {
                current_body.push_str(line);
                current_body.push('\n');
            }
        }

        if let Some(name) = current_name {
            sections.insert(name, current_body.trim_end_matches('\n').to_owned());
        }

        Self { sections }
    }

    /// Returns a PHPT section by name.
    #[must_use]
    pub fn section(&self, name: &str) -> Option<&str> {
        self.sections.get(name).map(String::as_str)
    }

    /// Returns all parsed section names in stable order.
    pub fn section_names(&self) -> impl Iterator<Item = &str> {
        self.sections.keys().map(String::as_str)
    }

    /// Returns the `--FILE--` body, if present.
    #[must_use]
    pub fn file_body(&self) -> Option<&str> {
        self.section("FILE")
    }

    /// Returns the supported expectation section.
    #[must_use]
    pub fn expectation(&self) -> Option<PhptExpectation<'_>> {
        match (self.section("EXPECT"), self.section("EXPECTF")) {
            (Some(_), Some(_)) => None,
            (Some(expect), None) => Some(PhptExpectation::Exact(expect)),
            (None, Some(expectf)) => Some(PhptExpectation::Format(expectf)),
            (None, None) => None,
        }
    }

    /// Classifies whether this PHPT can be run by the minimal smoke runner.
    #[must_use]
    pub fn disposition(&self) -> PhptDisposition {
        let unsupported = self.unsupported_sections();
        if !unsupported.is_empty() {
            return PhptDisposition::Skip(format!(
                "unsupported PHPT section(s): {}",
                unsupported.join(", ")
            ));
        }
        if self.section("INI").is_some() {
            return PhptDisposition::KnownGap("PHPT --INI-- is not modeled by php-vm".to_string());
        }
        if self.section("SKIPIF").is_some() {
            return PhptDisposition::Skip("PHPT --SKIPIF-- requested skip".to_string());
        }
        PhptDisposition::Run
    }

    /// Returns unsupported section names in stable order.
    #[must_use]
    pub fn unsupported_sections(&self) -> Vec<String> {
        self.section_names()
            .filter(|name| !SUPPORTED_SECTIONS.contains(name))
            .map(ToOwned::to_owned)
            .collect()
    }
}

/// Matches a tiny tested subset of PHPT `EXPECTF` placeholders.
///
/// Supported placeholders:
///
/// - `%%`: literal percent sign
/// - `%s`: any byte sequence, including newlines
/// - `%S`: any non-newline byte sequence
/// - `%d`: one or more decimal digits
/// - `%i`: optional sign followed by one or more decimal digits
/// - `%w`: zero or more ASCII whitespace bytes
#[must_use]
pub fn expectf_matches(pattern: &str, actual: &str) -> bool {
    let pattern = pattern.as_bytes();
    let actual = actual.as_bytes();
    match_expectf(pattern, 0, actual, 0)
}

fn match_expectf(pattern: &[u8], pattern_index: usize, actual: &[u8], actual_index: usize) -> bool {
    if pattern_index == pattern.len() {
        return actual_index == actual.len();
    }

    if pattern[pattern_index] != b'%' {
        return actual.get(actual_index) == Some(&pattern[pattern_index])
            && match_expectf(pattern, pattern_index + 1, actual, actual_index + 1);
    }

    let Some(kind) = pattern.get(pattern_index + 1).copied() else {
        return actual.get(actual_index) == Some(&b'%')
            && match_expectf(pattern, pattern_index + 1, actual, actual_index + 1);
    };

    match kind {
        b'%' => {
            actual.get(actual_index) == Some(&b'%')
                && match_expectf(pattern, pattern_index + 2, actual, actual_index + 1)
        }
        b's' => {
            for index in actual_index..=actual.len() {
                if match_expectf(pattern, pattern_index + 2, actual, index) {
                    return true;
                }
            }
            false
        }
        b'S' => {
            for index in actual_index..=actual.len() {
                if actual[actual_index..index].contains(&b'\n') {
                    break;
                }
                if match_expectf(pattern, pattern_index + 2, actual, index) {
                    return true;
                }
            }
            false
        }
        b'd' => {
            let mut index = actual_index;
            while actual.get(index).is_some_and(u8::is_ascii_digit) {
                index += 1;
            }
            index > actual_index && match_expectf(pattern, pattern_index + 2, actual, index)
        }
        b'i' => {
            let mut index = actual_index;
            if matches!(actual.get(index), Some(b'-' | b'+')) {
                index += 1;
            }
            let digits_start = index;
            while actual.get(index).is_some_and(u8::is_ascii_digit) {
                index += 1;
            }
            index > digits_start && match_expectf(pattern, pattern_index + 2, actual, index)
        }
        b'w' => {
            let mut index = actual_index;
            while actual.get(index).is_some_and(u8::is_ascii_whitespace) {
                index += 1;
            }
            match_expectf(pattern, pattern_index + 2, actual, index)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{PhptDisposition, PhptExpectation, PhptFile, expectf_matches};

    #[test]
    fn runtime_phpt_parser_reads_sections() {
        let phpt = PhptFile::parse("--TEST--\nhello\n--FILE--\n<?php echo 1;\n--EXPECT--\n1\n");
        assert_eq!(phpt.section("TEST"), Some("hello"));
        assert_eq!(phpt.section("EXPECT"), Some("1"));
        assert_eq!(phpt.file_body(), Some("<?php echo 1;"));
        assert_eq!(phpt.expectation(), Some(PhptExpectation::Exact("1")));
        assert_eq!(phpt.disposition(), PhptDisposition::Run);
    }

    #[test]
    fn runtime_phpt_parser_classifies_skip_known_gap_and_unsupported() {
        let skip = PhptFile::parse(
            "--TEST--\nskip\n--SKIPIF--\n<?php die('skip');\n--FILE--\n<?php\n--EXPECT--\n",
        );
        assert!(matches!(skip.disposition(), PhptDisposition::Skip(_)));

        let ini =
            PhptFile::parse("--TEST--\nini\n--INI--\nprecision=14\n--FILE--\n<?php\n--EXPECT--\n");
        assert!(matches!(ini.disposition(), PhptDisposition::KnownGap(_)));

        let unsupported =
            PhptFile::parse("--TEST--\nargs\n--ARGS--\n--x\n--FILE--\n<?php\n--EXPECT--\n");
        assert_eq!(unsupported.unsupported_sections(), vec!["ARGS"]);
        assert!(matches!(
            unsupported.disposition(),
            PhptDisposition::Skip(_)
        ));
    }

    #[test]
    fn runtime_phpt_expectf_subset_matches_only_documented_patterns() {
        assert!(expectf_matches("hello %S %d", "hello php 85"));
        assert!(expectf_matches("value=%i%%", "value=-7%"));
        assert!(expectf_matches("a%sb", "a\nmiddle\nb"));
        assert!(expectf_matches("a%wb", "a \n\tb"));
        assert!(!expectf_matches("hello %d", "hello php"));
        assert!(!expectf_matches("line %S", "line a\nb"));
        assert!(!expectf_matches("unsupported %x", "unsupported abc"));
    }
}
