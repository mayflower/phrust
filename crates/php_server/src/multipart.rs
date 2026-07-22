use crate::{metrics::ServerMetrics, request_pipeline::RequestUploadSet, response::RequestBody};
use futures_util::StreamExt;
use http_body_util::BodyExt;
use multer::{Constraints, Multipart, SizeLimit};
use php_runtime::api::{RuntimeInputPair, RuntimeUploadedFile};
use std::{
    path::PathBuf,
    sync::{Arc, Weak, atomic::Ordering},
};
use tokio::io::AsyncWriteExt;

pub(crate) const UPLOAD_ERR_OK: i64 = 0;
pub(crate) const UPLOAD_ERR_INI_SIZE: i64 = 1;
pub(crate) const UPLOAD_ERR_FORM_SIZE: i64 = 2;
pub(crate) const UPLOAD_ERR_PARTIAL: i64 = 3;
pub(crate) const UPLOAD_ERR_NO_FILE: i64 = 4;
pub(crate) const UPLOAD_ERR_NO_TMP_DIR: i64 = 6;
pub(crate) const UPLOAD_ERR_CANT_WRITE: i64 = 7;

struct UploadTempfileGauge {
    metrics: Weak<ServerMetrics>,
    bytes: u64,
}

impl UploadTempfileGauge {
    fn new(metrics: &Arc<ServerMetrics>) -> Self {
        metrics
            .upload_tempfiles_active
            .fetch_add(1, Ordering::Relaxed);
        Self {
            metrics: Arc::downgrade(metrics),
            bytes: 0,
        }
    }

    fn wrote(&mut self, bytes: u64) {
        self.bytes = self.bytes.saturating_add(bytes);
        if let Some(metrics) = self.metrics.upgrade() {
            metrics
                .upload_tempfile_bytes_active
                .fetch_add(bytes, Ordering::Relaxed);
            metrics
                .upload_bytes_written_total
                .fetch_add(bytes, Ordering::Relaxed);
        }
    }
}

