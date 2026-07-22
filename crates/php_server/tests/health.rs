use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    process::{Child, Command as Proc, Stdio},
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[test]
fn server_serves_healthz() {
    let docroot = temp_docroot();
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_get(&address, "/healthz");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.ends_with("ok\n"), "{response}");
}

#[test]
fn connection_admission_rejects_saturation_and_releases_the_permit() {
    let docroot = temp_docroot();
    let mut child = start_server(
        &docroot,
        &[
            "--max-connections",
            "1",
            "--connection-idle-timeout-ms",
            "2000",
        ],
    );
    let address = read_listening_address(&mut child);
    let held = TcpStream::connect(&address).expect("hold the only connection permit");
    std::thread::sleep(Duration::from_millis(50));

    let mut rejected = TcpStream::connect(&address).expect("connect saturated socket");
    rejected
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set saturated read timeout");
    let _ = rejected
        .write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    let mut byte = [0_u8; 1];
    match rejected.read(&mut byte) {
        Ok(0) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe
            ) => {}
        result => panic!("saturated connection was not dropped immediately: {result:?}"),
    }

    drop(rejected);
    drop(held);
    let health = http_request_after_connection_rejection(&address, "GET", "/healthz");
    let metrics = http_request_after_connection_rejection(&address, "GET", "/__phrust/metrics");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove connection admission docroot");
    assert!(health.starts_with("HTTP/1.1 200"), "{health}");
    assert!(
        metric_value(&metrics, "phrust_server_connection_limit_rejections_total") >= 1,
        "{metrics}"
    );
}

#[test]
fn h1_header_deadline_is_observable() {
    let docroot = temp_docroot();
    let mut child = start_server(
        &docroot,
        &[
            "--request-header-timeout-ms",
            "100",
            "--connection-idle-timeout-ms",
            "1000",
        ],
    );
    let address = read_listening_address(&mut child);

    let mut slow_header = TcpStream::connect(&address).expect("connect slow header");
    slow_header
        .write_all(b"GET /healthz HTTP/1.1\r\nHost: local")
        .expect("write partial header");
    slow_header
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set slow-header timeout");
    std::thread::sleep(Duration::from_millis(200));
    let mut byte = [0_u8; 1];
    match slow_header.read(&mut byte) {
        Ok(0) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe
            ) => {}
        result => panic!("slow header connection remained open: {result:?}"),
    }

    let metrics = http_request(&address, "GET", "/__phrust/metrics");
    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove header deadline docroot");
    assert!(
        metrics.contains("phrust_server_h1_header_timeouts_total 1\n"),
        "{metrics}"
    );
}

#[test]
fn inactive_keep_alive_uses_the_connection_idle_deadline() {
    let docroot = temp_docroot();
    let mut child = start_server(
        &docroot,
        &[
            "--request-header-timeout-ms",
            "1000",
            "--connection-idle-timeout-ms",
            "100",
        ],
    );
    let address = read_listening_address(&mut child);
    let mut idle = TcpStream::connect(&address).expect("connect idle keep-alive");
    idle.write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("write keep-alive request");
    idle.set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set idle read timeout");
    let started = Instant::now();
    let mut response = String::new();
    idle.read_to_string(&mut response)
        .expect("read response through idle close");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(
        started.elapsed() >= Duration::from_millis(75),
        "keep-alive closed before its idle budget"
    );

    let metrics = http_request(&address, "GET", "/__phrust/metrics");
    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove connection idle docroot");
    assert!(
        metrics.contains("phrust_server_connection_idle_timeouts_total 1\n"),
        "{metrics}"
    );
}

#[test]
fn active_php_request_outlives_the_connection_idle_budget() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("slow.php"),
        "<?php usleep(250000); echo \"done\\n\";",
    )
    .expect("write slow PHP fixture");
    let mut child = start_server(
        &docroot,
        &[
            "--connection-idle-timeout-ms",
            "50",
            "--max-execution-ms",
            "2000",
        ],
    );
    let address = read_listening_address(&mut child);
    let started = Instant::now();
    let response = http_request(&address, "GET", "/slow.php");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove active-idle docroot");
    assert_eq!(response_body(&response), "done\n", "{response}");
    assert!(started.elapsed() >= Duration::from_millis(200));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tls_handshake_timeout_and_protocol_failure_release_budgets() {
    use http_body_util::{BodyExt, Empty};
    use hyper_util::rt::TokioIo;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let docroot = temp_docroot();
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--max-connections",
            "1",
            "--tls-handshake-timeout-ms",
            "100",
        ],
    );
    let address = read_listening_address(&mut child);

    let mut slow = tokio::net::TcpStream::connect(&address)
        .await
        .expect("connect slow TLS handshake");
    tokio::time::sleep(Duration::from_millis(175)).await;
    let mut byte = [0_u8; 1];
    let slow_read = tokio::time::timeout(Duration::from_secs(1), slow.read(&mut byte))
        .await
        .expect("slow TLS socket closes");
    assert_eq!(slow_read.expect("read slow TLS close"), 0);
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut invalid = tokio::net::TcpStream::connect(&address)
        .await
        .expect("connect invalid TLS handshake");
    invalid
        .write_all(b"GET / HTTP/1.1\r\n\r\n")
        .await
        .expect("write non-TLS bytes");
    let mut alert = Vec::new();
    let invalid_read =
        tokio::time::timeout(Duration::from_secs(1), invalid.read_to_end(&mut alert))
            .await
            .expect("invalid TLS socket closes");
    invalid_read.expect("read invalid TLS close");
    tokio::time::sleep(Duration::from_millis(50)).await;

    let connector =
        tokio_rustls::TlsConnector::from(Arc::new(test_client_config(vec![b"http/1.1".to_vec()])));
    let retry_deadline = Instant::now() + Duration::from_secs(1);
    let tls = loop {
        let tcp = tokio::net::TcpStream::connect(&address)
            .await
            .expect("connect post-failure TLS client");
        match connector
            .connect(
                rustls_pki_types::ServerName::try_from("localhost").unwrap(),
                tcp,
            )
            .await
        {
            Ok(tls) => break tls,
            Err(_) if Instant::now() < retry_deadline => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(error) => panic!("TLS connection permit was not released: {error}"),
        }
    };
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(tls))
        .await
        .expect("perform post-failure H1 handshake");
    let connection_task = tokio::spawn(connection);
    let health = sender
        .send_request(
            hyper::Request::builder()
                .uri("https://localhost/healthz")
                .body(Empty::<bytes::Bytes>::new())
                .expect("build post-failure health request"),
        )
        .await
        .expect("send post-failure health request");
    let health_status = health.status();
    health
        .into_body()
        .collect()
        .await
        .expect("collect post-failure health response");
    let metrics = sender
        .send_request(
            hyper::Request::builder()
                .uri("https://localhost/__phrust/metrics")
                .body(Empty::<bytes::Bytes>::new())
                .expect("build post-failure metrics request"),
        )
        .await
        .expect("send post-failure metrics request")
        .into_body()
        .collect()
        .await
        .expect("collect post-failure metrics response")
        .to_bytes();
    let metrics = String::from_utf8(metrics.to_vec()).expect("TLS metrics are UTF-8");
    drop(sender);
    connection_task.abort();
    let _ = connection_task.await;

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove TLS admission docroot");
    assert_eq!(health_status, hyper::StatusCode::OK);
    for expected in [
        "phrust_server_tls_handshakes_active 0\n",
        "phrust_server_tls_handshake_timeouts_total 1\n",
        "phrust_server_tls_handshake_failures_total 1\n",
        "phrust_server_tls_handshake_protocol_failures_total 1\n",
    ] {
        assert!(
            metrics.contains(expected),
            "missing {expected:?}: {metrics}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http3_body_idle_and_quic_connection_idle_are_enforced() {
    use bytes::Buf;
    use quinn::crypto::rustls::QuicClientConfig;

    let docroot = temp_docroot();
    fs::write(
        docroot.join("body.php"),
        "<?php echo strlen(file_get_contents('php://input')), \"\\n\";",
    )
    .expect("write H3 idle body fixture");
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--enable-http3",
            "--request-body-idle-timeout-ms",
            "75",
            "--request-body-timeout-ms",
            "1000",
            "--connection-idle-timeout-ms",
            "250",
        ],
    );
    let address = read_listening_address(&mut child);

    let mut crypto = test_client_config(vec![b"h3".to_vec()]);
    crypto.enable_early_data = true;
    let client_config = quinn::ClientConfig::new(Arc::new(
        QuicClientConfig::try_from(crypto).expect("build H3 idle client config"),
    ));
    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap())
        .expect("create H3 idle client endpoint");
    endpoint.set_default_client_config(client_config.clone());
    let connection = endpoint
        .connect(address.parse().unwrap(), "localhost")
        .expect("start H3 body-idle connection")
        .await
        .expect("connect H3 body-idle client");
    let (mut driver, mut sender) = h3::client::new(h3_quinn::Connection::new(connection))
        .await
        .expect("create H3 body-idle client");
    let driver_task = tokio::spawn(async move { driver.wait_idle().await });
    let request = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri("https://localhost/body.php")
        .header(hyper::header::CONTENT_TYPE, "application/octet-stream")
        .header(hyper::header::CONTENT_LENGTH, "4")
        .body(())
        .expect("build H3 idle request");
    let mut stream = sender
        .send_request(request)
        .await
        .expect("send H3 idle request headers");
    stream
        .send_data(bytes::Bytes::from_static(b"a"))
        .await
        .expect("send first H3 body frame");
    let response = match tokio::time::timeout(Duration::from_secs(1), stream.recv_response()).await
    {
        Ok(Ok(response)) => response,
        result => {
            let _ = child.kill();
            let _ = child.wait();
            let mut stderr = String::new();
            if let Some(mut output) = child.stderr.take() {
                let _ = output.read_to_string(&mut stderr);
            }
            panic!("receive H3 body idle response: {result:?}; server stderr: {stderr}");
        }
    };
    assert_eq!(response.status(), hyper::StatusCode::REQUEST_TIMEOUT);
    while let Some(mut data) = stream.recv_data().await.expect("receive H3 408 body") {
        while data.has_remaining() {
            let _ = data.copy_to_bytes(data.remaining());
        }
    }
    drop(stream);
    drop(sender);
    endpoint.close(quinn::VarInt::from_u32(0), b"body idle complete");
    let _ = driver_task.await;

    let mut idle_endpoint =
        quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).expect("create QUIC idle endpoint");
    idle_endpoint.set_default_client_config(client_config);
    let idle_connection = idle_endpoint
        .connect(address.parse().unwrap(), "localhost")
        .expect("start QUIC idle connection")
        .await
        .expect("connect QUIC idle client");
    let (mut idle_driver, idle_sender) =
        h3::client::new(h3_quinn::Connection::new(idle_connection))
            .await
            .expect("create idle H3 connection");
    let _ = tokio::time::timeout(Duration::from_secs(2), idle_driver.wait_idle())
        .await
        .expect("QUIC idle timeout closes connection");
    drop(idle_sender);
    idle_endpoint.close(quinn::VarInt::from_u32(0), b"idle observed");

    let metrics = http3_request(&address, hyper::Method::GET, "/__phrust/metrics", &[], &[]).await;
    let metrics = String::from_utf8(metrics.body()).expect("H3 idle metrics are UTF-8");
    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove H3 idle docroot");
    for expected in [
        "phrust_server_request_body_idle_timeouts_total 1\n",
        "phrust_server_quic_idle_timeouts_total 1\n",
    ] {
        assert!(
            metrics.contains(expected),
            "missing {expected:?}: {metrics}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_and_http3_share_one_connection_budget() {
    use quinn::crypto::rustls::QuicClientConfig;

    let docroot = temp_docroot();
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--enable-http3",
            "--max-connections",
            "1",
            "--tls-handshake-timeout-ms",
            "2000",
        ],
    );
    let address = read_listening_address(&mut child);
    let held_tcp = tokio::net::TcpStream::connect(&address)
        .await
        .expect("hold shared connection permit with TCP");
    tokio::time::sleep(Duration::from_millis(50)).await;

    let crypto = test_client_config(vec![b"h3".to_vec()]);
    let client_config = quinn::ClientConfig::new(Arc::new(
        QuicClientConfig::try_from(crypto).expect("build shared-limit QUIC client config"),
    ));
    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap())
        .expect("create shared-limit QUIC endpoint");
    endpoint.set_default_client_config(client_config);
    let refused = endpoint
        .connect(address.parse().unwrap(), "localhost")
        .expect("start saturated QUIC connection");
    assert!(
        tokio::time::timeout(Duration::from_secs(1), refused)
            .await
            .expect("saturated QUIC connection resolves")
            .is_err(),
        "QUIC connection bypassed the held TCP permit"
    );
    endpoint.close(quinn::VarInt::from_u32(0), b"saturation observed");

    drop(held_tcp);
    // The server must first observe EOF and retire the TCP task that owns the
    // shared permit. Parallel integration tests can delay that scheduling.
    tokio::time::sleep(Duration::from_millis(250)).await;
    let health = http3_request(&address, hyper::Method::GET, "/healthz", &[], &[]).await;
    let metrics = http3_request(&address, hyper::Method::GET, "/__phrust/metrics", &[], &[]).await;
    let metrics = String::from_utf8(metrics.body()).expect("shared-limit metrics are UTF-8");
    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove shared connection limit docroot");
    assert_eq!(health.status, hyper::StatusCode::OK);
    assert!(
        metrics.contains("phrust_server_h3_connection_limit_rejections_total 1\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_connection_limit_rejections_total 1\n"),
        "{metrics}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http3_field_section_and_concurrent_stream_limits_are_enforced() {
    use quinn::crypto::rustls::QuicClientConfig;

    let docroot = temp_docroot();
    fs::write(
        docroot.join("body.php"),
        "<?php echo strlen(file_get_contents('php://input')), \"\\n\";",
    )
    .expect("write H3 stream-limit fixture");
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--enable-http3",
            "--max-streams-per-connection",
            "2",
        ],
    );
    let address = read_listening_address(&mut child);
    let crypto = test_client_config(vec![b"h3".to_vec()]);
    let client_config = quinn::ClientConfig::new(Arc::new(
        QuicClientConfig::try_from(crypto).expect("build H3 limit client config"),
    ));

    let mut header_endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap())
        .expect("create H3 field-limit endpoint");
    header_endpoint.set_default_client_config(client_config.clone());
    let header_connection = header_endpoint
        .connect(address.parse().unwrap(), "localhost")
        .expect("start H3 field-limit connection")
        .await
        .expect("connect H3 field-limit client");
    let (mut header_driver, mut header_sender) =
        h3::client::new(h3_quinn::Connection::new(header_connection))
            .await
            .expect("create H3 field-limit client");
    let header_driver_task = tokio::spawn(async move { header_driver.wait_idle().await });
    let large_value = hyper::header::HeaderValue::from_bytes(&vec![b'x'; 70_000])
        .expect("construct oversized H3 field value");
    let large_request = hyper::Request::builder()
        .uri("https://localhost/healthz")
        .header("x-large", large_value)
        .body(())
        .expect("build oversized H3 request");
    match header_sender.send_request(large_request).await {
        Err(_) => {}
        Ok(mut stream) => {
            let _ = stream.finish().await;
            let response = tokio::time::timeout(Duration::from_secs(1), stream.recv_response())
                .await
                .expect("oversized H3 field resolves");
            assert!(
                response.is_err()
                    || response.is_ok_and(|response| {
                        response.status() == hyper::StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE
                    }),
                "oversized H3 field section was accepted"
            );
        }
    }
    drop(header_sender);
    header_endpoint.close(quinn::VarInt::from_u32(0), b"field limit observed");
    header_driver_task.abort();
    let _ = header_driver_task.await;

    let mut stream_endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap())
        .expect("create H3 stream-limit endpoint");
    stream_endpoint.set_default_client_config(client_config);
    let stream_connection = stream_endpoint
        .connect(address.parse().unwrap(), "localhost")
        .expect("start H3 stream-limit connection")
        .await
        .expect("connect H3 stream-limit client");
    let (mut stream_driver, mut stream_sender) =
        h3::client::new(h3_quinn::Connection::new(stream_connection))
            .await
            .expect("create H3 stream-limit client");
    let stream_driver_task = tokio::spawn(async move { stream_driver.wait_idle().await });
    let held_request = || {
        hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri("https://localhost/body.php")
            .header(hyper::header::CONTENT_TYPE, "application/octet-stream")
            .header(hyper::header::CONTENT_LENGTH, "1")
            .body(())
            .expect("build held H3 request")
    };
    let first = stream_sender
        .send_request(held_request())
        .await
        .expect("open first H3 request stream");
    let second = stream_sender
        .send_request(held_request())
        .await
        .expect("open second H3 request stream");
    assert!(
        tokio::time::timeout(
            Duration::from_millis(150),
            stream_sender.send_request(held_request()),
        )
        .await
        .is_err(),
        "third H3 request stream bypassed max_streams_per_connection=2"
    );
    drop(first);
    drop(second);
    drop(stream_sender);
    stream_endpoint.close(quinn::VarInt::from_u32(0), b"stream limit observed");
    stream_driver_task.abort();
    let _ = stream_driver_task.await;

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove H3 limits docroot");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http2_advertised_stream_limit_bounds_admitted_requests() {
    use http_body_util::StreamBody;
    use hyper_util::rt::{TokioExecutor, TokioIo};

    let docroot = temp_docroot();
    fs::write(
        docroot.join("body.php"),
        "<?php echo strlen(file_get_contents('php://input')), \"\\n\";",
    )
    .expect("write H2 stream-limit fixture");
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--max-streams-per-connection",
            "2",
            "--request-body-timeout-ms",
            "5000",
            "--request-body-idle-timeout-ms",
            "5000",
        ],
    );
    let address = read_listening_address(&mut child);
    let tcp = tokio::net::TcpStream::connect(&address)
        .await
        .expect("connect H2 stream-limit client");
    let connector =
        tokio_rustls::TlsConnector::from(Arc::new(test_client_config(vec![b"h2".to_vec()])));
    let tls = connector
        .connect(
            rustls_pki_types::ServerName::try_from("localhost").unwrap(),
            tcp,
        )
        .await
        .expect("connect H2 stream-limit TLS");
    type PendingBody = StreamBody<
        futures_util::stream::Pending<
            Result<hyper::body::Frame<bytes::Bytes>, std::convert::Infallible>,
        >,
    >;
    let (sender, connection) = hyper::client::conn::http2::handshake::<_, _, PendingBody>(
        TokioExecutor::new(),
        TokioIo::new(tls),
    )
    .await
    .expect("perform H2 stream-limit handshake");
    let connection_task = tokio::spawn(connection);
    let held_request = || {
        hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri("https://localhost/body.php")
            .header(hyper::header::CONTENT_TYPE, "application/octet-stream")
            .header(hyper::header::CONTENT_LENGTH, "1")
            .body(StreamBody::new(futures_util::stream::pending()))
            .expect("build held H2 request")
    };
    let first_sender = sender.clone();
    let first = tokio::spawn(async move {
        let mut sender = first_sender;
        sender.send_request(held_request()).await
    });
    let second_sender = sender.clone();
    let second = tokio::spawn(async move {
        let mut sender = second_sender;
        sender.send_request(held_request()).await
    });
    let third_sender = sender.clone();
    let third = tokio::spawn(async move {
        let mut sender = third_sender;
        sender.send_request(held_request()).await
    });
    tokio::time::sleep(Duration::from_millis(150)).await;

    let metrics = http1_request(&address, hyper::Method::GET, "/__phrust/metrics", &[], &[]).await;
    let metrics = String::from_utf8(metrics.body()).expect("H2 stream-limit metrics are UTF-8");
    assert!(
        metrics.contains("phrust_server_in_flight 3\n"),
        "only two held H2 streams plus the metrics request may be admitted: {metrics}"
    );

    first.abort();
    second.abort();
    third.abort();
    let _ = first.await;
    let _ = second.await;
    let _ = third.await;
    drop(sender);
    connection_task.abort();
    let _ = connection_task.await;
    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove H2 stream-limit docroot");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sigterm_drains_h2_requests_and_switches_readiness_before_accept_stop() {
    use http_body_util::{BodyExt, Empty};
    use hyper_util::rt::{TokioExecutor, TokioIo};

    let docroot = temp_docroot();
    fs::write(
        docroot.join("slow.php"),
        "<?php usleep(350000); echo \"done\\n\";",
    )
    .expect("write drain PHP fixture");
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--max-in-flight",
            "1",
            "--request-admission-timeout-ms",
            "1500",
            "--graceful-shutdown-timeout-ms",
            "2000",
            "--max-execution-ms",
            "2000",
        ],
    );
    let address = read_listening_address(&mut child);
    let tcp = tokio::net::TcpStream::connect(&address)
        .await
        .expect("connect drain H2 client");
    let connector =
        tokio_rustls::TlsConnector::from(Arc::new(test_client_config(vec![b"h2".to_vec()])));
    let tls = connector
        .connect(
            rustls_pki_types::ServerName::try_from("localhost").unwrap(),
            tcp,
        )
        .await
        .expect("connect drain H2 TLS");
    let (sender, connection) = hyper::client::conn::http2::handshake::<_, _, Empty<bytes::Bytes>>(
        TokioExecutor::new(),
        TokioIo::new(tls),
    )
    .await
    .expect("perform drain H2 handshake");
    let connection_task = tokio::spawn(connection);
    let request_task =
        |mut sender: hyper::client::conn::http2::SendRequest<Empty<bytes::Bytes>>,
         path: &'static str| {
            tokio::spawn(async move {
                let request = hyper::Request::builder()
                    .uri(format!("https://localhost{path}"))
                    .body(Empty::new())
                    .expect("build drain request");
                let response = sender
                    .send_request(request)
                    .await
                    .expect("receive drain response head");
                let (parts, body) = response.into_parts();
                let body = body
                    .collect()
                    .await
                    .expect("collect drain response")
                    .to_bytes();
                (parts.status, parts.headers, body)
            })
        };

    let slow = request_task(sender.clone(), "/slow.php");
    tokio::time::sleep(Duration::from_millis(75)).await;
    let ready = request_task(sender.clone(), "/readyz");
    let health = request_task(sender.clone(), "/healthz");
    let metrics = request_task(sender, "/__phrust/metrics");
    tokio::time::sleep(Duration::from_millis(75)).await;

    send_sigterm(&child);
    tokio::time::sleep(Duration::from_millis(75)).await;
    let new_connection = tokio::time::timeout(
        Duration::from_millis(250),
        tokio::net::TcpStream::connect(&address),
    )
    .await;
    assert!(
        !matches!(new_connection, Ok(Ok(_))),
        "new TCP connection succeeded after readiness switched to draining"
    );

    let (slow, ready, health, metrics) = tokio::join!(slow, ready, health, metrics);
    let (slow_status, _, slow_body) = slow.expect("join slow drain request");
    let (ready_status, ready_headers, ready_body) = ready.expect("join readiness request");
    let (health_status, health_headers, health_body) = health.expect("join health request");
    let (metrics_status, _, metrics_body) = metrics.expect("join metrics request");
    assert_eq!(slow_status, hyper::StatusCode::OK);
    assert_eq!(slow_body, b"done\n".as_slice());
    assert_eq!(ready_status, hyper::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(ready_body, b"draining\n".as_slice());
    assert_eq!(health_status, hyper::StatusCode::OK);
    assert_eq!(health_body, b"ok\n".as_slice());
    assert!(!ready_headers.contains_key(hyper::header::CONNECTION));
    assert!(!health_headers.contains_key(hyper::header::CONNECTION));
    assert_eq!(metrics_status, hyper::StatusCode::OK);
    let metrics_body = String::from_utf8(metrics_body.to_vec()).expect("drain metrics are UTF-8");
    assert!(
        metrics_body.contains("phrust_server_graceful_shutdowns_total 1\n"),
        "{metrics_body}"
    );
    assert!(
        metrics_body.contains("phrust_server_readiness_state 0\n"),
        "{metrics_body}"
    );

    let status = wait_for_exit(&mut child, Duration::from_secs(3));
    let _ = connection_task.await;
    fs::remove_dir_all(docroot).expect("remove H2 drain docroot");
    assert!(status.success(), "drained server exited with {status}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sigterm_sends_http3_goaway_and_finishes_the_existing_stream() {
    use bytes::Buf;
    use quinn::crypto::rustls::QuicClientConfig;

    let docroot = temp_docroot();
    fs::write(
        docroot.join("slow.php"),
        "<?php echo \"first\\n\"; flush(); usleep(350000); echo \"done\\n\";",
    )
    .expect("write H3 drain fixture");
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--enable-http3",
            "--graceful-shutdown-timeout-ms",
            "2000",
            "--max-execution-ms",
            "2000",
        ],
    );
    let address = read_listening_address(&mut child);

    let crypto = test_client_config(vec![b"h3".to_vec()]);
    let client_config = quinn::ClientConfig::new(Arc::new(
        QuicClientConfig::try_from(crypto).expect("build drain QUIC client config"),
    ));
    let mut endpoint =
        quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).expect("create H3 drain endpoint");
    endpoint.set_default_client_config(client_config);
    let connection = endpoint
        .connect(address.parse().unwrap(), "localhost")
        .expect("start H3 drain connection")
        .await
        .expect("connect H3 drain client");
    let (mut driver, mut sender) = h3::client::new(h3_quinn::Connection::new(connection))
        .await
        .expect("create H3 drain client");
    let driver_task = tokio::spawn(async move { driver.wait_idle().await });
    let request = hyper::Request::builder()
        .uri("https://localhost/slow.php")
        .body(())
        .expect("build H3 drain request");
    let mut stream = sender
        .send_request(request)
        .await
        .expect("send H3 drain request");
    stream.finish().await.expect("finish H3 drain request");
    let response = stream
        .recv_response()
        .await
        .expect("receive H3 drain response head");
    assert_eq!(response.status(), hyper::StatusCode::OK);
    let mut body = Vec::new();
    let mut first = stream
        .recv_data()
        .await
        .expect("receive first H3 drain data")
        .expect("H3 drain response has initial data");
    while first.has_remaining() {
        body.extend_from_slice(&first.copy_to_bytes(first.remaining()));
    }
    assert_eq!(body, b"first\n");

    send_sigterm(&child);
    tokio::time::sleep(Duration::from_millis(100)).await;
    let rejected = sender
        .send_request(
            hyper::Request::builder()
                .uri("https://localhost/healthz")
                .body(())
                .expect("build post-GOAWAY request"),
        )
        .await;
    assert!(rejected.is_err(), "H3 accepted a request after GOAWAY");

    while let Ok(Some(mut data)) = stream.recv_data().await {
        while data.has_remaining() {
            body.extend_from_slice(&data.copy_to_bytes(data.remaining()));
        }
    }
    assert_eq!(body, b"first\ndone\n");
    drop(stream);
    drop(sender);
    let _driver_result = tokio::time::timeout(Duration::from_secs(2), driver_task)
        .await
        .expect("H3 drain driver stopped")
        .expect("join H3 drain driver");
    let status = wait_for_exit(&mut child, Duration::from_secs(3));
    endpoint.close(quinn::VarInt::from_u32(0), b"drain observed");
    fs::remove_dir_all(docroot).expect("remove H3 drain docroot");
    assert!(status.success(), "H3 drained server exited with {status}");
}

