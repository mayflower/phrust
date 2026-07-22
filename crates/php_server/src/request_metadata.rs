use std::{net::SocketAddr, str::FromStr, sync::atomic::Ordering};

use hyper::{
    Method, StatusCode, Version,
    header::{self, HeaderMap, HeaderName},
    http::{request::Parts, uri::Authority},
};

use crate::state::AppState;

const MAX_REQUEST_HEADERS: usize = 100;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HttpProtocol {
    Http10,
    Http11,
    Http2,
    Http3,
}

impl HttpProtocol {
    pub(crate) fn from_version(version: Version) -> Option<Self> {
        match version {
            Version::HTTP_10 => Some(Self::Http10),
            Version::HTTP_11 => Some(Self::Http11),
            Version::HTTP_2 => Some(Self::Http2),
            Version::HTTP_3 => Some(Self::Http3),
            _ => None,
        }
    }

    pub(crate) fn php_name(self) -> &'static str {
        match self {
            Self::Http10 => "HTTP/1.0",
            Self::Http11 => "HTTP/1.1",
            Self::Http2 => "HTTP/2.0",
            Self::Http3 => "HTTP/3.0",
        }
    }

    pub(crate) fn is_h1(self) -> bool {
        matches!(self, Self::Http10 | Self::Http11)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RequestMetadata {
    pub(crate) protocol: HttpProtocol,
    pub(crate) scheme: &'static str,
    pub(crate) authority: String,
    pub(crate) host_for_php: String,
    pub(crate) server_name: String,
    pub(crate) server_port: u16,
    pub(crate) remote_addr: SocketAddr,
    pub(crate) local_addr: SocketAddr,
    pub(crate) request_target: String,
    pub(crate) force_connection_close: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RequestLocalAddr(pub(crate) SocketAddr);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeclaredContentLength(pub(crate) Option<u64>);

#[derive(Clone, Copy, Debug)]
pub(crate) struct RequestValidationError {
    pub(crate) status: StatusCode,
    pub(crate) force_connection_close: bool,
}

#[derive(Clone, Debug)]
struct NormalizedAuthority {
    original: String,
    host: String,
    effective_port: u16,
}

pub(crate) fn validate_request(
    parts: &mut Parts,
    state: &AppState,
    remote_addr: SocketAddr,
) -> Result<RequestMetadata, RequestValidationError> {
    let Some(protocol) = HttpProtocol::from_version(parts.version) else {
        return Err(bad_request(false));
    };
    let request_target = request_target(parts);
    if request_target.len() > state.transport.limits.max_request_target_bytes {
        state
            .services
            .metrics
            .request_target_rejections_total
            .fetch_add(1, Ordering::Relaxed);
        return Err(RequestValidationError {
            status: StatusCode::URI_TOO_LONG,
            force_connection_close: protocol.is_h1(),
        });
    }
    if parts.method == Method::CONNECT {
        return Err(bad_request(protocol.is_h1()));
    }
    validate_request_target(parts, protocol, state)?;
    if parts.headers.len() > MAX_REQUEST_HEADERS {
        state
            .services
            .metrics
            .request_header_count_rejections_total
            .fetch_add(1, Ordering::Relaxed);
        return Err(header_too_large(protocol));
    }
    let header_bytes = parts.headers.iter().fold(0usize, |total, (name, value)| {
        total.saturating_add(name.as_str().len() + value.as_bytes().len() + 32)
    });
    if header_bytes > state.transport.limits.max_request_header_bytes {
        state
            .services
            .metrics
            .request_header_bytes_rejections_total
            .fetch_add(1, Ordering::Relaxed);
        return Err(header_too_large(protocol));
    }
    let declared_content_length = validate_framing(parts, protocol, state)?;
    parts
        .extensions
        .insert(DeclaredContentLength(declared_content_length));

    let local_addr = parts
        .extensions
        .get::<RequestLocalAddr>()
        .map_or(state.transport.local_addr, |address| address.0);
    let scheme = state.transport.request_scheme;
    let uri_authority = parts.uri.authority().map(Authority::as_str);
    let host_values = header_values(&parts.headers, header::HOST, protocol.is_h1())?;
    let host = match host_values.as_slice() {
        [] => None,
        [value] => Some(parse_authority(value, scheme).map_err(|_| {
            state
                .services
                .metrics
                .malformed_authority_total
                .fetch_add(1, Ordering::Relaxed);
            bad_request(protocol.is_h1())
        })?),
        _ => {
            state
                .services
                .metrics
                .malformed_authority_total
                .fetch_add(1, Ordering::Relaxed);
            return Err(bad_request(protocol.is_h1()));
        }
    };

    let selected = match protocol {
        HttpProtocol::Http11 => {
            if let Some(uri_authority) = uri_authority {
                if parts
                    .uri
                    .scheme_str()
                    .is_some_and(|value| !value.eq_ignore_ascii_case(scheme))
                {
                    return Err(malformed_authority(state, true));
                }
                let uri = parse_authority(uri_authority, scheme)
                    .map_err(|_| malformed_authority(state, true))?;
                if let Some(host) = &host
                    && !authorities_match(&uri, host)
                {
                    return Err(authority_conflict(state, true));
                }
                uri
            } else {
                host.ok_or_else(|| malformed_authority(state, true))?
            }
        }
        HttpProtocol::Http10 => {
            if let Some(uri_authority) = uri_authority {
                if parts
                    .uri
                    .scheme_str()
                    .is_some_and(|value| !value.eq_ignore_ascii_case(scheme))
                {
                    return Err(malformed_authority(state, true));
                }
                let uri = parse_authority(uri_authority, scheme)
                    .map_err(|_| malformed_authority(state, true))?;
                if let Some(host) = &host
                    && !authorities_match(&uri, host)
                {
                    return Err(authority_conflict(state, true));
                }
                uri
            } else if let Some(host) = host {
                host
            } else {
                parse_authority(&local_addr.to_string(), scheme)
                    .expect("local socket address is a valid authority")
            }
        }
        HttpProtocol::Http2 | HttpProtocol::Http3 => {
            if parts
                .uri
                .scheme_str()
                .is_some_and(|value| !value.eq_ignore_ascii_case(scheme))
            {
                return Err(malformed_authority(state, false));
            }
            let uri = uri_authority
                .ok_or_else(|| malformed_authority(state, false))
                .and_then(|value| {
                    parse_authority(value, scheme).map_err(|_| malformed_authority(state, false))
                })?;
            if let Some(host) = &host
                && !authorities_match(&uri, host)
            {
                return Err(authority_conflict(state, false));
            }
            uri
        }
    };

    Ok(RequestMetadata {
        protocol,
        scheme,
        authority: selected.original.clone(),
        host_for_php: selected.original,
        server_name: selected.host,
        server_port: local_addr.port(),
        remote_addr,
        local_addr,
        request_target,
        force_connection_close: false,
    })
}

fn request_target(parts: &Parts) -> String {
    if parts.uri.path() == "*" {
        return "*".to_string();
    }
    if matches!(parts.version, Version::HTTP_10 | Version::HTTP_11)
        && (parts.uri.scheme().is_some() || parts.uri.authority().is_some())
    {
        parts.uri.to_string()
    } else {
        parts
            .uri
            .path_and_query()
            .map_or_else(|| parts.uri.path().to_string(), ToString::to_string)
    }
}

fn validate_request_target(
    parts: &Parts,
    protocol: HttpProtocol,
    state: &AppState,
) -> Result<(), RequestValidationError> {
    let asterisk = parts.uri.path() == "*";
    if asterisk {
        if parts.method != Method::OPTIONS || parts.uri.query().is_some() {
            return Err(malformed_target(state, protocol));
        }
        return Ok(());
    }

    if !protocol.is_h1() {
        return Ok(());
    }

    let has_scheme = parts.uri.scheme().is_some();
    let has_authority = parts.uri.authority().is_some();
    if has_scheme != has_authority
        || (!has_scheme && !parts.uri.path().starts_with('/'))
        || parts.uri.path().is_empty()
    {
        return Err(malformed_target(state, protocol));
    }
    Ok(())
}

fn validate_framing(
    parts: &mut Parts,
    protocol: HttpProtocol,
    state: &AppState,
) -> Result<Option<u64>, RequestValidationError> {
    if protocol.is_h1() {
        if parts.headers.contains_key("proxy-connection") {
            return Err(hop_by_hop_rejection(state, true));
        }
        if parts.headers.contains_key(header::CONTENT_LENGTH)
            && parts.headers.contains_key(header::TRANSFER_ENCODING)
        {
            return Err(framing_rejection(state, true, StatusCode::BAD_REQUEST));
        }
        let transfer_codings = header_tokens(&parts.headers, header::TRANSFER_ENCODING)
            .map_err(|_| framing_rejection(state, true, StatusCode::BAD_REQUEST))?;
        if !transfer_codings.is_empty() {
            if transfer_codings
                .last()
                .is_none_or(|coding| coding != "chunked")
            {
                return Err(framing_rejection(state, true, StatusCode::NOT_IMPLEMENTED));
            }
            if transfer_codings.len() != 1 {
                return Err(framing_rejection(state, true, StatusCode::NOT_IMPLEMENTED));
            }
        }
        let nominated = connection_tokens(&parts.headers)
            .map_err(|_| framing_rejection(state, true, StatusCode::BAD_REQUEST))?;
        for name in nominated {
            parts.headers.remove(name);
        }
    } else {
        for name in [
            header::CONNECTION,
            header::TRANSFER_ENCODING,
            header::UPGRADE,
            HeaderName::from_static("keep-alive"),
            HeaderName::from_static("proxy-connection"),
        ] {
            if parts.headers.contains_key(&name) {
                return Err(hop_by_hop_rejection(state, false));
            }
        }
        if parts.headers.contains_key(header::TE) {
            let te = header_tokens(&parts.headers, header::TE)
                .map_err(|_| hop_by_hop_rejection(state, false))?;
            if te.as_slice() != ["trailers"] {
                return Err(hop_by_hop_rejection(state, false));
            }
        }
    }

    match validated_content_length(&parts.headers, state.request.max_body_bytes) {
        Ok(length) => Ok(length),
        Err(ContentLengthError::Invalid) => Err(framing_rejection(
            state,
            protocol.is_h1(),
            StatusCode::BAD_REQUEST,
        )),
        Err(ContentLengthError::TooLarge) => {
            state
                .services
                .metrics
                .body_too_large
                .fetch_add(1, Ordering::Relaxed);
            state
                .services
                .metrics
                .request_body_hard_limit_rejections_total
                .fetch_add(1, Ordering::Relaxed);
            Err(RequestValidationError {
                status: StatusCode::PAYLOAD_TOO_LARGE,
                force_connection_close: protocol.is_h1(),
            })
        }
    }
}

fn header_tokens(headers: &HeaderMap, name: HeaderName) -> Result<Vec<String>, ()> {
    let mut tokens = Vec::new();
    for value in headers.get_all(name) {
        let value = value.to_str().map_err(|_| ())?;
        for token in value.split(',') {
            let token = token.trim();
            if token.is_empty() {
                return Err(());
            }
            tokens.push(token.to_ascii_lowercase());
        }
    }
    Ok(tokens)
}

fn connection_tokens(headers: &HeaderMap) -> Result<Vec<HeaderName>, ()> {
    let mut tokens = Vec::new();
    for value in headers.get_all(header::CONNECTION) {
        let value = value.to_str().map_err(|_| ())?;
        for token in value.split(',') {
            let token = token.trim();
            if token.is_empty() {
                return Err(());
            }
            tokens.push(HeaderName::from_str(token).map_err(|_| ())?);
        }
    }
    Ok(tokens)
}

fn header_values(
    headers: &HeaderMap,
    name: HeaderName,
    close: bool,
) -> Result<Vec<String>, RequestValidationError> {
    headers
        .get_all(name)
        .iter()
        .map(|value| {
            value
                .to_str()
                .map(str::to_string)
                .map_err(|_| bad_request(close))
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContentLengthError {
    Invalid,
    TooLarge,
}

fn validated_content_length(
    headers: &HeaderMap,
    max_body_bytes: usize,
) -> Result<Option<u64>, ContentLengthError> {
    let mut parsed = None;
    for value in headers.get_all(header::CONTENT_LENGTH) {
        let value = value.to_str().map_err(|_| ContentLengthError::Invalid)?;
        for item in value.split(',') {
            let item = item.trim();
            if item.is_empty() || !item.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(ContentLengthError::Invalid);
            }
            let length = item
                .parse::<u64>()
                .map_err(|_| ContentLengthError::Invalid)?;
            if parsed.is_some_and(|previous| previous != length) {
                return Err(ContentLengthError::Invalid);
            }
            parsed = Some(length);
        }
    }
    if parsed.is_some_and(|length| length > max_body_bytes as u64) {
        Err(ContentLengthError::TooLarge)
    } else {
        Ok(parsed)
    }
}

fn parse_authority(value: &str, scheme: &str) -> Result<NormalizedAuthority, ()> {
    if value.is_empty()
        || value.contains('@')
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
    {
        return Err(());
    }
    let authority = Authority::from_str(value).map_err(|_| ())?;
    let explicit_port = authority.port_u16();
    if explicit_port == Some(0) {
        return Err(());
    }
    let effective_port = explicit_port.unwrap_or_else(|| default_port(scheme));
    let host = authority
        .host()
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or_else(|| authority.host())
        .to_ascii_lowercase();
    if host.is_empty() {
        return Err(());
    }
    Ok(NormalizedAuthority {
        original: authority.as_str().to_string(),
        host,
        effective_port,
    })
}

fn default_port(scheme: &str) -> u16 {
    if scheme.eq_ignore_ascii_case("https") {
        443
    } else {
        80
    }
}

fn authorities_match(left: &NormalizedAuthority, right: &NormalizedAuthority) -> bool {
    left.host.eq_ignore_ascii_case(&right.host) && left.effective_port == right.effective_port
}

fn malformed_authority(state: &AppState, close: bool) -> RequestValidationError {
    state
        .services
        .metrics
        .malformed_authority_total
        .fetch_add(1, Ordering::Relaxed);
    bad_request(close)
}

fn authority_conflict(state: &AppState, close: bool) -> RequestValidationError {
    state
        .services
        .metrics
        .host_authority_conflicts_total
        .fetch_add(1, Ordering::Relaxed);
    bad_request(close)
}

fn malformed_target(state: &AppState, protocol: HttpProtocol) -> RequestValidationError {
    state
        .services
        .metrics
        .request_target_rejections_total
        .fetch_add(1, Ordering::Relaxed);
    bad_request(protocol.is_h1())
}

fn framing_rejection(state: &AppState, close: bool, status: StatusCode) -> RequestValidationError {
    state
        .services
        .metrics
        .request_framing_rejections_total
        .fetch_add(1, Ordering::Relaxed);
    RequestValidationError {
        status,
        force_connection_close: close,
    }
}

fn hop_by_hop_rejection(state: &AppState, close: bool) -> RequestValidationError {
    state
        .services
        .metrics
        .request_hop_by_hop_rejections_total
        .fetch_add(1, Ordering::Relaxed);
    bad_request(close)
}

fn bad_request(close: bool) -> RequestValidationError {
    RequestValidationError {
        status: StatusCode::BAD_REQUEST,
        force_connection_close: close,
    }
}

fn header_too_large(protocol: HttpProtocol) -> RequestValidationError {
    RequestValidationError {
        status: StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
        force_connection_close: protocol.is_h1(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_normalization_handles_defaults_case_and_ipv6() {
        let left = parse_authority("Example.COM", "https").unwrap();
        let right = parse_authority("example.com:443", "https").unwrap();
        assert!(authorities_match(&left, &right));

        let ipv6 = parse_authority("[::1]:8443", "https").unwrap();
        assert_eq!(ipv6.host, "::1");
        assert_eq!(ipv6.effective_port, 8443);
    }

    #[test]
    fn authority_rejects_userinfo_whitespace_and_zero_port() {
        for authority in ["user@example.test", "example .test", "example.test:0"] {
            assert!(parse_authority(authority, "http").is_err(), "{authority}");
        }
    }

    #[test]
    fn content_length_rejects_invalid_conflicting_and_over_limit_values() {
        let mut headers = HeaderMap::new();
        headers.append(header::CONTENT_LENGTH, "12".parse().unwrap());
        headers.append(header::CONTENT_LENGTH, "12".parse().unwrap());
        assert_eq!(validated_content_length(&headers, 32), Ok(Some(12)));

        headers.append(header::CONTENT_LENGTH, "13".parse().unwrap());
        assert_eq!(
            validated_content_length(&headers, 32),
            Err(ContentLengthError::Invalid)
        );

        let mut oversized = HeaderMap::new();
        oversized.insert(header::CONTENT_LENGTH, "33554433".parse().unwrap());
        assert_eq!(
            validated_content_length(&oversized, 32 * 1024 * 1024),
            Err(ContentLengthError::TooLarge)
        );
    }
}