impl Drop for UploadTempfileGauge {
    fn drop(&mut self) {
        if let Some(metrics) = self.metrics.upgrade() {
            metrics
                .upload_tempfiles_active
                .fetch_sub(1, Ordering::Relaxed);
            metrics
                .upload_tempfile_bytes_active
                .fetch_sub(self.bytes, Ordering::Relaxed);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MultipartConfig {
    pub upload_temp_dir: PathBuf,
    pub max_body_bytes: usize,
    pub post_max_bytes: usize,
    pub max_upload_files: usize,
    pub max_upload_file_bytes: usize,
    pub max_multipart_parts: Option<usize>,
    pub max_input_vars: usize,
    pub file_uploads: bool,
    pub throw_limit_errors: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct MultipartStats {
    pub parts_total: u64,
    pub fields_total: u64,
    pub uploads_total: u64,
    pub upload_bytes_accepted: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct ParsedRequestData {
    pub(crate) post: Vec<RuntimeInputPair>,
    pub(crate) files: Vec<RuntimeUploadedFile>,
    pub(crate) uploads: Arc<RequestUploadSet>,
    pub(crate) stats: MultipartStats,
    pub(crate) post_limit_exceeded: bool,
    pub(crate) startup_warnings: Vec<String>,
}

impl ParsedRequestData {
    #[must_use]
    pub(crate) fn empty(metrics: &Arc<ServerMetrics>) -> Self {
        Self {
            post: Vec::new(),
            files: Vec::new(),
            uploads: Arc::new(RequestUploadSet::with_metrics(metrics)),
            stats: MultipartStats::default(),
            post_limit_exceeded: false,
            startup_warnings: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum MultipartError {
    TooLarge,
    Malformed(String),
    Limit(String),
}

impl std::fmt::Display for MultipartError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => formatter.write_str("multipart body exceeded the transport limit"),
            Self::Malformed(message) => write!(formatter, "malformed multipart body: {message}"),
            Self::Limit(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for MultipartError {}

pub(crate) fn validated_multipart_boundary(
    content_type: Option<&str>,
) -> Result<Option<String>, MultipartError> {
    let Some(content_type) = content_type else {
        return Ok(None);
    };
    let Ok(media_type) = content_type.parse::<mime::Mime>() else {
        return Ok(None);
    };
    if media_type.type_() != mime::MULTIPART || media_type.subtype() != mime::FORM_DATA {
        return Ok(None);
    }
    multer::parse_boundary(content_type)
        .map(Some)
        .map_err(|error| MultipartError::Malformed(error.to_string()))
}

pub(crate) async fn parse_multipart_stream(
    body: RequestBody,
    content_type: &str,
    config: &MultipartConfig,
    metrics: &Arc<ServerMetrics>,
) -> Result<ParsedRequestData, MultipartError> {
    let boundary = multer::parse_boundary(content_type)
        .map_err(|error| MultipartError::Malformed(error.to_string()))?;
    metrics
        .multipart_requests_total
        .fetch_add(1, Ordering::Relaxed);

    let counted_bytes = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let counted = Arc::clone(&counted_bytes);
    let stream = body.into_data_stream().map(move |result| {
        result.inspect(|bytes| {
            counted.fetch_add(bytes.len() as u64, Ordering::Relaxed);
        })
    });
    let absolute_limit = config.max_body_bytes as u64;
    let constraints = Constraints::new().size_limit(
        SizeLimit::new()
            .whole_stream(absolute_limit)
            .per_field(absolute_limit),
    );
    let mut multipart = Multipart::with_constraints(stream, boundary, constraints);
    let mut post = Vec::new();
    let mut files = Vec::new();
    let mut uploads = RequestUploadSet::with_metrics(metrics);
    let mut stats = MultipartStats::default();
    let mut max_file_size_form = None;
    let mut input_vars = 0usize;
    let mut file_parts = 0usize;
    let mut startup_warnings = Vec::new();
    let max_multipart_parts = config.max_multipart_parts.unwrap_or_else(|| {
        config
            .max_input_vars
            .saturating_add(config.max_upload_files)
    });

    loop {
        let next = multipart.next_field().await;
        if counted_bytes.load(Ordering::Relaxed) > absolute_limit {
            return Err(MultipartError::TooLarge);
        }
        let Some(mut field) = next.map_err(|error| MultipartError::Malformed(error.to_string()))?
        else {
            break;
        };
        stats.parts_total = stats.parts_total.saturating_add(1);
        metrics
            .multipart_parts_total
            .fetch_add(1, Ordering::Relaxed);
        if stats.parts_total as usize > max_multipart_parts {
            let message = format!(
                "Multipart body parts limit exceeded {max_multipart_parts}. To increase the limit change max_multipart_body_parts in php.ini."
            );
            if config.throw_limit_errors {
                return Err(MultipartError::Limit(message));
            }
            if !startup_warnings.iter().any(|warning| warning == &message) {
                startup_warnings.push(message);
            }
            drain_field(&mut field).await?;
            continue;
        }

        let field_name = field.name().unwrap_or("").to_string();
        // PHP 8.5.7 uses the classic `filename=` value and ignores RFC 5987
        // `filename*=` for upload metadata. Multer supplies that value without
        // a second server-owned Content-Disposition parser.
        let raw_file_name = field.file_name().map(ToOwned::to_owned);
        if let Some(full_path) = raw_file_name.map(|name| sanitize_full_path(&name)) {
            file_parts = file_parts.saturating_add(1);
            if full_path.is_empty() {
                files.push(upload_error_entry(
                    field_name,
                    String::new(),
                    String::new(),
                    field
                        .content_type()
                        .map_or_else(String::new, ToString::to_string),
                    UPLOAD_ERR_NO_FILE,
                ));
                drain_field(&mut field).await?;
                continue;
            }
            if !config.file_uploads {
                drain_field(&mut field).await?;
                continue;
            }
            if file_parts > config.max_upload_files {
                let message =
                    "Maximum number of allowable file uploads has been exceeded".to_string();
                if config.throw_limit_errors {
                    return Err(MultipartError::Limit(message));
                }
                if !startup_warnings.iter().any(|warning| warning == &message) {
                    startup_warnings.push(message);
                }
                metrics
                    .upload_limit_errors_total
                    .fetch_add(1, Ordering::Relaxed);
                metrics
                    .upload_files_rejected
                    .fetch_add(1, Ordering::Relaxed);
                drain_field(&mut field).await?;
                continue;
            }
            let client_filename = sanitize_client_filename(&full_path);
            let content_type = field
                .content_type()
                .map_or_else(String::new, ToString::to_string);
            let named = match tempfile::Builder::new()
                .prefix("phrust-upload-")
                .tempfile_in(&config.upload_temp_dir)
            {
                Ok(named) => named,
                Err(error) => {
                    metrics
                        .upload_tempfile_failures_total
                        .fetch_add(1, Ordering::Relaxed);
                    files.push(upload_error_entry(
                        field_name,
                        client_filename,
                        full_path,
                        content_type,
                        if error.kind() == std::io::ErrorKind::NotFound {
                            UPLOAD_ERR_NO_TMP_DIR
                        } else {
                            UPLOAD_ERR_CANT_WRITE
                        },
                    ));
                    drain_field(&mut field).await?;
                    continue;
                }
            };
            let temp_path_text = named.path().to_string_lossy().into_owned();
            let (file, temp_path) = named.into_parts();
            let mut file = tokio::fs::File::from_std(file);
            let mut active_tempfile = UploadTempfileGauge::new(metrics);
            let effective_file_limit = max_file_size_form
                .map_or(config.max_upload_file_bytes, |limit: usize| {
                    limit.min(config.max_upload_file_bytes)
                });
            let mut size = 0usize;
            let mut limit_error = None;
            loop {
                let chunk = match field.chunk().await {
                    Ok(Some(chunk)) => chunk,
                    Ok(None) => break,
                    Err(_) => {
                        if counted_bytes.load(Ordering::Relaxed) > absolute_limit {
                            return Err(MultipartError::TooLarge);
                        }
                        limit_error = Some(UPLOAD_ERR_PARTIAL);
                        break;
                    }
                };
                if counted_bytes.load(Ordering::Relaxed) > absolute_limit {
                    return Err(MultipartError::TooLarge);
                }
                let new_size = size.saturating_add(chunk.len());
                if limit_error.is_none() && new_size > effective_file_limit {
                    limit_error = Some(
                        if max_file_size_form
                            .is_some_and(|form_limit| form_limit < config.max_upload_file_bytes)
                        {
                            UPLOAD_ERR_FORM_SIZE
                        } else {
                            UPLOAD_ERR_INI_SIZE
                        },
                    );
                }
                if limit_error.is_none() {
                    if file.write_all(&chunk).await.is_err() {
                        limit_error = Some(UPLOAD_ERR_CANT_WRITE);
                        metrics
                            .upload_tempfile_failures_total
                            .fetch_add(1, Ordering::Relaxed);
                    } else {
                        size = new_size;
                        active_tempfile.wrote(chunk.len() as u64);
                    }
                }
            }
            if let Some(error) = limit_error {
                metrics
                    .upload_limit_errors_total
                    .fetch_add(1, Ordering::Relaxed);
                drop(file);
                drop(temp_path);
                files.push(upload_error_entry(
                    field_name,
                    client_filename,
                    full_path,
                    content_type,
                    error,
                ));
                continue;
            }
            if file.flush().await.is_err() {
                metrics
                    .upload_tempfile_failures_total
                    .fetch_add(1, Ordering::Relaxed);
                drop(file);
                drop(temp_path);
                files.push(upload_error_entry(
                    field_name,
                    client_filename,
                    full_path,
                    content_type,
                    UPLOAD_ERR_CANT_WRITE,
                ));
                continue;
            }
            drop(file);
            uploads.add_bytes(size as u64);
            uploads.push(temp_path);
            files.push(RuntimeUploadedFile {
                field_name,
                client_filename,
                full_path,
                content_type,
                temp_path: temp_path_text,
                error: UPLOAD_ERR_OK,
                size: size as u64,
            });
            stats.uploads_total = stats.uploads_total.saturating_add(1);
            stats.upload_bytes_accepted = stats.upload_bytes_accepted.saturating_add(size as u64);
            continue;
        }

        input_vars = input_vars.saturating_add(1);
        stats.fields_total = stats.fields_total.saturating_add(1);
        metrics
            .multipart_fields_total
            .fetch_add(1, Ordering::Relaxed);
        if input_vars > config.max_input_vars {
            let message = format!(
                "Input variables exceeded {}. To increase the limit change max_input_vars in php.ini.",
                config.max_input_vars
            );
            if config.throw_limit_errors {
                return Err(MultipartError::Limit(message));
            }
            if !startup_warnings.iter().any(|warning| warning == &message) {
                startup_warnings.push(message);
            }
            drain_field(&mut field).await?;
            continue;
        }
        let mut value = Vec::new();
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|error| MultipartError::Malformed(error.to_string()))?
        {
            if value.len().saturating_add(chunk.len()) > config.post_max_bytes {
                drain_field(&mut field).await?;
                break;
            }
            value.extend_from_slice(&chunk);
        }
        if field_name == "MAX_FILE_SIZE" {
            max_file_size_form = std::str::from_utf8(&value)
                .ok()
                .and_then(|value| value.parse::<usize>().ok());
        }
        post.push(RuntimeInputPair::new(field_name.into_bytes(), value));
    }

    let total_bytes = counted_bytes.load(Ordering::Relaxed);
    let post_limit_exceeded = total_bytes > config.post_max_bytes as u64;
    if post_limit_exceeded && config.throw_limit_errors {
        return Err(MultipartError::Limit(format!(
            "POST Content-Length of {total_bytes} bytes exceeds the limit of {} bytes",
            config.post_max_bytes
        )));
    }
    if post_limit_exceeded {
        post.clear();
        files.clear();
        uploads = RequestUploadSet::with_metrics(metrics);
    }
    metrics
        .uploads_total
        .fetch_add(stats.uploads_total, Ordering::Relaxed);
    metrics
        .upload_bytes_accepted
        .fetch_add(stats.upload_bytes_accepted, Ordering::Relaxed);
    if post_limit_exceeded {
        startup_warnings.push(format!(
            "PHP Request Startup: POST data of {total_bytes} bytes exceeds the limit of {} bytes",
            config.post_max_bytes
        ));
    }
    Ok(ParsedRequestData {
        post,
        files,
        uploads: Arc::new(uploads),
        stats,
        post_limit_exceeded,
        startup_warnings,
    })
}

async fn drain_field(field: &mut multer::Field<'_>) -> Result<(), MultipartError> {
    while field
        .chunk()
        .await
        .map_err(|error| MultipartError::Malformed(error.to_string()))?
        .is_some()
    {}
    Ok(())
}

fn upload_error_entry(
    field_name: String,
    client_filename: String,
    full_path: String,
    content_type: String,
    error: i64,
) -> RuntimeUploadedFile {
    RuntimeUploadedFile {
        field_name,
        client_filename,
        full_path,
        content_type,
        temp_path: String::new(),
        error,
        size: 0,
    }
}

pub(crate) fn sanitize_client_filename(value: &str) -> String {
    value
        .chars()
        .filter(|character| *character != '\0' && !character.is_control())
        .collect::<String>()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_string()
}

fn sanitize_full_path(value: &str) -> String {
    value
        .chars()
        .filter(|character| *character != '\0' && !character.is_control())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use futures_util::stream;
    use http_body_util::{BodyExt, Full, StreamBody};
    use hyper::body::Frame;
    use std::path::Path;

    fn request_body(bytes: &'static [u8]) -> RequestBody {
        Full::new(Bytes::from_static(bytes))
            .map_err(|never| match never {})
            .boxed()
    }

    fn chunked_request_body(bytes: &'static [u8], chunk_size: usize) -> RequestBody {
        let frames = bytes
            .chunks(chunk_size)
            .map(|chunk| Ok::<_, std::io::Error>(Frame::data(Bytes::copy_from_slice(chunk))))
            .collect::<Vec<_>>();
        BodyExt::boxed(StreamBody::new(stream::iter(frames)))
    }

    fn config(dir: &Path) -> MultipartConfig {
        MultipartConfig {
            upload_temp_dir: dir.to_path_buf(),
            max_body_bytes: 1024 * 1024,
            post_max_bytes: 1024 * 1024,
            max_upload_files: 20,
            max_upload_file_bytes: 1024 * 1024,
            max_multipart_parts: None,
            max_input_vars: 1000,
            file_uploads: true,
            throw_limit_errors: false,
        }
    }

    #[tokio::test]
    async fn streams_fields_and_uploads_into_secure_tempfiles() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let metrics = Arc::new(ServerMetrics::default());
        let body = b"--BOUNDARY\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nHello\r\n--BOUNDARY\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"../me.png\"\r\nContent-Type: image/png\r\n\r\nPNGDATA\r\n--BOUNDARY--\r\n";
        let parsed = parse_multipart_stream(
            request_body(body),
            "multipart/form-data; boundary=BOUNDARY",
            &config(dir.path()),
            &metrics,
        )
        .await
        .expect("parse multipart");
        assert_eq!(
            parsed.post,
            [RuntimeInputPair::new(b"title".to_vec(), b"Hello".to_vec())]
        );
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].client_filename, "me.png");
        assert_eq!(
            std::fs::read(&parsed.files[0].temp_path).unwrap(),
            b"PNGDATA"
        );
        assert_eq!(parsed.uploads.len(), 1);
    }

    #[tokio::test]
    async fn boundary_headers_binary_fields_and_files_survive_single_byte_frames() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let metrics = Arc::new(ServerMetrics::default());
        let body = b"--BOUNDARY\r\nContent-Disposition: form-data; name=\"binary\"\r\n\r\nA\0\xffB\r\n--BOUNDARY\r\nContent-Disposition: form-data; name=\"first\"; filename=\"a.txt\"\r\nContent-Type: one/type\r\n\r\nONE\r\n--BOUNDARY\r\nContent-Disposition: form-data; name=\"second\"; filename=\"nested/b.txt\"\r\nContent-Type: two/type\r\n\r\nTWO\r\n--BOUNDARY--\r\n";
        let parsed = parse_multipart_stream(
            chunked_request_body(body, 1),
            "multipart/form-data; boundary=BOUNDARY",
            &config(dir.path()),
            &metrics,
        )
        .await
        .expect("parse single-byte-frame multipart");

        assert_eq!(parsed.post[0].value, b"A\0\xffB");
        assert_eq!(parsed.files.len(), 2);
        assert_eq!(parsed.files[0].client_filename, "a.txt");
        assert_eq!(parsed.files[0].content_type, "one/type");
        assert_eq!(std::fs::read(&parsed.files[0].temp_path).unwrap(), b"ONE");
        assert_eq!(parsed.files[1].client_filename, "b.txt");
        assert_eq!(parsed.files[1].full_path, "nested/b.txt");
        assert_eq!(parsed.files[1].content_type, "two/type");
        assert_eq!(std::fs::read(&parsed.files[1].temp_path).unwrap(), b"TWO");
    }

    #[tokio::test]
    async fn ini_file_limit_and_file_count_limit_drain_and_keep_later_parts() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let metrics = Arc::new(ServerMetrics::default());
        let body = b"--B\r\nContent-Disposition: form-data; name=\"large\"; filename=\"large.bin\"\r\n\r\nTOO-LARGE\r\n--B\r\nContent-Disposition: form-data; name=\"second\"; filename=\"second.bin\"\r\n\r\nSECOND\r\n--B\r\nContent-Disposition: form-data; name=\"after\"\r\n\r\nyes\r\n--B--\r\n";
        let mut limits = config(dir.path());
        limits.max_upload_file_bytes = 4;
        limits.max_upload_files = 1;
        let parsed = parse_multipart_stream(
            chunked_request_body(body, 3),
            "multipart/form-data; boundary=B",
            &limits,
            &metrics,
        )
        .await
        .expect("parse limited multipart");

        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].error, UPLOAD_ERR_INI_SIZE);
        assert!(parsed.files[0].temp_path.is_empty());
        assert!(
            parsed
                .post
                .iter()
                .any(|pair| pair.name == b"after" && pair.value == b"yes")
        );
        assert!(
            parsed
                .startup_warnings
                .iter()
                .any(|warning| warning
                    == "Maximum number of allowable file uploads has been exceeded")
        );
        assert_eq!(
            std::fs::read_dir(dir.path()).expect("read uploads").count(),
            0
        );
    }

    #[tokio::test]
    async fn missing_upload_directory_reports_no_tmp_dir_and_continues() {
        let parent = tempfile::tempdir().expect("temporary directory");
        let missing = parent.path().join("missing");
        let metrics = Arc::new(ServerMetrics::default());
        let body = b"--B\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"a.txt\"\r\n\r\nFILE\r\n--B\r\nContent-Disposition: form-data; name=\"after\"\r\n\r\nyes\r\n--B--\r\n";
        let parsed = parse_multipart_stream(
            request_body(body),
            "multipart/form-data; boundary=B",
            &config(&missing),
            &metrics,
        )
        .await
        .expect("parse missing-tempdir multipart");

        assert_eq!(parsed.files[0].error, UPLOAD_ERR_NO_TMP_DIR);
        assert!(
            parsed
                .post
                .iter()
                .any(|pair| pair.name == b"after" && pair.value == b"yes")
        );
    }

    #[tokio::test]
    async fn default_part_limit_is_input_vars_plus_file_uploads() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let metrics = Arc::new(ServerMetrics::default());
        let body = b"--B\r\nContent-Disposition: form-data; name=\"one\"\r\n\r\n1\r\n--B\r\nContent-Disposition: form-data; name=\"two\"\r\n\r\n2\r\n--B\r\nContent-Disposition: form-data; name=\"three\"\r\n\r\n3\r\n--B--\r\n";
        let mut limits = config(dir.path());
        limits.max_input_vars = 1;
        limits.max_upload_files = 1;
        limits.max_multipart_parts = None;

        let parsed = parse_multipart_stream(
            request_body(body),
            "multipart/form-data; boundary=B",
            &limits,
            &metrics,
        )
        .await
        .expect("parse multipart");

        assert_eq!(parsed.stats.parts_total, 3);
        assert_eq!(parsed.post.len(), 1);
        assert_eq!(parsed.post[0].name, b"one");
        assert!(
            parsed
                .startup_warnings
                .iter()
                .any(|warning| warning.starts_with("Input variables exceeded 1."))
        );
        assert!(
            parsed
                .startup_warnings
                .iter()
                .any(|warning| warning.starts_with("Multipart body parts limit exceeded 2."))
        );
    }

