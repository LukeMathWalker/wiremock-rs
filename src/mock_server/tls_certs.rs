//! TLS Certificate generation helpers.
//!
// Based on https://github.com/rustls/rustls/blob/main/rustls/examples/internal/test_ca.rs
use std::{
    convert::TryInto,
    fmt::Display,
    net::IpAddr,
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
};

use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose,
    IsCa, KeyPair, KeyUsagePurpose, SanType, SerialNumber, SignatureAlgorithm, PKCS_ED25519,
};
use rustls_pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};

pub const DEFAULT_ALGORITHM: &SignatureAlgorithm = &PKCS_ED25519;

const ISSUER_KEY_USAGES: &[KeyUsagePurpose; 7] = &[
    KeyUsagePurpose::CrlSign,
    KeyUsagePurpose::KeyCertSign,
    KeyUsagePurpose::DigitalSignature,
    KeyUsagePurpose::ContentCommitment,
    KeyUsagePurpose::KeyEncipherment,
    KeyUsagePurpose::DataEncipherment,
    KeyUsagePurpose::KeyAgreement,
];

const ISSUER_EXTENDED_KEY_USAGES: &[ExtendedKeyUsagePurpose; 2] = &[
    ExtendedKeyUsagePurpose::ServerAuth,
    ExtendedKeyUsagePurpose::ClientAuth,
];

const EE_KEY_USAGES: &[KeyUsagePurpose; 2] = &[
    KeyUsagePurpose::DigitalSignature,
    KeyUsagePurpose::ContentCommitment,
];

static SERIAL_NUMBER: AtomicU64 = AtomicU64::new(1);

/// Configuration for a mock server TLS setup.
pub struct MockServerTlsConfig {
    /// Root CA certificate in DER format.
    pub root_cert_der: Vec<u8>,
    /// Server certificate in DER format.
    pub server_cert_der: Vec<u8>,
    /// Server keypair in DER format.
    pub server_keypair_der: Vec<u8>,
}

impl MockServerTlsConfig {
    #[inline]
    pub fn from_der(
        root_cert_der: Vec<u8>,
        server_cert_der: Vec<u8>,
        server_keypair_der: Vec<u8>,
    ) -> Self {
        Self {
            root_cert_der,
            server_cert_der,
            server_keypair_der,
        }
    }

    /// Create a new `MockServerTlsConfig` from PEM-encoded certificates and key.
    ///
    /// Panics if the data cannot be parsed as valid PEM.
    pub fn from_pem(
        root_cert_pem: Vec<u8>,
        server_cert_pem: Vec<u8>,
        server_keypair_pem: Vec<u8>,
    ) -> Self {
        let root_cert_der = CertificateDer::from_pem_slice(&root_cert_pem)
            .expect("Failed to parse root certificate from PEM")
            .to_vec();
        let server_cert_der = CertificateDer::from_pem_slice(&server_cert_pem)
            .expect("Failed to parse server certificate from PEM")
            .to_vec();
        let server_keypair_der = PrivateKeyDer::from_pem_slice(&server_keypair_pem)
            .expect("Failed to parse server key from PEM")
            .secret_der()
            .to_vec();

        Self {
            root_cert_der,
            server_cert_der,
            server_keypair_der,
        }
    }
}

/// Mock self-signed TLS certificates for testing purposes.
///
/// This struct generates a root CA certificate and a server certificate signed by it, along with a key pair for the
/// server.
pub struct MockTlsCertificates {
    pub root_cert: Certificate,
    pub server_cert: Certificate,
    pub server_keypair: KeyPair,
}

impl MockTlsCertificates {
    /// Generate server certificates and keypair with "localhost" and "127.0.0.1" as hostnames.
    ///
    /// Ed25519 is used as the default signature algorithm.
    // On the good old M1 processor it takes ~77 µs
    #[inline]
    pub fn random() -> Self {
        Self::random_with_hostnames(default_hostnames())
    }

    /// Generate server certificates and key with custom hostnames and IPs.
    pub fn random_with_hostnames(hostnames: impl Into<Vec<SanType>>) -> Self {
        let (root_cert, root_key) = gen_root_cert(DEFAULT_ALGORITHM);
        // We do not bother to have an intermediate certificate because CAs use them for flexibility only.
        let (server_cert, server_keypair) =
            gen_server_cert(&root_cert, &root_key, DEFAULT_ALGORITHM, hostnames.into());

        Self {
            root_cert,
            server_cert,
            server_keypair,
        }
    }

    /// Get the root CA certificate.
    #[inline]
    pub fn get_root_ca_cert(&self) -> &Certificate {
        &self.root_cert
    }

