//! Read-only PHAR archive support for local stream/include MVPs.

use crate::FilesystemCapabilities;
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

const HALT_COMPILER: &[u8] = b"__HALT_COMPILER();";
const MANIFEST_FIXED_LEN: usize = 18;
const FILE_COMPRESSION_MASK: u32 = 0x0000_F000;

/// Error returned by the read-only PHAR parser.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PharError {
    diagnostic_id: &'static str,
    message: String,
}

impl PharError {
    fn new(diagnostic_id: &'static str, message: impl Into<String>) -> Self {
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

    /// Human-readable deterministic message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PharError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.diagnostic_id, self.message)
    }
}

impl std::error::Error for PharError {}

/// One uncompressed file entry inside a PHAR archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PharEntry {
    /// Manifest filename, using forward slashes.
    pub name: String,
    /// Raw file bytes.
    pub contents: Vec<u8>,
    /// Entry flags from the manifest.
    pub flags: u32,
}

/// Parsed read-only PHAR archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PharArchive {
    /// Local archive path.
    pub path: PathBuf,
    /// Stub bytes before the manifest.
    pub stub: Vec<u8>,
    /// Alias from the manifest, when present.
    pub alias: Option<String>,
    entries: BTreeMap<String, PharEntry>,
}

impl PharArchive {
    /// Opens and parses a local `.phar` archive.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, PharError> {
        let path = path.into();
        let bytes = fs::read(&path).map_err(|error| {
            PharError::new(
                "E_PHP_RUNTIME_PHAR_OPEN",
                format!("{}: {error}", path.display()),
            )
        })?;
        Self::parse(path, &bytes)
    }

    /// Parses PHAR bytes from a local archive path.
    pub fn parse(path: PathBuf, bytes: &[u8]) -> Result<Self, PharError> {
        let halt_offset = find_halt_offset(bytes).ok_or_else(|| {
            PharError::new(
                "E_PHP_RUNTIME_PHAR_FORMAT",
                format!(
                    "internal corruption of phar `{}` (__HALT_COMPILER(); not found)",
                    path.display()
                ),
            )
        })?;
        let manifest_offset = manifest_offset_after_stub(bytes, halt_offset)?;
        let mut cursor = manifest_offset;
        let manifest_len = read_u32(bytes, &mut cursor, "manifest length")? as usize;
        let manifest_end = cursor.checked_add(manifest_len).ok_or_else(|| {
            PharError::new("E_PHP_RUNTIME_PHAR_FORMAT", "PHAR manifest length overflow")
        })?;
        if manifest_len < MANIFEST_FIXED_LEN || manifest_end > bytes.len() {
            return Err(PharError::new(
                "E_PHP_RUNTIME_PHAR_FORMAT",
                "internal corruption of phar (truncated manifest header)",
            ));
        }

        let mut manifest_cursor = cursor;
        let manifest_count = read_u32(bytes, &mut manifest_cursor, "manifest entry count")?;
        if manifest_count == 0 {
            return Err(PharError::new(
                "E_PHP_RUNTIME_PHAR_FORMAT",
                "manifest claims to have zero entries",
            ));
        }
        let _api_version = read_u16_be(bytes, &mut manifest_cursor, "manifest API version")?;
        let _global_flags = read_u32(bytes, &mut manifest_cursor, "global flags")?;
        let alias_len = read_u32(bytes, &mut manifest_cursor, "alias length")? as usize;
        let alias =
            read_bytes(bytes, &mut manifest_cursor, alias_len, "alias").and_then(|value| {
                String::from_utf8(value.to_vec()).map_err(|_| {
                    PharError::new("E_PHP_RUNTIME_PHAR_FORMAT", "PHAR alias is not UTF-8")
                })
            })?;
        let metadata_len = read_u32(bytes, &mut manifest_cursor, "metadata length")? as usize;
        let _metadata = read_bytes(bytes, &mut manifest_cursor, metadata_len, "metadata")?;

        let mut contents_offset = manifest_end;
        let mut pending = Vec::new();
        for _ in 0..manifest_count {
            let name_len = read_u32(bytes, &mut manifest_cursor, "filename length")? as usize;
            if name_len == 0 {
                return Err(PharError::new(
                    "E_PHP_RUNTIME_PHAR_FORMAT",
                    "zero-length filename encountered in phar",
                ));
            }
            let name_bytes = read_bytes(bytes, &mut manifest_cursor, name_len, "filename")?;
            let name = String::from_utf8(name_bytes.to_vec()).map_err(|_| {
                PharError::new("E_PHP_RUNTIME_PHAR_FORMAT", "PHAR filename is not UTF-8")
            })?;
            let uncompressed_size =
                read_u32(bytes, &mut manifest_cursor, "uncompressed size")? as usize;
            let _timestamp = read_u32(bytes, &mut manifest_cursor, "timestamp")?;
            let compressed_size =
                read_u32(bytes, &mut manifest_cursor, "compressed size")? as usize;
            let _crc32 = read_u32(bytes, &mut manifest_cursor, "crc32")?;
            let flags = read_u32(bytes, &mut manifest_cursor, "entry flags")?;
            let entry_metadata_len =
                read_u32(bytes, &mut manifest_cursor, "entry metadata length")? as usize;
            let _entry_metadata = read_bytes(
                bytes,
                &mut manifest_cursor,
                entry_metadata_len,
                "entry metadata",
            )?;
            if flags & FILE_COMPRESSION_MASK != 0 {
                return Err(PharError::new(
                    "E_PHP_RUNTIME_PHAR_COMPRESSION_GAP",
                    format!("compressed PHAR entry `{name}` is not supported"),
                ));
            }
            if compressed_size != uncompressed_size {
                return Err(PharError::new(
                    "E_PHP_RUNTIME_PHAR_FORMAT",
                    format!("PHAR entry `{name}` has inconsistent uncompressed size"),
                ));
            }
            pending.push((name, uncompressed_size, flags));
        }
        if manifest_cursor > manifest_end {
            return Err(PharError::new(
                "E_PHP_RUNTIME_PHAR_FORMAT",
                "PHAR manifest entries overrun manifest length",
            ));
        }

        let mut entries = BTreeMap::new();
        for (name, size, flags) in pending {
            let end = contents_offset.checked_add(size).ok_or_else(|| {
                PharError::new("E_PHP_RUNTIME_PHAR_FORMAT", "PHAR entry size overflow")
            })?;
            if end > bytes.len() {
                return Err(PharError::new(
                    "E_PHP_RUNTIME_PHAR_FORMAT",
                    format!("PHAR entry `{name}` is truncated"),
                ));
            }
            entries.insert(
                normalize_entry_name(&name),
                PharEntry {
                    name,
                    contents: bytes[contents_offset..end].to_vec(),
                    flags,
                },
            );
            contents_offset = end;
        }

        Ok(Self {
            path,
            stub: bytes[..manifest_offset].to_vec(),
            alias: (!alias.is_empty()).then_some(alias),
            entries,
        })
    }

    /// Returns an entry by name, accepting leading slash and `./` variants.
    #[must_use]
    pub fn entry(&self, name: &str) -> Option<&PharEntry> {
        self.entries.get(&normalize_entry_name(name))
    }

    /// Returns the number of manifest entries in the archive.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the archive contains no manifest entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Parsed local `phar://` URI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PharUri {
    /// Local archive path.
    pub archive_path: PathBuf,
    /// Entry path inside the archive.
    pub entry_path: String,
    /// Canonical synthetic path for source maps and include_once tracking.
    pub synthetic_path: PathBuf,
}