#[test]
fn request_body_idle_timeout_returns_408_and_cleans_spooling() {
    let docroot = temp_docroot();
    let spool_dir = temp_docroot();
    fs::write(
        docroot.join("body.php"),
        "<?php echo strlen(file_get_contents('php://input')), \"\\n\";",
    )
    .expect("write body fixture");
    let spool_arg = spool_dir.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--request-body-memory-bytes",
            "1",
            "--request-body-temp-dir",
            &spool_arg,
            "--request-body-idle-timeout-ms",
            "100",
            "--request-body-timeout-ms",
            "1000",
        ],
    );
    let address = read_listening_address(&mut child);
    let mut stream = TcpStream::connect(&address).expect("connect idle request body");
    stream
        .write_all(
            b"POST /body.php HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/octet-stream\r\nContent-Length: 4\r\nConnection: close\r\n\r\nab",
        )
        .expect("write partial request body");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set idle body response timeout");
    std::thread::sleep(Duration::from_millis(175));
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read idle body response");
    assert!(response.starts_with("HTTP/1.1 408"), "{response}");

    let metrics = http_request(&address, "GET", "/__phrust/metrics");
    stop_child(child);
    assert_eq!(
        fs::read_dir(&spool_dir)
            .expect("read idle body spool")
            .count(),
        0
    );
    fs::remove_dir_all(docroot).expect("remove idle body docroot");
    fs::remove_dir_all(spool_dir).expect("remove idle body spool");
    for expected in [
        "phrust_server_request_body_idle_timeouts_total 1\n",
        "phrust_server_request_body_total_timeouts_total 0\n",
        "phrust_server_request_body_tempfiles_active 0\n",
    ] {
        assert!(
            metrics.contains(expected),
            "missing {expected:?}: {metrics}"
        );
    }
}

#[test]
fn request_body_total_timeout_is_separate_from_continuous_progress() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("body.php"),
        "<?php echo strlen(file_get_contents('php://input')), \"\\n\";",
    )
    .expect("write total body fixture");
    let mut child = start_server(
        &docroot,
        &[
            "--request-body-idle-timeout-ms",
            "120",
            "--request-body-timeout-ms",
            "220",
        ],
    );
    let address = read_listening_address(&mut child);
    let mut stream = TcpStream::connect(&address).expect("connect total-timeout body");
    stream
        .write_all(
            b"POST /body.php HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/octet-stream\r\nContent-Length: 6\r\nConnection: close\r\n\r\na",
        )
        .expect("write total-timeout headers");
    for byte in [b'b', b'c', b'd'] {
        std::thread::sleep(Duration::from_millis(60));
        stream
            .write_all(&[byte])
            .expect("write progressing body byte");
    }
    std::thread::sleep(Duration::from_millis(60));
    let _ = stream.write_all(b"e");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set total body response timeout");
    let mut timed_out = String::new();
    stream
        .read_to_string(&mut timed_out)
        .expect("read total body timeout response");
    assert!(timed_out.starts_with("HTTP/1.1 408"), "{timed_out}");

    let mut successful = TcpStream::connect(&address).expect("connect progressing body");
    successful
        .write_all(
            b"POST /body.php HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/octet-stream\r\nContent-Length: 4\r\nConnection: close\r\n\r\na",
        )
        .expect("write progressing headers");
    for byte in [b'b', b'c', b'd'] {
        std::thread::sleep(Duration::from_millis(35));
        successful
            .write_all(&[byte])
            .expect("write successful body byte");
    }
    successful
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set progressing response timeout");
    let mut completed = String::new();
    successful
        .read_to_string(&mut completed)
        .expect("read progressing body response");
    assert!(completed.starts_with("HTTP/1.1 200"), "{completed}");
    assert_eq!(response_body(&completed), "4\n");

    let metrics = http_request(&address, "GET", "/__phrust/metrics");
    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove total body docroot");
    assert!(
        metrics.contains("phrust_server_request_body_total_timeouts_total 1\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_request_body_idle_timeouts_total 0\n"),
        "{metrics}"
    );
}

#[test]
fn slow_response_reader_hits_write_idle_and_aborts_once() {
    let docroot = temp_docroot();
    let large = fs::File::create(docroot.join("large.bin")).expect("create slow-reader fixture");
    large
        .set_len(64 * 1024 * 1024)
        .expect("size slow-reader fixture");
    let mut child = start_server(
        &docroot,
        &[
            "--response-write-idle-timeout-ms",
            "100",
            "--connection-idle-timeout-ms",
            "2000",
        ],
    );
    let address = read_listening_address(&mut child);
    let mut slow = TcpStream::connect(&address).expect("connect slow response reader");
    slow.write_all(b"GET /large.bin HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .expect("request large response");
    std::thread::sleep(Duration::from_millis(600));

    let metrics = http_request(&address, "GET", "/__phrust/metrics");
    drop(slow);
    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove slow-reader docroot");
    assert!(
        metrics.contains("phrust_server_response_write_idle_timeouts_total 1\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_transfers_aborted_total 1\n"),
        "{metrics}"
    );
}

#[test]
fn server_validates_authority_framing_and_request_limits_before_routing() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("headers.php"),
        "<?php echo isset($_SERVER['HTTP_X_REMOVE']) ? \"leaked\\n\" : \"clean\\n\";",
    )
    .expect("write header fixture");
    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);

    let missing_host = raw_http_request(
        &address,
        b"GET /healthz HTTP/1.1\r\nConnection: close\r\n\r\n",
    );
    assert!(missing_host.starts_with("HTTP/1.1 400"), "{missing_host}");

    let duplicate_host = raw_http_request(
        &address,
        b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert!(
        duplicate_host.starts_with("HTTP/1.1 400"),
        "{duplicate_host}"
    );

    let absolute = raw_http_request(
        &address,
        b"GET http://localhost/healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert!(absolute.starts_with("HTTP/1.1 200"), "{absolute}");

    let conflict = raw_http_request(
        &address,
        b"GET http://example.test/healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert!(conflict.starts_with("HTTP/1.1 400"), "{conflict}");

    let nominated = raw_http_request(
        &address,
        b"GET /headers.php HTTP/1.1\r\nHost: localhost\r\nConnection: close, X-Remove\r\nX-Remove: secret\r\n\r\n",
    );
    assert!(nominated.starts_with("HTTP/1.1 200"), "{nominated}");
    assert_eq!(response_body(&nominated), "clean\n");

    let smuggling = raw_http_request(
        &address,
        b"POST /headers.php HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n0\r\n\r\n",
    );
    assert!(smuggling.starts_with("HTTP/1.1 400"), "{smuggling}");

    let duplicate_equal_length = raw_http_request(
        &address,
        b"POST /headers.php HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    assert!(
        duplicate_equal_length.starts_with("HTTP/1.1 200"),
        "{duplicate_equal_length}"
    );

    let duplicate_conflicting_length = raw_http_request(
        &address,
        b"POST /headers.php HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nContent-Length: 1\r\nConnection: close\r\n\r\n",
    );
    assert!(
        duplicate_conflicting_length.starts_with("HTTP/1.1 400"),
        "{duplicate_conflicting_length}"
    );

    let unsupported_transfer_coding = raw_http_request(
        &address,
        b"POST /headers.php HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: gzip\r\nConnection: close\r\n\r\n",
    );
    assert!(
        unsupported_transfer_coding.starts_with("HTTP/1.1 400")
            || unsupported_transfer_coding.starts_with("HTTP/1.1 501"),
        "{unsupported_transfer_coding}"
    );

    let target = format!("/{}", "a".repeat(17_000));
    let request = format!("GET {target} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    let overlong = raw_http_request(&address, request.as_bytes());
    assert!(overlong.starts_with("HTTP/1.1 414"), "{overlong}");

    let proxy_connection = raw_http_request(
        &address,
        b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nProxy-Connection: keep-alive\r\n\r\n",
    );
    assert!(
        proxy_connection.starts_with("HTTP/1.1 400"),
        "{proxy_connection}"
    );

    let mut too_many_headers =
        b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n".to_vec();
    for index in 0..101 {
        too_many_headers.extend_from_slice(format!("X-H-{index}: v\r\n").as_bytes());
    }
    too_many_headers.extend_from_slice(b"\r\n");
    let too_many_headers = raw_http_request(&address, &too_many_headers);
    assert!(
        too_many_headers.starts_with("HTTP/1.1 431"),
        "{too_many_headers}"
    );

    let oversized_header = format!(
        "GET /healthz HTTP/1.1\r\nHost: localhost\r\nX-Large: {}\r\nConnection: close\r\n\r\n",
        "x".repeat(65_536)
    );
    let oversized_header = raw_http_request(&address, oversized_header.as_bytes());
    assert!(
        oversized_header.starts_with("HTTP/1.1 431"),
        "{oversized_header}"
    );

    let invalid_header = raw_http_request(
        &address,
        b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nInvalid Header\r\n\r\n",
    );
    assert!(
        invalid_header.starts_with("HTTP/1.1 400"),
        "{invalid_header}"
    );

    let obs_fold = raw_http_request(
        &address,
        b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nX-Test: one\r\n two\r\n\r\n",
    );
    assert!(obs_fold.starts_with("HTTP/1.1 400"), "{obs_fold}");

    let invalid_asterisk = raw_http_request(
        &address,
        b"GET * HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert!(
        invalid_asterisk.starts_with("HTTP/1.1 400"),
        "{invalid_asterisk}"
    );

    let closed_after_smuggling = raw_http_request(
        &address,
        b"POST /headers.php HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\nGET /healthz HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert!(
        closed_after_smuggling.starts_with("HTTP/1.1 400")
            && closed_after_smuggling.matches("HTTP/1.1").count() == 1,
        "{closed_after_smuggling}"
    );

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove request validation docroot");
}

#[test]
fn server_serves_static_file_and_head() {
    let docroot = temp_docroot();
    fs::write(docroot.join("static.txt"), "static bytes\n").expect("write static fixture");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let get_response = http_request(&address, "GET", "/static.txt");
    let head_response = http_request(&address, "HEAD", "/static.txt");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(
        get_response.starts_with("HTTP/1.1 200 OK"),
        "{get_response}"
    );
    assert!(
        get_response.contains("content-length: 13"),
        "{get_response}"
    );
    assert!(get_response.ends_with("static bytes\n"), "{get_response}");
    assert!(
        head_response.starts_with("HTTP/1.1 200 OK"),
        "{head_response}"
    );
    assert!(
        head_response.contains("content-length: 13"),
        "{head_response}"
    );
    assert!(
        !head_response.ends_with("static bytes\n"),
        "{head_response}"
    );
    assert_response_contains_header(&get_response, "accept-ranges", "bytes");
    assert_eq!(response_header_values(&get_response, "etag").len(), 1);
    assert_eq!(
        response_header_values(&get_response, "last-modified").len(),
        1
    );
}

#[test]
fn server_static_conditional_requests_return_304() {
    let docroot = temp_docroot();
    fs::write(docroot.join("static.txt"), "static bytes\n").expect("write static fixture");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let first = http_request(&address, "GET", "/static.txt");
    let etag = response_header_values(&first, "etag")[0].to_string();
    let last_modified = response_header_values(&first, "last-modified")[0].to_string();
    let etag_response = http_request_with_headers(
        &address,
        "GET",
        "/static.txt",
        &[("If-None-Match", &etag)],
        "",
    );
    let modified_response = http_request_with_headers(
        &address,
        "GET",
        "/static.txt",
        &[("If-Modified-Since", &last_modified)],
        "",
    );

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(
        etag_response.starts_with("HTTP/1.1 304 Not Modified"),
        "{etag_response}"
    );
    assert_eq!(response_body(&etag_response), "");
    assert!(
        modified_response.starts_with("HTTP/1.1 304 Not Modified"),
        "{modified_response}"
    );
    assert_eq!(response_body(&modified_response), "");
}

#[test]
fn server_static_range_requests_return_partial_content() {
    let docroot = temp_docroot();
    fs::write(docroot.join("static.txt"), "abcdef").expect("write static fixture");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let partial = http_request_with_headers(
        &address,
        "GET",
        "/static.txt",
        &[("Range", "bytes=1-3")],
        "",
    );
    let suffix =
        http_request_with_headers(&address, "GET", "/static.txt", &[("Range", "bytes=-2")], "");
    let invalid = http_request_with_headers(
        &address,
        "GET",
        "/static.txt",
        &[("Range", "bytes=20-30")],
        "",
    );

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(
        partial.starts_with("HTTP/1.1 206 Partial Content"),
        "{partial}"
    );
    assert_response_contains_header(&partial, "content-range", "bytes 1-3/6");
    assert_response_contains_header(&partial, "content-length", "3");
    assert_eq!(response_body(&partial), "bcd");
    assert!(
        suffix.starts_with("HTTP/1.1 206 Partial Content"),
        "{suffix}"
    );
    assert_eq!(response_body(&suffix), "ef");
    assert!(
        invalid.starts_with("HTTP/1.1 416 Range Not Satisfiable"),
        "{invalid}"
    );
    assert_response_contains_header(&invalid, "content-range", "bytes */6");
    assert_response_contains_header(&invalid, "content-length", "0");
}

#[test]
fn server_selects_precompressed_static_assets_when_accepted() {
    let docroot = temp_docroot();
    fs::write(docroot.join("app.js"), "plain asset\n").expect("write static fixture");
    fs::write(docroot.join("app.js.gz"), "precompressed asset\n").expect("write gzip fixture");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request_with_headers(
        &address,
        "GET",
        "/app.js",
        &[("Accept-Encoding", "gzip")],
        "",
    );

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_response_contains_header(&response, "content-encoding", "gzip");
    assert_response_contains_header(&response, "vary", "Accept-Encoding");
    assert_response_contains_header(&response, "content-type", "text/javascript");
    assert_eq!(response_body(&response), "precompressed asset\n");
}

#[test]
fn mutable_static_rejects_stale_and_orphaned_sidecars() {
    let docroot = temp_docroot();
    let identity_path = docroot.join("asset.txt");
    let sidecar_path = docroot.join("asset.txt.gz");
    fs::write(&identity_path, "identity").expect("write identity fixture");
    fs::write(&sidecar_path, "stale-sidecar").expect("write stale sidecar fixture");
    fs::write(docroot.join("orphan.txt.gz"), "orphan").expect("write orphan sidecar fixture");
    let old = UNIX_EPOCH + Duration::from_secs(1_000);
    let new = UNIX_EPOCH + Duration::from_secs(2_000);
    fs::File::open(&sidecar_path)
        .expect("open sidecar fixture")
        .set_times(fs::FileTimes::new().set_modified(old))
        .expect("set sidecar mtime");
    fs::File::open(&identity_path)
        .expect("open identity fixture")
        .set_times(fs::FileTimes::new().set_modified(new))
        .expect("set identity mtime");
    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);
    let stale = http_request_with_headers(
        &address,
        "GET",
        "/asset.txt",
        &[("Accept-Encoding", "gzip")],
        "",
    );
    let orphan = http_request_with_headers(
        &address,
        "GET",
        "/orphan.txt",
        &[("Accept-Encoding", "gzip")],
        "",
    );

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert_eq!(response_body(&stale), "identity");
    assert_response_lacks_header(&stale, "content-encoding", "gzip");
    assert!(orphan.starts_with("HTTP/1.1 404 Not Found"), "{orphan}");
}

#[test]
fn server_enforces_static_public_policy_and_directory_contract() {
    let docroot = temp_docroot();
    fs::create_dir(docroot.join("ordered")).expect("create ordered directory");
    fs::write(docroot.join("ordered/index.php"), "<?php echo 'php-index';")
        .expect("write PHP index");
    fs::write(docroot.join("ordered/index.html"), "html-index").expect("write HTML index");
    fs::create_dir(docroot.join("html-only")).expect("create HTML directory");
    fs::write(docroot.join("html-only/index.html"), "html-only").expect("write HTML-only index");
    fs::create_dir(docroot.join("empty")).expect("create empty directory");
    fs::create_dir(docroot.join(".well-known")).expect("create well-known directory");
    fs::write(docroot.join(".well-known/example.txt"), "well-known")
        .expect("write well-known fixture");
    fs::write(docroot.join(".well-known/.secret"), "secret").expect("write nested secret");
    fs::write(docroot.join("script.phtml"), "<?php echo 'phtml';").expect("write phtml fixture");
    for path in [
        ".env",
        ".htaccess",
        "web.config",
        "source.inc",
        "source.phar",
        "source.php5",
        "backup.txt.bak",
        "swap.swp",
        "temp.tmp",
        "app.js.br",
        "app.js.gz",
        "app.js.zst",
    ] {
        fs::write(docroot.join(path), "must-not-be-public").expect("write hidden fixture");
    }
    fs::create_dir(docroot.join(".git")).expect("create VCS directory");
    fs::write(docroot.join(".git/config"), "must-not-be-public").expect("write VCS fixture");

    let mut child = start_server(&docroot, &["--php-extensions", "php,phtml"]);
    let address = read_listening_address(&mut child);
    let redirect = http_request(&address, "GET", "/ordered?x=1");
    let ordered = http_request(&address, "GET", "/ordered/");
    let html = http_request(&address, "GET", "/html-only/");
    let empty = http_request(&address, "GET", "/empty/");
    let well_known = http_request(&address, "GET", "/.well-known/example.txt");
    let nested_secret = http_request(&address, "GET", "/.well-known/.secret");
    let phtml = http_request(&address, "GET", "/script.phtml");
    let hidden = [
        "/.env",
        "/.htaccess",
        "/web.config",
        "/source.inc",
        "/source.phar",
        "/source.php5",
        "/backup.txt.bak",
        "/swap.swp",
        "/temp.tmp",
        "/app.js.br",
        "/app.js.gz",
        "/app.js.zst",
        "/.git/config",
    ]
    .map(|path| http_request(&address, "GET", path));

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(
        redirect.starts_with("HTTP/1.1 308 Permanent Redirect"),
        "{redirect}"
    );
    assert_response_contains_header(&redirect, "location", "/ordered/?x=1");
    assert_eq!(response_body(&ordered), "php-index");
    assert_eq!(response_body(&html), "html-only");
    assert!(empty.starts_with("HTTP/1.1 404 Not Found"), "{empty}");
    assert_eq!(response_body(&well_known), "well-known");
    assert!(
        nested_secret.starts_with("HTTP/1.1 404 Not Found"),
        "{nested_secret}"
    );
    assert_eq!(response_body(&phtml), "phtml");
    for response in hidden {
        assert!(response.starts_with("HTTP/1.1 404 Not Found"), "{response}");
        assert!(!response.contains("must-not-be-public"), "{response}");
    }
}

#[test]
fn directory_without_index_is_terminal_before_front_controller() {
    let docroot = temp_docroot();
    fs::create_dir(docroot.join("empty")).expect("create empty directory");
    fs::write(docroot.join("index.php"), "<?php echo 'front';").expect("write front controller");

    for mode in ["dev", "immutable"] {
        let mut child = start_server(
            &docroot,
            &["--front-controller", "index.php", "--deployment-mode", mode],
        );
        let address = read_listening_address(&mut child);
        let directory = http_request(&address, "GET", "/empty/");
        let missing = http_request(&address, "GET", "/actually-missing");
        stop_child(child);

        assert!(
            directory.starts_with("HTTP/1.1 404 Not Found"),
            "mode={mode} {directory}"
        );
        assert_eq!(response_body(&missing), "front", "mode={mode}");
    }

    fs::remove_dir_all(docroot).expect("remove front-controller docroot");
}

#[test]
fn server_honors_custom_index_order_and_static_method_allow() {
    let docroot = temp_docroot();
    fs::create_dir(docroot.join("ordered")).expect("create ordered directory");
    fs::write(docroot.join("ordered/index.php"), "<?php echo 'php-index';")
        .expect("write PHP index");
    fs::write(docroot.join("ordered/index.html"), "html-index").expect("write HTML index");
    fs::write(docroot.join("asset.txt"), "asset").expect("write static fixture");
    let mut child = start_server(&docroot, &["--index", "index.html,index.php"]);
    let address = read_listening_address(&mut child);
    let ordered = http_request(&address, "GET", "/ordered/");
    let method = http_request(&address, "POST", "/asset.txt");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert_eq!(response_body(&ordered), "html-index");
    assert!(
        method.starts_with("HTTP/1.1 405 Method Not Allowed"),
        "{method}"
    );
    assert_response_contains_header(&method, "allow", "GET, HEAD");
}

#[test]
fn server_static_http_selection_matrix_is_representation_consistent() {
    let docroot = temp_docroot();
    fs::write(docroot.join("asset.a1b2c3d4.txt"), "identity-bytes")
        .expect("write identity fixture");
    fs::write(docroot.join("asset.a1b2c3d4.txt.br"), "brotli-bytes").expect("write br fixture");
    fs::write(docroot.join("asset.a1b2c3d4.txt.zst"), "zstd-bytes").expect("write zstd fixture");
    fs::write(docroot.join("asset.a1b2c3d4.txt.gz"), "gzip-bytes").expect("write gzip fixture");
    let mut child = start_server(&docroot, &["--deployment-mode", "immutable"]);
    let address = read_listening_address(&mut child);
    let path = "/asset.a1b2c3d4.txt";

    let identity = http_request(&address, "GET", path);
    let gzip_weighted = http_request_with_headers(
        &address,
        "GET",
        path,
        &[("Accept-Encoding", "gzip;q=1, br;q=0.2")],
        "",
    );
    let tie = http_request_with_headers(
        &address,
        "GET",
        path,
        &[("Accept-Encoding", "br;q=1,zstd;q=1,gzip;q=1,identity;q=1")],
        "",
    );
    let multiple_headers = http_request_with_headers(
        &address,
        "GET",
        path,
        &[
            ("Accept-Encoding", "br;q=0"),
            ("Accept-Encoding", "zstd;q=1"),
        ],
        "",
    );
    let unacceptable = http_request_with_headers(
        &address,
        "GET",
        path,
        &[("Accept-Encoding", "identity;q=0,br;q=0,zstd;q=0,gzip;q=0")],
        "",
    );
    let etag = response_header_values(&identity, "etag")[0].to_string();
    let not_modified = http_request_with_headers(
        &address,
        "GET",
        path,
        &[("If-None-Match", &format!("W/{etag}"))],
        "",
    );
    let failed = http_request_with_headers(&address, "GET", path, &[("If-Match", "\"wrong\"")], "");
    let ranged = http_request_with_headers(
        &address,
        "GET",
        path,
        &[("Range", "bytes=2-5"), ("If-Range", &etag)],
        "",
    );
    let ignored_range = http_request_with_headers(
        &address,
        "GET",
        path,
        &[("Range", "bytes=2-5"), ("If-Range", "W/\"wrong\"")],
        "",
    );
    let malformed =
        http_request_with_headers(&address, "GET", path, &[("Range", "bytes=oops")], "");
    let multiple_ranges =
        http_request_with_headers(&address, "GET", path, &[("Range", "bytes=0-1,3-4")], "");
    let unknown_range =
        http_request_with_headers(&address, "GET", path, &[("Range", "items=0-1")], "");
    let encoded_range = http_request_with_headers(
        &address,
        "GET",
        path,
        &[("Accept-Encoding", "gzip"), ("Range", "bytes=1-3")],
        "",
    );
    let unsatisfiable =
        http_request_with_headers(&address, "GET", path, &[("Range", "bytes=999-")], "");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert_eq!(response_body(&identity), "identity-bytes");
    assert_response_contains_header(&identity, "vary", "Accept-Encoding");
    assert_response_contains_header(
        &identity,
        "cache-control",
        "public, max-age=31536000, immutable",
    );
    assert_response_contains_header(&identity, "x-content-type-options", "nosniff");
    assert_eq!(response_body(&gzip_weighted), "gzip-bytes");
    assert_response_contains_header(&gzip_weighted, "content-encoding", "gzip");
    assert_ne!(
        response_header_values(&identity, "etag"),
        response_header_values(&gzip_weighted, "etag")
    );
    assert_eq!(response_body(&tie), "brotli-bytes");
    assert_response_contains_header(&tie, "content-encoding", "br");
    assert_eq!(response_body(&multiple_headers), "zstd-bytes");
    assert_response_contains_header(&multiple_headers, "content-encoding", "zstd");
    assert!(
        unacceptable.starts_with("HTTP/1.1 406 Not Acceptable"),
        "{unacceptable}"
    );
    assert!(
        not_modified.starts_with("HTTP/1.1 304 Not Modified"),
        "{not_modified}"
    );
    assert_response_contains_header(&not_modified, "vary", "Accept-Encoding");
    assert!(
        failed.starts_with("HTTP/1.1 412 Precondition Failed"),
        "{failed}"
    );
    assert_response_contains_header(&failed, "etag", &etag);
    assert!(
        ranged.starts_with("HTTP/1.1 206 Partial Content"),
        "{ranged}"
    );
    assert_eq!(response_body(&ranged), "enti");
    assert!(
        ignored_range.starts_with("HTTP/1.1 200 OK"),
        "{ignored_range}"
    );
    assert_eq!(response_body(&ignored_range), "identity-bytes");
    assert!(malformed.starts_with("HTTP/1.1 200 OK"), "{malformed}");
    assert!(
        multiple_ranges.starts_with("HTTP/1.1 200 OK"),
        "{multiple_ranges}"
    );
    assert!(
        unknown_range.starts_with("HTTP/1.1 200 OK"),
        "{unknown_range}"
    );
    assert!(
        encoded_range.starts_with("HTTP/1.1 206 Partial Content"),
        "{encoded_range}"
    );
    assert_response_contains_header(&encoded_range, "content-encoding", "gzip");
    assert_response_contains_header(&encoded_range, "content-range", "bytes 1-3/10");
    assert_eq!(response_body(&encoded_range), "zip");
    assert!(
        unsatisfiable.starts_with("HTTP/1.1 416 Range Not Satisfiable"),
        "{unsatisfiable}"
    );
    assert_response_contains_header(&unsatisfiable, "content-range", "bytes */14");
}

#[test]
fn immutable_static_index_rebuilds_and_uses_one_open_per_hit() {
    let docroot = temp_docroot();
    fs::write(docroot.join("first.txt"), "first").expect("write indexed fixture");
    let mut child = start_server(
        &docroot,
        &[
            "--deployment-mode",
            "immutable",
            "--enable-cache-clear-endpoint",
        ],
    );
    let address = read_listening_address(&mut child);
    let first = http_request(&address, "GET", "/first.txt");
    fs::write(docroot.join("later.txt"), "later").expect("write post-start fixture");
    let before = http_request(&address, "GET", "/later.txt");
    let metrics_before = http_request(&address, "GET", "/__phrust/metrics");
    let clear = http_request(&address, "POST", "/__phrust/cache/clear");
    let after = http_request(&address, "GET", "/later.txt");
    let metrics_after = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert_eq!(response_body(&first), "first");
    assert!(before.starts_with("HTTP/1.1 404 Not Found"), "{before}");
    assert!(
        metrics_before.contains("phrust_server_static_capability_opens_total 1"),
        "{metrics_before}"
    );
    assert!(clear.starts_with("HTTP/1.1 200 OK"), "{clear}");
    assert_eq!(response_body(&after), "later");
    assert!(
        metrics_after.contains("phrust_server_static_capability_opens_total 2"),
        "{metrics_after}"
    );
    assert!(
        metrics_after.contains("phrust_server_static_index_builds_total 2"),
        "{metrics_after}"
    );
}

#[cfg(unix)]
#[test]
fn immutable_static_index_terminates_symlink_loops_before_readiness() {
    use std::os::unix::{fs::symlink, net::UnixListener};

    let docroot = temp_docroot();
    fs::write(docroot.join("stable.txt"), "stable").expect("write loop fixture");
    fs::write(docroot.join(".env"), "hidden").expect("write hidden index fixture");
    fs::create_dir(docroot.join("nested")).expect("create loop directory");
    symlink("..", docroot.join("nested/loop")).expect("create directory symlink loop");
    symlink("stable.txt", docroot.join("internal.txt")).expect("create internal index symlink");
    let outside = docroot.with_extension("immutable-outside");
    fs::write(&outside, "outside-secret").expect("write outside index fixture");
    symlink(&outside, docroot.join("outside.txt")).expect("create outside index symlink");
    let fifo = docroot.join("pipe.txt");
    let status = Proc::new("mkfifo").arg(&fifo).status().expect("run mkfifo");
    assert!(status.success(), "mkfifo failed with {status}");
    let socket = UnixListener::bind(docroot.join("socket.txt")).expect("bind index socket fixture");

    let mut child = start_server(&docroot, &["--deployment-mode", "immutable"]);
    let address = read_listening_address(&mut child);
    let stable = http_request(&address, "GET", "/stable.txt");
    let internal = http_request(&address, "GET", "/internal.txt");
    let hidden = http_request(&address, "GET", "/.env");
    let escaped = http_request(&address, "GET", "/outside.txt");
    let fifo = http_request(&address, "GET", "/pipe.txt");
    let socket_response = http_request(&address, "GET", "/socket.txt");

    stop_child(child);
    drop(socket);
    fs::remove_dir_all(docroot).expect("remove loop docroot");
    fs::remove_file(outside).expect("remove outside index fixture");

    assert_eq!(response_body(&stable), "stable");
    assert_eq!(response_body(&internal), "stable");
    for response in [hidden, escaped, fifo, socket_response] {
        assert!(response.starts_with("HTTP/1.1 404 Not Found"), "{response}");
        assert!(!response.contains("outside-secret"), "{response}");
    }
}

#[cfg(unix)]
#[test]
fn immutable_static_index_failure_exits_without_readiness() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let docroot = temp_docroot();
    fs::write(docroot.join(OsString::from_vec(vec![0xff])), "invalid-name")
        .expect("write invalid startup fixture");
    let mut child = start_server(&docroot, &["--deployment-mode", "immutable"]);

    let status = wait_for_exit(&mut child, Duration::from_secs(5));
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .expect("startup stdout")
        .read_to_string(&mut stdout)
        .expect("read startup stdout");
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("startup stderr")
        .read_to_string(&mut stderr)
        .expect("read startup stderr");
    fs::remove_dir_all(docroot).expect("remove invalid startup docroot");

    assert!(
        !status.success(),
        "immutable startup unexpectedly succeeded"
    );
    assert!(!stdout.contains("listening "), "{stdout}");
    assert!(
        stderr.contains("static index entry under `.` is not UTF-8"),
        "{stderr}"
    );
}

#[cfg(unix)]
#[test]
fn failed_immutable_rebuild_keeps_the_previous_index_active() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let docroot = temp_docroot();
    fs::write(docroot.join("stable.txt"), "stable").expect("write stable fixture");
    let mut child = start_server(
        &docroot,
        &[
            "--deployment-mode",
            "immutable",
            "--enable-cache-clear-endpoint",
        ],
    );
    let address = read_listening_address(&mut child);
    let before = http_request(&address, "GET", "/stable.txt");
    fs::write(docroot.join(OsString::from_vec(vec![0xff])), "invalid-name")
        .expect("write non-UTF8 rebuild fixture");
    let clear = http_request(&address, "POST", "/__phrust/cache/clear");
    let after = http_request(&address, "GET", "/stable.txt");
    let metrics = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert_eq!(response_body(&before), "stable");
    assert!(
        clear.starts_with("HTTP/1.1 500 Internal Server Error"),
        "{clear}"
    );
    assert_eq!(response_body(&after), "stable");
    assert!(
        metrics.contains("phrust_server_static_index_build_failures_total 1"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_static_index_builds_total 1"),
        "{metrics}"
    );
}

#[test]
fn server_reports_static_file_metrics() {
    let docroot = temp_docroot();
    fs::write(docroot.join("static.txt"), "abcdef").expect("write static fixture");
    fs::write(docroot.join("static.txt.gz"), "gzipped").expect("write gzip fixture");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let first = http_request(&address, "GET", "/static.txt");
    let etag = response_header_values(&first, "etag")[0].to_string();
    let _ = http_request_with_headers(
        &address,
        "GET",
        "/static.txt",
        &[("If-None-Match", &etag)],
        "",
    );
    let _ = http_request_with_headers(
        &address,
        "GET",
        "/static.txt",
        &[("Range", "bytes=0-1")],
        "",
    );
    let _ = http_request_with_headers(
        &address,
        "GET",
        "/static.txt",
        &[("Accept-Encoding", "gzip")],
        "",
    );
    let metrics = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(
        metrics.contains("phrust_server_static_streamed_bytes_total 15"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_static_not_modified_total 1"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_static_partial_responses_total 1"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_static_precompressed_hits_total 1"),
        "{metrics}"
    );
}

#[test]
fn server_never_serves_php_scripts_as_static_source() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("source.php"),
        "<?php echo \"executed\\n\"; // static-source-marker\n",
    )
    .expect("write php fixture");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/source.php");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_eq!(response_body(&response), "executed\n");
    assert!(!response.contains("static-source-marker"), "{response}");
}

