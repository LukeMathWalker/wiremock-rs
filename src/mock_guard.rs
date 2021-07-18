use crate::mock_server::MockServerState;
use crate::mock_set::MockId;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MockGuard {
    pub(crate) mock_id: MockId,
    pub(crate) server_state: Arc<RwLock<MockServerState>>,
}

impl Drop for MockGuard {
    fn drop(&mut self) {
        let future = async move {
            let MockGuard {
                mock_id,
                server_state,
            } = self;
            let state = server_state.read().await;
            state.mock_set.verify(*mock_id);
        };
        futures::executor::block_on(future)
    }
}
