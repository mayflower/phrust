use cap_primitives::fs::{FollowSymlinks, OpenOptionsExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use php_runtime::api::RuntimeCancellationState;
use std::{
    fs,
    io::{self, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::OnceLock,
    thread,
    time::{Duration, Instant, SystemTime},
};

#[derive(Debug)]
pub struct FileSessionStore {
    root_path: PathBuf,
    root: OnceLock<Dir>,
    lock_timeout: Duration,
}

pub type SessionStore = FileSessionStore;

#[derive(Debug)]
pub struct SessionFileLease {
    id: String,
    file: fs::File,
    payload: Vec<u8>,
    existed: bool,
    finalized: bool,
}

impl SessionFileLease {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    #[must_use]
    pub const fn existed(&self) -> bool {
        self.existed
    }

    #[must_use]
    pub const fn finalized(&self) -> bool {
        self.finalized
    }
}

impl Drop for SessionFileLease {
    fn drop(&mut self) {
        if !self.finalized {
            let _ = self.file.unlock();
        }
    }
}

#[derive(Debug)]
pub enum SessionStoreError {
    Io(io::Error),
    InvalidId,
    LockTimeout,
    Cancelled,
    Unavailable,
}

impl std::fmt::Display for SessionStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::InvalidId => formatter.write_str("invalid session id"),
            Self::LockTimeout => formatter.write_str("session lock timeout"),
            Self::Cancelled => formatter.write_str("session lock wait cancelled"),
            Self::Unavailable => formatter.write_str("session store unavailable"),
        }
    }
}

impl std::error::Error for SessionStoreError {}