#[test]
fn server_exposes_internal_metrics() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let _ = http_request(&address, "GET", "/hello.php");
    let response = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(
        response.contains("# phrust-server MVP internal metrics"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_requests_total"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_php_responses_total"),
        "{response}"
    );
    // Queue-wait/admission scalability signal: every served request records the
    // in-flight admission wait as its own request phase.
    assert!(
        response.contains("phrust_server_request_phase_count{phase=\"admission_wait\"}"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_request_phase_nanos_total{phase=\"admission_wait\"}"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_script_cache_hits_total"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_script_cache_stale_invalidations_total"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_script_cache_compile_errors_total"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_include_resolution_hits_total"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_include_compile_hits_total"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_persistent_engine_policy_reuses_total"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_persistent_engine_immutable_metadata_reuses_total"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_persistent_engine_request_local_resets_total"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_persistent_engine_feedback_templates"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_persistent_engine_feedback_template_instantiations_total"),
        "{response}"
    );
    assert!(
        response.contains("phrust_server_persistent_engine_feedback_template_absorptions_total"),
        "{response}"
    );
    assert!(
        response.contains(
            "phrust_server_persistent_engine_rejected_persistence_total{reason=\"request_local_state\"}"
        ),
        "{response}"
    );
}

#[test]
fn server_reuses_compiled_script_cache_for_repeated_php_requests() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let first_response = http_request(&address, "GET", "/hello.php");
    let second_response = http_request(&address, "GET", "/hello.php");
    let metrics = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);

    assert!(
        first_response.starts_with("HTTP/1.1 200 OK"),
        "{first_response}"
    );
    assert!(
        second_response.starts_with("HTTP/1.1 200 OK"),
        "{second_response}"
    );
    assert!(
        metrics.contains("phrust_server_script_cache_hits_total 1\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_script_cache_misses_total 1\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_persistent_engine_immutable_metadata_reuses_total 1\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_persistent_engine_request_local_resets_total 2\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains(
            "phrust_server_persistent_engine_rejected_persistence_total{reason=\"request_local_state\"} 2\n"
        ),
        "{metrics}"
    );
}

#[test]
fn server_protects_metrics_with_configured_token() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &["--metrics-token", "secret"]);

    let address = read_listening_address(&mut child);
    let forbidden = http_request(&address, "GET", "/__phrust/metrics");
    let authorized = http_request_with_headers(
        &address,
        "GET",
        "/__phrust/metrics",
        &[("Authorization", "Bearer secret")],
        "",
    );

    stop_child(child);

    assert!(
        forbidden.starts_with("HTTP/1.1 403 Forbidden"),
        "{forbidden}"
    );
    assert!(authorized.starts_with("HTTP/1.1 200 OK"), "{authorized}");
    assert!(
        authorized.contains("phrust_server_requests_total"),
        "{authorized}"
    );
}

#[test]
fn server_writes_compact_access_log_line() {
    let docroot = temp_docroot();
    let log_path = docroot.join("access.log");
    let log_arg = log_path.to_string_lossy().to_string();
    fs::write(docroot.join("static.txt"), "static bytes\n").expect("write static fixture");
    let mut child = start_server(&docroot, &["--access-log", &log_arg]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/static.txt?cache=1");

    stop_child(child);
    let log = fs::read_to_string(&log_path).expect("read access log");
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(log.contains("method=GET"), "{log}");
    assert!(log.contains("path=\"/static.txt?cache=1\""), "{log}");
    assert!(log.contains("status=200"), "{log}");
    assert!(log.contains("emitted_bytes=13"), "{log}");
    assert!(log.contains("route=static"), "{log}");
    assert!(log.contains("cache=-"), "{log}");
    assert!(log.contains("outcome=completed"), "{log}");
}

#[test]
fn server_request_profile_alone_stays_in_summary_mode() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("summary.php"),
        "<?php\necho strtoupper('ok');\n",
    )
    .expect("write summary fixture");
    let profile_dir = temp_docroot();
    let profile_arg = profile_dir.to_string_lossy().to_string();
    let mut child = start_server(&docroot, &["--request-profile", &profile_arg]);

    let address = read_listening_address(&mut child);
    let response = http_request_with_headers(
        &address,
        "GET",
        "/summary.php",
        &[("x-phrust-request-profile", "1")],
        "",
    );

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_eq!(response_body(&response), "OK");
    let profile_path = wait_for_request_profile(&profile_dir);
    stop_child(child);
    let profile: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(profile_path).expect("read profile json"))
            .expect("parse profile json");
    // A request profile alone must not pay native hot-counter overhead; it
    // still records phase timings.
    assert_eq!(profile["schema_version"], serde_json::Value::from(6));
    assert_eq!(profile["native"], serde_json::json!({}));
    assert!(profile["phases_nanos"].is_object());

    fs::remove_dir_all(profile_dir).expect("remove profile dir");
    fs::remove_dir_all(docroot).expect("remove temp docroot");
}

#[test]
fn server_request_profile_without_trigger_header_does_not_write_profile() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("summary.php"),
        "<?php\necho strtoupper('ok');\n",
    )
    .expect("write summary fixture");
    let profile_dir = temp_docroot();
    let profile_arg = profile_dir.to_string_lossy().to_string();
    let mut child = start_server(&docroot, &["--request-profile", &profile_arg]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/summary.php");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_eq!(response_body(&response), "OK");
    let profiles = fs::read_dir(&profile_dir)
        .expect("read profile dir")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "json")
        })
        .count();
    assert_eq!(profiles, 0);

    fs::remove_dir_all(profile_dir).expect("remove profile dir");
    fs::remove_dir_all(docroot).expect("remove temp docroot");
}

#[test]
fn server_request_profile_vm_counter_mode_collects_native_counters() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("lib.php"),
        "<?php\nfunction profile_helper(array $items) { return count($items); }\nclass ProfileThing { public $name = 'thing'; public function name() { return $this->name; } }\n",
    )
    .expect("write include fixture");
    fs::write(
        docroot.join("profile.php"),
        "<?php\nfunction profile_direct(int $value): int { return $value + 1; }\ninclude __DIR__ . '/lib.php';\nob_start();\n$items = ['a' => 1, 'b' => 2];\n$thing = new ProfileThing();\necho profile_helper($items) + $items['a'];\necho $thing->name();\necho strtoupper('x');\necho profile_direct(1);\necho ob_get_clean();\n",
    )
    .expect("write profile fixture");
    let profile_dir = temp_docroot();
    let profile_arg = profile_dir.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--request-profile",
            &profile_arg,
            "--request-profile-vm-counters",
        ],
    );

    let address = read_listening_address(&mut child);
    let response = http_request_with_headers(
        &address,
        "GET",
        "/profile.php",
        &[("x-phrust-request-profile", "1")],
        "",
    );

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_eq!(response_body(&response), "3thingX2");
    let profile_path = wait_for_request_profile(&profile_dir);
    stop_child(child);
    let profile: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(profile_path).expect("read profile json"))
            .expect("parse profile json");
    assert_eq!(profile["schema_version"], serde_json::Value::from(6));
    assert!(
        profile["native"]["execution_entries"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "native execution must be visible in the request profile: {profile}"
    );
    for counter in [
        "compile_successes",
        "compile_time_nanos",
        "cache_hits",
        "cache_misses",
        "cache_compile_waits",
        "cache_evictions",
        "region_side_exits",
        "runtime_helper_calls",
        "runtime_helper_time_nanos",
        "execution_time_nanos",
        "call_direct",
        "call_dynamic",
        "transition_count",
        "transition_time_nanos",
        "runtime_helper_object_release_fast_paths",
        "runtime_helper_object_release_root_scans",
        "value_encodes",
        "value_decodes",
        "value_table_allocations",
        "value_table_reuses",
        "value_table_high_water",
        "ssa_promoted_locals",
        "ssa_promoted_registers",
        "ownership_moves",
        "versions_published",
    ] {
        assert!(
            profile["native"][counter].is_u64(),
            "native counter {counter} must be present: {profile}"
        );
    }
    for counter in [
        "runtime_helper_calls_by_id",
        "runtime_helper_time_nanos_by_id",
        "transition_by_reason",
        "transition_time_nanos_by_reason",
        "runtime_helper_local_read_by_reason",
        "runtime_helper_local_store_by_reason",
        "runtime_helper_truthy_by_value_class",
        "runtime_helper_retain_by_reason",
        "runtime_helper_release_by_reason",
        "runtime_helper_object_release_root_scans_by_reason",
        "call_dynamic_by_reason",
        "slow_path_entries_by_reason",
    ] {
        assert!(
            profile["native"][counter].is_object(),
            "native counter {counter} must be present: {profile}"
        );
    }
    let helper_calls = profile["native"]["runtime_helper_calls"]
        .as_u64()
        .expect("runtime helper call total");
    let helper_call_sum = profile["native"]["runtime_helper_calls_by_id"]
        .as_object()
        .expect("runtime helper call breakdown")
        .values()
        .map(|value| value.as_u64().expect("runtime helper count"))
        .sum::<u64>();
    assert!(
        helper_calls > 0,
        "fixture must cross native helpers: {profile}"
    );
    assert_eq!(
        helper_calls, helper_call_sum,
        "helper count totals: {profile}"
    );
    let helper_time = profile["native"]["runtime_helper_time_nanos"]
        .as_u64()
        .expect("runtime helper time total");
    let helper_time_sum = profile["native"]["runtime_helper_time_nanos_by_id"]
        .as_object()
        .expect("runtime helper time breakdown")
        .values()
        .map(|value| value.as_u64().expect("runtime helper time"))
        .sum::<u64>();
    assert!(
        helper_time > 0,
        "fixture must measure native helpers: {profile}"
    );
    assert_eq!(
        helper_time, helper_time_sum,
        "helper time totals: {profile}"
    );

    fs::remove_dir_all(profile_dir).expect("remove profile dir");
    fs::remove_dir_all(docroot).expect("remove temp docroot");
}

#[test]
fn server_debug_log_records_request_timeline_and_redacts_secrets() {
    let docroot = fixture_docroot("fixtures/server/php");
    let debug_dir = temp_docroot();
    let debug_log = debug_dir.join("server-debug.jsonl");
    let debug_log_arg = debug_log.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--debug",
            "--error-format",
            "json",
            "--debug-log",
            &debug_log_arg,
        ],
    );

    let address = read_listening_address(&mut child);
    let response = http_request_with_headers(
        &address,
        "GET",
        "/hello.php?token=secret-token",
        &[
            ("Authorization", "Bearer secret-token"),
            ("Cookie", "PHPSESSID=session-secret"),
        ],
        "",
    );

    let log = wait_for_file_to_contain(&debug_log, "D_PHRUST_SERVER_RESPONSE");
    stop_child(child);
    fs::remove_dir_all(debug_dir).expect("remove debug temp dir");

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(log.contains("\"request_id\":\"req-00000001\""), "{log}");
    assert!(log.contains("D_PHRUST_SERVER_REQUEST_ACCEPTED"), "{log}");
    assert!(log.contains("D_PHRUST_SERVER_ROUTE_RESOLVED"), "{log}");
    assert!(log.contains("D_PHRUST_SERVER_SCRIPT_CACHE_END"), "{log}");
    assert!(log.contains("D_PHRUST_SERVER_EXECUTE_END"), "{log}");
    assert!(log.contains("D_PHRUST_SERVER_RESPONSE"), "{log}");
    assert!(log.contains("\"cache_hit\":\"false\""), "{log}");
    assert!(log.contains("\"status\":\"200\""), "{log}");
    assert!(!log.contains("Bearer secret-token"), "{log}");
    assert!(!log.contains("session-secret"), "{log}");
    assert!(!log.contains("secret-token"), "{log}");

    for line in log.lines() {
        let value: serde_json::Value = serde_json::from_str(line).expect("debug JSON line");
        for key in [
            "kind",
            "schema_version",
            "code",
            "layer",
            "phase",
            "message",
        ] {
            assert!(value.get(key).is_some(), "missing {key} in {line}");
        }
        assert_eq!(value["kind"], "debug_event");
    }
}

#[test]
fn server_debug_log_contains_request_failure_diagnostics_without_secrets() {
    let docroot = temp_docroot();
    fs::write(docroot.join("index.php"), "<?php echo 'ok';").expect("write index");
    let outside = docroot.with_extension("outside-diagnostic");
    fs::write(&outside, "secret").expect("write outside file");
    std::os::unix::fs::symlink(&outside, docroot.join("link.php")).expect("create symlink");
    let debug_dir = temp_docroot();
    let debug_log = debug_dir.join("server-debug.jsonl");
    let debug_log_arg = debug_log.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--debug",
            "--error-format",
            "json",
            "--debug-log",
            &debug_log_arg,
        ],
    );

    let address = read_listening_address(&mut child);
    let missing = http_request_with_headers(
        &address,
        "GET",
        "/missing.php?token=secret-token",
        &[("Cookie", "PHPSESSID=session-secret")],
        "",
    );
    let forbidden = http_request(&address, "GET", "/link.php");
    let multipart = http_request_with_headers(
        &address,
        "POST",
        "/index.php",
        &[
            ("Content-Type", "multipart/form-data"),
            ("Content-Length", "24"),
            ("Cookie", "PHPSESSID=session-secret"),
        ],
        "secret-body-never-log-it",
    );

    stop_child(child);
    let log = fs::read_to_string(&debug_log).expect("read debug log");
    fs::remove_dir_all(debug_dir).expect("remove debug temp dir");
    fs::remove_dir_all(docroot).expect("remove temp docroot");
    fs::remove_file(outside).expect("remove outside file");

    assert!(missing.starts_with("HTTP/1.1 404 Not Found"), "{missing}");
    assert!(
        forbidden.starts_with("HTTP/1.1 404 Not Found"),
        "{forbidden}"
    );
    assert!(multipart.starts_with("HTTP/1.1 200 OK"), "{multipart}");
    assert!(
        log.contains("E_PHP_SERVER_SCRIPT_RESOLUTION_FAILED"),
        "{log}"
    );
    assert!(!log.contains("E_PHP_SERVER_OUTSIDE_DOCUMENT_ROOT"), "{log}");
    assert!(log.contains("\"method\":\"GET\""), "{log}");
    assert!(
        log.contains("\"uri\":\"/missing.php?token=secret-token\""),
        "{log}"
    );
    assert!(log.contains("\"document_root\""), "{log}");
    assert!(log.contains("\"allowed_roots\""), "{log}");
    assert!(log.contains("\"function_name\":\"resolve_route\""), "{log}");
    assert!(!log.contains("session-secret"), "{log}");
    assert!(!log.contains("secret-body-never-log-it"), "{log}");
}

#[test]
fn server_debug_log_samples_successful_runtime_diagnostics() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("warn.php"),
        "<?php $items = []; echo $items['missing']; echo 'after';",
    )
    .expect("write warning fixture");
    let debug_dir = temp_docroot();
    let debug_log = debug_dir.join("server-debug.jsonl");
    let debug_log_arg = debug_log.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--debug",
            "--error-format",
            "json",
            "--debug-log",
            &debug_log_arg,
        ],
    );

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/warn.php");

    let log = wait_for_file_to_contain(&debug_log, "D_PHRUST_SERVER_EXECUTE_END");
    stop_child(child);
    fs::remove_dir_all(debug_dir).expect("remove debug temp dir");
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(log.contains("D_PHRUST_SERVER_EXECUTE_END"), "{log}");
    assert!(
        log.contains("runtime_diagnostic_samples"),
        "missing diagnostic samples in {log}"
    );
    assert!(
        log.contains("E_PHP_RUNTIME_UNDEFINED_ARRAY_KEY_WARNING"),
        "{log}"
    );
    assert!(log.contains("missing"), "{log}");
}

