use super::{
    metrics::ServerMetrics,
    response::{self, ResponseBody},
};
use crate::routing::NormalizedRequestPath;
use bytes::Bytes;
use cap_std::{
    ambient_authority,
    fs::{Dir, File as CapFile, Metadata, OpenOptions},
};
use hyper::{
    Method, Response, StatusCode, header,
    http::{HeaderMap, request::Parts},
};
use php_vm::api::DeploymentRootMode;
use std::{
    collections::{HashMap, HashSet, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    io,
    io::SeekFrom,
    path::{Path, PathBuf},
    sync::{Arc, RwLock, atomic::Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
};

const MAX_STATIC_INDEX_DEPTH: usize = 64;
const MAX_ENTITY_TAG_HEADER_BYTES: usize = 8 * 1024;
const MAX_ENTITY_TAGS: usize = 32;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ContentEncoding {
    Identity,
    Brotli,
    Zstandard,
    Gzip,
}

impl ContentEncoding {
    fn response_value(self) -> Option<&'static str> {
        match self {
            Self::Identity => None,
            Self::Brotli => Some("br"),
            Self::Zstandard => Some("zstd"),
            Self::Gzip => Some("gzip"),
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Self::Identity => "",
            Self::Brotli => ".br",
            Self::Zstandard => ".zst",
            Self::Gzip => ".gz",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublicFileClass {
    AllowedStatic,
    ExecutablePhp,
    HiddenSource,
    HiddenDotOrMetadata,
    HiddenBackup,
    HiddenSidecar,
}

impl PublicFileClass {
    fn is_hidden(self) -> bool {
        !matches!(self, Self::AllowedStatic | Self::ExecutablePhp)
    }
}

#[derive(Debug)]
pub(crate) struct OpenedStaticRepresentation {
    file: File,
    pub(crate) resource_path: Arc<str>,
    pub(crate) encoding: ContentEncoding,
    pub(crate) length: u64,
    pub(crate) modified: Option<SystemTime>,
    pub(crate) etag: Arc<str>,
    pub(crate) content_type: Arc<str>,
    pub(crate) cache_control: &'static str,
    pub(crate) vary_accept_encoding: bool,
}

#[derive(Debug)]
pub(crate) enum StaticResolution {
    Static(OpenedStaticRepresentation),
    PhpScript {
        script_path: PathBuf,
        path_info: Option<String>,
    },
    Directory,
    DirectoryWithoutIndex,
    Missing,
    Hidden(PublicFileClass),
    NotAcceptable {
        vary_accept_encoding: bool,
    },
    Error(String),
}

#[derive(Clone, Debug)]
struct IndexedRepresentation {
    relative_path: Arc<str>,
    encoding: ContentEncoding,
    length: u64,
    modified: Option<SystemTime>,
    etag: Arc<str>,
}

#[derive(Debug)]
struct StaticAsset {
    resource_path: Arc<str>,
    content_type: Arc<str>,
    cache_control: &'static str,
    representations: [Option<IndexedRepresentation>; 4],
    vary_accept_encoding: bool,
}

#[derive(Debug, Default)]
struct StaticAssetIndex {
    assets: HashMap<Arc<str>, Arc<StaticAsset>>,
    directories: HashSet<Arc<str>>,
    php_files: HashSet<Arc<str>>,
}

#[derive(Debug)]
struct RawIndexedFile {
    path: Arc<str>,
    metadata: Metadata,
}

#[derive(Debug)]
pub(crate) struct StaticFileService {
    root: Dir,
    root_path: PathBuf,
    deployment_mode: DeploymentRootMode,
    indexes: Arc<[String]>,
    php_extensions: Arc<[String]>,
    deployment_identity: u64,
    index: RwLock<Option<Arc<StaticAssetIndex>>>,
    metrics: Arc<ServerMetrics>,
}

impl StaticFileService {
    pub(crate) fn new(
        root_path: PathBuf,
        deployment_mode: DeploymentRootMode,
        indexes: Vec<String>,
        php_extensions: Vec<String>,
        metrics: Arc<ServerMetrics>,
    ) -> io::Result<Self> {
        let root = Dir::open_ambient_dir(&root_path, ambient_authority())?;
        let mut identity = DefaultHasher::new();
        root_path.hash(&mut identity);
        root.dir_metadata()?
            .modified()
            .ok()
            .map(|time| time.into_std())
            .hash(&mut identity);
        let service = Self {
            root,
            root_path,
            deployment_mode,
            indexes: indexes.into(),
            php_extensions: php_extensions.into(),
            deployment_identity: identity.finish(),
            index: RwLock::new(None),
            metrics,
        };
        if deployment_mode == DeploymentRootMode::ImmutableDeclared {
            let index = service.build_index()?;
            service
                .metrics
                .static_index_entries
                .store(index.assets.len() as u64, Ordering::Relaxed);
            service
                .metrics
                .static_index_builds
                .fetch_add(1, Ordering::Relaxed);
            *service
                .index
                .write()
                .unwrap_or_else(|error| error.into_inner()) = Some(Arc::new(index));
        }
        Ok(service)
    }

    pub(crate) async fn resolve(
        self: &Arc<Self>,
        path: NormalizedRequestPath,
        headers: HeaderMap,
        front_controller: Option<PathBuf>,
    ) -> StaticResolution {
        let service = Arc::clone(self);
        match tokio::task::spawn_blocking(move || {
            service.resolve_blocking(&path, &headers, front_controller.as_deref())
        })
        .await
        {
            Ok(resolution) => resolution,
            Err(error) => {
                StaticResolution::Error(format!("static resolver worker failed: {error}"))
            }
        }
    }

    pub(crate) async fn rebuild_index(self: &Arc<Self>) -> Result<(), String> {
        if self.deployment_mode != DeploymentRootMode::ImmutableDeclared {
            return Ok(());
        }
        let service = Arc::clone(self);
        let rebuilt = tokio::task::spawn_blocking(move || service.build_index())
            .await
            .map_err(|error| format!("static index worker failed: {error}"))?
            .map_err(|error| format!("static index rebuild failed: {error}"));
        match rebuilt {
            Ok(index) => {
                self.metrics
                    .static_index_entries
                    .store(index.assets.len() as u64, Ordering::Relaxed);
                self.metrics
                    .static_index_builds
                    .fetch_add(1, Ordering::Relaxed);
                *self
                    .index
                    .write()
                    .unwrap_or_else(|error| error.into_inner()) = Some(Arc::new(index));
                Ok(())
            }
            Err(error) => {
                self.metrics
                    .static_index_build_failures
                    .fetch_add(1, Ordering::Relaxed);
                Err(error)
            }
        }
    }

    fn resolve_blocking(
        &self,
        path: &NormalizedRequestPath,
        headers: &HeaderMap,
        front_controller: Option<&Path>,
    ) -> StaticResolution {
        let class = self.classify(path.segments());
        if class.is_hidden() {
            self.metrics
                .static_policy_denied
                .fetch_add(1, Ordering::Relaxed);
            return StaticResolution::Hidden(class);
        }
        if class == PublicFileClass::ExecutablePhp {
            return self.open_php(path.relative_path(), None);
        }

        let direct = if self.deployment_mode == DeploymentRootMode::ImmutableDeclared {
            self.resolve_immutable(path, headers)
        } else {
            self.metrics
                .static_mutable_resolutions
                .fetch_add(1, Ordering::Relaxed);
            self.resolve_mutable(path, headers)
        };
        if !matches!(direct, StaticResolution::Missing) {
            return direct;
        }

        for split in 1..path.segments().len() {
            let prefix = &path.segments()[..split];
            if self.classify(prefix) != PublicFileClass::ExecutablePhp {
                continue;
            }
            let relative = prefix.join("/");
            let path_info = format!("/{}", path.segments()[split..].join("/"));
            let route = self.open_php(Path::new(&relative), Some(path_info));
            if !matches!(route, StaticResolution::Missing) {
                return route;
            }
        }

        if let Some(front_controller) = front_controller {
            let Some(front) = front_controller.to_str() else {
                return StaticResolution::Error("front controller is not UTF-8".to_owned());
            };
            let segments = front.split('/').map(str::to_owned).collect::<Vec<_>>();
            if self.classify(&segments) != PublicFileClass::ExecutablePhp {
                return StaticResolution::Error(
                    "front controller is not a configured PHP file".to_owned(),
                );
            }
            let path_info = (!path.is_root()).then(|| path.uri_path().to_owned());
            return self.open_php(front_controller, path_info);
        }
        StaticResolution::Missing
    }

    fn resolve_mutable(
        &self,
        path: &NormalizedRequestPath,
        headers: &HeaderMap,
    ) -> StaticResolution {
        if path.is_root() {
            let Ok(directory) = self.root.try_clone() else {
                return StaticResolution::Missing;
            };
            return self.resolve_mutable_directory(path, headers, directory);
        }
        let relative = path.relative_path();
        let Some(file_name) = relative.file_name() else {
            return StaticResolution::Missing;
        };
        let parent_path = relative.parent().unwrap_or_else(|| Path::new(""));
        let parent = if parent_path.as_os_str().is_empty() {
            match self.root.try_clone() {
                Ok(parent) => parent,
                Err(_) => return StaticResolution::Missing,
            }
        } else {
            match self.open_directory_request(parent_path) {
                Ok(parent) => parent,
                Err(_) => return StaticResolution::Missing,
            }
        };
        let identity = match self.open_request_in(&parent, file_name) {
            Ok(file) => file,
            Err(_) => return StaticResolution::Missing,
        };
        let metadata = match identity.metadata() {
            Ok(metadata) => metadata,
            Err(_) => return StaticResolution::Missing,
        };
        if metadata.is_dir() {
            let directory = Dir::from_std_file(identity.into_std());
            return self.resolve_mutable_directory(path, headers, directory);
        }
        if !metadata.is_file() {
            return StaticResolution::Hidden(PublicFileClass::HiddenSource);
        }
        self.select_mutable(
            path.relative_string(),
            file_name,
            &parent,
            identity,
            metadata,
            headers,
        )
    }

    fn resolve_mutable_directory(
        &self,
        path: &NormalizedRequestPath,
        headers: &HeaderMap,
        directory: Dir,
    ) -> StaticResolution {
        if !path.is_root() && !path.trailing_slash() {
            return StaticResolution::Directory;
        }
        for index in self.indexes.iter() {
            let relative = join_relative(path.relative_string(), index);
            let segments = relative.split('/').map(str::to_owned).collect::<Vec<_>>();
            match self.classify(&segments) {
                PublicFileClass::ExecutablePhp => {
                    let Ok(file) = self.open_request_in(&directory, index) else {
                        continue;
                    };
                    let Ok(metadata) = file.metadata() else {
                        continue;
                    };
                    if metadata.is_file() {
                        return StaticResolution::PhpScript {
                            script_path: self.root_path.join(&relative),
                            path_info: None,
                        };
                    }
                }
                PublicFileClass::AllowedStatic => {
                    let Ok(file) = self.open_request_in(&directory, index) else {
                        continue;
                    };
                    let Ok(metadata) = file.metadata() else {
                        continue;
                    };
                    if metadata.is_file() {
                        return self.select_mutable(
                            &relative,
                            index.as_ref(),
                            &directory,
                            file,
                            metadata,
                            headers,
                        );
                    }
                }
                _ => {}
            }
        }
        StaticResolution::DirectoryWithoutIndex
    }

    fn select_mutable(
        &self,
        resource: &str,
        file_name: &std::ffi::OsStr,
        directory: &Dir,
        identity: CapFile,
        identity_metadata: Metadata,
        headers: &HeaderMap,
    ) -> StaticResolution {
        let identity_modified = cap_modified(&identity_metadata);
        let mut candidates: [Option<(CapFile, Metadata)>; 4] = [None, None, None, None];
        candidates[encoding_index(ContentEncoding::Identity)] = Some((identity, identity_metadata));
        for encoding in [
            ContentEncoding::Brotli,
            ContentEncoding::Zstandard,
            ContentEncoding::Gzip,
        ] {
            let sidecar = format!("{}{}", file_name.to_string_lossy(), encoding.suffix());
            let Ok(file) = self.open_request_in(directory, &sidecar) else {
                continue;
            };
            let Ok(metadata) = file.metadata() else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            if let (Some(identity_modified), Some(sidecar_modified)) =
                (identity_modified, cap_modified(&metadata))
                && sidecar_modified < identity_modified
            {
                self.metrics
                    .static_stale_sidecars
                    .fetch_add(1, Ordering::Relaxed);
                continue;
            }
            candidates[encoding_index(encoding)] = Some((file, metadata));
        }
        let available = candidates.each_ref().map(Option::is_some);
        let Some(encoding) = select_encoding(headers, available) else {
            self.metrics
                .static_not_acceptable
                .fetch_add(1, Ordering::Relaxed);
            return StaticResolution::NotAcceptable {
                vary_accept_encoding: available[1..].iter().any(|available| *available),
            };
        };
        let vary = available[1..].iter().any(|available| *available);
        let (file, metadata) = candidates[encoding_index(encoding)]
            .take()
            .expect("selected representation exists");
        StaticResolution::Static(
            self.opened_representation(resource, encoding, file, &metadata, vary),
        )
    }

    fn resolve_immutable(
        &self,
        path: &NormalizedRequestPath,
        headers: &HeaderMap,
    ) -> StaticResolution {
        let index = self
            .index
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let Some(index) = index else {
            return StaticResolution::Error("immutable static index is unavailable".to_owned());
        };
        let key = path.relative_string();
        if index.directories.contains(key) {
            if !path.is_root() && !path.trailing_slash() {
                return StaticResolution::Directory;
            }
            for candidate in self.indexes.iter() {
                let relative = join_relative(key, candidate);
                if index.php_files.contains(relative.as_str()) {
                    return self.open_php(Path::new(&relative), None);
                }
                if let Some(asset) = index.assets.get(relative.as_str()) {
                    return self.open_indexed_asset(asset, headers);
                }
            }
            return StaticResolution::DirectoryWithoutIndex;
        }
        let Some(asset) = index.assets.get(key) else {
            self.metrics
                .static_index_misses
                .fetch_add(1, Ordering::Relaxed);
            return StaticResolution::Missing;
        };
        self.metrics
            .static_index_hits
            .fetch_add(1, Ordering::Relaxed);
        self.open_indexed_asset(asset, headers)
    }

    fn open_indexed_asset(&self, asset: &StaticAsset, headers: &HeaderMap) -> StaticResolution {
        let available = asset.representations.each_ref().map(Option::is_some);
        let Some(encoding) = select_encoding(headers, available) else {
            self.metrics
                .static_not_acceptable
                .fetch_add(1, Ordering::Relaxed);
            return StaticResolution::NotAcceptable {
                vary_accept_encoding: asset.vary_accept_encoding,
            };
        };
        let indexed = asset.representations[encoding_index(encoding)]
            .as_ref()
            .expect("selected indexed representation exists");
        let file = match self.open_request(indexed.relative_path.as_ref()) {
            Ok(file) => file,
            Err(_) => return StaticResolution::Missing,
        };
        StaticResolution::Static(OpenedStaticRepresentation {
            file: File::from_std(file.into_std()),
            resource_path: Arc::clone(&asset.resource_path),
            encoding: indexed.encoding,
            length: indexed.length,
            modified: indexed.modified,
            etag: Arc::clone(&indexed.etag),
            content_type: Arc::clone(&asset.content_type),
            cache_control: asset.cache_control,
            vary_accept_encoding: asset.vary_accept_encoding,
        })
    }

    fn opened_representation(
        &self,
        resource: &str,
        encoding: ContentEncoding,
        file: CapFile,
        metadata: &Metadata,
        vary_accept_encoding: bool,
    ) -> OpenedStaticRepresentation {
        OpenedStaticRepresentation {
            file: File::from_std(file.into_std()),
            resource_path: Arc::from(resource),
            encoding,
            length: metadata.len(),
            modified: cap_modified(metadata),
            etag: Arc::from(self.etag(resource, encoding, metadata, false)),
            content_type: Arc::from(content_type_for(resource)),
            cache_control: cache_control(resource, false),
            vary_accept_encoding,
        }
    }

    fn open_php(&self, relative: &Path, path_info: Option<String>) -> StaticResolution {
        let Ok(file) = self.open_request(relative) else {
            return StaticResolution::Missing;
        };
        let Ok(metadata) = file.metadata() else {
            return StaticResolution::Missing;
        };
        if !metadata.is_file() {
            return StaticResolution::Missing;
        }
        StaticResolution::PhpScript {
            script_path: self.root_path.join(relative),
            path_info,
        }
    }

    fn open_request(&self, relative: impl AsRef<Path>) -> io::Result<CapFile> {
        self.open_request_in(&self.root, relative)
    }

    fn open_request_in(&self, directory: &Dir, relative: impl AsRef<Path>) -> io::Result<CapFile> {
        let opened = directory.open_with(relative, &static_open_options());
        if opened.is_ok() {
            self.metrics
                .static_capability_opens
                .fetch_add(1, Ordering::Relaxed);
        }
        opened
    }

    fn open_directory_request(&self, relative: impl AsRef<Path>) -> io::Result<Dir> {
        let opened = self.root.open_dir(relative);
        if opened.is_ok() {
            self.metrics
                .static_capability_opens
                .fetch_add(1, Ordering::Relaxed);
        }
        opened
    }

    pub(crate) fn classify(&self, segments: &[String]) -> PublicFileClass {
        classify_public_path(segments, &self.php_extensions)
    }

    fn etag(
        &self,
        resource: &str,
        encoding: ContentEncoding,
        metadata: &Metadata,
        immutable: bool,
    ) -> String {
        let mut hash = DefaultHasher::new();
        resource.hash(&mut hash);
        encoding.hash(&mut hash);
        metadata.len().hash(&mut hash);
        cap_modified(metadata).hash(&mut hash);
        metadata_identity(metadata).hash(&mut hash);
        if immutable {
            self.deployment_identity.hash(&mut hash);
            format!("\"{:016x}\"", hash.finish())
        } else {
            format!("W/\"{:016x}\"", hash.finish())
        }
    }

    fn build_index(&self) -> io::Result<StaticAssetIndex> {
        let mut raw = HashMap::<Arc<str>, RawIndexedFile>::new();
        let mut directories = HashSet::<Arc<str>>::new();
        let mut php_files = HashSet::<Arc<str>>::new();
        let mut visited = HashSet::new();
        self.walk_directory(
            self.root.try_clone()?,
            String::new(),
            0,
            &mut visited,
            &mut directories,
            &mut raw,
            &mut php_files,
        )?;
        let mut assets = HashMap::new();
        for file in raw.values() {
            let segments = file.path.split('/').map(str::to_owned).collect::<Vec<_>>();
            if self.classify(&segments) != PublicFileClass::AllowedStatic {
                continue;
            }
            let resource = Arc::clone(&file.path);
            let mut representations: [Option<IndexedRepresentation>; 4] = [None, None, None, None];
            for encoding in [
                ContentEncoding::Identity,
                ContentEncoding::Brotli,
                ContentEncoding::Zstandard,
                ContentEncoding::Gzip,
            ] {
                let candidate_path: Arc<str> = if encoding == ContentEncoding::Identity {
                    Arc::clone(&resource)
                } else {
                    Arc::from(format!("{resource}{}", encoding.suffix()))
                };
                let Some(candidate) = raw.get(candidate_path.as_ref()) else {
                    continue;
                };
                if encoding != ContentEncoding::Identity
                    && let (Some(identity_modified), Some(sidecar_modified)) = (
                        cap_modified(&file.metadata),
                        cap_modified(&candidate.metadata),
                    )
                    && sidecar_modified < identity_modified
                {
                    continue;
                }
                representations[encoding_index(encoding)] = Some(IndexedRepresentation {
                    relative_path: candidate_path,
                    encoding,
                    length: candidate.metadata.len(),
                    modified: cap_modified(&candidate.metadata),
                    etag: Arc::from(self.etag(&resource, encoding, &candidate.metadata, true)),
                });
            }
            let vary = representations[1..].iter().any(Option::is_some);
            assets.insert(
                Arc::clone(&resource),
                Arc::new(StaticAsset {
                    resource_path: Arc::clone(&resource),
                    content_type: Arc::from(content_type_for(&resource)),
                    cache_control: cache_control(&resource, true),
                    representations,
                    vary_accept_encoding: vary,
                }),
            );
        }
        Ok(StaticAssetIndex {
            assets,
            directories,
            php_files,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn walk_directory(
        &self,
        directory: Dir,
        relative: String,
        depth: usize,
        visited: &mut HashSet<DirectoryIdentity>,
        directories: &mut HashSet<Arc<str>>,
        raw: &mut HashMap<Arc<str>, RawIndexedFile>,
        php_files: &mut HashSet<Arc<str>>,
    ) -> io::Result<()> {
        if depth > MAX_STATIC_INDEX_DEPTH {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "static index traversal depth exceeded",
            ));
        }
        let identity = directory_identity(&directory.dir_metadata()?);
        if !visited.insert(identity) {
            return Ok(());
        }
        directories.insert(Arc::from(relative.as_str()));
        let entries = directory
            .entries()
            .map_err(|error| index_error(&relative, "read directory", error))?;
        for entry in entries {
            let entry = entry.map_err(|error| index_error(&relative, "read entry", error))?;
            let name = entry.file_name().into_string().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "static index entry under `{}` is not UTF-8",
                        display_index_path(&relative)
                    ),
                )
            })?;
            let child = join_relative(&relative, &name);
            let file_type = entry
                .file_type()
                .map_err(|error| index_error(&child, "inspect entry type", error))?;
            let segments = child.split('/').map(str::to_owned).collect::<Vec<_>>();
            let class = self.classify(&segments);
            if class.is_hidden() && class != PublicFileClass::HiddenSidecar {
                continue;
            }
            match entry.open_dir() {
                Ok(subdirectory) => {
                    if class == PublicFileClass::AllowedStatic {
                        self.walk_directory(
                            subdirectory,
                            child,
                            depth + 1,
                            visited,
                            directories,
                            raw,
                            php_files,
                        )?;
                    }
                    continue;
                }
                Err(error) if file_type.is_dir() => {
                    return Err(index_error(&child, "open directory", error));
                }
                Err(_) => {}
            }
            let file = match entry.open_with(&static_open_options()) {
                Ok(file) => file,
                Err(_) if file_type.is_symlink() || !file_type.is_file() => continue,
                Err(error) => return Err(index_error(&child, "open file", error)),
            };
            let metadata = file
                .metadata()
                .map_err(|error| index_error(&child, "inspect opened file", error))?;
            if !metadata.is_file() {
                continue;
            }
            let path: Arc<str> = Arc::from(child);
            if class == PublicFileClass::ExecutablePhp {
                php_files.insert(Arc::clone(&path));
            }
            raw.insert(Arc::clone(&path), RawIndexedFile { path, metadata });
        }
        Ok(())
    }
}

#[cfg(unix)]
type DirectoryIdentity = (u64, u64);
#[cfg(not(unix))]
type DirectoryIdentity = (u64, u64);

#[cfg(unix)]
fn directory_identity(metadata: &Metadata) -> DirectoryIdentity {
    use cap_std::fs::MetadataExt;
    (metadata.dev(), metadata.ino())
}

#[cfg(not(unix))]
fn directory_identity(metadata: &Metadata) -> DirectoryIdentity {
    (
        metadata.len(),
        cap_modified(metadata)
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |time| time.as_nanos() as u64),
    )
}

