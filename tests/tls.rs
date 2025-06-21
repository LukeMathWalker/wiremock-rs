#[cfg(feature = "tls")]
mod tlstests {
    use hyper_server::tls_rustls::RustlsConfig;
    use wiremock::tls_certs::MockTlsCertificates;

    #[async_std::test]
    async fn test_tls_basic() {
        let certs = MockTlsCertificates::new();

        let mock_server = wiremock::MockServer::builder()
            .start_https(
                certs
                    .get_rustls_config()
                    .await
                    .expect("Failed to create RustlsConfig"),
            )
            .await;
        let uri = mock_server.uri();
        let port = mock_server.address().port();

        assert_eq!(uri, format!("https://127.0.0.1:{}", port));
    }

    #[async_std::test]
    async fn test_tls_invalid() {
        let certs = MockTlsCertificates::new();

        let mock_server = wiremock::MockServer::builder()
            .start_https(
                certs
                    .get_rustls_config()
                    .await
                    .expect("Failed to create RustlsConfig"),
            )
            .await;
        let uri = mock_server.uri();

        let client = reqwest::Client::builder()
            .use_rustls_tls() // It fails on MacOS with native-tls no mattter what, so use rustls.
            .build()
            .expect("Failed to build HTTP client");

        client
            .get(uri.clone())
            .send()
            .await
            .expect_err("Expected request to fail due to invalid TLS certificate");
    }

    #[async_std::test]
    async fn test_tls_anonymous() {
        let certs = MockTlsCertificates::new();

        let mock_server = wiremock::MockServer::builder()
            .start_https(
                certs
                    .get_rustls_config()
                    .await
                    .expect("Failed to create RustlsConfig"),
            )
            .await;
        let uri = mock_server.uri();

        let reqwest_root_certificate = reqwest::Certificate::from_der(certs.get_root_cert().der())
            .expect("Failed to create certificate from DER");
        let client = reqwest::Client::builder()
            .add_root_certificate(reqwest_root_certificate)
            .use_rustls_tls() // It fails on MacOS with native-tls no mattter what, so use rustls.
            .build()
            .expect("Failed to build HTTP client");

        client
            .get(uri.clone())
            .send()
            .await
            .expect("Failed to send request to the mock server");
    }

    #[async_std::test]
    async fn test_tls_with_client_cert() {
        let certs = MockTlsCertificates::new();

        let mock_server = wiremock::MockServer::builder()
            .start_https(
                certs
                    .get_rustls_config()
                    .await
                    .expect("Failed to create RustlsConfig"),
            )
            .await;
        let uri = mock_server.uri();

        let reqwest_root_certificate = reqwest::Certificate::from_der(certs.get_root_cert().der())
            .expect("Failed to create certificate from DER");
        let (client_cert, client_key) = certs.gen_client_cert("johnny@house-of-leaves.test");
        let client_cert_pem = client_cert.pem();
        let client_key_pem = client_key.serialize_pem();
        let client_cert =
            reqwest::Identity::from_pem((client_cert_pem + &client_key_pem).as_bytes())
                .expect("Failed to create Identity from PEM");
        let client = reqwest::Client::builder()
            .add_root_certificate(reqwest_root_certificate)
            .identity(client_cert)
            .use_rustls_tls() // It fails on MacOS with native-tls no mattter what, so use rustls.
            .build()
            .expect("Failed to build HTTP client");

        client
            .get(uri.clone())
            .send()
            .await
            .expect("Failed to send request to the mock server");
    }

    #[async_std::test]
    async fn test_tls_from_file() {
        let rustls_config = RustlsConfig::from_pem(
            include_bytes!("fixtures/tls/server.crt").to_vec(),
            include_bytes!("fixtures/tls/server.key").to_vec(),
        )
        .await
        .expect("Failed to create RustlsConfig from file");

        let mock_server = wiremock::MockServer::builder()
            .start_https(rustls_config)
            .await;
        let uri = mock_server.uri();

        let root_cert = reqwest::Certificate::from_pem(include_bytes!("fixtures/tls/rootCA.crt"))
            .expect("Failed to create certificate from PEM");
        let client = reqwest::Client::builder()
            .add_root_certificate(root_cert)
            .use_rustls_tls() // It fails on MacOS with native-tls no mattter what, so use rustls.
            .build()
            .expect("Failed to build HTTP client");

        client
            .get(uri.clone())
            .send()
            .await
            .expect("Failed to send request to the mock server");
    }
}
