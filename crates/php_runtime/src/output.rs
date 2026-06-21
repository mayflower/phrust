//! Byte-oriented output buffering.

use crate::string::PhpString;

/// Runtime output buffer.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OutputBuffer {
    bytes: Vec<u8>,
}

impl OutputBuffer {
    /// Creates an empty output buffer.
    #[must_use]
    pub const fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Appends exact bytes.
    pub fn write_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        self.bytes.extend_from_slice(bytes.as_ref());
    }

    /// Appends a PHP string's exact bytes.
    pub fn write_php_string(&mut self, value: &PhpString) {
        self.write_bytes(value.as_bytes());
    }

    /// Convenience for tests and ASCII literals.
    pub fn write_test_str(&mut self, text: &str) {
        self.write_bytes(text.as_bytes());
    }

    /// Returns the exact output bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the exact buffered byte count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns true when no output bytes have been buffered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Consumes the buffer and returns exact output bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Test/debug convenience for textual assertions.
    #[must_use]
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }

    /// Clears buffered output.
    pub fn clear(&mut self) {
        self.bytes.clear();
    }
}