#[cfg(unix)]
fn metadata_identity(metadata: &Metadata) -> u64 {
    use cap_std::fs::MetadataExt;
    metadata.ino()
}

#[cfg(not(unix))]
fn metadata_identity(_metadata: &Metadata) -> u64 {
    0
}

fn cap_modified(metadata: &Metadata) -> Option<SystemTime> {
    metadata.modified().ok().map(|time| time.into_std())
}

fn static_open_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true)._cap_fs_ext_nonblock(true);
    options
}

fn join_relative(base: &str, child: &str) -> String {
    if base.is_empty() {
        child.to_owned()
    } else {
        format!("{base}/{child}")
    }
}

fn display_index_path(path: &str) -> &str {
    if path.is_empty() { "." } else { path }
}

fn index_error(path: &str, operation: &str, error: io::Error) -> io::Error {
    io::Error::new(
        error.kind(),
        format!(
            "static index failed to {operation} `{}`: {error}",
            display_index_path(path)
        ),
    )
}

fn encoding_index(encoding: ContentEncoding) -> usize {
    match encoding {
        ContentEncoding::Identity => 0,
        ContentEncoding::Brotli => 1,
        ContentEncoding::Zstandard => 2,
        ContentEncoding::Gzip => 3,
    }
}

pub(crate) fn classify_public_path(
    segments: &[String],
    php_extensions: &[String],
) -> PublicFileClass {
    if segments.is_empty() {
        return PublicFileClass::AllowedStatic;
    }
    for (index, segment) in segments.iter().enumerate() {
        let lower = segment.to_ascii_lowercase();
        if segment.starts_with('.') && !(index == 0 && lower == ".well-known") {
            return PublicFileClass::HiddenDotOrMetadata;
        }
        if matches!(
            lower.as_str(),
            ".git"
                | ".svn"
                | ".hg"
                | ".bzr"
                | "cvs"
                | ".env"
                | ".htaccess"
                | ".htpasswd"
                | "web.config"
        ) {
            return PublicFileClass::HiddenDotOrMetadata;
        }
        if lower.ends_with('~')
            || [
                ".bak", ".backup", ".old", ".orig", ".rej", ".swp", ".swo", ".tmp", ".temp",
            ]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
        {
            return PublicFileClass::HiddenBackup;
        }
    }
    let lower = segments
        .last()
        .expect("segments is non-empty")
        .to_ascii_lowercase();
    if [".br", ".gz", ".zst"]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
    {
        return PublicFileClass::HiddenSidecar;
    }
    let extension = Path::new(&lower)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if php_extensions
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(extension))
    {
        return PublicFileClass::ExecutablePhp;
    }
    if matches!(
        extension,
        "php" | "phtml" | "phar" | "inc" | "php3" | "php4" | "php5" | "php7" | "phps"
    ) {
        return PublicFileClass::HiddenSource;
    }
    PublicFileClass::AllowedStatic
}