#[test]
fn server_debug_off_does_not_emit_request_timeline() {
    let docroot = fixture_docroot("fixtures/server/php");
    let debug_dir = temp_docroot();
    let debug_log = debug_dir.join("server-debug.jsonl");
    let debug_log_arg = debug_log.to_string_lossy().to_string();
    let mut child = start_server(&docroot, &["--debug-log", &debug_log_arg]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/hello.php");

    stop_child(child);
    let log_exists = debug_log.exists();
    fs::remove_dir_all(debug_dir).expect("remove debug temp dir");

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(
        !log_exists,
        "debug log should not be written unless --debug is set"
    );
}

#[test]
fn server_executes_php_script() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/hello.php");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.ends_with("hello\n"), "{response}");
}

#[test]
fn php_flush_makes_first_chunk_visible_before_script_end() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);
    let mut stream = TcpStream::connect(&address).expect("connect streaming request");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set streaming read timeout");
    stream
        .write_all(
            b"GET /stream_flush.php HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .expect("write streaming request");

    let mut received = Vec::new();
    let mut buffer = [0_u8; 256];
    while !received
        .windows(b"first\n".len())
        .any(|window| window == b"first\n")
    {
        let read = stream
            .read(&mut buffer)
            .expect("read first streaming chunk");
        assert_ne!(read, 0, "response ended before first chunk");
        received.extend_from_slice(&buffer[..read]);
    }
    assert!(
        !received
            .windows(b"second\n".len())
            .any(|window| window == b"second\n"),
        "first and second chunks arrived together instead of at flush: {}",
        String::from_utf8_lossy(&received)
    );
    stream
        .read_to_end(&mut received)
        .expect("read remaining streaming response");

    stop_child(child);

    let response = String::from_utf8_lossy(&received);
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.contains("first\n"), "{response}");
    assert!(response.contains("second\n"), "{response}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn php_flush_makes_first_chunk_visible_over_http2() {
    let docroot = fixture_docroot("fixtures/server/php");
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(&docroot, &["--tls-cert", &cert_arg, "--tls-key", &key_arg]);
    let address = read_listening_address(&mut child);

    let (status, chunks, first_elapsed) = http2_get(&address, "/stream_flush.php").await;

    stop_child(child);
    assert_eq!(status, hyper::StatusCode::OK);
    assert_eq!(chunks.concat(), b"first\nsecond\n");
    assert!(chunks.len() >= 2, "flush response arrived in one H2 frame");
    assert!(
        first_elapsed < Duration::from_millis(400),
        "first H2 chunk arrived only after script delay: {first_elapsed:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn php_flush_makes_first_chunk_visible_over_http3() {
    let docroot = fixture_docroot("fixtures/server/php");
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--enable-http3",
        ],
    );
    let address = read_listening_address(&mut child);

    let (status, chunks, first_elapsed) = http3_get(&address, "/stream_flush.php").await;

    stop_child(child);
    assert_eq!(status, hyper::StatusCode::OK);
    assert_eq!(chunks.concat(), b"first\nsecond\n");
    assert!(chunks.len() >= 2, "flush response arrived in one H3 frame");
    assert!(
        first_elapsed < Duration::from_millis(400),
        "first H3 chunk arrived only after script delay: {first_elapsed:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn static_semantics_match_over_http1_http2_and_http3() {
    let docroot = temp_docroot();
    fs::write(docroot.join("asset.a1b2c3d4.txt"), vec![b'x'; 192 * 1024])
        .expect("write large identity fixture");
    fs::write(
        docroot.join("asset.a1b2c3d4.txt.br"),
        vec![b'b'; 192 * 1024],
    )
    .expect("write large br fixture");
    fs::write(docroot.join("asset.a1b2c3d4.txt.zst"), "zstd").expect("write zstd fixture");
    fs::write(docroot.join("asset.a1b2c3d4.txt.gz"), "gzip").expect("write gzip fixture");
    fs::create_dir(docroot.join("html")).expect("create HTML index directory");
    fs::write(docroot.join("html/index.html"), "html-index").expect("write HTML index");
    fs::create_dir(docroot.join("php")).expect("create PHP index directory");
    fs::write(docroot.join("php/index.php"), "<?php echo 'php-index';").expect("write PHP index");
    fs::write(docroot.join(".env"), "secret").expect("write hidden fixture");
    #[cfg(unix)]
    let outside = {
        let outside = docroot.with_extension("h2-h3-outside");
        fs::write(&outside, "outside-secret").expect("write outside protocol fixture");
        std::os::unix::fs::symlink(&outside, docroot.join("outside.txt"))
            .expect("create outside protocol symlink");
        outside
    };
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--enable-http3",
            "--deployment-mode",
            "immutable",
        ],
    );
    let address = read_listening_address(&mut child);
    let path = "/asset.a1b2c3d4.txt";

    let identity = protocol_request_set(&address, hyper::Method::GET, path, &[]).await;
    for response in &identity {
        assert_eq!(response.status, hyper::StatusCode::OK);
        assert_eq!(response.body().len(), 192 * 1024);
        assert!(response.chunks.len() >= 2, "{response:?}");
        assert_eq!(
            response.header(hyper::header::X_CONTENT_TYPE_OPTIONS),
            Some("nosniff")
        );
        assert_eq!(
            response.header(hyper::header::CACHE_CONTROL),
            Some("public, max-age=31536000, immutable")
        );
        assert_eq!(
            response.header(hyper::header::VARY),
            Some("Accept-Encoding")
        );
    }
    let etag = identity[0]
        .header(hyper::header::ETAG)
        .expect("H2 ETag")
        .to_owned();
    assert_eq!(identity[1].header(hyper::header::ETAG), Some(etag.as_str()));

    let head = protocol_request_set(&address, hyper::Method::HEAD, path, &[]).await;
    for response in &head {
        assert_eq!(response.status, hyper::StatusCode::OK);
        assert!(response.body().is_empty());
        assert_eq!(
            response.header(hyper::header::CONTENT_LENGTH),
            Some((192 * 1024).to_string().as_str())
        );
        assert_eq!(
            response.header(hyper::header::X_CONTENT_TYPE_OPTIONS),
            Some("nosniff")
        );
        assert_eq!(
            response.header(hyper::header::VARY),
            Some("Accept-Encoding")
        );
    }

    for (coding, body) in [("zstd", b"zstd".as_slice()), ("gzip", b"gzip")] {
        let responses = protocol_request_set(
            &address,
            hyper::Method::GET,
            path,
            &[("accept-encoding", coding)],
        )
        .await;
        for response in responses {
            assert_eq!(response.status, hyper::StatusCode::OK);
            assert_eq!(
                response.header(hyper::header::CONTENT_ENCODING),
                Some(coding)
            );
            assert_eq!(
                response.header(hyper::header::X_CONTENT_TYPE_OPTIONS),
                Some("nosniff")
            );
            assert_eq!(
                response.header(hyper::header::VARY),
                Some("Accept-Encoding")
            );
            assert_eq!(response.body(), body);
        }
    }
    let brotli = protocol_request_set(
        &address,
        hyper::Method::GET,
        path,
        &[("accept-encoding", "br")],
    )
    .await;
    for response in brotli {
        assert_eq!(response.status, hyper::StatusCode::OK);
        assert_eq!(response.header(hyper::header::CONTENT_ENCODING), Some("br"));
        assert_eq!(response.body().len(), 192 * 1024);
        assert!(response.chunks.len() >= 2, "{response:?}");
    }

    let range = protocol_request_set(
        &address,
        hyper::Method::GET,
        path,
        &[("range", "bytes=2-5"), ("if-range", &etag)],
    )
    .await;
    for response in range {
        assert_eq!(response.status, hyper::StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.body(), b"xxxx");
        assert_eq!(
            response.header(hyper::header::CONTENT_RANGE),
            Some("bytes 2-5/196608")
        );
        assert_eq!(
            response.header(hyper::header::X_CONTENT_TYPE_OPTIONS),
            Some("nosniff")
        );
        assert_eq!(
            response.header(hyper::header::VARY),
            Some("Accept-Encoding")
        );
    }

    let not_modified = protocol_request_set(
        &address,
        hyper::Method::GET,
        path,
        &[("if-none-match", &etag)],
    )
    .await;
    let failed = protocol_request_set(
        &address,
        hyper::Method::GET,
        path,
        &[("if-match", "\"wrong\"")],
    )
    .await;
    let unsatisfiable = protocol_request_set(
        &address,
        hyper::Method::GET,
        path,
        &[("range", "bytes=999999-")],
    )
    .await;
    let unacceptable = protocol_request_set(
        &address,
        hyper::Method::GET,
        path,
        &[("accept-encoding", "identity;q=0,br;q=0,zstd;q=0,gzip;q=0")],
    )
    .await;
    let redirect = protocol_request_set(&address, hyper::Method::GET, "/html?x=1", &[]).await;
    let html = protocol_request_set(&address, hyper::Method::GET, "/html/", &[]).await;
    let php = protocol_request_set(&address, hyper::Method::GET, "/php/", &[]).await;
    let hidden = protocol_request_set(&address, hyper::Method::GET, "/.env", &[]).await;
    #[cfg(unix)]
    let escaped = protocol_request_set(&address, hyper::Method::GET, "/outside.txt", &[]).await;
    let if_range_mismatch = protocol_request_set(
        &address,
        hyper::Method::GET,
        path,
        &[("range", "bytes=2-5"), ("if-range", "\"wrong\"")],
    )
    .await;

    stop_child(child);
    let mut dev_child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--enable-http3",
            "--deployment-mode",
            "dev",
        ],
    );
    let dev_address = read_listening_address(&mut dev_child);
    let dev = protocol_request_set(&dev_address, hyper::Method::GET, path, &[]).await;
    stop_child(dev_child);

    for response in not_modified {
        assert_eq!(response.status, hyper::StatusCode::NOT_MODIFIED);
        assert!(response.body().is_empty());
        assert_eq!(
            response.header(hyper::header::VARY),
            Some("Accept-Encoding")
        );
        assert_eq!(
            response.header(hyper::header::X_CONTENT_TYPE_OPTIONS),
            Some("nosniff")
        );
    }
    for response in failed {
        assert_eq!(response.status, hyper::StatusCode::PRECONDITION_FAILED);
        assert!(response.body().is_empty());
        assert_eq!(
            response.header(hyper::header::VARY),
            Some("Accept-Encoding")
        );
        assert_eq!(
            response.header(hyper::header::X_CONTENT_TYPE_OPTIONS),
            Some("nosniff")
        );
    }
    for response in unsatisfiable {
        assert_eq!(response.status, hyper::StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(
            response.header(hyper::header::CONTENT_RANGE),
            Some("bytes */196608")
        );
        assert_eq!(
            response.header(hyper::header::VARY),
            Some("Accept-Encoding")
        );
        assert_eq!(
            response.header(hyper::header::X_CONTENT_TYPE_OPTIONS),
            Some("nosniff")
        );
    }
    for response in unacceptable {
        assert_eq!(response.status, hyper::StatusCode::NOT_ACCEPTABLE);
        assert_eq!(
            response.header(hyper::header::VARY),
            Some("Accept-Encoding")
        );
        assert_eq!(
            response.header(hyper::header::X_CONTENT_TYPE_OPTIONS),
            Some("nosniff")
        );
    }
    for response in redirect {
        assert_eq!(response.status, hyper::StatusCode::PERMANENT_REDIRECT);
        assert_eq!(response.header(hyper::header::LOCATION), Some("/html/?x=1"));
    }
    for response in html {
        assert_eq!(response.status, hyper::StatusCode::OK);
        assert_eq!(response.body(), b"html-index");
        assert_eq!(
            response.header(hyper::header::CACHE_CONTROL),
            Some("no-cache")
        );
        assert_eq!(
            response.header(hyper::header::X_CONTENT_TYPE_OPTIONS),
            Some("nosniff")
        );
    }
    for response in php {
        assert_eq!(response.status, hyper::StatusCode::OK);
        assert_eq!(response.body(), b"php-index");
    }
    for response in hidden {
        assert_eq!(response.status, hyper::StatusCode::NOT_FOUND);
        assert!(!response.body().windows(6).any(|window| window == b"secret"));
    }
    #[cfg(unix)]
    for response in escaped {
        assert_eq!(response.status, hyper::StatusCode::NOT_FOUND);
        assert!(
            !response
                .body()
                .windows(b"outside-secret".len())
                .any(|window| window == b"outside-secret")
        );
    }
    for response in if_range_mismatch {
        assert_eq!(response.status, hyper::StatusCode::OK);
        assert_eq!(response.body().len(), 192 * 1024);
    }
    for response in dev {
        assert_eq!(response.status, hyper::StatusCode::OK);
        assert_eq!(
            response.header(hyper::header::CACHE_CONTROL),
            Some("no-cache")
        );
        assert!(
            response
                .header(hyper::header::ETAG)
                .is_some_and(|etag| etag.starts_with("W/"))
        );
    }

    fs::remove_dir_all(docroot).expect("remove temp docroot");
    #[cfg(unix)]
    fs::remove_file(outside).expect("remove outside protocol fixture");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn static_client_aborts_finalize_over_http1_http2_and_http3() {
    let docroot = temp_docroot();
    let large = fs::File::create(docroot.join("large.bin")).expect("create abort fixture");
    large
        .set_len(256 * 1024 * 1024)
        .expect("size abort fixture");
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--enable-http3",
            "--deployment-mode",
            "immutable",
        ],
    );
    let address = read_listening_address(&mut child);

    abort_http1_response(&address, "/large.bin").await;
    wait_for_metric_value(&address, "phrust_server_transfers_aborted_total", 1).await;
    abort_http2_response(&address, "/large.bin").await;
    wait_for_metric_value(&address, "phrust_server_transfers_aborted_total", 2).await;
    abort_http3_response(&address, "/large.bin").await;
    wait_for_metric_value(&address, "phrust_server_transfers_aborted_total", 3).await;

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove abort docroot");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oversized_http3_request_body_returns_413() {
    let docroot = temp_docroot();
    fs::write(docroot.join("post.php"), "<?php echo \"unexpected\\n\";\n")
        .expect("write H3 POST fixture");
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--enable-http3",
            "--max-body-bytes",
            "8",
        ],
    );
    let address = read_listening_address(&mut child);

    let status = http3_post_status(&address, "/post.php", b"0123456789abcdef").await;

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");
    assert_eq!(status, hyper::StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn php_request_bodies_match_over_http1_http2_and_http3() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("raw.php"),
        "<?php $first = file_get_contents('php://input'); $second = file_get_contents('php://input'); echo strlen($first), \"\\n\", strlen($second), \"\\n\";",
    )
    .expect("write raw body fixture");
    fs::write(
        docroot.join("form.php"),
        "<?php echo strlen($_POST['value']), \"\\n\", ord($_POST['value'][1]), \"\\n\", $_POST['nested']['key'], \"\\n\", implode(',', $_POST['list']), \"\\n\";",
    )
    .expect("write form fixture");
    fs::write(
        docroot.join("multipart.php"),
        "<?php echo $_POST['title'], \"\\n\", $_POST['nested']['key'], \"\\n\", $_FILES['avatar']['name'], \"\\n\", $_FILES['avatar']['size'], \"\\n\", $_FILES['avatar']['error'], \"\\n\", file_get_contents($_FILES['avatar']['tmp_name']), \"\\n\";",
    )
    .expect("write automatic multipart fixture");
    fs::write(
        docroot.join("parse.php"),
        "<?php [$post, $files] = request_parse_body(); echo $post['title'], \"\\n\", $files['avatar']['name'], \"\\n\", file_get_contents($files['avatar']['tmp_name']), \"\\n\", count($_POST), \"\\n\", count($_FILES), \"\\n\";",
    )
    .expect("write explicit multipart fixture");
    let spool_dir = temp_docroot();
    let upload_dir = temp_docroot();
    let spool_arg = spool_dir.to_string_lossy().to_string();
    let upload_arg = upload_dir.to_string_lossy().to_string();
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--enable-http3",
            "--max-body-bytes",
            "350000",
            "--request-body-memory-bytes",
            "128",
            "--request-body-temp-dir",
            &spool_arg,
            "--upload-temp-dir",
            &upload_arg,
        ],
    );
    let address = read_listening_address(&mut child);

    let raw = vec![b'x'; 300 * 1024];
    let raw_responses = protocol_request_body_set(
        &address,
        hyper::Method::POST,
        "/raw.php",
        &[("content-type", "application/octet-stream")],
        &raw,
    )
    .await;
    assert_protocol_bodies(&raw_responses, b"307200\n307200\n");

    let form = b"value=A%00B&nested%5Bkey%5D=inside&list%5B%5D=one&list%5B%5D=two";
    let form_responses = protocol_request_body_set(
        &address,
        hyper::Method::POST,
        "/form.php",
        &[("content-type", "application/x-www-form-urlencoded")],
        form,
    )
    .await;
    assert_protocol_bodies(&form_responses, b"3\n0\ninside\none,two\n");

    let multipart = b"--BOUNDARY\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nHello\r\n--BOUNDARY\r\nContent-Disposition: form-data; name=\"nested[key]\"\r\n\r\ninside\r\n--BOUNDARY\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"../me.txt\"\r\nContent-Type: text/plain\r\n\r\nUPLOAD\0DATA\r\n--BOUNDARY--\r\n";
    let multipart_responses = protocol_request_body_set(
        &address,
        hyper::Method::POST,
        "/multipart.php",
        &[("content-type", "multipart/form-data; boundary=BOUNDARY")],
        multipart,
    )
    .await;
    assert_protocol_bodies(
        &multipart_responses,
        b"Hello\ninside\nme.txt\n11\n0\nUPLOAD\0DATA\n",
    );

    let parse_responses = protocol_request_body_set(
        &address,
        hyper::Method::PUT,
        "/parse.php",
        &[("content-type", "multipart/form-data; boundary=BOUNDARY")],
        multipart,
    )
    .await;
    assert_protocol_bodies(&parse_responses, b"Hello\nme.txt\nUPLOAD\0DATA\n0\n0\n");

    let oversized = vec![b'z'; 400_000];
    let oversized_responses = protocol_request_body_set(
        &address,
        hyper::Method::POST,
        "/raw.php",
        &[("content-type", "application/octet-stream")],
        &oversized,
    )
    .await;
    for response in oversized_responses {
        assert_eq!(response.status, hyper::StatusCode::PAYLOAD_TOO_LARGE);
    }

    let metrics = http2_request(&address, hyper::Method::GET, "/__phrust/metrics", &[], &[]).await;
    let metrics = String::from_utf8(metrics.body()).expect("metrics are UTF-8");
    for expected in [
        "phrust_server_request_body_tempfiles_active 0\n",
        "phrust_server_request_body_tempfile_bytes_active 0\n",
        "phrust_server_upload_tempfiles_active 0\n",
        "phrust_server_upload_tempfile_bytes_active 0\n",
    ] {
        assert!(
            metrics.contains(expected),
            "missing {expected:?}: {metrics}"
        );
    }

    stop_child(child);
    assert_eq!(fs::read_dir(&spool_dir).expect("read spool dir").count(), 0);
    assert_eq!(
        fs::read_dir(&upload_dir).expect("read upload dir").count(),
        0
    );
    fs::remove_dir_all(docroot).expect("remove protocol body docroot");
    fs::remove_dir_all(spool_dir).expect("remove protocol body spool dir");
    fs::remove_dir_all(upload_dir).expect("remove protocol upload dir");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn php_transport_metadata_and_response_finalization_match_all_protocols() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("metadata.php"),
        "<?php foreach (['HTTP_HOST','SERVER_NAME','SERVER_PORT','REQUEST_SCHEME','HTTPS','REMOTE_ADDR','SERVER_ADDR','SERVER_PROTOCOL'] as $key) echo $key, '=', $_SERVER[$key] ?? '', \"\\n\";",
    )
    .expect("write metadata fixture");
    fs::write(
        docroot.join("headers.php"),
        "<?php header('Connection: X-Remove'); header('X-Remove: secret'); header('Keep-Alive: timeout=5'); header('Proxy-Connection: close'); header('Transfer-Encoding: chunked'); header('Upgrade: websocket'); header('Trailer: X-End'); echo \"clean\\n\";",
    )
    .expect("write response-header fixture");
    fs::write(
        docroot.join("bodyless.php"),
        "<?php http_response_code((int)($_GET['status'] ?? 204)); echo \"forbidden-body\\n\";",
    )
    .expect("write bodyless fixture");
    let (cert, key) = tls_fixture_paths();
    let cert_arg = cert.to_string_lossy().to_string();
    let key_arg = key.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--tls-cert",
            &cert_arg,
            "--tls-key",
            &key_arg,
            "--enable-http3",
        ],
    );
    let address = read_listening_address(&mut child);
    let port = address.rsplit_once(':').unwrap().1;

    let metadata = protocol_request_set(&address, hyper::Method::GET, "/metadata.php", &[]).await;
    for (index, response) in metadata.iter().enumerate() {
        assert_eq!(response.status, hyper::StatusCode::OK, "{response:?}");
        let body = String::from_utf8(response.body()).unwrap();
        for line in [
            "HTTP_HOST=localhost",
            "SERVER_NAME=localhost",
            &format!("SERVER_PORT={port}"),
            "REQUEST_SCHEME=https",
            "HTTPS=on",
            "REMOTE_ADDR=127.0.0.1",
            "SERVER_ADDR=127.0.0.1",
        ] {
            assert!(body.contains(line), "missing {line:?}: {body}");
        }
        let protocol = ["HTTP/1.1", "HTTP/2.0", "HTTP/3.0"][index];
        assert!(
            body.contains(&format!("SERVER_PROTOCOL={protocol}")),
            "{body}"
        );
    }

    let headers = protocol_request_set(&address, hyper::Method::GET, "/headers.php", &[]).await;
    for response in headers {
        assert_eq!(response.status, hyper::StatusCode::OK, "{response:?}");
        assert_eq!(response.body(), b"clean\n");
        for name in [
            "connection",
            "x-remove",
            "keep-alive",
            "proxy-connection",
            "transfer-encoding",
            "upgrade",
            "trailer",
        ] {
            assert!(!response.headers.contains_key(name), "{name}: {response:?}");
        }
    }

    for status in [204, 205, 304] {
        let path = format!("/bodyless.php?status={status}");
        let responses = protocol_request_set(&address, hyper::Method::GET, &path, &[]).await;
        for response in responses {
            assert_eq!(response.status.as_u16(), status, "{response:?}");
            assert!(response.body().is_empty(), "{response:?}");
            if status == 205 {
                assert_eq!(response.header(hyper::header::CONTENT_LENGTH), Some("0"));
            }
        }
    }

    let h2_conflict = http2_request(
        &address,
        hyper::Method::GET,
        "/healthz",
        &[("host", "example.test")],
        &[],
    )
    .await;
    assert_eq!(h2_conflict.status, hyper::StatusCode::BAD_REQUEST);

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove transport metadata docroot");
}

#[test]
fn php_client_disconnect_cancels_execution_and_releases_worker() {
    let docroot = temp_docroot();
    let marker = docroot.join("disconnect-finished");
    let marker_php = marker
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\'', "\\'");
    fs::write(
        docroot.join("disconnect.php"),
        format!(
            "<?php echo \"first\\n\"; flush(); for ($i = 0; $i < 1000000000; $i++) {{}} file_put_contents('{marker_php}', 'finished');\n"
        ),
    )
    .expect("write disconnect fixture");
    fs::write(docroot.join("next.php"), "<?php echo \"next\\n\";\n")
        .expect("write follow-up fixture");
    let mut child = start_server(
        &docroot,
        &[
            "--max-in-flight",
            "2",
            "--cpu-execution-limit",
            "1",
            "--max-execution-ms",
            "10000",
        ],
    );
    let address = read_listening_address(&mut child);
    let mut stream = TcpStream::connect(&address).expect("connect disconnect request");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set disconnect response timeout");
    stream
        .write_all(b"GET /disconnect.php HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .expect("write disconnect request");
    let mut received = Vec::new();
    let mut buffer = [0_u8; 256];
    while !received.windows(6).any(|window| window == b"first\n") {
        let read = stream.read(&mut buffer).expect("read first PHP chunk");
        assert_ne!(read, 0, "stream ended before first PHP chunk");
        received.extend_from_slice(&buffer[..read]);
    }
    drop(stream);

    let started = Instant::now();
    let follow_up = http_request(&address, "GET", "/next.php");
    let elapsed = started.elapsed();
    let metrics = http_request(&address, "GET", "/__phrust/metrics");
    let marker_exists = marker.exists();

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");
    assert!(follow_up.starts_with("HTTP/1.1 200 OK"), "{follow_up}");
    assert_eq!(response_body(&follow_up), "next\n");
    assert!(
        elapsed < Duration::from_secs(2),
        "cancelled PHP worker was not released promptly: {elapsed:?}"
    );
    assert!(
        !marker_exists,
        "cancelled script reached its final statement"
    );
    assert!(
        metrics.contains("phrust_server_transfers_aborted_total 1\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_client_disconnect_cancellations_total 1\n"),
        "{metrics}"
    );
}