impl From<io::Error> for SessionStoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl FileSessionStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root_path: root.into(),
            root: OnceLock::new(),
            lock_timeout: Duration::from_millis(5_000),
        }
    }

    #[must_use]
    pub fn with_lock_timeout(root: impl Into<PathBuf>, lock_timeout: Duration) -> Self {
        Self {
            lock_timeout,
            ..Self::new(root)
        }
    }

    pub fn ensure_ready(&self) -> Result<(), SessionStoreError> {
        if self.root.get().is_some() {
            return Ok(());
        }
        match fs::symlink_metadata(&self.root_path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(SessionStoreError::Io(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "session_save_path must be a real directory, not a symlink",
                    )));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.root_path)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&self.root_path, fs::Permissions::from_mode(0o700))?;
                }
            }
            Err(error) => return Err(error.into()),
        }
        let dir = Dir::open_ambient_dir(&self.root_path, ambient_authority())?;
        self.root
            .set(dir)
            .map_err(|_| SessionStoreError::Unavailable)
    }

    fn root(&self) -> Result<&Dir, SessionStoreError> {
        self.ensure_ready()?;
        self.root.get().ok_or(SessionStoreError::Unavailable)
    }

    pub fn load(&self, id: &str) -> Result<Vec<u8>, SessionStoreError> {
        Ok(self.acquire(id)?.payload().to_vec())
    }

    pub fn save(&self, id: &str, payload: &[u8]) -> Result<(), SessionStoreError> {
        let mut lease = self.acquire(id)?;
        self.commit(&mut lease, payload, false).map(|_| ())
    }

    pub fn delete(&self, id: &str) -> Result<(), SessionStoreError> {
        validate_id(id)?;
        let name = session_filename(id);
        match self.root()?.remove_file(name) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn acquire(&self, id: &str) -> Result<SessionFileLease, SessionStoreError> {
        self.acquire_impl(id, true, None)?
            .ok_or(SessionStoreError::Unavailable)
    }

    pub fn acquire_cancellable(
        &self,
        id: &str,
        cancellation: &RuntimeCancellationState,
    ) -> Result<SessionFileLease, SessionStoreError> {
        self.acquire_impl(id, true, Some(cancellation))?
            .ok_or(SessionStoreError::Unavailable)
    }

    pub fn acquire_existing(
        &self,
        id: &str,
    ) -> Result<Option<SessionFileLease>, SessionStoreError> {
        self.acquire_impl(id, false, None)
    }

    pub fn acquire_existing_cancellable(
        &self,
        id: &str,
        cancellation: &RuntimeCancellationState,
    ) -> Result<Option<SessionFileLease>, SessionStoreError> {
        self.acquire_impl(id, false, Some(cancellation))
    }

    fn acquire_impl(
        &self,
        id: &str,
        create: bool,
        cancellation: Option<&RuntimeCancellationState>,
    ) -> Result<Option<SessionFileLease>, SessionStoreError> {
        validate_id(id)?;
        let started = Instant::now();
        loop {
            let remaining = self
                .lock_timeout
                .checked_sub(started.elapsed())
                .ok_or(SessionStoreError::LockTimeout)?;
            let opened = if create {
                Some(self.open_or_create(id)?)
            } else {
                self.open_existing(id)?.map(|file| (file, true))
            };
            let Some((mut file, existed)) = opened else {
                return Ok(None);
            };
            lock_exclusive(&file, remaining, cancellation)?;
            if let Err(error) = verify_current_entry(self.root()?, id, &file) {
                let _ = file.unlock();
                if matches!(
                    &error,
                    SessionStoreError::Io(io_error)
                        if matches!(io_error.kind(), io::ErrorKind::NotFound)
                ) {
                    continue;
                }
                return Err(error);
            }
            file.seek(SeekFrom::Start(0))?;
            let mut payload = Vec::new();
            file.read_to_end(&mut payload)?;
            return Ok(Some(SessionFileLease {
                id: id.to_string(),
                file,
                payload,
                existed,
                finalized: false,
            }));
        }
    }

    pub fn commit(
        &self,
        lease: &mut SessionFileLease,
        payload: &[u8],
        lazy_write: bool,
    ) -> Result<bool, SessionStoreError> {
        if lease.finalized {
            return Err(SessionStoreError::Unavailable);
        }
        let wrote = !lazy_write || payload != lease.payload;
        if wrote {
            write_payload(&mut lease.file, payload)?;
            lease.payload.clear();
            lease.payload.extend_from_slice(payload);
        } else {
            lease.file.set_times(
                fs::FileTimes::new()
                    .set_accessed(SystemTime::now())
                    .set_modified(SystemTime::now()),
            )?;
        }
        lease.file.unlock()?;
        lease.finalized = true;
        Ok(wrote)
    }

    pub fn abort(&self, lease: &mut SessionFileLease) -> Result<(), SessionStoreError> {
        if lease.finalized {
            return Ok(());
        }
        lease.file.unlock()?;
        lease.finalized = true;
        Ok(())
    }

    pub fn destroy(&self, lease: &mut SessionFileLease) -> Result<(), SessionStoreError> {
        if lease.finalized {
            return Err(SessionStoreError::Unavailable);
        }
        let name = session_filename(&lease.id);
        match self.root()?.remove_file(name) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        lease.file.unlock()?;
        lease.finalized = true;
        Ok(())
    }

    pub fn regenerate(
        &self,
        old_lease: &mut SessionFileLease,
        new_id: &str,
        payload: &[u8],
        delete_old: bool,
    ) -> Result<SessionFileLease, SessionStoreError> {
        if old_lease.finalized {
            return Err(SessionStoreError::Unavailable);
        }
        let mut new_lease = self.acquire_new(new_id)?;
        if let Err(error) = write_payload(&mut new_lease.file, payload) {
            self.discard_regeneration_lease(&mut new_lease);
            return Err(error);
        }
        new_lease.payload.extend_from_slice(payload);
        if delete_old {
            match self.root()?.remove_file(session_filename(&old_lease.id)) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    self.discard_regeneration_lease(&mut new_lease);
                    return Err(error.into());
                }
            }
        }
        if old_lease.file.unlock().is_ok() {
            old_lease.finalized = true;
        }
        Ok(new_lease)
    }

    fn discard_regeneration_lease(&self, lease: &mut SessionFileLease) {
        let _ = lease.file.unlock();
        lease.finalized = true;
        let _ = self.root().and_then(|root| {
            root.remove_file(session_filename(&lease.id))
                .map_err(SessionStoreError::from)
        });
    }

    pub fn exists(&self, id: &str) -> Result<bool, SessionStoreError> {
        Ok(self.open_existing(id)?.is_some())
    }

    pub fn gc(&self, max_lifetime: Duration) -> Result<usize, SessionStoreError> {
        let now = SystemTime::now();
        let mut deleted = 0usize;
        for entry in self.root()?.entries()? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let Some(id) = name.strip_prefix("sess_") else {
                continue;
            };
            if !valid_session_id(id) {
                continue;
            }
            let metadata = self.root()?.symlink_metadata(name)?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                continue;
            }
            let Ok(age) = metadata.modified().and_then(|modified| {
                now.duration_since(modified.into_std())
                    .map_err(|error| io::Error::other(error.to_string()))
            }) else {
                continue;
            };
            if age <= max_lifetime {
                continue;
            }
            let Some(file) = self.open_existing(id)? else {
                continue;
            };
            match file.try_lock() {
                Ok(()) => {}
                Err(fs::TryLockError::WouldBlock) => continue,
                Err(fs::TryLockError::Error(error)) => return Err(error.into()),
            }
            let still_expired = file
                .metadata()
                .and_then(|metadata| metadata.modified())
                .and_then(|modified| {
                    now.duration_since(modified)
                        .map_err(|error| io::Error::other(error.to_string()))
                })
                .is_ok_and(|age| age > max_lifetime);
            if still_expired
                && verify_current_entry(self.root()?, id, &file).is_ok()
                && self.root()?.remove_file(name).is_ok()
            {
                deleted = deleted.saturating_add(1);
            }
            let _ = file.unlock();
        }
        Ok(deleted)
    }

    fn acquire_new(&self, id: &str) -> Result<SessionFileLease, SessionStoreError> {
        validate_id(id)?;
        let name = session_filename(id);
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        options._cap_fs_ext_follow(FollowSymlinks::No);
        #[cfg(unix)]
        options.mode(0o600);
        let file = self.root()?.open_with(name, &options)?.into_std();
        ensure_regular_file(&file)?;
        lock_exclusive(&file, self.lock_timeout, None)?;
        verify_current_entry(self.root()?, id, &file)?;
        Ok(SessionFileLease {
            id: id.to_string(),
            file,
            payload: Vec::new(),
            existed: false,
            finalized: false,
        })
    }

    fn open_existing(&self, id: &str) -> Result<Option<fs::File>, SessionStoreError> {
        validate_id(id)?;
        let name = session_filename(id);
        let mut options = OpenOptions::new();
        options.read(true).write(true);
        options._cap_fs_ext_follow(FollowSymlinks::No);
        match self.root()?.open_with(name, &options) {
            Ok(file) => {
                let file = file.into_std();
                ensure_regular_file(&file)?;
                Ok(Some(file))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn open_or_create(&self, id: &str) -> Result<(fs::File, bool), SessionStoreError> {
        if let Some(file) = self.open_existing(id)? {
            return Ok((file, true));
        }
        let name = session_filename(id);
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        options._cap_fs_ext_follow(FollowSymlinks::No);
        #[cfg(unix)]
        {
            options.mode(0o600);
        }
        match self.root()?.open_with(name, &options) {
            Ok(file) => {
                let file = file.into_std();
                ensure_regular_file(&file)?;
                Ok((file, false))
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => self
                .open_existing(id)?
                .map(|file| (file, true))
                .ok_or_else(|| {
                    SessionStoreError::Io(io::Error::new(
                        io::ErrorKind::NotFound,
                        "session file disappeared during creation",
                    ))
                }),
            Err(error) => Err(error.into()),
        }
    }
}

fn lock_exclusive(
    file: &fs::File,
    timeout: Duration,
    cancellation: Option<&RuntimeCancellationState>,
) -> Result<(), SessionStoreError> {
    let started = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(fs::TryLockError::WouldBlock) => {
                if cancellation.is_some_and(RuntimeCancellationState::is_cancelled) {
                    return Err(SessionStoreError::Cancelled);
                }
                if started.elapsed() >= timeout {
                    return Err(SessionStoreError::LockTimeout);
                }
                thread::sleep(Duration::from_millis(2));
            }
            Err(fs::TryLockError::Error(error)) => return Err(error.into()),
        }
    }
}

fn ensure_regular_file(file: &fs::File) -> Result<(), SessionStoreError> {
    if file.metadata()?.is_file() {
        Ok(())
    } else {
        Err(SessionStoreError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            "session entry is not a regular file",
        )))
    }
}