fn select_encoding(headers: &HeaderMap, available: [bool; 4]) -> Option<ContentEncoding> {
    if !headers.contains_key(header::ACCEPT_ENCODING) {
        return available[0].then_some(ContentEncoding::Identity);
    }
    let mut explicit = HashMap::<String, u16>::new();
    let mut wildcard = None;
    for value in headers.get_all(header::ACCEPT_ENCODING) {
        let Ok(value) = value.to_str() else {
            continue;
        };
        for member in value.split(',') {
            let mut pieces = member.trim().split(';');
            let token = pieces
                .next()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            if token.is_empty() {
                continue;
            }
            let mut quality = Some(1000);
            for parameter in pieces {
                let Some((name, value)) = parameter.trim().split_once('=') else {
                    quality = None;
                    break;
                };
                if name.trim().eq_ignore_ascii_case("q") {
                    quality = parse_quality(value.trim());
                }
            }
            let Some(quality) = quality else {
                continue;
            };
            if token == "*" {
                wildcard = Some(quality);
            } else {
                explicit.insert(token, quality);
            }
        }
    }
    let quality = |encoding: ContentEncoding| -> u16 {
        let token = match encoding {
            ContentEncoding::Identity => "identity",
            ContentEncoding::Brotli => "br",
            ContentEncoding::Zstandard => "zstd",
            ContentEncoding::Gzip => "gzip",
        };
        explicit.get(token).copied().unwrap_or_else(|| {
            if encoding == ContentEncoding::Identity {
                wildcard.unwrap_or(1000)
            } else {
                wildcard.unwrap_or(0)
            }
        })
    };
    [
        ContentEncoding::Brotli,
        ContentEncoding::Zstandard,
        ContentEncoding::Gzip,
        ContentEncoding::Identity,
    ]
    .into_iter()
    .filter(|encoding| available[encoding_index(*encoding)])
    .map(|encoding| (quality(encoding), encoding))
    .filter(|(quality, _)| *quality > 0)
    .max_by_key(|(quality, encoding)| (*quality, encoding_preference(*encoding)))
    .map(|(_, encoding)| encoding)
}