    /// Get the server certificate signed by CA certificate, in DER format.
    #[inline]
    pub fn server_cert_der(&self) -> &CertificateDer {
        self.server_cert.der()
    }

    /// Get the server keypair in DER format.
    pub fn server_private_keypair_der(&self) -> PrivateKeyDer {
        self.server_keypair
            .serialize_der()
            .try_into()
            .expect("Failed to deserialize a serialized key")
    }

    /// Generate a client certificate signed by the CA certificate.
    ///
    /// Each invocation generates a new keypair and a certificate.
    #[inline]
    pub fn generate_client_cert(&self, email: &str) -> (Certificate, KeyPair) {
        gen_client_cert(
            &self.server_cert,
            &self.server_keypair,
            DEFAULT_ALGORITHM,
            email,
        )
    }

    /// Get the server configuration for use with a mock server.
    #[inline]
    pub fn get_server_config(&self) -> MockServerTlsConfig {
        MockServerTlsConfig {
            root_cert_der: self.root_cert.der().to_vec(),
            server_cert_der: self.server_cert.der().to_vec(),
            server_keypair_der: self.server_keypair.serialize_der(),
        }
    }
}

// The methods are not const, so we use a function.
fn default_hostnames() -> Vec<SanType> {
    vec![
        SanType::DnsName(("localhost".to_string()).try_into().unwrap()),
        SanType::IpAddress(IpAddr::from_str("127.0.0.1").unwrap()),
    ]
}

fn gen_root_cert(alg: &'static SignatureAlgorithm) -> (Certificate, KeyPair) {
    let keypair = KeyPair::generate_for(alg).unwrap();
    let serial = SERIAL_NUMBER.fetch_add(1, Ordering::SeqCst);

    let mut params = CertificateParams::default();
    params.distinguished_name = root_common_name(serial);
    params.use_authority_key_identifier_extension = true;
    params.serial_number = Some(SerialNumber::from_slice(&serial.to_be_bytes()[..]));

    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = ISSUER_KEY_USAGES.to_vec();
    params.extended_key_usages = ISSUER_EXTENDED_KEY_USAGES.to_vec();

    let cert = params.self_signed(&keypair).unwrap();
    (cert, keypair)
}

fn gen_server_cert(
    signer_cert: &Certificate,
    signer_keypair: &KeyPair,
    alg: &'static SignatureAlgorithm,
    hostnames: Vec<SanType>,
) -> (Certificate, KeyPair) {
    let keypair = KeyPair::generate_for(alg).unwrap();
    let serial = SERIAL_NUMBER.fetch_add(1, Ordering::SeqCst);

    let mut params = CertificateParams::default();
    params.distinguished_name = server_common_name(serial);
    params.use_authority_key_identifier_extension = true;
    params.serial_number = Some(SerialNumber::from_slice(&serial.to_be_bytes()[..]));
    params.is_ca = IsCa::NoCa;
    params.key_usages = EE_KEY_USAGES.to_vec();
    params.subject_alt_names = hostnames;

    let cert = params.signed_by(&keypair, signer_cert, signer_keypair).unwrap();
    (cert, keypair)
}

fn gen_client_cert(
    signer_cert: &Certificate,
    signer_keypair: &KeyPair,
    alg: &'static SignatureAlgorithm,
    email: &str,
) -> (Certificate, KeyPair) {
    let keypair = KeyPair::generate_for(alg).unwrap();
    let serial = SERIAL_NUMBER.fetch_add(1, Ordering::SeqCst);

    let mut params = CertificateParams::default();
    params.distinguished_name = client_common_name(serial, email);
    params.use_authority_key_identifier_extension = true;
    params.serial_number = Some(SerialNumber::from_slice(&serial.to_be_bytes()[..]));
    params.is_ca = IsCa::NoCa;
    params.key_usages = EE_KEY_USAGES.to_vec();
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    params.subject_alt_names = vec![SanType::Rfc822Name(
        email
            .to_string()
            .try_into()
            .unwrap_or_else(|_| panic!("Invalid email: {}", email)),
    )];

    let cert = params.signed_by(&keypair, signer_cert, signer_keypair).unwrap();
    (cert, keypair)
}

fn root_common_name(id: impl Display) -> DistinguishedName {
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(
        rcgen::DnType::CommonName,
        format!("Test-only temporary root CA #{id}"),
    );

    distinguished_name
}

fn server_common_name(id: impl Display) -> DistinguishedName {
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(
        rcgen::DnType::CommonName,
        format!("Test-only temporary server #{id}"),
    );

    distinguished_name
}

fn client_common_name(id: impl Display, name: &str) -> DistinguishedName {
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(
        rcgen::DnType::CommonName,
        format!("Test-only temporary client {name} #{id}"),
    );

    distinguished_name
}
