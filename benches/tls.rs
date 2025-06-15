#[cfg(feature = "tls")]
use criterion::{criterion_group, criterion_main, Criterion};

#[cfg(feature = "tls")]
use wiremock::tls::MockTlsCertificates;

#[cfg(feature = "tls")]
// On the good old M1 processor it takes ~77 µs
pub fn tls_mock_tls_certificates_new(c: &mut Criterion) {
    c.bench_function("MockTlsCertificates::new", |b| {
        b.iter(|| MockTlsCertificates::new())
    });
}

#[cfg(feature = "tls")]
// TODO measure with a charger connected
pub fn tls_mock_tls_certificates_client(c: &mut Criterion) {
    let mock_tls_certificates = MockTlsCertificates::new();
    c.bench_function("MockTlsCertificates::gen_client", |b| {
        b.iter(|| mock_tls_certificates.gen_client("user@myserver.test"))
    });
}

#[cfg(feature = "tls")]
criterion_group!(
    benches,
    tls_mock_tls_certificates_new,
    tls_mock_tls_certificates_client
);

#[cfg(feature = "tls")]
criterion_main!(benches);

#[cfg(not(feature = "tls"))]
fn main() {}