fn verify_current_entry(root: &Dir, id: &str, file: &fs::File) -> Result<(), SessionStoreError> {
    let entry = root.symlink_metadata(session_filename(id))?;
    if entry.file_type().is_symlink() || !entry.is_file() {
        return Err(SessionStoreError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            "session entry changed type while acquiring its lock",
        )));
    }
    #[cfg(unix)]
    {
        use cap_primitives::fs::MetadataExt as CapMetadataExt;
        use std::os::unix::fs::MetadataExt as StdMetadataExt;
        let opened = file.metadata()?;
        if CapMetadataExt::dev(&entry) != StdMetadataExt::dev(&opened)
            || CapMetadataExt::ino(&entry) != StdMetadataExt::ino(&opened)
        {
            return Err(SessionStoreError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "session entry was replaced while acquiring its lock",
            )));
        }
    }
    Ok(())
}

fn session_filename(id: &str) -> String {
    format!("sess_{id}")
}

fn validate_id(id: &str) -> Result<(), SessionStoreError> {
    valid_session_id(id)
        .then_some(())
        .ok_or(SessionStoreError::InvalidId)
}

fn write_payload(file: &mut fs::File, payload: &[u8]) -> Result<(), SessionStoreError> {
    file.seek(SeekFrom::Start(0))?;
    file.write_all(payload)?;
    file.set_len(payload.len() as u64)?;
    file.flush()?;
    Ok(())
}