#[test]
fn ignore_user_abort_observes_disconnect_and_discards_followup_output() {
    let docroot = temp_docroot();
    let marker = docroot.join("ignore-user-abort-status");
    let marker_php = marker
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\'', "\\'");
    fs::write(
        docroot.join("ignore.php"),
        format!(
            "<?php ignore_user_abort(true); echo \"first\\n\"; flush(); usleep(300000); echo str_repeat('x', 1048576); file_put_contents('{marker_php}', connection_aborted() . ':' . connection_status());\n"
        ),
    )
    .expect("write ignore-user-abort fixture");
    let mut child = start_server(
        &docroot,
        &[
            "--max-in-flight",
            "2",
            "--cpu-execution-limit",
            "1",
            "--max-execution-ms",
            "5000",
        ],
    );
    let address = read_listening_address(&mut child);
    let mut stream = TcpStream::connect(&address).expect("connect ignore-user-abort request");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set ignore-user-abort response timeout");
    stream
        .write_all(b"GET /ignore.php HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .expect("write ignore-user-abort request");
    let mut received = Vec::new();
    let mut buffer = [0_u8; 256];
    while !received.windows(6).any(|window| window == b"first\n") {
        let read = stream.read(&mut buffer).expect("read first PHP chunk");
        assert_ne!(read, 0, "stream ended before first PHP chunk");
        received.extend_from_slice(&buffer[..read]);
    }
    drop(stream);

    let deadline = Instant::now() + Duration::from_secs(2);
    while !marker.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let status = fs::read_to_string(&marker).expect("ignored request completed and wrote status");
    let metrics = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");
    assert_eq!(status, "1:1");
    assert!(
        metrics.contains("phrust_server_client_disconnect_cancellations_total 1\n"),
        "{metrics}"
    );
}

#[test]
fn php_head_large_output_is_counted_without_a_body() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("large.php"),
        "<?php echo str_repeat('x', 10485760);\n",
    )
    .expect("write large HEAD fixture");
    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);

    let response = http_request(&address, "HEAD", "/large.php");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_response_contains_header(&response, "content-length", "10485760");
    assert_eq!(response_body(&response), "");
}

#[test]
fn php_headers_freeze_at_first_transport_flush() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("headers.php"),
        "<?php header('X-Before: yes'); echo \"first\\n\"; flush(); header('X-After: no'); echo headers_sent() ? \"sent\\n\" : \"open\\n\";\n",
    )
    .expect("write header commit fixture");
    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);

    let response = http_request(&address, "GET", "/headers.php");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_response_contains_header(&response, "x-before", "yes");
    assert_response_lacks_header(&response, "x-after", "no");
    assert!(response_body(&response).contains("first\n"), "{response}");
    assert!(response_body(&response).contains("sent\n"), "{response}");
    assert_eq!(response_header_count(&response, "content-length"), 0);
}

#[test]
fn php_fatal_before_commit_can_set_500() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("fatal.php"),
        "<?php phrust_missing_function();\n",
    )
    .expect("write pre-commit fatal fixture");
    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);

    let response = http_request(&address, "GET", "/fatal.php");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");
    assert!(
        response.starts_with("HTTP/1.1 500 Internal Server Error"),
        "{response}"
    );
}

#[test]
fn php_fatal_after_commit_does_not_rewrite_status() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("fatal.php"),
        "<?php echo \"first\\n\"; flush(); phrust_missing_function();\n",
    )
    .expect("write post-commit fatal fixture");
    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);

    let response = http_request(&address, "GET", "/fatal.php");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response_body(&response).contains("first\n"), "{response}");
}

#[test]
fn server_default_engine_profile_matches_baseline_output() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("profile.php"),
        "<?php $value = 2 + 4; echo $value, \"\\n\";\n",
    )
    .expect("write profile fixture");

    let mut default_child = start_server(&docroot, &["--engine-preset", "default"]);
    let default_address = read_listening_address(&mut default_child);
    let default_response = http_request(&default_address, "GET", "/profile.php");
    stop_child(default_child);

    let mut baseline_child = start_server(&docroot, &["--engine-preset", "baseline"]);
    let baseline_address = read_listening_address(&mut baseline_child);
    let baseline_response = http_request(&baseline_address, "GET", "/profile.php");
    stop_child(baseline_child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(
        default_response.starts_with("HTTP/1.1 200 OK"),
        "{default_response}"
    );
    assert!(
        baseline_response.starts_with("HTTP/1.1 200 OK"),
        "{baseline_response}"
    );
    assert_eq!(
        response_body(&default_response),
        response_body(&baseline_response)
    );
    assert_eq!(response_body(&default_response), "6\n");
}

#[test]
fn server_execution_deadline_returns_timeout_response_and_metric() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("loop.php"),
        "<?php while (true) { usleep(1000); }\n",
    )
    .expect("write loop fixture");
    let mut child = start_server(&docroot, &["--max-execution-ms", "1"]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/loop.php");
    let metrics = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(
        response.starts_with("HTTP/1.1 504 Gateway Timeout"),
        "{response}"
    );
    assert_eq!(response_body(&response), "php execution timeout\n");
    assert!(
        metrics.contains("phrust_server_execution_timeouts_total 1"),
        "{metrics}"
    );
}

#[test]
fn server_execution_deadline_leaves_short_script_unaffected() {
    let docroot = temp_docroot();
    fs::write(docroot.join("short.php"), "<?php echo \"short\\n\";\n")
        .expect("write short fixture");
    let mut child = start_server(&docroot, &["--max-execution-ms", "1000"]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/short.php");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_eq!(response_body(&response), "short\n");
}

#[test]
fn server_set_time_limit_zero_disables_mutable_native_deadline() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("deadline-reset.php"),
        "<?php set_time_limit(0); $i = 0; while ($i < 5) { usleep(1000); $i++; } echo \"done\\n\";\n",
    )
    .expect("write deadline reset fixture");
    let mut child = start_server(&docroot, &["--max-execution-ms", "1"]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/deadline-reset.php");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_eq!(response_body(&response), "done\n");
}

#[test]
fn server_reports_disabled_execution_deadline_metric() {
    let docroot = temp_docroot();
    fs::write(docroot.join("short.php"), "<?php echo \"short\\n\";\n")
        .expect("write short fixture");
    let mut child = start_server(&docroot, &["--disable-execution-deadline"]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/short.php");
    let metrics = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(
        metrics.contains("phrust_server_execution_deadline_disabled_total 1"),
        "{metrics}"
    );
}

#[test]
fn server_applies_php_response_header() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/header.php");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_response_contains_header(&response, "x-test", "yes");
    assert!(response.ends_with("ok\n"), "{response}");
}

#[test]
fn server_preserves_php_response_state_from_included_script() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/include_header.php");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 302 Found"), "{response}");
    assert_response_contains_header(&response, "location", "/included-next");
    assert_response_contains_header(&response, "x-include-state", "preserved");
    assert!(response.ends_with("included\n"), "{response}");
}

#[test]
fn server_replaces_php_response_header_by_default() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/header_replace.php");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_response_contains_header(&response, "x-test", "two");
    assert_response_lacks_header(&response, "x-test", "one");
}

#[test]
fn server_applies_php_response_status() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/status.php");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 201 Created"), "{response}");
    assert!(response.ends_with("created\n"), "{response}");
}

#[test]
fn server_exposes_headers_list_builtin() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/headers_list.php");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_response_contains_header(&response, "x-test", "yes");
    assert!(response.ends_with("X-Test: yes\n"), "{response}");
}

#[test]
fn server_preserves_multiple_set_cookie_headers() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/cookie_builtin.php");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_response_contains_header(
        &response,
        "set-cookie",
        "login=hello%20world; Path=/; Secure; HttpOnly; SameSite=Lax",
    );
    assert_response_contains_header(&response, "set-cookie", "raw=a=b; Path=/raw");
    assert_eq!(
        response_header_count(&response, "set-cookie"),
        2,
        "{response}"
    );
    assert_eq!(
        response_body(&response),
        "Set-Cookie: login=hello%20world; Path=/; Secure; HttpOnly; SameSite=Lax\nSet-Cookie: raw=a=b; Path=/raw\n",
        "{response}"
    );
}

#[test]
fn incoming_cookie_header_seeds_cookie_without_response_cookie() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request_with_headers(
        &address,
        "GET",
        "/incoming_cookie.php",
        &[("Cookie", "theme=dark")],
        "",
    );

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_eq!(response_body(&response), "dark\n", "{response}");
    assert_eq!(
        response_header_count(&response, "set-cookie"),
        0,
        "{response}"
    );
}

#[test]
fn non_session_request_with_session_cookie_does_not_load_session_store() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request_with_headers(
        &address,
        "GET",
        "/hello.php",
        &[("Cookie", "PHPSESSID=session-secret")],
        "",
    );
    let deadline = Instant::now() + Duration::from_secs(2);
    let metrics = loop {
        let metrics = http_request(&address, "GET", "/__phrust/metrics");
        if metrics.contains("phrust_server_session_finalize_skipped_inactive_total 1\n")
            || Instant::now() >= deadline
        {
            break metrics;
        }
        std::thread::sleep(Duration::from_millis(10));
    };

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(
        metrics.contains("phrust_server_session_seed_attempts_total 1\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_session_store_loads_total 0\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_session_lazy_loads_total 0\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_session_id_generations_total 0\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_session_finalize_skipped_inactive_total 1\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_request_headers_seen_total 3\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_request_headers_materialized_total 1\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_request_headers_skipped_direct_total 2\n"),
        "{metrics}"
    );
}

#[test]
fn server_persists_web_sessions_across_requests() {
    let docroot = fixture_docroot("fixtures/server/php");
    let session_dir = temp_docroot();
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut child = start_server(&docroot, &["--session-save-path", &session_arg]);

    let address = read_listening_address(&mut child);
    let first = http_request(&address, "GET", "/session_counter.php");
    assert!(first.starts_with("HTTP/1.1 200 OK"), "{first}");
    let set_cookie = response_header_values(&first, "set-cookie");
    assert_eq!(set_cookie.len(), 1, "{first}");
    assert!(set_cookie[0].ends_with("; Path=/; HttpOnly"), "{first}");
    let cookie_pair = set_cookie[0]
        .split_once(';')
        .map_or(set_cookie[0], |(pair, _)| pair)
        .to_string();
    let session_id = cookie_pair
        .strip_prefix("PHPSESSID=")
        .expect("session cookie name")
        .to_string();
    assert_eq!(
        response_body(&first),
        format!("id={session_id}\nn=1\nstatus=2\n")
    );

    let second = http_request_with_headers(
        &address,
        "GET",
        "/session_counter.php",
        &[("Cookie", &cookie_pair)],
        "",
    );
    assert!(second.starts_with("HTTP/1.1 200 OK"), "{second}");
    assert_eq!(
        response_body(&second),
        format!("id={session_id}\nn=2\nstatus=2\n")
    );
    assert_eq!(response_header_count(&second, "set-cookie"), 0, "{second}");

    let destroy = http_request_with_headers(
        &address,
        "GET",
        "/session_destroy.php",
        &[("Cookie", &cookie_pair)],
        "",
    );
    assert!(destroy.starts_with("HTTP/1.1 200 OK"), "{destroy}");
    assert_eq!(
        response_body(&destroy),
        format!("id={session_id}\ndestroyed=yes\n")
    );
    assert!(
        !session_dir.join(format!("sess_{session_id}")).exists(),
        "destroyed session file should be removed"
    );

    let after_destroy = http_request_with_headers(
        &address,
        "GET",
        "/session_counter.php",
        &[("Cookie", &cookie_pair)],
        "",
    );
    let metrics = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);
    fs::remove_dir_all(session_dir).expect("remove session temp dir");

    assert!(
        after_destroy.starts_with("HTTP/1.1 200 OK"),
        "{after_destroy}"
    );
    assert_eq!(
        response_body(&after_destroy),
        format!("id={session_id}\nn=1\nstatus=2\n")
    );
    assert!(
        metrics.contains("phrust_server_session_id_generations_total 1\n"),
        "{metrics}"
    );
}

#[test]
fn file_sessions_survive_a_server_restart() {
    let docroot = fixture_docroot("fixtures/server/php");
    let session_dir = temp_docroot();
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut first_server = start_server(&docroot, &["--session-save-path", &session_arg]);
    let first_address = read_listening_address(&mut first_server);
    let first = http_request(&first_address, "GET", "/session_counter.php");
    let cookie = response_header_values(&first, "set-cookie")[0]
        .split_once(';')
        .map_or("", |(pair, _)| pair)
        .to_string();
    send_sigterm(&first_server);
    let first_status = wait_for_exit(&mut first_server, Duration::from_secs(3));
    assert!(first_status.success(), "first server exit: {first_status}");

    let mut second_server = start_server(&docroot, &["--session-save-path", &session_arg]);
    let second_address = read_listening_address(&mut second_server);
    let second = http_request_with_headers(
        &second_address,
        "GET",
        "/session_counter.php",
        &[("Cookie", &cookie)],
        "",
    );
    stop_child(second_server);

    assert!(second.starts_with("HTTP/1.1 200 OK"), "{second}");
    assert!(response_body(&second).contains("n=2\n"), "{second}");
    fs::remove_dir_all(session_dir).expect("remove session dir");
}

#[test]
fn session_cookie_lifetime_and_attributes_are_published_together() {
    let docroot = fixture_docroot("fixtures/server/php");
    let session_dir = temp_docroot();
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--session-save-path",
            &session_arg,
            "--session-cookie-lifetime",
            "3600",
            "--session-cookie-domain",
            "example.test",
            "--enable-session-cookie-secure",
            "--session-cookie-samesite",
            "Strict",
            "--enable-session-cookie-partitioned",
        ],
    );
    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/session_counter.php");
    stop_child(child);

    let cookies = response_header_values(&response, "set-cookie");
    assert_eq!(cookies.len(), 1, "{response}");
    let cookie = cookies[0];
    assert!(cookie.contains("; Expires="), "{cookie}");
    assert!(cookie.contains(" GMT; Max-Age=3600; Path=/"), "{cookie}");
    assert!(cookie.contains("; Domain=example.test"), "{cookie}");
    assert!(cookie.contains("; Secure"), "{cookie}");
    assert!(cookie.contains("; HttpOnly"), "{cookie}");
    assert!(cookie.contains("; SameSite=Strict"), "{cookie}");
    assert!(cookie.ends_with("; Partitioned"), "{cookie}");
    fs::remove_dir_all(session_dir).expect("remove session dir");
}

#[test]
fn session_regeneration_moves_the_locked_file_and_updates_the_cookie() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("regenerate.php"),
        "<?php session_start(); $old = session_id(); $_SESSION[\"n\"] = 7; var_dump(session_regenerate_id(true)); echo $old, \"\\n\", session_id(), \"\\n\";",
    )
    .expect("write regeneration fixture");
    fs::write(
        docroot.join("read.php"),
        "<?php session_start([\"read_and_close\" => true]); echo $_SESSION[\"n\"], \"\\n\";",
    )
    .expect("write session reader fixture");
    let session_dir = temp_docroot();
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut child = start_server(&docroot, &["--session-save-path", &session_arg]);
    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/regenerate.php");
    let lines = response_body(&response).lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 3, "{response}");
    assert_eq!(lines[0], "bool(true)", "{response}");
    let old_id = lines[1];
    let new_id = lines[2];
    assert_ne!(old_id, new_id);
    let cookies = response_header_values(&response, "set-cookie");
    assert_eq!(cookies.len(), 1, "{response}");
    assert!(
        cookies[0].starts_with(&format!("PHPSESSID={new_id};")),
        "cookie={}, response={response}",
        cookies[0]
    );
    assert!(!session_dir.join(format!("sess_{old_id}")).exists());
    assert!(session_dir.join(format!("sess_{new_id}")).exists());

    let cookie = format!("PHPSESSID={new_id}");
    let read = http_request_with_headers(&address, "GET", "/read.php", &[("Cookie", &cookie)], "");
    stop_child(child);
    assert_eq!(response_body(&read), "7\n", "{read}");
    fs::remove_dir_all(docroot).expect("remove regeneration docroot");
    fs::remove_dir_all(session_dir).expect("remove session dir");
}

#[test]
fn deprecated_session_id_ini_settings_warn_and_remain_effective() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("sid.php"),
        "<?php ob_start(); ini_set(\"session.sid_length\", \"22\"); ini_set(\"session.sid_bits_per_character\", \"6\"); session_start(); echo strlen(session_id()), \"\\n\"; ob_end_flush();",
    )
    .expect("write sid fixture");
    let session_dir = temp_docroot();
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut child = start_server(&docroot, &["--session-save-path", &session_arg]);
    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/sid.php");
    stop_child(child);

    let body = response_body(&response);
    assert!(
        body.contains("ini_set(): session.sid_length INI setting is deprecated"),
        "{response}"
    );
    assert!(
        body.contains("ini_set(): session.sid_bits_per_character INI setting is deprecated"),
        "{response}"
    );
    assert!(body.ends_with("22\n"), "{response}");
    fs::remove_dir_all(docroot).expect("remove sid docroot");
    fs::remove_dir_all(session_dir).expect("remove session dir");
}

#[test]
fn request_local_session_serializer_controls_the_files_payload() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("binary.php"),
        "<?php ini_set(\"session.serialize_handler\", \"php_binary\"); session_start(); $_SESSION[\"n\"] = 7; echo session_id(), \"\\n\";",
    )
    .expect("write serializer fixture");
    let session_dir = temp_docroot();
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut child = start_server(&docroot, &["--session-save-path", &session_arg]);
    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/binary.php");
    let id = response_body(&response).trim();
    let session_file = session_dir.join(format!("sess_{id}"));
    wait_for_file_contents(&session_file, b"\x01ni:7;", Duration::from_secs(1));
    stop_child(child);

    assert_eq!(
        fs::read(session_file).expect("read session payload"),
        b"\x01ni:7;"
    );
    fs::remove_dir_all(docroot).expect("remove serializer docroot");
    fs::remove_dir_all(session_dir).expect("remove session dir");
}

#[test]
fn same_session_requests_serialize_without_lost_updates() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("seed.php"),
        "<?php session_start(); $_SESSION[\"n\"] = 0; session_write_close(); echo \"seeded\\n\";",
    )
    .expect("write seed script");
    fs::write(
        docroot.join("increment.php"),
        "<?php session_start(); $n = $_SESSION[\"n\"]; usleep(150000); $_SESSION[\"n\"] = $n + 1; session_write_close(); echo \"done\\n\";",
    )
    .expect("write increment script");
    fs::write(
        docroot.join("read.php"),
        "<?php session_start([\"read_and_close\" => true]); echo $_SESSION[\"n\"], \"\\n\";",
    )
    .expect("write read script");
    let session_dir = temp_docroot();
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--session-save-path",
            &session_arg,
            "--cpu-execution-limit",
            "2",
        ],
    );
    let address = read_listening_address(&mut child);
    let seed = http_request(&address, "GET", "/seed.php");
    let set_cookie = response_header_values(&seed, "set-cookie");
    let cookie = set_cookie[0]
        .split_once(';')
        .map_or_else(|| set_cookie[0].to_string(), |(pair, _)| pair.to_string());
    let start = Arc::new(Barrier::new(3));
    let mut requests = Vec::new();
    for _ in 0..2 {
        let address = address.clone();
        let cookie = cookie.clone();
        let start = Arc::clone(&start);
        requests.push(std::thread::spawn(move || {
            start.wait();
            http_request_with_headers(
                &address,
                "GET",
                "/increment.php",
                &[("Cookie", &cookie)],
                "",
            )
        }));
    }
    start.wait();
    for request in requests {
        let response = request.join().expect("increment request");
        assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    }
    let read = http_request_with_headers(&address, "GET", "/read.php", &[("Cookie", &cookie)], "");
    stop_child(child);
    assert_eq!(response_body(&read), "2\n", "{read}");
    fs::remove_dir_all(docroot).expect("remove session concurrency docroot");
    fs::remove_dir_all(session_dir).expect("remove session concurrency store");
}

#[test]
fn session_write_close_releases_the_file_lock_before_script_end() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("seed.php"),
        "<?php session_start(); $_SESSION[\"n\"] = 0; session_write_close();",
    )
    .expect("write seed script");
    fs::write(
        docroot.join("close_sleep.php"),
        "<?php session_start(); $_SESSION[\"n\"] = $_SESSION[\"n\"] + 1; session_write_close(); file_put_contents($_SERVER[\"DOCUMENT_ROOT\"] . \"/released.marker\", \"yes\"); usleep(500000); echo \"done\\n\";",
    )
    .expect("write close-and-sleep script");
    fs::write(
        docroot.join("read.php"),
        "<?php session_start([\"read_and_close\" => true]); echo $_SESSION[\"n\"], \"\\n\";",
    )
    .expect("write read script");
    let session_dir = temp_docroot();
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--session-save-path",
            &session_arg,
            "--cpu-execution-limit",
            "2",
        ],
    );
    let address = read_listening_address(&mut child);
    let seed = http_request(&address, "GET", "/seed.php");
    let set_cookie = response_header_values(&seed, "set-cookie");
    let cookie = set_cookie[0]
        .split_once(';')
        .map_or_else(|| set_cookie[0].to_string(), |(pair, _)| pair.to_string());
    let first_address = address.clone();
    let first_cookie = cookie.clone();
    let first = std::thread::spawn(move || {
        http_request_with_headers(
            &first_address,
            "GET",
            "/close_sleep.php",
            &[("Cookie", &first_cookie)],
            "",
        )
    });
    let marker = docroot.join("released.marker");
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !marker.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(marker.exists(), "write_close marker was not published");
    let started = std::time::Instant::now();
    let read = http_request_with_headers(&address, "GET", "/read.php", &[("Cookie", &cookie)], "");
    let elapsed = started.elapsed();
    let first = first.join().expect("close-and-sleep request");
    stop_child(child);
    assert_eq!(response_body(&read), "1\n", "{read}");
    assert!(
        elapsed < Duration::from_millis(300),
        "lock remained held for {elapsed:?}"
    );
    assert_eq!(response_body(&first), "done\n", "{first}");
    fs::remove_dir_all(docroot).expect("remove write-close docroot");
    fs::remove_dir_all(session_dir).expect("remove write-close session store");
}

#[test]
fn session_read_and_close_and_abort_release_without_writing() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("seed.php"),
        "<?php session_start(); $_SESSION['n'] = 1; session_write_close(); echo \"seeded\\n\";",
    )
    .expect("write lifecycle seed fixture");
    fs::write(
        docroot.join("read_close.php"),
        "<?php session_start(['read_and_close' => true]); $_SESSION['n'] = 9; echo \"closed\\n\";",
    )
    .expect("write read-and-close fixture");
    fs::write(
        docroot.join("abort.php"),
        "<?php session_start(); $_SESSION['n'] = 8; var_dump(session_abort());",
    )
    .expect("write abort fixture");
    fs::write(
        docroot.join("read.php"),
        "<?php session_start(['read_and_close' => true]); echo $_SESSION['n'], \"\\n\";",
    )
    .expect("write lifecycle reader fixture");
    let session_dir = temp_docroot();
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut child = start_server(&docroot, &["--session-save-path", &session_arg]);
    let address = read_listening_address(&mut child);
    let seed = http_request(&address, "GET", "/seed.php");
    let cookie = response_header_values(&seed, "set-cookie")[0]
        .split_once(';')
        .map_or("", |(pair, _)| pair)
        .to_string();

    let read_close = http_request_with_headers(
        &address,
        "GET",
        "/read_close.php",
        &[("Cookie", &cookie)],
        "",
    );
    let after_read_close =
        http_request_with_headers(&address, "GET", "/read.php", &[("Cookie", &cookie)], "");
    let abort =
        http_request_with_headers(&address, "GET", "/abort.php", &[("Cookie", &cookie)], "");
    let after_abort =
        http_request_with_headers(&address, "GET", "/read.php", &[("Cookie", &cookie)], "");

    stop_child(child);
    assert_eq!(response_body(&read_close), "closed\n", "{read_close}");
    assert_eq!(
        response_body(&after_read_close),
        "1\n",
        "{after_read_close}"
    );
    assert_eq!(response_body(&abort), "bool(true)\n", "{abort}");
    assert_eq!(response_body(&after_abort), "1\n", "{after_abort}");
    fs::remove_dir_all(docroot).expect("remove lifecycle docroot");
    fs::remove_dir_all(session_dir).expect("remove lifecycle session store");
}

