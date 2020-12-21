use crate::bare_mock_server::BareMockServer;
use async_trait::async_trait;
use deadpool::managed::{Object, Pool};
use once_cell::sync::Lazy;
use std::convert::Infallible;

static MOCK_SERVER_POOL: Lazy<Pool<BareMockServer, Infallible>> =
    Lazy::new(|| Pool::new(MockServerPoolManager, 1000));

pub(crate) async fn get_pooled_mock_server() -> Object<BareMockServer, Infallible> {
    MOCK_SERVER_POOL
        .get()
        .await
        .expect("Failed to get a MockServer from the pool")
}

pub(crate) struct MockServerPoolManager;

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
