use crate::server::ServerError;
use quinn::crypto::rustls::QuicServerConfig;
use rustls_pki_types::pem::PemObject;
use std::{path::Path, sync::Arc, time::Duration};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        ServerConfig as RustlsServerConfig,
        pki_types::{CertificateDer, PrivateKeyDer},
    },
};

pub(crate) fn build_tls_acceptor(
    cert_path: Option<&Path>,
    key_path: Option<&Path>,
) -> Result<Option<TlsAcceptor>, ServerError> {
    let (Some(cert_path), Some(key_path)) = (cert_path, key_path) else {
        return Ok(None);
    };
    let certs = load_tls_certs(cert_path)?;
    let key = load_tls_private_key(key_path)?;
    let mut config = RustlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|error| {
            ServerError::Tls(format!(
                "TLS certificate/key configuration is invalid: {error}"
            ))
        })?;
    config.alpn_protocols = tls_alpn_protocols();
    Ok(Some(TlsAcceptor::from(Arc::new(config))))
}

pub(crate) fn tls_alpn_protocols() -> Vec<Vec<u8>> {
    vec![b"h2".to_vec(), b"http/1.1".to_vec()]
}

pub(crate) fn http3_alpn_protocols() -> Vec<Vec<u8>> {
    vec![b"h3".to_vec()]
}

pub(crate) fn build_quic_server_config(
    cert_path: &Path,
    key_path: &Path,
    max_streams_per_connection: u32,
    connection_idle_timeout: Duration,
) -> Result<quinn::ServerConfig, ServerError> {
    let certs = load_tls_certs(cert_path)?;
    let key = load_tls_private_key(key_path)?;
    let mut crypto = RustlsServerConfig::builder_with_provider(Arc::new(
        tokio_rustls::rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&tokio_rustls::rustls::version::TLS13])
    .map_err(|error| ServerError::Tls(format!("HTTP/3 TLS 1.3 setup failed: {error}")))?
    .with_no_client_auth()
    .with_single_cert(certs, key)
    .map_err(|error| {
        ServerError::Tls(format!(
            "HTTP/3 TLS certificate/key configuration is invalid: {error}"
        ))
    })?;
    crypto.alpn_protocols = http3_alpn_protocols();
    let mut server =
        quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(crypto).map_err(
            |error| ServerError::Tls(format!("HTTP/3 QUIC TLS configuration is invalid: {error}")),
        )?));
    let mut transport = quinn::TransportConfig::default();
    transport
        .max_concurrent_bidi_streams(quinn::VarInt::from_u32(max_streams_per_connection))
        .max_concurrent_uni_streams(quinn::VarInt::from_u32(16))
        .max_idle_timeout(Some(
            quinn::IdleTimeout::try_from(connection_idle_timeout).map_err(|error| {
                ServerError::Tls(format!("HTTP/3 idle timeout is out of range: {error}"))
            })?,
        ))
        .stream_receive_window(quinn::VarInt::from_u32(1024 * 1024))
        .receive_window(quinn::VarInt::from_u32(8 * 1024 * 1024))
        .send_window(8 * 1024 * 1024)
        .keep_alive_interval(None)
        .datagram_receive_buffer_size(None)
        .datagram_send_buffer_size(0);
    server.transport_config(Arc::new(transport));
    Ok(server)
}

pub(crate) fn load_tls_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, ServerError> {
    let certs = CertificateDer::pem_file_iter(path)
        .map_err(|error| {
            ServerError::Tls(format!(
                "TLS certificate `{}` cannot be parsed: {error}",
                path.display()
            ))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            ServerError::Tls(format!(
                "TLS certificate `{}` cannot be parsed: {error}",
                path.display()
            ))
        })?;
    if certs.is_empty() {
        return Err(ServerError::Tls(format!(
            "TLS certificate `{}` does not contain any certificates",
            path.display()
        )));
    }
    Ok(certs)
}

pub(crate) fn load_tls_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, ServerError> {
    PrivateKeyDer::from_pem_file(path).map_err(|error| {
        ServerError::Tls(format!(
            "TLS private key `{}` cannot be parsed: {error}",
            path.display()
        ))
    })
}
