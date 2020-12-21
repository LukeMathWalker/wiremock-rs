use crate::mock_server::bare_server::BareMockServer;
use async_trait::async_trait;
use deadpool::managed::{Object, Pool};
use once_cell::sync::Lazy;
use std::convert::Infallible;

/// A pool of `BareMockServer`s.
///
/// ## Design constraints
///
/// `wiremock`'s pooling is designed to be an invisible optimisation: users of the crate, if
/// we are successful, should never have to reason about it.
///
/// ## Motivation
///
/// Why are we pooling `BareMockServer`s?
/// Mostly to reduce the number of `TcpListener`s that are being opened and closed, therefore
/// mitigating risk of our users having to fight/raise OS limits for the maximum number of open
/// connections (e.g. ulimit on Linux).
///
/// It is also marginally faster to get a pooled `BareMockServer` than to create a new one, but
/// the absolute time is so small (<1 ms) that it does not make a material difference in a real
/// world test suite.
static MOCK_SERVER_POOL: Lazy<Pool<BareMockServer, Infallible>> = Lazy::new(|| {
    // We are choosing an arbitrarily high max_size because we never want a test to "wait" for
    // a `BareMockServer` instance to become available.
    //
    // We might expose in the future a way for a crate user to tune this value.
    Pool::new(MockServerPoolManager, 1000)
});

/// Retrieve a `BareMockServer` from the pool.
/// The operation should never fail.
pub(crate) async fn get_pooled_mock_server() -> Object<BareMockServer, Infallible> {
    MOCK_SERVER_POOL
        .get()
        .await
        .expect("Failed to get a MockServer from the pool")
}

/// The `BareMockServer` pool manager.
///
/// It:
/// - creates a new `BareMockServer` if there is none to borrow from the pool;
/// - "cleans up" used `BareMockServer`s before making them available again for other tests to use.
struct MockServerPoolManager;

#[async_trait]
impl deadpool::managed::Manager<BareMockServer, Infallible> for MockServerPoolManager {
    async fn create(&self) -> Result<BareMockServer, Infallible> {
        Ok(BareMockServer::start().await)
    }

    async fn recycle(
        &self,
        mock_server: &mut BareMockServer,
    ) -> deadpool::managed::RecycleResult<Infallible> {
        // Remove all existing settings - we want to start clean when the mock server
        // is picked up again from the pool.
        mock_server.reset().await;
        Ok(())
    }
}