/// Returns whether `uri` uses the `phar://` stream wrapper.
#[must_use]
pub fn is_phar_uri(uri: &str) -> bool {
    uri.starts_with("phar://")
}

/// Parses a local `phar://archive.phar/path` URI under runtime capabilities.
pub fn parse_uri(
    uri: &str,
    cwd: &Path,
    capabilities: &FilesystemCapabilities,
) -> Result<PharUri, PharError> {
    let Some(rest) = uri.strip_prefix("phar://") else {
        return Err(PharError::new(
            "E_PHP_RUNTIME_PHAR_URI",
            format!("not a phar:// URI: `{uri}`"),
        ));
    };
    let (archive_part, entry_part) = split_archive_and_entry(rest)?;
    let archive_path = normalize_path(&if Path::new(archive_part).is_absolute() {
        PathBuf::from(archive_part)
    } else {
        cwd.join(archive_part)
    });
    if !capabilities.allows_path(&archive_path) {
        return Err(PharError::new(
            "E_PHP_RUNTIME_PHAR_CAPABILITY",
            format!(
                "PHAR archive `{}` is outside allowed filesystem roots",
                archive_path.display()
            ),
        ));
    }
    Ok(PharUri {
        archive_path,
        entry_path: normalize_entry_name(entry_part),
        synthetic_path: PathBuf::from(format!("phar://{}", rest)),
    })
}

/// Reads one file entry from a `phar://` URI.
pub fn read_uri(
    uri: &str,
    cwd: &Path,
    capabilities: &FilesystemCapabilities,
) -> Result<Vec<u8>, PharError> {
    let parsed = parse_uri(uri, cwd, capabilities)?;
    read_entry(&parsed.archive_path, &parsed.entry_path)
}

/// Reads one file entry from a parsed local archive path.
pub fn read_entry(archive_path: &Path, entry_path: &str) -> Result<Vec<u8>, PharError> {
    let archive = PharArchive::open(archive_path)?;
    archive
        .entry(entry_path)
        .map(|entry| entry.contents.clone())
        .ok_or_else(|| {
            PharError::new(
                "E_PHP_RUNTIME_PHAR_ENTRY_MISSING",
                format!(
                    "PHAR entry `{}` not found in `{}`",
                    entry_path,
                    archive_path.display()
                ),
            )
        })
}

