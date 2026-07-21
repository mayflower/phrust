use crate::static_files::{OpenedStaticRepresentation, StaticFileService, StaticResolution};
use hyper::{Method, Uri, http::HeaderMap};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteConfig {
    pub docroot: PathBuf,
    pub front_controller: Option<PathBuf>,
    pub builtin_router: Option<PathBuf>,
    pub request_rewrites: Vec<RequestRewriteRule>,
    pub metrics_endpoint_enabled: bool,
    pub cache_clear_endpoint_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestRewriteRule {
    pub path_prefix: String,
    pub query_parameter: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NormalizedRequestPath {
    uri_path: String,
    segments: Vec<String>,
    relative: PathBuf,
    relative_string: String,
    trailing_slash: bool,
}

impl NormalizedRequestPath {
    pub(crate) fn parse(path: &str) -> Result<Self, ()> {
        if !path.starts_with('/') || path.starts_with("//") {
            return Err(());
        }
        let trailing_slash = path.len() > 1 && path.ends_with('/');
        let mut segments = Vec::new();
        for encoded in path[1..].split('/') {
            if encoded.is_empty() {
                continue;
            }
            let decoded = percent_decode_segment(encoded.as_bytes())?;
            if decoded
                .iter()
                .any(|byte| *byte <= 0x1f || *byte == 0x7f || matches!(*byte, b'/' | b'\\'))
            {
                return Err(());
            }
            let decoded = String::from_utf8(decoded).map_err(|_| ())?;
            if matches!(decoded.as_str(), "." | "..") || decoded.contains('\\') {
                return Err(());
            }
            if segments.is_empty()
                && decoded.as_bytes().get(1) == Some(&b':')
                && decoded.as_bytes()[0].is_ascii_alphabetic()
            {
                return Err(());
            }
            segments.push(decoded);
        }
        let relative_string = segments.join("/");
        let relative = PathBuf::from(&relative_string);
        Ok(Self {
            uri_path: path.to_owned(),
            segments,
            relative,
            relative_string,
            trailing_slash,
        })
    }

    pub(crate) fn segments(&self) -> &[String] {
        &self.segments
    }
    pub(crate) fn relative_path(&self) -> &Path {
        &self.relative
    }
    pub(crate) fn relative_string(&self) -> &str {
        &self.relative_string
    }
    pub(crate) fn trailing_slash(&self) -> bool {
        self.trailing_slash
    }
    pub(crate) fn uri_path(&self) -> &str {
        &self.uri_path
    }
    pub(crate) fn is_root(&self) -> bool {
        self.segments.is_empty()
    }
}

fn percent_decode_segment(input: &[u8]) -> Result<Vec<u8>, ()> {
    let mut output = Vec::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        if input[index] == b'%' {
            let high = *input.get(index + 1).ok_or(())?;
            let low = *input.get(index + 2).ok_or(())?;
            output.push(hex_value(high).ok_or(())? << 4 | hex_value(low).ok_or(())?);
            index += 3;
        } else {
            output.push(input[index]);
            index += 1;
        }
    }
    Ok(output)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[derive(Debug)]
pub(crate) enum ResolvedRoute {
    Health,
    Metrics,
    CacheClear,
    StaticFile(OpenedStaticRepresentation),
    PhpScript {
        script_path: PathBuf,
        path_info: Option<String>,
    },
    DirectoryRedirect {
        location: String,
    },
    NotAcceptable {
        vary_accept_encoding: bool,
    },
    NotFound,
    BadRequest,
    MethodNotAllowed {
        allow: &'static str,
    },
    InternalError(String),
}

pub(crate) async fn resolve_route(
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    config: &RouteConfig,
    static_files: &Arc<StaticFileService>,
) -> ResolvedRoute {
    let path = uri.path();
    if path == "/healthz" {
        return ResolvedRoute::Health;
    }
    if path == "/__phrust/metrics" {
        if !config.metrics_endpoint_enabled {
            return ResolvedRoute::NotFound;
        }
        return if method == Method::GET {
            ResolvedRoute::Metrics
        } else {
            ResolvedRoute::MethodNotAllowed { allow: "GET" }
        };
    }
    if path == "/__phrust/cache/clear" {
        if !config.cache_clear_endpoint_enabled {
            return ResolvedRoute::NotFound;
        }
        return if method == Method::POST {
            ResolvedRoute::CacheClear
        } else {
            ResolvedRoute::MethodNotAllowed { allow: "POST" }
        };
    }
    let Ok(normalized) = NormalizedRequestPath::parse(path) else {
        return ResolvedRoute::BadRequest;
    };
    let resolution = static_files
        .resolve(
            normalized.clone(),
            headers.clone(),
            config.front_controller.clone(),
        )
        .await;
    match resolution {
        StaticResolution::Static(file) => {
            if matches!(*method, Method::GET | Method::HEAD) {
                ResolvedRoute::StaticFile(file)
            } else {
                ResolvedRoute::MethodNotAllowed { allow: "GET, HEAD" }
            }
        }
        StaticResolution::PhpScript {
            script_path,
            path_info,
        } => {
            if is_php_application_method(method) {
                ResolvedRoute::PhpScript {
                    script_path,
                    path_info,
                }
            } else {
                ResolvedRoute::MethodNotAllowed {
                    allow: "GET, HEAD, POST, OPTIONS, PUT, PATCH, DELETE",
                }
            }
        }
        StaticResolution::Directory => {
            if !matches!(*method, Method::GET | Method::HEAD) {
                return ResolvedRoute::MethodNotAllowed { allow: "GET, HEAD" };
            }
            let mut location = normalized.uri_path().to_owned();
            if !location.ends_with('/') {
                location.push('/');
            }
            if let Some(query) = uri.query() {
                location.push('?');
                location.push_str(query);
            }
            ResolvedRoute::DirectoryRedirect { location }
        }
        StaticResolution::NotAcceptable {
            vary_accept_encoding,
        } => {
            if matches!(*method, Method::GET | Method::HEAD) {
                ResolvedRoute::NotAcceptable {
                    vary_accept_encoding,
                }
            } else {
                ResolvedRoute::MethodNotAllowed { allow: "GET, HEAD" }
            }
        }
        StaticResolution::Missing => ResolvedRoute::NotFound,
        StaticResolution::DirectoryWithoutIndex => ResolvedRoute::NotFound,
        StaticResolution::Hidden(class) => {
            let _denial_class = class;
            ResolvedRoute::NotFound
        }
        StaticResolution::Error(error) => ResolvedRoute::InternalError(error),
    }
}

fn is_php_application_method(method: &Method) -> bool {
    matches!(
        *method,
        Method::GET
            | Method::HEAD
            | Method::POST
            | Method::OPTIONS
            | Method::PUT
            | Method::PATCH
            | Method::DELETE
    )
}

#[cfg(test)]
mod tests {
    use super::NormalizedRequestPath;

    #[test]
    fn normalizes_segments_once_and_preserves_double_encoding() {
        let path = NormalizedRequestPath::parse("/assets/a%20b/%252f.txt/").expect("valid path");
        assert_eq!(path.segments(), &["assets", "a b", "%2f.txt"]);
        assert!(path.trailing_slash());
    }

    #[test]
    fn rejects_traversal_separators_controls_and_prefixes() {
        for path in [
            "/../secret",
            "/%2e%2e/secret",
            "/bad%00name",
            "/bad%1fname",
            "/a%2fb",
            "/a%5cb",
            "/a\\b",
            "/bad%xx",
            "//absolute",
            "/C:/secret",
        ] {
            assert!(NormalizedRequestPath::parse(path).is_err(), "{path}");
        }
    }
}
