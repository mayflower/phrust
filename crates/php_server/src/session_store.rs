use php_runtime::{PhpArray, PhpString, UnserializeOptions, Value, serialize, unserialize};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug)]
pub struct SessionStore {
    root: PathBuf,
    temp_counter: AtomicU64,
}

#[derive(Debug)]
pub enum SessionStoreError {
    Io(io::Error),
    InvalidId,
    Decode(String),
    Encode(String),
}

impl std::fmt::Display for SessionStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::InvalidId => f.write_str("invalid session id"),
            Self::Decode(message) => write!(f, "session decode failed: {message}"),
            Self::Encode(message) => write!(f, "session encode failed: {message}"),
        }
    }
}

impl std::error::Error for SessionStoreError {}

impl From<io::Error> for SessionStoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl SessionStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            temp_counter: AtomicU64::new(0),
        }
    }

    pub fn ensure_ready(&self) -> Result<(), SessionStoreError> {
        fs::create_dir_all(&self.root)?;
        Ok(())
    }

    pub fn load(&self, id: &str) -> Result<PhpArray, SessionStoreError> {
        let path = self.path_for_id(id)?;
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(PhpArray::new()),
            Err(error) => return Err(SessionStoreError::Io(error)),
        };
        let value = unserialize(
            &PhpString::from_bytes(bytes),
            UnserializeOptions {
                max_bytes: 1_048_576,
                ..UnserializeOptions::default()
            },
        )
        .map_err(|error| SessionStoreError::Decode(error.message().to_string()))?;
        match value {
            Value::Array(array) => Ok(array),
            _ => Err(SessionStoreError::Decode(
                "session payload is not an array".to_string(),
            )),
        }
    }

    pub fn save(&self, id: &str, data: &PhpArray) -> Result<(), SessionStoreError> {
        self.ensure_ready()?;
        let path = self.path_for_id(id)?;
        let bytes = serialize(&Value::Array(data.clone()))
            .map_err(|error| SessionStoreError::Encode(error.message().to_string()))?
            .into_bytes();
        let temp = self.temp_path_for(id)?;
        fs::write(&temp, bytes)?;
        fs::rename(&temp, path)?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), SessionStoreError> {
        let path = self.path_for_id(id)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(SessionStoreError::Io(error)),
        }
    }

    fn path_for_id(&self, id: &str) -> Result<PathBuf, SessionStoreError> {
        if !valid_session_id(id) {
            return Err(SessionStoreError::InvalidId);
        }
        Ok(self.root.join(format!("sess_{id}")))
    }

    fn temp_path_for(&self, id: &str) -> Result<PathBuf, SessionStoreError> {
        if !valid_session_id(id) {
            return Err(SessionStoreError::InvalidId);
        }
        let counter = self.temp_counter.fetch_add(1, Ordering::Relaxed);
        Ok(self
            .root
            .join(format!(".sess_{id}.{}.{}.tmp", std::process::id(), counter)))
    }
}

pub fn valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b','))
}

pub fn generate_session_id() -> io::Result<String> {
    let mut bytes = [0u8; 24];
    fs::File::open(Path::new("/dev/urandom")).and_then(|mut file| file.read_exact(&mut bytes))?;
    Ok(hex_bytes(&bytes))
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = Vec::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize]);
        output.push(HEX[(byte & 0x0f) as usize]);
    }
    String::from_utf8(output).expect("hex is utf-8")
}

#[cfg(test)]
mod tests {
    use super::{SessionStore, generate_session_id, valid_session_id};
    use php_runtime::{ArrayKey, PhpArray, PhpString, Value};

    #[test]
    fn session_ids_are_strict_path_segments() {
        assert!(valid_session_id("abcDEF0123-,"));
        assert!(!valid_session_id(""));
        assert!(!valid_session_id("../bad"));
        assert!(!valid_session_id("bad/slash"));
        assert!(!valid_session_id("bad\nid"));
    }

    #[test]
    fn session_store_roundtrips_php_array_payloads() {
        let root =
            std::env::temp_dir().join(format!("phrust-session-store-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let store = SessionStore::new(&root);
        let mut data = PhpArray::new();
        data.insert(
            ArrayKey::String(PhpString::from_test_str("n")),
            Value::Int(2),
        );

        store.save("abc123", &data).expect("save session");
        assert_eq!(store.load("abc123").expect("load session"), data);
        store.delete("abc123").expect("delete session");
        assert!(store.load("abc123").expect("load missing").is_empty());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn generated_session_ids_are_valid() {
        let id = generate_session_id().expect("generate session id");
        assert!(valid_session_id(&id), "{id}");
    }
}