fn encoding_preference(encoding: ContentEncoding) -> u8 {
    match encoding {
        ContentEncoding::Brotli => 4,
        ContentEncoding::Zstandard => 3,
        ContentEncoding::Gzip => 2,
        ContentEncoding::Identity => 1,
    }
}

fn parse_quality(value: &str) -> Option<u16> {
    if value == "0" {
        return Some(0);
    }
    if value == "1" {
        return Some(1000);
    }
    if let Some(fraction) = value.strip_prefix("0.")
        && fraction.len() <= 3
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        if fraction.is_empty() {
            return Some(0);
        }
        return fraction
            .parse::<u16>()
            .ok()
            .map(|value| value * 10_u16.pow((3 - fraction.len()) as u32));
    }
    if let Some(fraction) = value.strip_prefix("1.")
        && fraction.len() <= 3
        && fraction.bytes().all(|byte| byte == b'0')
    {
        return Some(1000);
    }
    None
}

fn content_type_for(path: &str) -> String {
    let extension = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "js" | "mjs" => "text/javascript".to_owned(),
        "html" | "htm" => "text/html; charset=UTF-8".to_owned(),
        "css" => "text/css; charset=UTF-8".to_owned(),
        "txt" => "text/plain; charset=UTF-8".to_owned(),
        "md" => "text/markdown; charset=UTF-8".to_owned(),
        "csv" => "text/csv; charset=UTF-8".to_owned(),
        "json" | "map" => "application/json".to_owned(),
        "wasm" => "application/wasm".to_owned(),
        _ => mime_guess::from_path(path)
            .first_raw()
            .unwrap_or("application/octet-stream")
            .to_owned(),
    }
}

