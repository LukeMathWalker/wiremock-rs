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
            let mut state = server_state.write().await;
            let report = state.mock_set.verify(*mock_id);

            if !report.is_satisfied() {
                let received_requests_message = if let Some(received_requests) =
                    &state.received_requests
                {
                    if received_requests.is_empty() {
                        "The server did not receive any request.".into()
                    } else {
                        format!(
                            "Received requests:\n{}",
                            received_requests
                                .iter()
                                .enumerate()
                                .map(|(index, request)| {
                                    format!(
                                        "- Request #{}\n{}",
                                        index + 1,
                                        &format!("\t{}", request)
                                    )
                                })
                                .collect::<String>()
                        )
                    }
                } else {
                    "Enable request recording on the mock server to get the list of incoming requests as part of the panic message.".into()
                };

                let verifications_error = format!("- {}\n", report.error_message());
                let error_message = format!(
                    "Verification failed for a scoped mock:\n{}\n{}",
                    verifications_error, received_requests_message
                );
                if std::thread::panicking() {
                    log::debug!("{}", &error_message);
                } else {
                    panic!("{}", &error_message);
                }
            } else {
                state.mock_set.deactivate(*mock_id);
            }
        };
        futures::executor::block_on(future)
    }
}