pub fn valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b','))
}

pub fn generate_session_id() -> io::Result<String> {
    generate_session_id_with_policy(32, 4, "")
}

pub fn generate_session_id_with_policy(
    length: usize,
    bits_per_character: u8,
    prefix: &str,
) -> io::Result<String> {
    if !(22..=256).contains(&length)
        || !matches!(bits_per_character, 4..=6)
        || prefix.len().saturating_add(length) > 256
        || !prefix.bytes().all(is_session_id_byte)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid session id generation policy",
        ));
    }
    let alphabet = match bits_per_character {
        4 => b"0123456789abcdef".as_slice(),
        5 => b"0123456789abcdefghijklmnopqrstuv".as_slice(),
        6 => b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ,-".as_slice(),
        _ => unreachable!(),
    };
    let mut output = String::with_capacity(prefix.len() + length);
    output.push_str(prefix);
    let mut random = [0_u8; 64];
    while output.len() < prefix.len() + length {
        getrandom::fill(&mut random).map_err(io::Error::other)?;
        for byte in random {
            let alphabet_len = alphabet.len() as u8;
            let unbiased_ceiling = u8::MAX - (u8::MAX % alphabet_len);
            if byte >= unbiased_ceiling {
                continue;
            }
            output.push(char::from(alphabet[(byte % alphabet_len) as usize]));
            if output.len() == prefix.len() + length {
                break;
            }
        }
    }
    Ok(output)
}

fn is_session_id_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b',')
}

#[cfg(test)]
mod tests {
    use super::{
        SessionStore, SessionStoreError, generate_session_id, generate_session_id_with_policy,
        valid_session_id,
    };
    use php_runtime::api::{
        ArrayKey, PhpArray, PhpString, Value, decode_runtime_session_payload,
        encode_runtime_session_payload,
    };
    use std::time::{Duration, SystemTime};

    fn data(value: i64) -> PhpArray {
        let mut data = PhpArray::new();
        data.insert(
            ArrayKey::String(PhpString::from_test_str("n")),
            Value::Int(value),
        );
        data
    }

    fn payload(value: i64) -> Vec<u8> {
        encode_runtime_session_payload("php", &data(value), -1).expect("encode payload")
    }

    #[test]
    fn session_ids_are_strict_path_segments() {
        assert!(valid_session_id("abcDEF0123-,"));
        assert!(!valid_session_id(""));
        assert!(!valid_session_id("../bad"));
        assert!(!valid_session_id("bad/slash"));
        assert!(!valid_session_id("bad\\slash"));
        assert!(!valid_session_id("bad\nid"));
    }