#[test]
fn session_strict_mode_rotates_a_missing_incoming_id() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("id.php"),
        "<?php session_start(); echo session_id(), \"\\n\";",
    )
    .expect("write strict-mode fixture");
    let session_dir = temp_docroot();
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--session-save-path",
            &session_arg,
            "--enable-session-strict-mode",
        ],
    );
    let address = read_listening_address(&mut child);
    let response = http_request_with_headers(
        &address,
        "GET",
        "/id.php",
        &[("Cookie", "PHPSESSID=missing-but-valid")],
        "",
    );
    stop_child(child);

    let generated = response_body(&response).trim();
    assert_ne!(generated, "missing-but-valid", "{response}");
    assert!(!generated.is_empty(), "{response}");
    assert!(!session_dir.join("sess_missing-but-valid").exists());
    assert!(session_dir.join(format!("sess_{generated}")).exists());
    assert!(
        response_header_values(&response, "set-cookie")[0]
            .starts_with(&format!("PHPSESSID={generated};"))
    );
    fs::remove_dir_all(docroot).expect("remove strict-mode docroot");
    fs::remove_dir_all(session_dir).expect("remove strict-mode session store");
}

#[test]
fn explicit_session_gc_deletes_only_expired_unlocked_session_files() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("gc.php"),
        "<?php ini_set('session.gc_maxlifetime', '1'); session_start(['gc_probability' => 0]); var_dump(session_gc());",
    )
    .expect("write explicit GC fixture");
    let session_dir = temp_docroot();
    let expired = session_dir.join("sess_expired");
    fs::write(&expired, b"n|i:1;").expect("write expired session");
    let old = SystemTime::now() - Duration::from_secs(60);
    fs::File::open(&expired)
        .expect("open expired session")
        .set_times(fs::FileTimes::new().set_modified(old))
        .expect("age expired session");
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut child = start_server(&docroot, &["--session-save-path", &session_arg]);
    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/gc.php");
    stop_child(child);

    assert_eq!(response_body(&response), "int(1)\n", "{response}");
    assert!(!expired.exists());
    let generated = response_header_values(&response, "set-cookie")[0]
        .split_once(';')
        .map_or("", |(pair, _)| pair)
        .trim_start_matches("PHPSESSID=");
    assert!(session_dir.join(format!("sess_{generated}")).exists());
    fs::remove_dir_all(docroot).expect("remove GC docroot");
    fs::remove_dir_all(session_dir).expect("remove GC session store");
}

#[test]
fn fatal_error_and_timeout_release_the_session_lock() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("seed.php"),
        "<?php session_start(); $_SESSION['n'] = 1; session_write_close();",
    )
    .expect("write finalizer seed fixture");
    fs::write(
        docroot.join("fatal.php"),
        "<?php session_start(); $_SESSION['n'] = 2; missing_welle3_function();",
    )
    .expect("write fatal fixture");
    fs::write(
        docroot.join("timeout.php"),
        "<?php session_start(); while (true) { usleep(1000); }",
    )
    .expect("write timeout fixture");
    fs::write(
        docroot.join("read.php"),
        "<?php session_start(['read_and_close' => true]); echo $_SESSION['n'], \"\\n\";",
    )
    .expect("write finalizer reader fixture");
    let session_dir = temp_docroot();
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--session-save-path",
            &session_arg,
            "--max-execution-ms",
            "25",
        ],
    );
    let address = read_listening_address(&mut child);
    let seed = http_request(&address, "GET", "/seed.php");
    let cookie = response_header_values(&seed, "set-cookie")[0]
        .split_once(';')
        .map_or("", |(pair, _)| pair)
        .to_string();

    let fatal =
        http_request_with_headers(&address, "GET", "/fatal.php", &[("Cookie", &cookie)], "");
    let after_fatal =
        http_request_with_headers(&address, "GET", "/read.php", &[("Cookie", &cookie)], "");
    let timeout =
        http_request_with_headers(&address, "GET", "/timeout.php", &[("Cookie", &cookie)], "");
    let after_timeout =
        http_request_with_headers(&address, "GET", "/read.php", &[("Cookie", &cookie)], "");

    stop_child(child);
    assert!(
        fatal.starts_with("HTTP/1.1 500 Internal Server Error"),
        "{fatal}"
    );
    assert_eq!(response_body(&after_fatal), "2\n", "{after_fatal}");
    assert!(
        timeout.starts_with("HTTP/1.1 504 Gateway Timeout"),
        "{timeout}"
    );
    assert_eq!(response_body(&after_timeout), "2\n", "{after_timeout}");
    fs::remove_dir_all(docroot).expect("remove finalizer docroot");
    fs::remove_dir_all(session_dir).expect("remove finalizer session store");
}

#[test]
fn client_disconnect_releases_the_session_lock() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("seed.php"),
        "<?php session_start(); $_SESSION['n'] = 1; session_write_close();",
    )
    .expect("write disconnect seed fixture");
    fs::write(
        docroot.join("disconnect.php"),
        "<?php session_start(); $_SESSION['n'] = 2; echo \"first\\n\"; flush(); while (true) { usleep(1000); }",
    )
    .expect("write session disconnect fixture");
    fs::write(
        docroot.join("read.php"),
        "<?php session_start(['read_and_close' => true]); echo $_SESSION['n'], \"\\n\";",
    )
    .expect("write disconnect reader fixture");
    let session_dir = temp_docroot();
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--session-save-path",
            &session_arg,
            "--max-execution-ms",
            "5000",
        ],
    );
    let address = read_listening_address(&mut child);
    let seed = http_request(&address, "GET", "/seed.php");
    let cookie = response_header_values(&seed, "set-cookie")[0]
        .split_once(';')
        .map_or("", |(pair, _)| pair)
        .to_string();

    let mut stream = TcpStream::connect(&address).expect("connect disconnect request");
    stream
        .write_all(
            format!(
                "GET /disconnect.php HTTP/1.1\r\nHost: localhost\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        )
        .expect("write disconnect request");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set disconnect read timeout");
    let mut received = Vec::new();
    let mut buffer = [0_u8; 512];
    while !received
        .windows(b"first\n".len())
        .any(|window| window == b"first\n")
    {
        let count = stream
            .read(&mut buffer)
            .expect("read streamed session response");
        assert!(count > 0, "session response ended before first chunk");
        received.extend_from_slice(&buffer[..count]);
    }
    drop(stream);

    let started = Instant::now();
    let read = http_request_with_headers(&address, "GET", "/read.php", &[("Cookie", &cookie)], "");
    let elapsed = started.elapsed();
    stop_child(child);

    assert_eq!(response_body(&read), "2\n", "{read}");
    assert!(
        elapsed < Duration::from_secs(2),
        "session lock survived client disconnect for {elapsed:?}"
    );
    fs::remove_dir_all(docroot).expect("remove disconnect docroot");
    fs::remove_dir_all(session_dir).expect("remove disconnect session store");
}

#[test]
fn server_reports_headers_not_sent_during_php_execution() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/headers_sent.php");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.ends_with("not-sent\n"), "{response}");
}

#[test]
fn server_rejects_response_splitting_header() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/invalid_header.php");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_response_lacks_header(&response, "x-evil", "yes");
    assert!(response.ends_with("ok\n"), "{response}");
}

#[test]
fn server_does_not_share_php_response_headers_between_requests() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let first_response = http_request(&address, "GET", "/header.php");
    let second_response = http_request(&address, "GET", "/hello.php");

    stop_child(child);

    assert_response_contains_header(&first_response, "x-test", "yes");
    assert!(
        second_response.starts_with("HTTP/1.1 200 OK"),
        "{second_response}"
    );
    assert_response_lacks_header(&second_response, "x-test", "yes");
    assert!(second_response.ends_with("hello\n"), "{second_response}");
}

#[test]
fn server_exposes_query_superglobal() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/query.php?name=phrust");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.ends_with("phrust\n"), "{response}");
}

#[test]
fn server_filter_input_array_reads_query_snapshot() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("filter.php"),
        r#"<?php
$result = filter_input_array(INPUT_GET, [
    "id" => FILTER_VALIDATE_INT,
    "missing" => FILTER_VALIDATE_INT,
]);
echo filter_has_var(INPUT_GET, "id") ? "yes|" : "no|";
echo $result["id"], "|";
var_dump($result["missing"]);
"#,
    )
    .expect("write filter fixture");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/filter.php?id=42");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.ends_with("yes|42|NULL\n"), "{response}");
}

#[test]
fn server_exposes_post_superglobal() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request_with_body(
        &address,
        "POST",
        "/post.php",
        "application/x-www-form-urlencoded",
        "name=phrust",
    );

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.ends_with("phrust\n"), "{response}");
}

#[test]
fn server_exposes_selected_server_superglobals() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/server.php?name=phrust");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(
        response.ends_with("GET|/server.php|/server.php?name=phrust\n"),
        "{response}"
    );
}

#[test]
fn server_executes_front_controller() {
    let docroot = fixture_docroot("fixtures/server/front/public");
    let mut child = start_server(&docroot, &["--front-controller", "index.php"]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/users/123?name=phrust");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(
        response.ends_with("/index.php|/users/123|phrust\n"),
        "{response}"
    );
}

#[test]
fn server_basic_app_fixture_outputs_match_exactly() {
    let docroot = fixture_docroot("fixtures/server/apps/basic/public");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let echo = http_request(&address, "GET", "/echo.php");
    let static_file = http_request(&address, "GET", "/static.txt");
    let query = http_request(&address, "GET", "/query.php?name=phrust");
    let form = http_request_with_body(
        &address,
        "POST",
        "/form.php",
        "application/x-www-form-urlencoded",
        "name=phrust",
    );
    let cookie = http_request_with_headers(
        &address,
        "GET",
        "/cookie.php",
        &[("Cookie", "sid=abc; theme=dark")],
        "",
    );
    let server = http_request(&address, "GET", "/server.php?name=phrust");
    let include = http_request(&address, "GET", "/include.php");
    let header = http_request(&address, "GET", "/header.php");

    stop_child(child);

    assert!(echo.starts_with("HTTP/1.1 200 OK"), "{echo}");
    assert_eq!(response_body(&echo), "basic echo\n");
    assert!(static_file.starts_with("HTTP/1.1 200 OK"), "{static_file}");
    assert_eq!(response_body(&static_file), "basic static fixture\n");
    assert!(query.starts_with("HTTP/1.1 200 OK"), "{query}");
    assert_eq!(response_body(&query), "query=phrust\n");
    assert!(form.starts_with("HTTP/1.1 200 OK"), "{form}");
    assert_eq!(response_body(&form), "form=phrust\n");
    assert!(cookie.starts_with("HTTP/1.1 200 OK"), "{cookie}");
    assert_eq!(response_body(&cookie), "cookie=dark\n");
    assert!(server.starts_with("HTTP/1.1 200 OK"), "{server}");
    assert_eq!(
        response_body(&server),
        format!(
            "server=GET|/server.php?name=phrust|/server.php|/server.php|{}|{}\n",
            docroot.join("server.php").to_string_lossy(),
            docroot.to_string_lossy()
        )
    );
    assert!(include.starts_with("HTTP/1.1 200 OK"), "{include}");
    assert_eq!(response_body(&include), "include=from required file\n");
    assert!(header.starts_with("HTTP/1.1 202 Accepted"), "{header}");
    assert_response_contains_header(&header, "x-app-fixture", "basic");
    assert_eq!(response_body(&header), "accepted\n");
}

#[test]
fn server_reuses_include_cache_across_requests() {
    let docroot = fixture_docroot("fixtures/server/apps/compat/public");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let first = http_request(&address, "GET", "/include-entry.php");
    let second = http_request(&address, "GET", "/include-entry.php");
    let metrics = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);

    assert!(first.starts_with("HTTP/1.1 200 OK"), "{first}");
    assert_eq!(response_body(&first), "compat include helper\n");
    assert!(second.starts_with("HTTP/1.1 200 OK"), "{second}");
    assert_eq!(response_body(&second), "compat include helper\n");
    assert!(
        metrics.contains("phrust_server_include_resolution_misses_total 1"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_include_resolution_hits_total 1"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_include_compile_misses_total 1"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_include_compile_hits_total 1"),
        "{metrics}"
    );
}

#[test]
fn server_front_controller_app_fixture_dispatches_from_path_info() {
    let docroot = fixture_docroot("fixtures/server/apps/front-controller/public");
    let mut child = start_server(&docroot, &["--front-controller", "index.php"]);

    let address = read_listening_address(&mut child);
    let user = http_request(&address, "GET", "/users/42?name=phrust");
    let missing = http_request(&address, "GET", "/missing");

    stop_child(child);

    assert!(user.starts_with("HTTP/1.1 200 OK"), "{user}");
    assert_eq!(
        response_body(&user),
        "front=user|/index.php|/index.php/users/42|/users/42|/users/42?name=phrust\n"
    );
    assert!(missing.starts_with("HTTP/1.1 404 Not Found"), "{missing}");
    assert_eq!(response_body(&missing), "front=missing|/missing\n");
}

#[test]
fn server_front_controller_hotpath_maps_request_environment() {
    let docroot = fixture_docroot("fixtures/server/apps/front-controller-hotpath/public");
    let mut child = start_server(&docroot, &["--front-controller", "index.php"]);

    let address = read_listening_address(&mut child);
    let home = http_request(&address, "GET", "/");
    let index = http_request(&address, "GET", "/index.php?preview=1");
    let install = http_request(&address, "GET", "/admin/install.php?step=2");
    let admin = http_request(&address, "GET", "/admin/");
    let pretty = http_request(&address, "GET", "/category/news?paged=2");
    let encoded = http_request(&address, "GET", "/index.php/wp%20admin/setup?ok=1");

    stop_child(child);

    assert!(home.starts_with("HTTP/1.1 200 OK"), "{home}");
    assert_eq!(
        response_body(&home),
        "front-controller-hotpath|alpha=1|route=home|class=yes|function=yes|cookie=none|post=42:Hello Hotpath:7|post=43:Cache Warm:11|post=44:Array Lookup:13|beta=1\n"
    );
    assert!(index.starts_with("HTTP/1.1 200 OK"), "{index}");
    assert_eq!(
        response_body(&index),
        "front-controller-hotpath|alpha=1|route=home|class=yes|function=yes|cookie=none|post=42:Hello Hotpath:7|post=43:Cache Warm:11|post=44:Array Lookup:13|beta=1\n"
    );
    assert!(install.starts_with("HTTP/1.1 200 OK"), "{install}");
    assert_eq!(
        response_body(&install),
        "install|/admin/install.php?step=2|/admin/install.php|/admin/install.php|/admin/install.php||step=2|front-controller-loader\n"
    );
    assert!(admin.starts_with("HTTP/1.1 200 OK"), "{admin}");
    assert_eq!(
        response_body(&admin),
        "admin-index|/admin/|/admin/index.php|/admin/index.php|\n"
    );
    assert!(pretty.starts_with("HTTP/1.1 200 OK"), "{pretty}");
    assert_eq!(
        response_body(&pretty),
        "front-controller-hotpath|alpha=1|route=archive|class=yes|function=yes|cookie=none|post=42:Hello Hotpath:7|post=43:Cache Warm:11|post=44:Array Lookup:13|beta=1\n"
    );
    assert!(encoded.starts_with("HTTP/1.1 200 OK"), "{encoded}");
    assert_eq!(
        response_body(&encoded),
        "front-controller-hotpath|alpha=1|route=archive|class=yes|function=yes|cookie=none|post=42:Hello Hotpath:7|post=43:Cache Warm:11|post=44:Array Lookup:13|beta=1\n"
    );
}

#[test]
fn server_synthetic_plugin_theme_fixture_runs() {
    let docroot = fixture_docroot("fixtures/integration/plugin_theme_synthetic/public");
    let mut child = start_server(&docroot, &["--front-controller", "index.php"]);

    let address = read_listening_address(&mut child);
    let page = http_request(&address, "GET", "/?name=demo");
    let redirect = http_request(&address, "GET", "/?redirect=1");
    let upload_body = "--BOUNDARY\r\nContent-Disposition: form-data; name=\"package\"; filename=\"sample.txt\"\r\nContent-Type: text/plain\r\n\r\nsynthetic package payload\n\r\n--BOUNDARY--";
    let upload = http_request_with_body(
        &address,
        "POST",
        "/?upload=1",
        "multipart/form-data; boundary=BOUNDARY",
        upload_body,
    );

    stop_child(child);

    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert_response_contains_header(&page, "x-synthetic-fixture", "ok");
    assert_response_contains_header(
        &page,
        "set-cookie",
        "synthetic_demo=enabled; Path=/; SameSite=Lax",
    );
    assert_eq!(
        response_body(&page),
        "template=demo\nplugin=active\npackage_size=14\nupload=none\n"
    );
    assert!(redirect.starts_with("HTTP/1.1 302 Found"), "{redirect}");
    assert_response_contains_header(&redirect, "location", "/activated");
    assert_response_contains_header(
        &redirect,
        "set-cookie",
        "synthetic_demo=redirect; Path=/; SameSite=Lax",
    );
    assert!(upload.starts_with("HTTP/1.1 200 OK"), "{upload}");
    assert_eq!(
        response_body(&upload),
        "template=synthetic\nplugin=active\npackage_size=14\nupload=moved\nupload_size=26\n"
    );
}

#[test]
fn successful_php_redirect_without_stdout_has_empty_body() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("redirect.php"),
        "<?php header('Location: /next', true, 302); exit;",
    )
    .expect("write redirect fixture");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let redirect = http_request(&address, "GET", "/redirect.php");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(redirect.starts_with("HTTP/1.1 302 Found"), "{redirect}");
    assert_response_contains_header(&redirect, "location", "/next");
    assert_response_contains_header(&redirect, "content-length", "0");
    assert_eq!(response_body(&redirect), "");
}

#[cfg(unix)]
#[test]
fn server_rejects_symlink_escape_from_docroot() {
    let docroot = temp_docroot();
    fs::write(docroot.join("index.php"), "<?php echo 'ok';").expect("write index");
    let outside = docroot.with_extension("outside");
    fs::write(&outside, "secret").expect("write outside file");
    std::os::unix::fs::symlink(&outside, docroot.join("link.php")).expect("create symlink");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/link.php");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");
    fs::remove_file(outside).expect("remove outside file");

    assert!(response.starts_with("HTTP/1.1 404 Not Found"), "{response}");
    assert_eq!(response_body(&response), "not found\n");
}

#[cfg(unix)]
#[test]
fn server_static_capability_blocks_special_files_and_symlink_swap_races() {
    use std::os::unix::{fs::symlink, net::UnixListener};

    let docroot = temp_docroot();
    let inside = docroot.join("inside.txt");
    fs::write(&inside, "public").expect("write internal target");
    let outside = docroot.with_extension("outside-static");
    fs::write(&outside, "EXTERNAL-SECRET").expect("write external target");
    symlink("inside.txt", docroot.join("internal-link.txt")).expect("create internal symlink");
    symlink(&outside, docroot.join("external-link.txt")).expect("create external symlink");
    let fifo = docroot.join("pipe.txt");
    let status = Proc::new("mkfifo").arg(&fifo).status().expect("run mkfifo");
    assert!(status.success(), "mkfifo failed with {status}");
    let socket_path = docroot.join("socket.txt");
    let socket = UnixListener::bind(&socket_path).expect("bind Unix socket fixture");
    let race_path = docroot.join("race.txt");
    symlink(&inside, &race_path).expect("create initial race symlink");

    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);
    let internal = http_request(&address, "GET", "/internal-link.txt");
    let external = http_request(&address, "GET", "/external-link.txt");
    let fifo_response = http_request(&address, "GET", "/pipe.txt");
    let socket_response = http_request(&address, "GET", "/socket.txt");

    let stop = Arc::new(AtomicBool::new(false));
    let swap_stop = Arc::clone(&stop);
    let swap_outside = outside.clone();
    let swap_path = race_path.clone();
    let swapper = std::thread::spawn(move || {
        while !swap_stop.load(Ordering::Relaxed) {
            let _ = fs::remove_file(&swap_path);
            let _ = symlink(&swap_outside, &swap_path);
            std::thread::yield_now();
            let _ = fs::remove_file(&swap_path);
            let _ = symlink("inside.txt", &swap_path);
        }
    });
    for _ in 0..64 {
        let response = http_request(&address, "GET", "/race.txt");
        assert!(!response.contains("EXTERNAL-SECRET"), "{response}");
        assert!(
            response.starts_with("HTTP/1.1 200 OK")
                || response.starts_with("HTTP/1.1 404 Not Found")
                || response.starts_with("HTTP/1.1 308 Permanent Redirect"),
            "{response}"
        );
    }
    stop.store(true, Ordering::Relaxed);
    swapper.join().expect("join symlink swapper");

    stop_child(child);
    drop(socket);
    fs::remove_dir_all(docroot).expect("remove temp docroot");
    fs::remove_file(outside).expect("remove outside file");

    assert_eq!(response_body(&internal), "public");
    assert!(external.starts_with("HTTP/1.1 404 Not Found"), "{external}");
    assert!(
        fifo_response.starts_with("HTTP/1.1 404 Not Found"),
        "{fifo_response}"
    );
    assert!(
        socket_response.starts_with("HTTP/1.1 404 Not Found"),
        "{socket_response}"
    );
}

#[test]
fn server_rejects_unsafe_uri_paths_without_double_decoding() {
    let docroot = temp_docroot();
    fs::write(docroot.join("%2f.txt"), "single-decode").expect("write double-encoding fixture");
    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);

    for path in [
        "/../secret",
        "/%2e%2e/secret",
        "/bad%2fname",
        "/bad%5cname",
        "/bad\\name",
        "/bad%00name",
        "/bad%1fname",
        "/bad%xxname",
        "//absolute",
        "/C:/prefix",
    ] {
        let response = http_request(&address, "GET", path);
        assert!(
            response.starts_with("HTTP/1.1 400 Bad Request"),
            "path={path} response={response}"
        );
    }
    let double_encoded = http_request(&address, "GET", "/%252f.txt");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert_eq!(response_body(&double_encoded), "single-decode");
}

#[test]
fn server_returns_404_for_missing_php_script() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request(&address, "GET", "/missing.php");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 404 Not Found"), "{response}");
}

#[test]
fn server_rejects_request_body_over_limit() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(&docroot, &["--max-body-bytes", "4"]);

    let address = read_listening_address(&mut child);
    let response = http_request_with_body(
        &address,
        "POST",
        "/post.php",
        "application/x-www-form-urlencoded",
        "name=phrust",
    );

    stop_child(child);

    assert!(
        response.starts_with("HTTP/1.1 413 Payload Too Large"),
        "{response}"
    );
}

#[test]
fn server_exposes_multipart_post_and_files_superglobals() {
    let docroot = fixture_docroot("fixtures/server/apps/compat/public");
    let upload_temp_dir = temp_docroot();
    let upload_temp_arg = upload_temp_dir.to_string_lossy().to_string();
    let mut child = start_server(&docroot, &["--upload-temp-dir", &upload_temp_arg]);

    let address = read_listening_address(&mut child);
    let body = "--BOUNDARY\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nHello\r\n--BOUNDARY\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"../me.png\"\r\nContent-Type: image/png\r\n\r\nPNGDATA\r\n--BOUNDARY--";
    let response = http_request_with_body(
        &address,
        "POST",
        "/upload.php",
        "multipart/form-data; boundary=BOUNDARY",
        body,
    );
    let metrics = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_eq!(
        response_body(&response),
        "title=Hello\nname=me.png\ntype=image/png\nsize=7\nerror=0\nuploaded=yes\nmoved=yes\ncontent=PNGDATA\nuploaded_after=no\n"
    );
    let moved_upload = docroot.join("moved-upload.txt");
    assert_eq!(fs::read_to_string(&moved_upload).unwrap(), "PNGDATA");
    fs::remove_file(moved_upload).expect("remove moved upload");
    let metrics = response_body(&metrics);
    assert!(
        metrics.contains("phrust_server_upload_tempfiles_active 0\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_upload_tempfile_bytes_active 0\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_upload_bytes_written_total 7\n"),
        "{metrics}"
    );
    assert_eq!(fs::read_dir(&upload_temp_dir).unwrap().count(), 0);
    fs::remove_dir_all(upload_temp_dir).expect("remove upload temp dir");
}

#[test]
fn server_runs_script_with_empty_inputs_for_malformed_automatic_multipart() {
    let docroot = fixture_docroot("fixtures/server/apps/compat/public");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let response = http_request_with_body(
        &address,
        "POST",
        "/upload.php",
        "multipart/form-data",
        "not multipart",
    );

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
}

#[test]
fn multipart_post_without_boundary_keeps_php_input_replayable() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("input.php"),
        "<?php echo count($_POST), \" \", count($_FILES), \" \", file_get_contents(\"php://input\"), \"\\n\";",
    )
    .expect("write malformed multipart fixture");
    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);
    let response =
        http_request_with_body(&address, "POST", "/input.php", "multipart/form-data", "abc");
    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    let body = response_body(&response);
    assert!(
        body.contains("PHP Request Startup: Missing boundary in multipart/form-data POST data"),
        "{response}"
    );
    assert!(body.ends_with("0 0 abc\n"), "{response}");
    fs::remove_dir_all(docroot).expect("remove malformed multipart docroot");
}

#[test]
fn post_max_size_warning_keeps_oversized_post_input_replayable() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("input.php"),
        "<?php echo count($_POST), \" \", file_get_contents(\"php://input\"), \"\\n\";",
    )
    .expect("write post max fixture");
    let mut child = start_server(&docroot, &["--post-max-bytes", "4"]);
    let address = read_listening_address(&mut child);
    let response = http_request_with_body(
        &address,
        "POST",
        "/input.php",
        "application/x-www-form-urlencoded",
        "a=1&b=2",
    );
    stop_child(child);

    let body = response_body(&response);
    assert!(
        body.contains(
            "PHP Request Startup: POST Content-Length of 7 bytes exceeds the limit of 4 bytes"
        ),
        "{response}"
    );
    assert!(body.ends_with("0 a=1&b=2\n"), "{response}");
    fs::remove_dir_all(docroot).expect("remove post max docroot");
}