    #[tokio::test]
    async fn request_parse_mode_turns_multipart_limits_into_errors() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let metrics = Arc::new(ServerMetrics::default());
        let body = b"--B\r\nContent-Disposition: form-data; name=\"one\"\r\n\r\n1\r\n--B\r\nContent-Disposition: form-data; name=\"two\"\r\n\r\n2\r\n--B--\r\n";
        let mut limits = config(dir.path());
        limits.max_input_vars = 1;
        limits.throw_limit_errors = true;

        let error = parse_multipart_stream(
            request_body(body),
            "multipart/form-data; boundary=B",
            &limits,
            &metrics,
        )
        .await
        .expect_err("request parser limit error");

        assert!(matches!(error, MultipartError::Limit(_)));
        assert!(error.to_string().starts_with("Input variables exceeded 1."));
    }

    #[tokio::test]
    async fn upload_limit_errors_drain_the_file_and_preserve_later_fields() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let metrics = Arc::new(ServerMetrics::default());
        let body = b"--B\r\nContent-Disposition: form-data; name=\"MAX_FILE_SIZE\"\r\n\r\n4\r\n--B\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"me.png\"\r\nContent-Type: image/png\r\n\r\nPNGDATA\r\n--B\r\nContent-Disposition: form-data; name=\"after\"\r\n\r\nyes\r\n--B--\r\n";
        let parsed = parse_multipart_stream(
            request_body(body),
            "multipart/form-data; boundary=B",
            &config(dir.path()),
            &metrics,
        )
        .await
        .expect("parse multipart");

        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].error, UPLOAD_ERR_FORM_SIZE);
        assert_eq!(parsed.files[0].size, 0);
        assert!(parsed.files[0].temp_path.is_empty());
        assert!(
            parsed
                .post
                .iter()
                .any(|pair| pair.name == b"after" && pair.value == b"yes")
        );
        assert_eq!(
            std::fs::read_dir(dir.path()).expect("read uploads").count(),
            0
        );
    }

    #[tokio::test]
    async fn empty_upload_field_reports_no_file_without_a_tempfile() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let metrics = Arc::new(ServerMetrics::default());
        let body = b"--B\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"\"\r\nContent-Type: application/octet-stream\r\n\r\n\r\n--B--\r\n";
        let parsed = parse_multipart_stream(
            request_body(body),
            "multipart/form-data; boundary=B",
            &config(dir.path()),
            &metrics,
        )
        .await
        .expect("parse multipart");

        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].error, UPLOAD_ERR_NO_FILE);
        assert_eq!(
            std::fs::read_dir(dir.path()).expect("read uploads").count(),
            0
        );
    }

    #[test]
    fn filename_sanitization_never_affects_temp_paths() {
        assert_eq!(sanitize_client_filename("../avatar.png"), "avatar.png");
        assert_eq!(
            sanitize_client_filename("C:\\tmp\\avatar\0.png"),
            "avatar.png"
        );
    }

    #[tokio::test]
    async fn filename_star_is_ignored_like_php_8_5() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let metrics = Arc::new(ServerMetrics::default());
        let body = b"--B\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"dir/fallback.txt\"; filename*=UTF-8''caf%C3%A9.txt\r\nContent-Type: text/plain\r\n\r\nx\r\n--B--\r\n";
        let parsed = parse_multipart_stream(
            request_body(body),
            "multipart/form-data; boundary=B",
            &config(dir.path()),
            &metrics,
        )
        .await
        .expect("parse filename-star multipart");

        assert_eq!(parsed.files[0].client_filename, "fallback.txt");
        assert_eq!(parsed.files[0].full_path, "dir/fallback.txt");
    }
}
