//! Byte-oriented PHP string representation.

use std::fmt;

/// PHP string bytes without an implicit UTF-8 invariant.
#[derive(Clone, Default, Eq, Hash, PartialEq)]
pub struct PhpString {
    bytes: Vec<u8>,
}

impl PhpString {
    /// Creates a PHP string from raw bytes.
    #[must_use]
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            bytes: bytes.into(),
        }
    }

    /// Convenience constructor for tests and ASCII literals.
    #[must_use]
    pub fn from_test_str(text: &str) -> Self {
        Self::from_bytes(text.as_bytes().to_vec())
    }

    /// Returns the exact underlying bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the string and returns the exact bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns true when the string has no bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Returns the byte length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Test/debug convenience for non-runtime display.
    #[must_use]
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

impl From<Vec<u8>> for PhpString {
    fn from(bytes: Vec<u8>) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<&[u8]> for PhpString {
    fn from(bytes: &[u8]) -> Self {
        Self::from_bytes(bytes.to_vec())
    }
}

impl From<&str> for PhpString {
    fn from(text: &str) -> Self {
        Self::from_test_str(text)
    }
}

impl fmt::Debug for PhpString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PhpString")
            .field("bytes", &self.bytes)
            .field("lossy", &self.to_string_lossy())
            .finish()
    }
}

impl fmt::Display for PhpString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}