#[test]
fn server_reports_upload_file_over_limit_in_files_array() {
    let docroot = fixture_docroot("fixtures/server/apps/compat/public");
    let mut child = start_server(&docroot, &["--max-upload-file-bytes", "4"]);

    let address = read_listening_address(&mut child);
    let body = "--BOUNDARY\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"me.png\"\r\nContent-Type: image/png\r\n\r\nPNGDATA\r\n--BOUNDARY--";
    let response = http_request_with_body(
        &address,
        "POST",
        "/upload.php",
        "multipart/form-data; boundary=BOUNDARY",
        body,
    );

    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response_body(&response).contains("error=1\n"), "{response}");
}

#[test]
fn request_parse_body_reparses_put_without_mutating_post() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("parse.php"),
        "<?php [$post, $files] = request_parse_body(); echo $post[\"name\"], \"\\n\"; echo count($_POST), \"\\n\"; [$again] = request_parse_body(); echo $again[\"name\"], \"\\n\";",
    )
    .expect("write request_parse_body fixture");
    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);
    let response = http_request_with_headers(
        &address,
        "PUT",
        "/parse.php",
        &[
            ("Content-Type", "application/x-www-form-urlencoded"),
            ("Content-Length", "10"),
        ],
        "name=Alice",
    );
    stop_child(child);
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_eq!(response_body(&response), "Alice\n0\nAlice\n");
    fs::remove_dir_all(docroot).expect("remove request_parse_body docroot");
}

#[test]
fn invalid_request_parse_body_limits_do_not_consume_the_body() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("parse.php"),
        "<?php try { request_parse_body([\"post_max_size\" => \"64M\"]); } catch (ValueError $error) { echo \"invalid\\n\"; } [$post] = request_parse_body(); echo $post[\"name\"], \"\\n\";",
    )
    .expect("write request parser fixture");
    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);
    let response = http_request_with_body(
        &address,
        "PUT",
        "/parse.php",
        "application/x-www-form-urlencoded",
        "name=Johann",
    );
    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_eq!(response_body(&response), "invalid\nJohann\n", "{response}");
    fs::remove_dir_all(docroot).expect("remove parser docroot");
}

#[test]
fn php_input_remains_replayable_after_successful_request_parse_body() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("parse.php"),
        "<?php [$post] = request_parse_body(); echo $post[\"a\"], \"\\n\"; echo file_get_contents(\"php://input\"), \"\\n\";",
    )
    .expect("write request parser fixture");
    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);
    let response = http_request_with_body(
        &address,
        "PUT",
        "/parse.php",
        "application/x-www-form-urlencoded",
        "a=1&b=2",
    );
    stop_child(child);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_eq!(response_body(&response), "1\na=1&b=2\n", "{response}");
    fs::remove_dir_all(docroot).expect("remove parser docroot");
}

#[test]
fn php_input_observation_prevents_later_request_parse_body_data() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("observed.php"),
        "<?php echo file_get_contents(\"php://input\"), \"\\n\"; [$post, $files] = request_parse_body(); echo count($post), \"\\n\";",
    )
    .expect("write php input observation fixture");
    let mut child = start_server(&docroot, &[]);
    let address = read_listening_address(&mut child);
    let response = http_request_with_headers(
        &address,
        "PUT",
        "/observed.php",
        &[
            ("Content-Type", "application/x-www-form-urlencoded"),
            ("Content-Length", "3"),
        ],
        "a=1",
    );
    stop_child(child);
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_eq!(response_body(&response), "a=1\n0\n");
    fs::remove_dir_all(docroot).expect("remove php input observation docroot");
}

#[test]
fn file_backed_php_input_cleans_spool_and_resets_active_gauges() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("input.php"),
        "<?php echo strlen(file_get_contents(\"php://input\")), \"\\n\";",
    )
    .expect("write php input fixture");
    let spool_dir = temp_docroot();
    let spool_arg = spool_dir.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--request-body-memory-bytes",
            "32",
            "--request-body-temp-dir",
            &spool_arg,
        ],
    );
    let address = read_listening_address(&mut child);
    let body = "x".repeat(4096);
    let response = http_request_with_body(&address, "POST", "/input.php", "text/plain", &body);
    assert_eq!(response_body(&response), "4096\n", "{response}");
    let deadline = Instant::now() + Duration::from_secs(2);
    let metrics = loop {
        let metrics = http_request(&address, "GET", "/__phrust/metrics");
        if (metrics.contains("phrust_server_request_body_tempfiles_active 0\n")
            && metrics.contains("phrust_server_request_body_tempfile_bytes_active 0\n"))
            || Instant::now() >= deadline
        {
            break metrics;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    stop_child(child);
    let metrics = response_body(&metrics);
    assert!(
        metrics.contains("phrust_server_request_body_spooled_total 1\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_request_body_tempfiles_active 0\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_request_body_tempfile_bytes_active 0\n"),
        "{metrics}"
    );
    assert_eq!(fs::read_dir(&spool_dir).expect("read spool dir").count(), 0);
    fs::remove_dir_all(docroot).expect("remove php input docroot");
    fs::remove_dir_all(spool_dir).expect("remove body spool dir");
}

#[test]
fn request_body_memory_threshold_is_inclusive_and_spills_only_after_crossing() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("input.php"),
        "<?php echo strlen(file_get_contents('php://input')), \"\\n\";",
    )
    .expect("write threshold fixture");
    let spool_dir = temp_docroot();
    let spool_arg = spool_dir.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--request-body-memory-bytes",
            "32",
            "--request-body-temp-dir",
            &spool_arg,
        ],
    );
    let address = read_listening_address(&mut child);

    let at_limit = http_request_with_body(
        &address,
        "POST",
        "/input.php",
        "application/octet-stream",
        &"m".repeat(32),
    );
    let over_limit = http_request_with_body(
        &address,
        "POST",
        "/input.php",
        "application/octet-stream",
        &"s".repeat(33),
    );
    let metrics = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);
    assert_eq!(response_body(&at_limit), "32\n", "{at_limit}");
    assert_eq!(response_body(&over_limit), "33\n", "{over_limit}");
    assert!(
        metrics.contains("phrust_server_request_body_memory_total 1\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_request_body_spooled_total 1\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_request_body_spooled_bytes_total 33\n"),
        "{metrics}"
    );
    assert_eq!(fs::read_dir(&spool_dir).expect("read spool dir").count(), 0);
    fs::remove_dir_all(docroot).expect("remove threshold docroot");
    fs::remove_dir_all(spool_dir).expect("remove threshold spool dir");
}

#[test]
fn request_parse_body_streams_put_multipart_through_the_shared_parser() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("multipart.php"),
        "<?php [$post, $files] = request_parse_body(); echo $post[\"title\"], \"\\n\"; echo $files[\"avatar\"][\"name\"], \"\\n\"; echo $files[\"avatar\"][\"error\"], \"\\n\"; echo file_get_contents($files[\"avatar\"][\"tmp_name\"]), \"\\n\"; echo count($_FILES), \"\\n\";",
    )
    .expect("write multipart request_parse_body fixture");
    let upload_dir = temp_docroot();
    let upload_arg = upload_dir.to_string_lossy().to_string();
    let mut child = start_server(&docroot, &["--upload-temp-dir", &upload_arg]);
    let address = read_listening_address(&mut child);
    let body = "--BOUNDARY\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nHello\r\n--BOUNDARY\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"me.txt\"\r\nContent-Type: text/plain\r\n\r\nUPLOAD\r\n--BOUNDARY--\r\n";
    let response = http_request_with_body(
        &address,
        "PUT",
        "/multipart.php",
        "multipart/form-data; boundary=BOUNDARY",
        body,
    );
    let metrics = http_request(&address, "GET", "/__phrust/metrics");
    stop_child(child);
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert_eq!(response_body(&response), "Hello\nme.txt\n0\nUPLOAD\n0\n");
    let metrics = response_body(&metrics);
    assert!(
        metrics.contains("phrust_server_upload_tempfiles_active 0\n"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_upload_tempfile_bytes_active 0\n"),
        "{metrics}"
    );
    assert_eq!(
        fs::read_dir(&upload_dir).expect("read upload dir").count(),
        0
    );
    fs::remove_dir_all(docroot).expect("remove multipart parse docroot");
    fs::remove_dir_all(upload_dir).expect("remove multipart upload dir");
}

#[test]
fn server_handles_default_two_hundred_concurrent_requests() {
    let docroot = temp_docroot();
    fs::write(docroot.join("static.txt"), "ok\n").expect("write static fixture");
    let mut child = start_server(&docroot, &[]);

    let address = read_listening_address(&mut child);
    let start = Arc::new(Barrier::new(201));
    let mut handles = Vec::with_capacity(200);
    for _ in 0..200 {
        let address = address.clone();
        let start = Arc::clone(&start);
        handles.push(std::thread::spawn(move || {
            start.wait();
            http_request(&address, "GET", "/static.txt")
        }));
    }
    start.wait();
    let responses = handles
        .into_iter()
        .map(|handle| handle.join().expect("join request thread"))
        .collect::<Vec<_>>();

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    for response in responses {
        assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
        assert_eq!(response_body(&response), "ok\n");
    }
}

#[test]
fn server_waits_for_in_flight_capacity_before_overload() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(
        &docroot,
        &["--max-in-flight", "1", "--request-body-timeout-ms", "5000"],
    );

    let address = read_listening_address(&mut child);
    let mut held_stream = TcpStream::connect(&address).expect("connect held request");
    held_stream
        .write_all(
            b"POST /post.php HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 11\r\nConnection: close\r\n\r\n",
        )
        .expect("write held request headers");

    let queued_address = address.clone();
    let queued = std::thread::spawn(move || http_request(&queued_address, "GET", "/hello.php"));
    std::thread::sleep(Duration::from_millis(100));
    held_stream
        .write_all(b"name=queued")
        .expect("finish held request body");
    held_stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set held read timeout");
    let mut held_response = String::new();
    held_stream
        .read_to_string(&mut held_response)
        .expect("read held response");
    let queued_response = queued.join().expect("join queued request");

    stop_child(child);

    assert!(
        held_response.starts_with("HTTP/1.1 200 OK"),
        "{held_response}"
    );
    assert!(held_response.ends_with("queued\n"), "{held_response}");
    assert!(
        queued_response.starts_with("HTTP/1.1 200 OK"),
        "{queued_response}"
    );
    assert!(queued_response.ends_with("hello\n"), "{queued_response}");
}

#[test]
fn server_runs_session_requests_without_global_execution_lock() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("session_slow.php"),
        "<?php\nsession_start();\nusleep(400000);\necho \"done\\n\";\n",
    )
    .expect("write slow session fixture");
    let mut child = start_server(
        &docroot,
        &[
            "--max-in-flight",
            "4",
            "--cpu-execution-limit",
            "3",
            "--max-execution-ms",
            "5000",
        ],
    );

    let address = read_listening_address(&mut child);
    let warmup = http_request(&address, "GET", "/session_slow.php");
    assert!(warmup.starts_with("HTTP/1.1 200 OK"), "{warmup}");

    let start = Arc::new(Barrier::new(4));
    let started = Instant::now();
    let mut handles = Vec::with_capacity(3);
    for _ in 0..3 {
        let address = address.clone();
        let start = Arc::clone(&start);
        handles.push(std::thread::spawn(move || {
            start.wait();
            http_request(&address, "GET", "/session_slow.php")
        }));
    }
    start.wait();
    let responses = handles
        .into_iter()
        .map(|handle| handle.join().expect("join session request thread"))
        .collect::<Vec<_>>();
    let elapsed = started.elapsed();

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    for response in responses {
        assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
        assert!(response.ends_with("done\n"), "{response}");
    }
    assert!(
        elapsed < Duration::from_millis(1_000),
        "session requests were serialized: elapsed={elapsed:?}"
    );
}

#[test]
fn server_cpu_execution_limit_queues_php_without_limiting_http_admission() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("cpu_queue.php"),
        "<?php\nusleep(250000);\necho \"done\\n\";\n",
    )
    .expect("write CPU queue fixture");
    let mut child = start_server(
        &docroot,
        &[
            "--max-in-flight",
            "4",
            "--cpu-execution-limit",
            "1",
            "--cpu-queue-timeout-ms",
            "5000",
            "--max-execution-ms",
            "5000",
        ],
    );

    let address = read_listening_address(&mut child);
    let start = Arc::new(Barrier::new(4));
    let started = Instant::now();
    let mut handles = Vec::with_capacity(3);
    for _ in 0..3 {
        let address = address.clone();
        let start = Arc::clone(&start);
        handles.push(std::thread::spawn(move || {
            start.wait();
            http_request(&address, "GET", "/cpu_queue.php")
        }));
    }
    start.wait();
    let responses = handles
        .into_iter()
        .map(|handle| handle.join().expect("join queued request"))
        .collect::<Vec<_>>();
    let elapsed = started.elapsed();
    let metrics = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    for response in responses {
        assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
        assert!(response.ends_with("done\n"), "{response}");
    }
    assert!(
        elapsed >= Duration::from_millis(650),
        "CPU execution limit did not serialize PHP work: elapsed={elapsed:?}"
    );
    assert!(
        metrics.contains("phrust_server_cpu_execution_queued_total 2"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_cpu_execution_current 0"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_cpu_execution_timeouts_total 0"),
        "{metrics}"
    );
}

#[test]
fn server_cpu_execution_queue_timeout_is_observable() {
    let docroot = temp_docroot();
    fs::write(
        docroot.join("cpu_queue_timeout.php"),
        "<?php\nusleep(250000);\necho \"done\\n\";\n",
    )
    .expect("write CPU queue timeout fixture");
    let mut child = start_server(
        &docroot,
        &[
            "--max-in-flight",
            "4",
            "--cpu-execution-limit",
            "1",
            "--cpu-queue-timeout-ms",
            "100",
            "--max-execution-ms",
            "5000",
        ],
    );

    let address = read_listening_address(&mut child);
    let start = Arc::new(Barrier::new(3));
    let handles = (0..2)
        .map(|_| {
            let address = address.clone();
            let start = Arc::clone(&start);
            std::thread::spawn(move || {
                start.wait();
                http_request(&address, "GET", "/cpu_queue_timeout.php")
            })
        })
        .collect::<Vec<_>>();
    start.wait();
    let responses = handles
        .into_iter()
        .map(|handle| handle.join().expect("join queue timeout request"))
        .collect::<Vec<_>>();
    let metrics = http_request(&address, "GET", "/__phrust/metrics");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert_eq!(
        responses
            .iter()
            .filter(|response| response.starts_with("HTTP/1.1 200 OK"))
            .count(),
        1,
        "{responses:?}"
    );
    assert_eq!(
        responses
            .iter()
            .filter(|response| response.starts_with("HTTP/1.1 503 Service Unavailable"))
            .count(),
        1,
        "{responses:?}"
    );
    assert!(
        metrics.contains("phrust_server_cpu_execution_timeouts_total 1"),
        "{metrics}"
    );
    assert!(
        metrics.contains("phrust_server_cpu_execution_rejected_total 1"),
        "{metrics}"
    );
}

#[test]
fn server_returns_503_when_max_in_flight_wait_expires() {
    let docroot = fixture_docroot("fixtures/server/php");
    let mut child = start_server(
        &docroot,
        &["--max-in-flight", "1", "--request-body-timeout-ms", "5000"],
    );

    let address = read_listening_address(&mut child);
    let mut held_stream = TcpStream::connect(&address).expect("connect held request");
    held_stream
        .write_all(
            b"POST /post.php HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 11\r\nConnection: close\r\n\r\n",
        )
        .expect("write held request headers");
    std::thread::sleep(Duration::from_millis(100));

    let started = Instant::now();
    let response = http_request(&address, "GET", "/hello.php");
    let elapsed = started.elapsed();

    drop(held_stream);
    stop_child(child);

    assert!(
        response.starts_with("HTTP/1.1 503 Service Unavailable"),
        "{response}"
    );
    assert_response_contains_header(&response, "retry-after", "1");
    assert!(response.ends_with("server overloaded\n"), "{response}");
    assert!(
        elapsed >= Duration::from_millis(450),
        "overload response did not wait for capacity: elapsed={elapsed:?}"
    );
}

#[test]
fn static_transfer_holds_in_flight_capacity_until_body_drop() {
    let docroot = temp_docroot();
    let large_path = docroot.join("large.bin");
    let large = fs::File::create(&large_path).expect("create large static fixture");
    large
        .set_len(64 * 1024 * 1024)
        .expect("size large static fixture");
    let mut child = start_server(&docroot, &["--max-in-flight", "1"]);
    let address = read_listening_address(&mut child);
    let mut held = TcpStream::connect(&address).expect("connect held static request");
    held.set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set held response timeout");
    held.write_all(b"GET /large.bin HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .expect("write held static request");
    let mut prefix = Vec::new();
    let mut read_buffer = [0_u8; 1024];
    while !prefix.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = held
            .read(&mut read_buffer)
            .expect("read static response head");
        assert_ne!(read, 0, "large static response ended unexpectedly");
        prefix.extend_from_slice(&read_buffer[..read]);
    }

    let overloaded = http_request(&address, "GET", "/healthz");
    assert!(
        overloaded.starts_with("HTTP/1.1 503 Service Unavailable"),
        "{overloaded}"
    );

    drop(held);
    std::thread::sleep(Duration::from_millis(100));
    let available = http_request(&address, "GET", "/healthz");

    stop_child(child);
    fs::remove_dir_all(docroot).expect("remove temp docroot");
    assert!(available.starts_with("HTTP/1.1 200 OK"), "{available}");
}

#[test]
fn server_shutdown_signal_does_not_panic() {
    let docroot = temp_docroot();
    let mut child = start_server(&docroot, &["--graceful-shutdown-timeout-ms", "30000"]);

    let address = read_listening_address(&mut child);
    let response = http_get(&address, "/healthz");
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");

    let started = Instant::now();
    send_sigint(&child);
    let status = wait_for_exit(&mut child, Duration::from_secs(2));
    let elapsed = started.elapsed();
    fs::remove_dir_all(docroot).expect("remove temp docroot");

    assert!(status.success(), "server exited with {status}");
    assert!(
        elapsed < Duration::from_secs(1),
        "idle shutdown waited for the configured drain deadline: {elapsed:?}"
    );
}

#[test]
fn drain_deadline_forces_incomplete_upload_and_removes_spool_file() {
    let docroot = temp_docroot();
    let spool_dir = temp_docroot();
    fs::write(
        docroot.join("upload.php"),
        "<?php echo strlen(file_get_contents('php://input')), \"\\n\";",
    )
    .expect("write forced-drain upload fixture");
    let spool_arg = spool_dir.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--request-body-memory-bytes",
            "1",
            "--request-body-temp-dir",
            &spool_arg,
            "--request-body-timeout-ms",
            "10000",
            "--request-body-idle-timeout-ms",
            "10000",
            "--graceful-shutdown-timeout-ms",
            "250",
        ],
    );
    let address = read_listening_address(&mut child);
    let mut upload = TcpStream::connect(&address).expect("connect forced-drain upload");
    upload
        .write_all(
            b"POST /upload.php HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/octet-stream\r\nContent-Length: 1048576\r\n\r\npartial",
        )
        .expect("start forced-drain upload");
    wait_for_directory_entry(&spool_dir, Duration::from_secs(2));

    let started = Instant::now();
    send_sigterm(&child);
    let status = wait_for_exit(&mut child, Duration::from_secs(2));
    let elapsed = started.elapsed();
    drop(upload);

    assert!(status.success(), "forced-drain server exited with {status}");
    assert!(
        elapsed >= Duration::from_millis(150),
        "incomplete request escaped the configured drain deadline: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "forced drain exceeded its bounded cleanup window: {elapsed:?}"
    );
    assert_eq!(
        fs::read_dir(&spool_dir)
            .expect("read forced-drain spool dir")
            .count(),
        0,
        "forced shutdown leaked a request-body spool file"
    );
    fs::remove_dir_all(docroot).expect("remove forced-drain docroot");
    fs::remove_dir_all(spool_dir).expect("remove forced-drain spool dir");
}

#[test]
fn second_shutdown_signal_forces_immediate_cleanup() {
    let docroot = temp_docroot();
    let spool_dir = temp_docroot();
    fs::write(
        docroot.join("upload.php"),
        "<?php echo strlen(file_get_contents('php://input')), \"\\n\";",
    )
    .expect("write second-signal upload fixture");
    let spool_arg = spool_dir.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--request-body-memory-bytes",
            "1",
            "--request-body-temp-dir",
            &spool_arg,
            "--request-body-timeout-ms",
            "10000",
            "--request-body-idle-timeout-ms",
            "10000",
            "--graceful-shutdown-timeout-ms",
            "5000",
        ],
    );
    let address = read_listening_address(&mut child);
    let mut upload = TcpStream::connect(&address).expect("connect second-signal upload");
    upload
        .write_all(
            b"POST /upload.php HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/octet-stream\r\nContent-Length: 1048576\r\n\r\npartial",
        )
        .expect("start second-signal upload");
    wait_for_directory_entry(&spool_dir, Duration::from_secs(2));

    send_sigterm(&child);
    std::thread::sleep(Duration::from_millis(75));
    assert!(
        child.try_wait().expect("poll draining child").is_none(),
        "first shutdown signal did not begin a drain"
    );
    let started = Instant::now();
    send_sigint(&child);
    let status = wait_for_exit(&mut child, Duration::from_secs(2));
    let elapsed = started.elapsed();
    drop(upload);

    assert!(
        status.success(),
        "second-signal server exited with {status}"
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "second signal did not force immediate shutdown: {elapsed:?}"
    );
    assert_eq!(
        fs::read_dir(&spool_dir)
            .expect("read second-signal spool dir")
            .count(),
        0,
        "second-signal shutdown leaked a request-body spool file"
    );
    fs::remove_dir_all(docroot).expect("remove second-signal docroot");
    fs::remove_dir_all(spool_dir).expect("remove second-signal spool dir");
}

#[test]
fn sigterm_finishes_large_h1_static_response() {
    let docroot = temp_docroot();
    let static_size = 64 * 1024 * 1024_u64;
    fs::File::create(docroot.join("large.bin"))
        .expect("create drain static fixture")
        .set_len(static_size)
        .expect("size drain static fixture");
    let mut child = start_server(
        &docroot,
        &[
            "--graceful-shutdown-timeout-ms",
            "3000",
            "--response-write-idle-timeout-ms",
            "3000",
        ],
    );
    let address = read_listening_address(&mut child);
    let mut stream = TcpStream::connect(&address).expect("connect draining static request");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("set draining static timeout");
    stream
        .write_all(b"GET /large.bin HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("send draining static request");
    let mut response = Vec::new();
    let mut buffer = [0_u8; 8192];
    while !response.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream
            .read(&mut buffer)
            .expect("read draining static response head");
        assert_ne!(read, 0, "static response ended before its header");
        response.extend_from_slice(&buffer[..read]);
    }

    send_sigterm(&child);
    stream
        .read_to_end(&mut response)
        .expect("read complete draining static response");
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("draining static response delimiter")
        + 4;
    let headers = String::from_utf8_lossy(&response[..header_end]);
    assert!(headers.starts_with("HTTP/1.1 200 OK"), "{headers}");
    assert!(
        headers
            .to_ascii_lowercase()
            .contains(&format!("content-length: {static_size}\r\n")),
        "{headers}"
    );
    assert_eq!(response.len() - header_end, static_size as usize);

    let status = wait_for_exit(&mut child, Duration::from_secs(4));
    fs::remove_dir_all(docroot).expect("remove drain static docroot");
    assert!(status.success(), "static-drain server exited with {status}");
}