fn cache_control(path: &str, immutable: bool) -> &'static str {
    let extension = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !immutable || extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm")
    {
        return "no-cache";
    }
    if is_fingerprinted(path) {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=3600"
    }
}

fn is_fingerprinted(path: &str) -> bool {
    let Some(file_name) = Path::new(path).file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    let Some((stem, _)) = file_name.rsplit_once('.') else {
        return false;
    };
    stem.rsplit(['.', '-', '_']).next().is_some_and(|token| {
        (8..=64).contains(&token.len()) && token.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ByteRange {
    pub(crate) start: u64,
    pub(crate) end: u64,
}
impl ByteRange {
    fn len(self) -> u64 {
        self.end - self.start + 1
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RangeDecision {
    Ignore,
    Unsatisfiable,
    Partial(ByteRange),
}

fn parse_range(value: &str, full_len: u64) -> RangeDecision {
    let Some(range) = value.trim().strip_prefix("bytes=") else {
        return RangeDecision::Ignore;
    };
    if range.contains(',') {
        return RangeDecision::Ignore;
    }
    let Some((start, end)) = range.split_once('-') else {
        return RangeDecision::Ignore;
    };
    if start.is_empty() {
        let Ok(suffix) = end.parse::<u64>() else {
            return RangeDecision::Ignore;
        };
        if suffix == 0 {
            return RangeDecision::Ignore;
        }
        if full_len == 0 {
            return RangeDecision::Unsatisfiable;
        }
        return RangeDecision::Partial(ByteRange {
            start: full_len.saturating_sub(suffix),
            end: full_len - 1,
        });
    }
    let Ok(start) = start.parse::<u64>() else {
        return RangeDecision::Ignore;
    };
    if full_len == 0 || start >= full_len {
        return RangeDecision::Unsatisfiable;
    }
    let end = if end.is_empty() {
        full_len - 1
    } else {
        let Ok(end) = end.parse::<u64>() else {
            return RangeDecision::Ignore;
        };
        if end < start {
            return RangeDecision::Ignore;
        }
        end.min(full_len - 1)
    };
    RangeDecision::Partial(ByteRange { start, end })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StaticSelection {
    Full,
    Head,
    NotModified,
    PreconditionFailed,
    Partial(ByteRange),
    Unsatisfiable,
}

fn select_response(parts: &Parts, representation: &OpenedStaticRepresentation) -> StaticSelection {
    let headers = &parts.headers;
    let current = EntityTag::parse(&representation.etag);
    let if_match_present = headers.contains_key(header::IF_MATCH);
    if let Some(value) = headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
        && !etag_condition_matches(value, current.as_ref(), true)
    {
        return StaticSelection::PreconditionFailed;
    }
    if !if_match_present
        && let (Some(modified), Some(limit)) = (
            representation.modified,
            headers
                .get(header::IF_UNMODIFIED_SINCE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| httpdate::parse_http_date(value).ok()),
        )
        && unix_seconds(modified) > unix_seconds(limit)
    {
        return StaticSelection::PreconditionFailed;
    }
    let if_none_present = headers.contains_key(header::IF_NONE_MATCH);
    if let Some(value) = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        && etag_condition_matches(value, current.as_ref(), false)
    {
        return StaticSelection::NotModified;
    }
    if !if_none_present
        && let (Some(modified), Some(limit)) = (
            representation.modified,
            headers
                .get(header::IF_MODIFIED_SINCE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| httpdate::parse_http_date(value).ok()),
        )
        && unix_seconds(modified) <= unix_seconds(limit)
    {
        return StaticSelection::NotModified;
    }
    if parts.method == Method::HEAD {
        return StaticSelection::Head;
    }
    let mut range_values = headers.get_all(header::RANGE).iter();
    let Some(value) = range_values.next().and_then(|value| value.to_str().ok()) else {
        return StaticSelection::Full;
    };
    if range_values.next().is_some() {
        return StaticSelection::Full;
    }
    if let Some(if_range) = headers
        .get(header::IF_RANGE)
        .and_then(|value| value.to_str().ok())
        && !if_range_matches(if_range, representation)
    {
        return StaticSelection::Full;
    }
    match parse_range(value, representation.length) {
        RangeDecision::Ignore => StaticSelection::Full,
        RangeDecision::Unsatisfiable => StaticSelection::Unsatisfiable,
        RangeDecision::Partial(range) => StaticSelection::Partial(range),
    }
}

#[derive(Debug)]
struct EntityTag<'a> {
    weak: bool,
    opaque: &'a str,
}
impl<'a> EntityTag<'a> {
    fn parse(value: &'a str) -> Option<Self> {
        let value = value.trim();
        let (weak, value) = value
            .strip_prefix("W/")
            .map_or((false, value), |value| (true, value));
        if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
            return None;
        }
        let opaque = &value[1..value.len() - 1];
        opaque
            .bytes()
            .all(|byte| byte == 0x21 || (0x23..=0x7e).contains(&byte))
            .then_some(Self { weak, opaque })
    }
}

fn etag_condition_matches(value: &str, current: Option<&EntityTag<'_>>, strong: bool) -> bool {
    if value.len() > MAX_ENTITY_TAG_HEADER_BYTES {
        return false;
    }
    let Some(current) = current else {
        return false;
    };
    let mut quoted = false;
    let mut start = 0;
    let mut count = 0;
    for (index, byte) in value.bytes().enumerate() {
        match byte {
            b'"' => quoted = !quoted,
            b',' if !quoted => {
                count += 1;
                if count > MAX_ENTITY_TAGS {
                    return false;
                }
                if entity_tag_member_matches(&value[start..index], current, strong) {
                    return true;
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    if quoted || count >= MAX_ENTITY_TAGS {
        return false;
    }
    entity_tag_member_matches(&value[start..], current, strong)
}

fn entity_tag_member_matches(candidate: &str, current: &EntityTag<'_>, strong: bool) -> bool {
    let candidate = candidate.trim();
    if candidate == "*" {
        return true;
    }
    EntityTag::parse(candidate).is_some_and(|candidate| {
        candidate.opaque == current.opaque && (!strong || (!candidate.weak && !current.weak))
    })
}

fn if_range_matches(value: &str, representation: &OpenedStaticRepresentation) -> bool {
    if let Some(candidate) = EntityTag::parse(value) {
        return !candidate.weak
            && EntityTag::parse(&representation.etag)
                .is_some_and(|current| !current.weak && current.opaque == candidate.opaque);
    }
    let Some(candidate) = httpdate::parse_http_date(value).ok() else {
        return false;
    };
    representation
        .modified
        .is_some_and(|modified| unix_seconds(modified) == unix_seconds(candidate))
}

pub(crate) async fn static_file_response(
    parts: &Parts,
    metrics: &ServerMetrics,
    mut representation: OpenedStaticRepresentation,
) -> Response<ResponseBody> {
    debug_assert!(!representation.resource_path.starts_with('/'));
    let selection = select_response(parts, &representation);
    match selection {
        StaticSelection::NotModified => {
            metrics.static_not_modified.fetch_add(1, Ordering::Relaxed);
            static_empty_response(StatusCode::NOT_MODIFIED, &representation, None, None)
        }
        StaticSelection::PreconditionFailed => static_empty_response(
            StatusCode::PRECONDITION_FAILED,
            &representation,
            Some(0),
            None,
        ),
        StaticSelection::Unsatisfiable => {
            let range = format!("bytes */{}", representation.length);
            static_empty_response(
                StatusCode::RANGE_NOT_SATISFIABLE,
                &representation,
                Some(0),
                Some(&range),
            )
        }
        StaticSelection::Head => static_empty_response(
            StatusCode::OK,
            &representation,
            Some(representation.length),
            None,
        ),
        StaticSelection::Full => {
            record_encoding_metric(metrics, representation.encoding);
            let length = representation.length;
            static_stream_response(StatusCode::OK, representation, length, None)
        }
        StaticSelection::Partial(range) => {
            metrics
                .static_partial_responses
                .fetch_add(1, Ordering::Relaxed);
            record_encoding_metric(metrics, representation.encoding);
            if range.start > 0
                && representation
                    .file
                    .seek(SeekFrom::Start(range.start))
                    .await
                    .is_err()
            {
                return response::text(StatusCode::INTERNAL_SERVER_ERROR, "static file failed\n");
            }
            let content_range = format!(
                "bytes {}-{}/{}",
                range.start, range.end, representation.length
            );
            static_stream_response(
                StatusCode::PARTIAL_CONTENT,
                representation,
                range.len(),
                Some(&content_range),
            )
        }
    }
}

pub(crate) fn static_not_acceptable_response(vary_accept_encoding: bool) -> Response<ResponseBody> {
    let mut response = response::text(StatusCode::NOT_ACCEPTABLE, "not acceptable\n");
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        hyper::http::HeaderValue::from_static("nosniff"),
    );
    if vary_accept_encoding {
        response.headers_mut().insert(
            header::VARY,
            hyper::http::HeaderValue::from_static("Accept-Encoding"),
        );
    }
    response
}

fn record_encoding_metric(metrics: &ServerMetrics, encoding: ContentEncoding) {
    let counter = match encoding {
        ContentEncoding::Identity => &metrics.static_identity_responses,
        ContentEncoding::Brotli => &metrics.static_br_responses,
        ContentEncoding::Zstandard => &metrics.static_zstd_responses,
        ContentEncoding::Gzip => &metrics.static_gzip_responses,
    };
    counter.fetch_add(1, Ordering::Relaxed);
    if encoding != ContentEncoding::Identity {
        metrics
            .static_precompressed_hits
            .fetch_add(1, Ordering::Relaxed);
    }
}

fn static_stream_response(
    status: StatusCode,
    representation: OpenedStaticRepresentation,
    content_len: u64,
    content_range: Option<&str>,
) -> Response<ResponseBody> {
    let builder =
        static_response_builder(status, &representation, Some(content_len), content_range);
    builder
        .body(response::reader_body_with_length(
            representation.file.take(content_len),
            content_len,
        ))
        .expect("static stream response builder is valid")
}

fn static_empty_response(
    status: StatusCode,
    representation: &OpenedStaticRepresentation,
    content_len: Option<u64>,
    content_range: Option<&str>,
) -> Response<ResponseBody> {
    static_response_builder(status, representation, content_len, content_range)
        .body(response::full_body(Bytes::new()))
        .expect("static empty response builder is valid")
}

fn static_response_builder(
    status: StatusCode,
    representation: &OpenedStaticRepresentation,
    content_len: Option<u64>,
    content_range: Option<&str>,
) -> hyper::http::response::Builder {
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, representation.content_type.as_ref())
        .header(header::ETAG, representation.etag.as_ref())
        .header(header::CACHE_CONTROL, representation.cache_control)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");
    if let Some(content_len) = content_len {
        builder = builder.header(header::CONTENT_LENGTH, content_len.to_string());
    }
    if let Some(modified) = representation.modified {
        builder = builder.header(header::LAST_MODIFIED, httpdate::fmt_http_date(modified));
    }
    if let Some(encoding) = representation.encoding.response_value() {
        builder = builder.header(header::CONTENT_ENCODING, encoding);
    }
    if representation.vary_accept_encoding {
        builder = builder.header(header::VARY, "Accept-Encoding");
    }
    if let Some(content_range) = content_range {
        builder = builder.header(header::CONTENT_RANGE, content_range);
    }
    builder
}

fn unix_seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{Request, header::HeaderValue};

    fn encoding_headers(values: &[&str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for value in values {
            headers.append(
                header::ACCEPT_ENCODING,
                HeaderValue::from_str(value).expect("valid test header"),
            );
        }
        headers
    }

    fn representation(etag: &str) -> OpenedStaticRepresentation {
        let executable = std::env::current_exe().expect("test executable path");
        let file = std::fs::File::open(executable).expect("open test executable");
        OpenedStaticRepresentation {
            file: File::from_std(file),
            resource_path: Arc::from("asset.txt"),
            encoding: ContentEncoding::Identity,
            length: 10,
            modified: Some(UNIX_EPOCH + std::time::Duration::from_secs(1_000)),
            etag: Arc::from(etag),
            content_type: Arc::from("text/plain; charset=UTF-8"),
            cache_control: "no-cache",
            vary_accept_encoding: true,
        }
    }

    fn request_parts(method: Method, headers: &[(&str, &str)]) -> Parts {
        let mut request = Request::builder().method(method).uri("/asset.txt");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        request.body(()).expect("valid request").into_parts().0
    }

    #[test]
    fn quality_parser_is_strict() {
        assert_eq!(parse_quality("0"), Some(0));
        assert_eq!(parse_quality("0."), Some(0));
        assert_eq!(parse_quality("0.2"), Some(200));
        assert_eq!(parse_quality("0.123"), Some(123));
        assert_eq!(parse_quality("1.000"), Some(1000));
        assert_eq!(parse_quality("1.1"), None);
        assert_eq!(parse_quality("0.1234"), None);
    }

    #[test]
    fn encoding_selection_honors_quality_wildcard_identity_and_ties() {
        let all = [true; 4];
        assert_eq!(
            select_encoding(&encoding_headers(&["gzip;q=1, br;q=0.2"]), all),
            Some(ContentEncoding::Gzip)
        );
        assert_eq!(
            select_encoding(
                &encoding_headers(&["identity;q=1, gzip;q=1, zstd;q=1, br;q=1"]),
                all,
            ),
            Some(ContentEncoding::Brotli)
        );
        assert_eq!(
            select_encoding(&encoding_headers(&["*;q=0.7, gzip;q=0.2"]), all),
            Some(ContentEncoding::Brotli)
        );
        assert_eq!(
            select_encoding(
                &encoding_headers(&["identity;q=0, br;q=0, zstd;q=0, gzip;q=0"]),
                all,
            ),
            None
        );
        assert_eq!(
            select_encoding(&encoding_headers(&["zst;q=1", "identity;q=0"]), all,),
            None
        );
        assert_eq!(
            select_encoding(&HeaderMap::new(), all),
            Some(ContentEncoding::Identity)
        );
        assert_eq!(
            select_encoding(&encoding_headers(&[""]), all),
            Some(ContentEncoding::Identity)
        );
    }

    #[test]
    fn mime_matrix_uses_identity_extension_and_modern_overrides() {
        for (path, expected) in [
            ("a.html", "text/html; charset=UTF-8"),
            ("a.css", "text/css; charset=UTF-8"),
            ("a.js", "text/javascript"),
            ("a.mjs", "text/javascript"),
            ("a.json", "application/json"),
            ("a.map", "application/json"),
            ("a.svg", "image/svg+xml"),
            ("a.wasm", "application/wasm"),
            ("a.woff", "application/font-woff"),
            ("a.woff2", "font/woff2"),
            ("a.png", "image/png"),
            ("a.jpg", "image/jpeg"),
            ("a.webp", "image/webp"),
            ("a.avif", "image/avif"),
            ("a.ico", "image/x-icon"),
            ("a.pdf", "application/pdf"),
            ("a.mp3", "audio/mpeg"),
            ("a.mp4", "video/mp4"),
            ("a.xml", "text/xml"),
            ("a.txt", "text/plain; charset=UTF-8"),
            ("a.unknown-phrust", "application/octet-stream"),
        ] {
            assert_eq!(content_type_for(path), expected, "{path}");
        }
    }

    #[test]
    fn preconditions_follow_precedence_and_strong_comparison_rules() {
        let representation = representation("\"strong\"");
        let wildcard = request_parts(Method::GET, &[("if-match", "*")]);
        assert_eq!(
            select_response(&wildcard, &representation),
            StaticSelection::Full
        );
        let parts = request_parts(
            Method::GET,
            &[("if-match", "W/\"strong\""), ("if-none-match", "\"other\"")],
        );
        assert_eq!(
            select_response(&parts, &representation),
            StaticSelection::PreconditionFailed
        );

        let parts = request_parts(
            Method::GET,
            &[
                ("if-match", "\"strong\""),
                ("if-unmodified-since", "Thu, 01 Jan 1970 00:00:00 GMT"),
                ("if-none-match", "W/\"strong\""),
            ],
        );
        assert_eq!(
            select_response(&parts, &representation),
            StaticSelection::NotModified
        );

        let parts = request_parts(
            Method::GET,
            &[
                ("if-none-match", "\"other\""),
                ("if-modified-since", "Thu, 01 Jan 2100 00:00:00 GMT"),
            ],
        );
        assert_eq!(
            select_response(&parts, &representation),
            StaticSelection::Full
        );
    }

    #[test]
    fn if_range_and_head_apply_the_defined_range_contract() {
        let representation = representation("\"strong\"");
        let matching = request_parts(
            Method::GET,
            &[("range", "bytes=2-4"), ("if-range", "\"strong\"")],
        );
        assert_eq!(
            select_response(&matching, &representation),
            StaticSelection::Partial(ByteRange { start: 2, end: 4 })
        );
        for validator in ["W/\"strong\"", "\"other\"", "malformed"] {
            let parts = request_parts(
                Method::GET,
                &[("range", "bytes=2-4"), ("if-range", validator)],
            );
            assert_eq!(
                select_response(&parts, &representation),
                StaticSelection::Full
            );
        }
        let matching_date = request_parts(
            Method::GET,
            &[
                ("range", "bytes=2-4"),
                ("if-range", "Thu, 01 Jan 1970 00:16:40 GMT"),
            ],
        );
        assert_eq!(
            select_response(&matching_date, &representation),
            StaticSelection::Partial(ByteRange { start: 2, end: 4 })
        );
        let mismatching_date = request_parts(
            Method::GET,
            &[
                ("range", "bytes=2-4"),
                ("if-range", "Thu, 01 Jan 1970 00:16:39 GMT"),
            ],
        );
        assert_eq!(
            select_response(&mismatching_date, &representation),
            StaticSelection::Full
        );
        let head = request_parts(Method::HEAD, &[("range", "bytes=2-4")]);
        assert_eq!(
            select_response(&head, &representation),
            StaticSelection::Head
        );
        let repeated = request_parts(
            Method::GET,
            &[("range", "bytes=0-1"), ("range", "bytes=3-4")],
        );
        assert_eq!(
            select_response(&repeated, &representation),
            StaticSelection::Full
        );
    }

    #[test]
    fn fingerprint_detection_is_bounded_hex() {
        assert!(is_fingerprinted("assets/app.a1b2c3d4.js"));
        assert!(is_fingerprinted("assets/app-a1b2c3d4.css"));
        assert!(!is_fingerprinted("assets/app.main.js"));
        assert!(!is_fingerprinted("assets/app.1234567.js"));
    }

    #[test]
    fn malformed_and_multiple_ranges_are_ignored() {
        assert_eq!(parse_range("items=0-1", 10), RangeDecision::Ignore);
        assert_eq!(parse_range("bytes=0-1,3-4", 10), RangeDecision::Ignore);
        assert_eq!(parse_range("bytes=overflow-", 10), RangeDecision::Ignore);
        assert_eq!(
            parse_range("bytes=18446744073709551616-", 10),
            RangeDecision::Ignore
        );
        assert_eq!(parse_range("bytes=20-", 10), RangeDecision::Unsatisfiable);
        assert_eq!(
            parse_range("bytes=4-", 10),
            RangeDecision::Partial(ByteRange { start: 4, end: 9 })
        );
        assert_eq!(
            parse_range("bytes=-99", 10),
            RangeDecision::Partial(ByteRange { start: 0, end: 9 })
        );
        assert_eq!(parse_range("bytes=0-0", 0), RangeDecision::Unsatisfiable);
    }

    #[test]
    fn public_policy_hides_sources_and_sidecars() {
        let php = ["php".to_owned()];
        assert_eq!(
            classify_public_path(&["index.php".to_owned()], &php),
            PublicFileClass::ExecutablePhp
        );
        assert_eq!(
            classify_public_path(&["index.phtml".to_owned()], &php),
            PublicFileClass::HiddenSource
        );
        assert_eq!(
            classify_public_path(&["app.js.br".to_owned()], &php),
            PublicFileClass::HiddenSidecar
        );
        assert_eq!(
            classify_public_path(&[".env".to_owned()], &php),
            PublicFileClass::HiddenDotOrMetadata
        );
        assert_eq!(
            classify_public_path(&[".well-known".to_owned(), "asset.txt".to_owned()], &php),
            PublicFileClass::AllowedStatic
        );
    }
}