    #[test]
    fn session_store_roundtrips_file_payloads() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("sessions");
        let store = SessionStore::new(&root);
        store.ensure_ready().expect("session store ready");
        store.save("abc123", &payload(2)).expect("save session");
        assert_eq!(store.load("abc123").expect("load session"), payload(2));
        assert!(root.join("sess_abc123").is_file());
        store.delete("abc123").expect("delete session");
        assert!(store.load("abc123").expect("load missing").is_empty());
    }

    #[test]
    fn symlink_root_is_rejected() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let temp = tempfile::tempdir().expect("tempdir");
            let target = temp.path().join("target");
            std::fs::create_dir(&target).expect("target");
            let link = temp.path().join("sessions");
            symlink(&target, &link).expect("symlink");
            assert!(SessionStore::new(link).ensure_ready().is_err());
        }
    }

    #[test]
    fn generated_session_ids_are_valid() {
        let id = generate_session_id().expect("generate session id");
        assert!(valid_session_id(&id));
    }

    #[test]
    fn generated_session_ids_follow_length_alphabet_and_prefix_policy() {
        for (bits, alphabet) in [
            (4, "0123456789abcdef"),
            (5, "0123456789abcdefghijklmnopqrstuv"),
            (
                6,
                "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ,-",
            ),
        ] {
            let id = generate_session_id_with_policy(48, bits, "pre-")
                .expect("generate policy session id");
            assert_eq!(id.len(), 52);
            assert!(id.starts_with("pre-"));
            assert!(
                id[4..]
                    .bytes()
                    .all(|byte| alphabet.as_bytes().contains(&byte))
            );
        }
        assert!(generate_session_id_with_policy(21, 4, "").is_err());
        assert!(generate_session_id_with_policy(257, 4, "").is_err());
        assert!(generate_session_id_with_policy(32, 3, "").is_err());
        assert!(generate_session_id_with_policy(32, 4, "bad/").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn server_created_session_root_and_files_have_private_modes() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("sessions");
        let store = SessionStore::new(&root);
        store.ensure_ready().expect("session store ready");
        store.save("private", b"payload").expect("save session");

        assert_eq!(
            std::fs::metadata(&root)
                .expect("session root metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(root.join("sess_private"))
                .expect("session file metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[test]
    fn session_codecs_share_the_runtime_payload_implementation() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(temp.path());
        store.ensure_ready().expect("session store ready");
        for (handler, expected) in [
            ("php", b"n|i:7;".as_slice()),
            ("php_binary", b"\x01ni:7;".as_slice()),
            ("php_serialize", b"a:1:{s:1:\"n\";i:7;}".as_slice()),
        ] {
            let id = handler.replace('_', "-");
            let mut lease = store.acquire(&id).expect("acquire");
            assert!(
                store
                    .commit(
                        &mut lease,
                        &encode_runtime_session_payload(handler, &data(7), -1).expect("encode"),
                        false,
                    )
                    .expect("commit")
            );
            assert_eq!(
                std::fs::read(temp.path().join(format!("sess_{id}"))).expect("read payload"),
                expected
            );
            let lease = store.acquire(&id).expect("reacquire");
            assert_eq!(
                decode_runtime_session_payload(handler, lease.payload()).expect("decode"),
                data(7)
            );
        }
    }

    #[test]
    fn same_id_lock_times_out_while_different_id_remains_available() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::with_lock_timeout(temp.path(), Duration::from_millis(20));
        store.ensure_ready().expect("session store ready");
        let _held = store.acquire("same").expect("first lease");
        assert!(matches!(
            store.acquire("same"),
            Err(SessionStoreError::LockTimeout)
        ));
        assert!(store.acquire("different").is_ok());
    }

    #[test]
    fn cancelled_lock_wait_returns_without_waiting_for_the_timeout() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::with_lock_timeout(temp.path(), Duration::from_secs(5));
        store.ensure_ready().expect("session store ready");
        let _held = store.acquire("same").expect("first lease");
        let cancellation = php_runtime::api::RuntimeCancellationState::new();
        cancellation.cancel();
        let started = std::time::Instant::now();
        assert!(matches!(
            store.acquire_cancellable("same", &cancellation),
            Err(SessionStoreError::Cancelled)
        ));
        assert!(started.elapsed() < Duration::from_millis(100));
    }

    #[test]
    fn lazy_write_touches_without_rewriting_and_regeneration_transfers_the_lease() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(temp.path());
        store.ensure_ready().expect("session store ready");
        let mut lease = store.acquire("old").expect("acquire old");
        store
            .commit(&mut lease, &payload(1), false)
            .expect("initial commit");
        let mut lease = store.acquire("old").expect("reacquire old");
        assert!(
            !store
                .commit(&mut lease, &payload(1), true)
                .expect("lazy touch")
        );
        let mut lease = store.acquire("old").expect("lock old");
        let mut new_lease = store
            .regenerate(&mut lease, "new", &payload(2), true)
            .expect("regenerate");
        assert!(!temp.path().join("sess_old").exists());
        assert_eq!(new_lease.payload(), payload(2));
        store.abort(&mut new_lease).expect("release new");
    }

    #[test]
    fn failed_regeneration_keeps_the_old_lease_intact() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(temp.path());
        store.ensure_ready().expect("session store ready");
        store.save("old", &payload(1)).expect("save old");
        store.save("new", &payload(9)).expect("reserve new");
        let mut old = store.acquire("old").expect("lock old");

        assert!(matches!(
            store.regenerate(&mut old, "new", &payload(2), false),
            Err(SessionStoreError::Io(_))
        ));
        assert!(temp.path().join("sess_old").exists());
        assert_eq!(store.load("new").expect("existing new"), payload(9));
        assert!(!old.finalized());
        store.abort(&mut old).expect("release old");
    }

    #[test]
    fn gc_skips_locked_files_and_deletes_unlocked_expired_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(temp.path());
        store.ensure_ready().expect("session store ready");
        store.save("expired", &payload(1)).expect("save expired");
        store.save("locked", &payload(1)).expect("save locked");
        let old = SystemTime::now() - Duration::from_secs(60);
        for id in ["expired", "locked"] {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(temp.path().join(format!("sess_{id}")))
                .expect("open session file");
            file.set_times(std::fs::FileTimes::new().set_modified(old))
                .expect("age session file");
        }
        let _locked = store.acquire("locked").expect("hold locked session");
        assert_eq!(store.gc(Duration::from_secs(30)).expect("gc"), 1);
        assert!(!temp.path().join("sess_expired").exists());
        assert!(temp.path().join("sess_locked").exists());
    }

    #[cfg(unix)]
    #[test]
    fn replacement_while_waiting_for_lock_reopens_the_current_inode() {
        use std::{fs::OpenOptions, sync::Arc, thread, time::Duration};

        let temp = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(SessionStore::with_lock_timeout(
            temp.path(),
            Duration::from_secs(2),
        ));
        store.ensure_ready().expect("session store ready");
        store.save("race", b"old").expect("save old payload");
        let path = temp.path().join("sess_race");
        let stale = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("open old inode");
        stale.lock().expect("lock old inode");

        let waiter_store = Arc::clone(&store);
        let waiter = thread::spawn(move || {
            waiter_store
                .acquire("race")
                .map(|lease| lease.payload().to_vec())
        });
        thread::sleep(Duration::from_millis(30));
        std::fs::remove_file(&path).expect("unlink old inode");
        std::fs::write(&path, b"new").expect("write replacement inode");
        stale.unlock().expect("unlock old inode");

        assert_eq!(
            waiter
                .join()
                .expect("join lock waiter")
                .expect("acquire replacement"),
            b"new"
        );
    }

    #[test]
    fn symlink_session_entry_is_never_followed() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let temp = tempfile::tempdir().expect("tempdir");
            let outside = temp.path().join("outside");
            std::fs::write(&outside, b"outside").expect("outside file");
            let root = temp.path().join("sessions");
            let store = SessionStore::new(&root);
            store.ensure_ready().expect("session root");
            symlink(&outside, root.join("sess_attack")).expect("session symlink");
            assert!(store.acquire("attack").is_err());
            assert_eq!(
                std::fs::read(&outside).expect("outside unchanged"),
                b"outside"
            );
        }
    }
}