#[test]
fn sigterm_finishes_h1_php_session_and_stops_keep_alive() {
    let docroot = temp_docroot();
    let session_dir = temp_docroot();
    fs::write(
        docroot.join("session.php"),
        "<?php session_start(); echo \"first\\n\"; flush(); usleep(350000); $_SESSION['done'] = true; session_write_close(); echo \"done\\n\";",
    )
    .expect("write H1 session drain fixture");
    let session_arg = session_dir.to_string_lossy().to_string();
    let mut child = start_server(
        &docroot,
        &[
            "--session-save-path",
            &session_arg,
            "--graceful-shutdown-timeout-ms",
            "2000",
            "--max-execution-ms",
            "2000",
        ],
    );
    let address = read_listening_address(&mut child);
    let mut stream = TcpStream::connect(&address).expect("connect H1 session drain request");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("set H1 session drain timeout");
    stream
        .write_all(b"GET /session.php HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("send H1 session drain request");
    let mut response = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !response
        .windows(b"first\n".len())
        .any(|window| window == b"first\n")
    {
        let read = stream
            .read(&mut buffer)
            .expect("read first H1 session drain chunk");
        assert_ne!(read, 0, "session response ended before first flush");
        response.extend_from_slice(&buffer[..read]);
    }

    send_sigterm(&child);
    std::thread::sleep(Duration::from_millis(75));
    let _ = stream.write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\n\r\n");
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => response.extend_from_slice(&buffer[..read]),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::BrokenPipe
                ) =>
            {
                break;
            }
            Err(error) => panic!("read remaining H1 session drain response: {error}"),
        }
    }
    let response = String::from_utf8_lossy(&response);
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.contains("first\n"), "{response}");
    assert!(response.contains("done\n"), "{response}");
    if response.matches("HTTP/1.1 ").count() == 2 {
        assert!(
            response.contains("HTTP/1.1 503 Service Unavailable"),
            "{response}"
        );
    } else {
        assert_eq!(response.matches("HTTP/1.1 ").count(), 1, "{response}");
    }

    let status = wait_for_exit(&mut child, Duration::from_secs(3));
    let session_written = fs::read_dir(&session_dir)
        .expect("read drained session directory")
        .map(|entry| fs::read(entry.expect("drained session entry").path()).unwrap())
        .any(|contents| {
            contents
                .windows(b"done|b:1;".len())
                .any(|w| w == b"done|b:1;")
        });
    fs::remove_dir_all(docroot).expect("remove H1 session drain docroot");
    fs::remove_dir_all(session_dir).expect("remove H1 session drain directory");
    assert!(
        status.success(),
        "H1 session-drain server exited with {status}"
    );
    assert!(
        session_written,
        "drained PHP request did not persist its session"
    );
}

fn tls_fixture_paths() -> (std::path::PathBuf, std::path::PathBuf) {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    (
        root.join("fixtures/server/tls/localhost.crt"),
        root.join("fixtures/server/tls/localhost.key"),
    )
}

fn wait_for_request_profile(dir: &std::path::Path) -> std::path::PathBuf {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(path) = fs::read_dir(dir)
            .expect("read profile dir")
            .map(|entry| entry.expect("profile entry").path())
            .find(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
        {
            return path;
        }
        assert!(Instant::now() < deadline, "request profile json");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn wait_for_file_to_contain(path: &std::path::Path, needle: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Ok(contents) = fs::read_to_string(path)
            && contents.contains(needle)
        {
            return contents;
        }
        assert!(Instant::now() < deadline, "{path:?} to contain {needle}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[derive(Debug)]
struct TestCertificateVerifier(Arc<tokio_rustls::rustls::crypto::CryptoProvider>);

impl TestCertificateVerifier {
    fn new() -> Arc<Self> {
        Arc::new(Self(Arc::new(
            tokio_rustls::rustls::crypto::ring::default_provider(),
        )))
    }
}

impl tokio_rustls::rustls::client::danger::ServerCertVerifier for TestCertificateVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls_pki_types::CertificateDer<'_>,
        _intermediates: &[rustls_pki_types::CertificateDer<'_>],
        _server_name: &rustls_pki_types::ServerName<'_>,
        _ocsp: &[u8],
        _now: rustls_pki_types::UnixTime,
    ) -> Result<tokio_rustls::rustls::client::danger::ServerCertVerified, tokio_rustls::rustls::Error>
    {
        Ok(tokio_rustls::rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls_pki_types::CertificateDer<'_>,
        dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        tokio_rustls::rustls::Error,
    > {
        tokio_rustls::rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls_pki_types::CertificateDer<'_>,
        dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        tokio_rustls::rustls::Error,
    > {
        tokio_rustls::rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<tokio_rustls::rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn test_client_config(alpn: Vec<Vec<u8>>) -> tokio_rustls::rustls::ClientConfig {
    let mut config = tokio_rustls::rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(TestCertificateVerifier::new())
        .with_no_client_auth();
    config.alpn_protocols = alpn;
    config
}

async fn http2_get(address: &str, path: &str) -> (hyper::StatusCode, Vec<Vec<u8>>, Duration) {
    use http_body_util::{BodyExt, Empty};
    use hyper_util::rt::{TokioExecutor, TokioIo};

    let tcp = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect H2 test client");
    let connector =
        tokio_rustls::TlsConnector::from(Arc::new(test_client_config(vec![b"h2".to_vec()])));
    let tls = connector
        .connect(
            rustls_pki_types::ServerName::try_from("localhost").expect("valid test server name"),
            tcp,
        )
        .await
        .expect("connect H2 TLS");
    let (mut sender, connection) =
        hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(tls))
            .await
            .expect("perform H2 handshake");
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let request = hyper::Request::builder()
        .uri(format!("https://localhost{path}"))
        .body(Empty::<bytes::Bytes>::new())
        .expect("build H2 request");
    let started = Instant::now();
    let response = sender.send_request(request).await.expect("send H2 request");
    let status = response.status();
    let mut body = response.into_body();
    let mut chunks = Vec::new();
    let mut first_elapsed = None;
    while let Some(frame) = body.frame().await {
        let frame = frame.expect("receive H2 response frame");
        if let Ok(data) = frame.into_data()
            && !data.is_empty()
        {
            first_elapsed.get_or_insert_with(|| started.elapsed());
            chunks.push(data.to_vec());
        }
    }
    (
        status,
        chunks,
        first_elapsed.expect("H2 response has a data frame"),
    )
}

#[derive(Debug)]
struct ProtocolResponse {
    status: hyper::StatusCode,
    headers: hyper::HeaderMap,
    chunks: Vec<Vec<u8>>,
}

impl ProtocolResponse {
    fn body(&self) -> Vec<u8> {
        self.chunks.concat()
    }

    fn header(&self, name: hyper::header::HeaderName) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }
}

fn assert_protocol_bodies(responses: &[ProtocolResponse; 3], expected: &[u8]) {
    for response in responses {
        assert_eq!(response.status, hyper::StatusCode::OK, "{response:?}");
        assert_eq!(response.body(), expected, "{response:?}");
    }
}

async fn protocol_request_set(
    address: &str,
    method: hyper::Method,
    path: &str,
    headers: &[(&str, &str)],
) -> [ProtocolResponse; 3] {
    protocol_request_body_set(address, method, path, headers, &[]).await
}

async fn protocol_request_body_set(
    address: &str,
    method: hyper::Method,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> [ProtocolResponse; 3] {
    let h1 = http1_request(address, method.clone(), path, headers, body).await;
    let h2 = http2_request(address, method.clone(), path, headers, body).await;
    let h3 = http3_request(address, method, path, headers, body).await;
    [h1, h2, h3]
}

async fn http1_request(
    address: &str,
    method: hyper::Method,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> ProtocolResponse {
    use http_body_util::{BodyExt, StreamBody};
    use hyper_util::rt::TokioIo;

    let tcp = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect H1 test client");
    let connector =
        tokio_rustls::TlsConnector::from(Arc::new(test_client_config(vec![b"http/1.1".to_vec()])));
    let tls = connector
        .connect(
            rustls_pki_types::ServerName::try_from("localhost").expect("valid test server name"),
            tcp,
        )
        .await
        .expect("connect H1 TLS");
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(tls))
        .await
        .expect("perform H1 handshake");
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let mut request = hyper::Request::builder()
        .method(method)
        .uri(format!("https://localhost{path}"));
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let request_body = bytes::Bytes::copy_from_slice(body);
    let body_stream = futures_util::stream::once(async move {
        Ok::<_, std::convert::Infallible>(hyper::body::Frame::data(request_body))
    });
    let response = sender
        .send_request(
            request
                .body(StreamBody::new(body_stream))
                .expect("build H1 request"),
        )
        .await
        .expect("send H1 request");
    let (parts, mut body) = response.into_parts();
    let mut chunks = Vec::new();
    while let Some(frame) = body.frame().await {
        let frame = frame.expect("receive H1 response frame");
        if let Ok(data) = frame.into_data()
            && !data.is_empty()
        {
            chunks.push(data.to_vec());
        }
    }
    ProtocolResponse {
        status: parts.status,
        headers: parts.headers,
        chunks,
    }
}

async fn http2_request(
    address: &str,
    method: hyper::Method,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> ProtocolResponse {
    use http_body_util::{BodyExt, Full};
    use hyper_util::rt::{TokioExecutor, TokioIo};

    let tcp = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect H2 test client");
    let connector =
        tokio_rustls::TlsConnector::from(Arc::new(test_client_config(vec![b"h2".to_vec()])));
    let tls = connector
        .connect(
            rustls_pki_types::ServerName::try_from("localhost").expect("valid test server name"),
            tcp,
        )
        .await
        .expect("connect H2 TLS");
    let (mut sender, connection) =
        hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(tls))
            .await
            .expect("perform H2 handshake");
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let mut request = hyper::Request::builder()
        .method(method)
        .uri(format!("https://localhost{path}"));
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = sender
        .send_request(
            request
                .body(Full::new(bytes::Bytes::copy_from_slice(body)))
                .expect("build H2 request"),
        )
        .await
        .expect("send H2 request");
    let (parts, mut body) = response.into_parts();
    let mut chunks = Vec::new();
    while let Some(frame) = body.frame().await {
        let frame = frame.expect("receive H2 response frame");
        if let Ok(data) = frame.into_data()
            && !data.is_empty()
        {
            chunks.push(data.to_vec());
        }
    }
    ProtocolResponse {
        status: parts.status,
        headers: parts.headers,
        chunks,
    }
}

async fn abort_http1_response(address: &str, path: &str) {
    use http_body_util::{BodyExt, Empty};
    use hyper_util::rt::TokioIo;

    let tcp = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect abort H1 client");
    let connector =
        tokio_rustls::TlsConnector::from(Arc::new(test_client_config(vec![b"http/1.1".to_vec()])));
    let tls = connector
        .connect(
            rustls_pki_types::ServerName::try_from("localhost").expect("valid test server name"),
            tcp,
        )
        .await
        .expect("connect abort H1 TLS");
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(tls))
        .await
        .expect("perform abort H1 handshake");
    let connection_task = tokio::spawn(async move {
        let _ = connection.await;
    });
    let request = hyper::Request::builder()
        .uri(format!("https://localhost{path}"))
        .body(Empty::<bytes::Bytes>::new())
        .expect("build abort H1 request");
    let response = sender
        .send_request(request)
        .await
        .expect("send abort H1 request");
    let mut body = response.into_body();
    loop {
        let frame = body
            .frame()
            .await
            .expect("abort H1 response has a frame")
            .expect("receive abort H1 response frame");
        if frame.data_ref().is_some_and(|data| !data.is_empty()) {
            break;
        }
    }
    drop(body);
    drop(sender);
    connection_task.abort();
    let _ = connection_task.await;
}

async fn abort_http2_response(address: &str, path: &str) {
    use http_body_util::{BodyExt, Empty};
    use hyper_util::rt::{TokioExecutor, TokioIo};

    let tcp = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect abort H2 client");
    let connector =
        tokio_rustls::TlsConnector::from(Arc::new(test_client_config(vec![b"h2".to_vec()])));
    let tls = connector
        .connect(
            rustls_pki_types::ServerName::try_from("localhost").expect("valid test server name"),
            tcp,
        )
        .await
        .expect("connect abort H2 TLS");
    let (mut sender, connection) =
        hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(tls))
            .await
            .expect("perform abort H2 handshake");
    let connection_task = tokio::spawn(async move {
        let _ = connection.await;
    });
    let request = hyper::Request::builder()
        .uri(format!("https://localhost{path}"))
        .body(Empty::<bytes::Bytes>::new())
        .expect("build abort H2 request");
    let response = sender
        .send_request(request)
        .await
        .expect("send abort H2 request");
    let mut body = response.into_body();
    loop {
        let frame = body
            .frame()
            .await
            .expect("abort H2 response has a frame")
            .expect("receive abort H2 response frame");
        if frame.data_ref().is_some_and(|data| !data.is_empty()) {
            break;
        }
    }
    drop(body);
    drop(sender);
    connection_task.abort();
    let _ = connection_task.await;
}

async fn http3_get(address: &str, path: &str) -> (hyper::StatusCode, Vec<Vec<u8>>, Duration) {
    use bytes::Buf;
    use quinn::crypto::rustls::QuicClientConfig;

    let mut crypto = test_client_config(vec![b"h3".to_vec()]);
    crypto.enable_early_data = true;
    let client_config = quinn::ClientConfig::new(Arc::new(
        QuicClientConfig::try_from(crypto).expect("build QUIC client config"),
    ));
    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().expect("client bind address"))
        .expect("create QUIC client endpoint");
    endpoint.set_default_client_config(client_config);
    let connection = endpoint
        .connect(address.parse().expect("server socket address"), "localhost")
        .expect("start QUIC connection")
        .await
        .expect("connect QUIC client");
    let (mut driver, mut sender) = h3::client::new(h3_quinn::Connection::new(connection))
        .await
        .expect("create H3 client");
    let driver_task = tokio::spawn(async move { driver.wait_idle().await });
    let request = hyper::Request::builder()
        .uri(format!("https://localhost{path}"))
        .body(())
        .expect("build H3 request");
    let started = Instant::now();
    let mut stream = sender.send_request(request).await.expect("send H3 request");
    stream.finish().await.expect("finish H3 request");
    let response = stream.recv_response().await.expect("receive H3 response");
    let status = response.status();
    let mut chunks = Vec::new();
    let mut first_elapsed = None;
    while let Some(mut data) = stream.recv_data().await.expect("receive H3 data") {
        let mut chunk = Vec::with_capacity(data.remaining());
        while data.has_remaining() {
            chunk.extend_from_slice(&data.copy_to_bytes(data.remaining()));
        }
        if !chunk.is_empty() {
            first_elapsed.get_or_insert_with(|| started.elapsed());
            chunks.push(chunk);
        }
    }
    drop(sender);
    endpoint.close(quinn::VarInt::from_u32(0), b"test complete");
    let _ = driver_task.await;
    (
        status,
        chunks,
        first_elapsed.expect("H3 response has a data frame"),
    )
}

async fn http3_request(
    address: &str,
    method: hyper::Method,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> ProtocolResponse {
    use bytes::Buf;
    use quinn::crypto::rustls::QuicClientConfig;

    let mut crypto = test_client_config(vec![b"h3".to_vec()]);
    crypto.enable_early_data = true;
    let client_config = quinn::ClientConfig::new(Arc::new(
        QuicClientConfig::try_from(crypto).expect("build QUIC client config"),
    ));
    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().expect("client bind address"))
        .expect("create QUIC client endpoint");
    endpoint.set_default_client_config(client_config);
    let connection = endpoint
        .connect(address.parse().expect("server socket address"), "localhost")
        .expect("start QUIC connection")
        .await
        .expect("connect QUIC client");
    let (mut driver, mut sender) = h3::client::new(h3_quinn::Connection::new(connection))
        .await
        .expect("create H3 client");
    let driver_task = tokio::spawn(async move { driver.wait_idle().await });
    let mut request = hyper::Request::builder()
        .method(method)
        .uri(format!("https://localhost{path}"));
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let mut stream = sender
        .send_request(request.body(()).expect("build H3 request"))
        .await
        .expect("send H3 request");
    if !body.is_empty() {
        stream
            .send_data(bytes::Bytes::copy_from_slice(body))
            .await
            .expect("send H3 request body");
    }
    stream.finish().await.expect("finish H3 request");
    let response = stream.recv_response().await.expect("receive H3 response");
    let (parts, ()) = response.into_parts();
    let mut chunks = Vec::new();
    while let Some(mut data) = stream.recv_data().await.expect("receive H3 data") {
        let mut chunk = Vec::with_capacity(data.remaining());
        while data.has_remaining() {
            chunk.extend_from_slice(&data.copy_to_bytes(data.remaining()));
        }
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
    }
    drop(sender);
    endpoint.close(quinn::VarInt::from_u32(0), b"test complete");
    let _ = driver_task.await;
    ProtocolResponse {
        status: parts.status,
        headers: parts.headers,
        chunks,
    }
}

async fn abort_http3_response(address: &str, path: &str) {
    use bytes::Buf;
    use quinn::crypto::rustls::QuicClientConfig;

    let mut crypto = test_client_config(vec![b"h3".to_vec()]);
    crypto.enable_early_data = true;
    let client_config = quinn::ClientConfig::new(Arc::new(
        QuicClientConfig::try_from(crypto).expect("build abort QUIC client config"),
    ));
    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().expect("client bind address"))
        .expect("create abort QUIC client endpoint");
    endpoint.set_default_client_config(client_config);
    let connection = endpoint
        .connect(address.parse().expect("server socket address"), "localhost")
        .expect("start abort QUIC connection")
        .await
        .expect("connect abort QUIC client");
    let (mut driver, mut sender) = h3::client::new(h3_quinn::Connection::new(connection))
        .await
        .expect("create abort H3 client");
    let driver_task = tokio::spawn(async move { driver.wait_idle().await });
    let request = hyper::Request::builder()
        .uri(format!("https://localhost{path}"))
        .body(())
        .expect("build abort H3 request");
    let mut stream = sender
        .send_request(request)
        .await
        .expect("send abort H3 request");
    stream.finish().await.expect("finish abort H3 request");
    stream
        .recv_response()
        .await
        .expect("receive abort H3 response");
    let data = stream
        .recv_data()
        .await
        .expect("receive abort H3 data")
        .expect("abort H3 response has data");
    assert!(data.has_remaining(), "abort H3 data frame is empty");
    stream.stop_sending(h3::error::Code::H3_REQUEST_CANCELLED);
    drop(stream);
    drop(sender);
    endpoint.close(quinn::VarInt::from_u32(0), b"static response aborted");
    driver_task.abort();
    let _ = driver_task.await;
}

async fn wait_for_metric_value(address: &str, metric: &str, expected: u64) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let response =
            http2_request(address, hyper::Method::GET, "/__phrust/metrics", &[], &[]).await;
        let body = String::from_utf8(response.body()).expect("metrics response is UTF-8");
        let value = body
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(' ')?;
                (name == metric)
                    .then(|| value.parse::<u64>().ok())
                    .flatten()
            })
            .unwrap_or_default();
        if value >= expected {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "metric {metric} remained {value}, expected at least {expected}: {body}"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn http3_post_status(address: &str, path: &str, body: &[u8]) -> hyper::StatusCode {
    use quinn::crypto::rustls::QuicClientConfig;

    let mut crypto = test_client_config(vec![b"h3".to_vec()]);
    crypto.enable_early_data = true;
    let client_config = quinn::ClientConfig::new(Arc::new(
        QuicClientConfig::try_from(crypto).expect("build QUIC client config"),
    ));
    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().expect("client bind address"))
        .expect("create QUIC client endpoint");
    endpoint.set_default_client_config(client_config);
    let connection = endpoint
        .connect(address.parse().expect("server socket address"), "localhost")
        .expect("start QUIC connection")
        .await
        .expect("connect QUIC client");
    let (mut driver, mut sender) = h3::client::new(h3_quinn::Connection::new(connection))
        .await
        .expect("create H3 client");
    let driver_task = tokio::spawn(async move { driver.wait_idle().await });
    let request = hyper::Request::post(format!("https://localhost{path}"))
        .body(())
        .expect("build H3 POST request");
    let mut stream = sender.send_request(request).await.expect("send H3 request");
    stream
        .send_data(bytes::Bytes::copy_from_slice(body))
        .await
        .expect("send H3 request body");
    stream.finish().await.expect("finish H3 request");
    let status = stream
        .recv_response()
        .await
        .expect("receive H3 response")
        .status();
    while stream
        .recv_data()
        .await
        .expect("receive H3 response body")
        .is_some()
    {}
    drop(sender);
    endpoint.close(quinn::VarInt::from_u32(0), b"test complete");
    let _ = driver_task.await;
    status
}

fn start_server(docroot: &std::path::Path, extra_args: &[&str]) -> Child {
    let mut command = Proc::new(env!("CARGO_BIN_EXE_phrust-server"));
    command
        .args(["--listen", "127.0.0.1:0", "--docroot"])
        .arg(docroot)
        .args(extra_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.spawn().expect("spawn phrust-server")
}

fn temp_docroot() -> std::path::PathBuf {
    for attempt in 0..100 {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "phrust-server-health-{}-{unique}-{attempt}",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("create temp docroot: {error}"),
        }
    }
    panic!("create unique temp docroot");
}

fn fixture_docroot(path: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
        .canonicalize()
        .expect("fixture docroot")
}

fn read_listening_address(child: &mut Child) -> String {
    let stdout = child.stdout.take().expect("child stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("read listening line from server");
    line.strip_prefix("listening http://")
        .or_else(|| line.strip_prefix("listening https://"))
        .expect("listening line prefix")
        .trim()
        .to_string()
}

fn http_get(address: &str, path: &str) -> String {
    http_request(address, "GET", path)
}

fn http_request(address: &str, method: &str, path: &str) -> String {
    http_request_with_headers(address, method, path, &[], "")
}

fn http_request_after_connection_rejection(address: &str, method: &str, path: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let mut stream = TcpStream::connect(address).expect("connect after connection rejection");
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .expect("write request after connection rejection");
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("set timeout after connection rejection");
        let mut response = String::new();
        match stream.read_to_string(&mut response) {
            Ok(_) if !response.is_empty() => return response,
            Ok(_) | Err(_) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            result => panic!("server did not recover its connection permit: {result:?}"),
        }
    }
}

fn metric_value(metrics: &str, metric: &str) -> u64 {
    metrics
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(' ')?;
            (name == metric)
                .then(|| value.parse::<u64>().ok())
                .flatten()
        })
        .unwrap_or_default()
}

fn http_request_with_body(
    address: &str,
    method: &str,
    path: &str,
    content_type: &str,
    body: &str,
) -> String {
    let content_length = body.len().to_string();
    http_request_with_headers(
        address,
        method,
        path,
        &[
            ("Content-Type", content_type),
            ("Content-Length", content_length.as_str()),
        ],
        body,
    )
}

fn http_request_with_headers(
    address: &str,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> String {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match TcpStream::connect(address) {
            Ok(mut stream) => {
                let mut request =
                    format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
                for (name, value) in headers {
                    request.push_str(name);
                    request.push_str(": ");
                    request.push_str(value);
                    request.push_str("\r\n");
                }
                request.push_str("\r\n");
                request.push_str(body);
                stream.write_all(request.as_bytes()).expect("write request");
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .expect("set read timeout");
                let mut response = String::new();
                stream.read_to_string(&mut response).expect("read response");
                return response;
            }
            Err(error) if std::time::Instant::now() < deadline => {
                let _ = error;
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => panic!("connect to server: {error}"),
        }
    }
}

fn raw_http_request(address: &str, request: &[u8]) -> String {
    let mut stream = TcpStream::connect(address).expect("connect raw HTTP request");
    stream.write_all(request).expect("write raw HTTP request");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set raw HTTP timeout");
    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => response.extend_from_slice(&buffer[..read]),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::UnexpectedEof
                ) =>
            {
                break;
            }
            Err(error) => panic!("read raw HTTP response: {error}"),
        }
    }
    String::from_utf8_lossy(&response).into_owned()
}

fn stop_child(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn wait_for_file_contents(path: &std::path::Path, expected: &[u8], timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        if fs::read(path).is_ok_and(|contents| contents == expected) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "file {} did not reach expected contents",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn wait_for_directory_entry(path: &std::path::Path, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        if fs::read_dir(path)
            .expect("read temporary directory")
            .next()
            .is_some()
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "directory {} remained empty",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn send_sigint(child: &Child) {
    let status = Proc::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("send SIGINT");
    assert!(status.success(), "kill -INT failed with {status}");
}

fn send_sigterm(child: &Child) {
    let status = Proc::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .expect("send SIGTERM");
    assert!(status.success(), "kill -TERM failed with {status}");
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> std::process::ExitStatus {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().expect("poll child exit") {
            return status;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("server did not exit within {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn assert_response_contains_header(response: &str, name: &str, value: &str) {
    assert!(
        response_headers(response).any(|line| header_line_matches(line, name, value)),
        "missing header {name}: {value}\n{response}"
    );
}

fn assert_response_lacks_header(response: &str, name: &str, value: &str) {
    assert!(
        !response_headers(response).any(|line| header_line_matches(line, name, value)),
        "unexpected header {name}: {value}\n{response}"
    );
}

fn response_headers(response: &str) -> impl Iterator<Item = &str> {
    response
        .split_once("\r\n\r\n")
        .map_or(response, |(headers, _)| headers)
        .lines()
        .skip(1)
}

fn response_header_count(response: &str, expected_name: &str) -> usize {
    response_headers(response)
        .filter(|line| {
            line.split_once(':')
                .is_some_and(|(name, _)| name.trim().eq_ignore_ascii_case(expected_name))
        })
        .count()
}

fn response_header_values<'a>(response: &'a str, expected_name: &str) -> Vec<&'a str> {
    response_headers(response)
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case(expected_name)
                .then_some(value.trim())
        })
        .collect()
}

fn response_body(response: &str) -> &str {
    response.split_once("\r\n\r\n").map_or("", |(_, body)| body)
}

fn header_line_matches(line: &str, expected_name: &str, expected_value: &str) -> bool {
    let Some((name, value)) = line.split_once(':') else {
        return false;
    };
    name.trim().eq_ignore_ascii_case(expected_name) && value.trim() == expected_value
}