fn split_archive_and_entry(rest: &str) -> Result<(&str, &str), PharError> {
    let marker = ".phar";
    let Some(marker_index) = rest.find(marker) else {
        return Err(PharError::new(
            "E_PHP_RUNTIME_PHAR_URI",
            format!("PHAR URI `{rest}` does not contain a .phar archive path"),
        ));
    };
    let archive_end = marker_index + marker.len();
    if rest.len() == archive_end {
        return Err(PharError::new(
            "E_PHP_RUNTIME_PHAR_URI",
            "PHAR URI must name an entry inside the archive",
        ));
    }
    if rest.as_bytes().get(archive_end) != Some(&b'/') {
        return Err(PharError::new(
            "E_PHP_RUNTIME_PHAR_URI",
            "PHAR URI archive path must be followed by /entry",
        ));
    }
    Ok((&rest[..archive_end], &rest[archive_end + 1..]))
}

fn find_halt_offset(bytes: &[u8]) -> Option<usize> {
    php_source::byte_kernel::find_bytes(bytes, HALT_COMPILER)
        .map(|index| index + HALT_COMPILER.len())
}

fn manifest_offset_after_stub(bytes: &[u8], halt_offset: usize) -> Result<usize, PharError> {
    let mut offset = halt_offset;
    if bytes
        .get(offset)
        .is_some_and(|byte| *byte == b' ' || *byte == b'\n')
        && bytes.get(offset + 1) == Some(&b'?')
        && bytes.get(offset + 2) == Some(&b'>')
    {
        offset += 3;
        if bytes.get(offset) == Some(&b'\r') {
            if bytes.get(offset + 1) != Some(&b'\n') {
                return Err(PharError::new(
                    "E_PHP_RUNTIME_PHAR_FORMAT",
                    "PHAR stub has carriage return not followed by newline",
                ));
            }
            offset += 1;
        }
        if bytes.get(offset) == Some(&b'\n') {
            offset += 1;
        }
    }
    Ok(offset)
}

fn read_u32(bytes: &[u8], cursor: &mut usize, field: &str) -> Result<u32, PharError> {
    let value = read_bytes(bytes, cursor, 4, field)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_u16_be(bytes: &[u8], cursor: &mut usize, field: &str) -> Result<u16, PharError> {
    let value = read_bytes(bytes, cursor, 2, field)?;
    Ok(u16::from_be_bytes([value[0], value[1]]))
}

fn read_bytes<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    len: usize,
    field: &str,
) -> Result<&'a [u8], PharError> {
    let end = cursor.checked_add(len).ok_or_else(|| {
        PharError::new(
            "E_PHP_RUNTIME_PHAR_FORMAT",
            format!("PHAR {field} length overflow"),
        )
    })?;
    let Some(value) = bytes.get(*cursor..end) else {
        return Err(PharError::new(
            "E_PHP_RUNTIME_PHAR_FORMAT",
            format!("PHAR {field} is truncated"),
        ));
    };
    *cursor = end;
    Ok(value)
}

fn normalize_entry_name(name: &str) -> String {
    let mut parts = Vec::new();
    for part in name.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.join("/")
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_uncompressed_phar_entry() {
        let bytes = fixture_phar();
        let archive = PharArchive::parse(PathBuf::from("fixture.phar"), &bytes).expect("parse");

        assert_eq!(archive.alias.as_deref(), Some("fixture.phar"));
        assert_eq!(
            archive
                .entry("lib/hello.php")
                .map(|entry| entry.contents.as_slice()),
            Some(b"<?php echo 'hello';\n".as_slice())
        );
        assert_eq!(
            archive
                .entry("./data.txt")
                .map(|entry| entry.contents.as_slice()),
            Some(b"payload".as_slice())
        );
    }

    fn fixture_phar() -> Vec<u8> {
        let entries = [
            ("lib/hello.php", b"<?php echo 'hello';\n".as_slice()),
            ("data.txt", b"payload".as_slice()),
        ];
        let mut bytes = b"<?php __HALT_COMPILER(); ?>\n".to_vec();
        let mut manifest = Vec::new();
        push_u32(&mut manifest, entries.len() as u32);
        manifest.extend_from_slice(&[0x11, 0x01]);
        push_u32(&mut manifest, 0);
        push_u32(&mut manifest, "fixture.phar".len() as u32);
        manifest.extend_from_slice(b"fixture.phar");
        push_u32(&mut manifest, 0);
        for (name, contents) in entries {
            push_u32(&mut manifest, name.len() as u32);
            manifest.extend_from_slice(name.as_bytes());
            push_u32(&mut manifest, contents.len() as u32);
            push_u32(&mut manifest, 1_704_067_200);
            push_u32(&mut manifest, contents.len() as u32);
            push_u32(&mut manifest, 0);
            push_u32(&mut manifest, 0);
            push_u32(&mut manifest, 0);
        }
        push_u32(&mut bytes, manifest.len() as u32);
        bytes.extend_from_slice(&manifest);
        for (_, contents) in entries {
            bytes.extend_from_slice(contents);
        }
        bytes
    }

    fn push_u32(buffer: &mut Vec<u8>, value: u32) {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
}
