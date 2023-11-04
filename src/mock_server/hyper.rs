use crate::mock_server::bare_server::MockServerState;
use hyper::service::{make_service_fn, service_fn};
use std::net::TcpListener;
use std::sync::Arc;
use tokio::sync::RwLock;

type DynError = Box<dyn std::error::Error + Send + Sync>;

/// The actual HTTP server responding to incoming requests according to the specified mocks.
pub(super) async fn run_server(
    listener: TcpListener,
    server_state: Arc<RwLock<MockServerState>>,
    shutdown_signal: tokio::sync::oneshot::Receiver<()>,
) {
    let request_handler = make_service_fn(move |_| {
        let server_state = server_state.clone();
        async move {
            Ok::<_, DynError>(service_fn(move |request: hyper::Request<hyper::Body>| {
                let server_state = server_state.clone();
                async move {
                    let wiremock_request = crate::Request::from_hyper(request).await;
                    let (response, delay) = server_state
                        .write()
                        .await
                        .handle_request(wiremock_request)
                        .await;

                    // We do not wait for the delay within the handler otherwise we would be
                    // holding on to the write-side of the `RwLock` on `mock_set`.
                    // Holding on the lock while waiting prevents us from handling other requests until
                    // we have waited the whole duration specified in the delay.
                    // In particular, we cannot perform even perform read-only operation -
                    // e.g. check that mock assumptions have been verified.
                    // Using long delays in tests without handling the delay as we are doing here
                    // caused tests to hang (see https://github.com/seanmonstar/reqwest/issues/1147)
                    if let Some(delay) = delay {
                        delay.await;
                    }

                    Ok::<_, DynError>(response)
                }
            }))
        }
    });

    let server = hyper::Server::from_tcp(listener)
        .unwrap()
        .executor(LocalExec)
        .serve(request_handler)
        .with_graceful_shutdown(async {
            // This futures resolves when either:
            // - the sender half of the channel gets dropped (i.e. MockServer is dropped)
            // - the sender is used, therefore sending a poison pill willingly as a shutdown signal
            let _ = shutdown_signal.await;
        });

    if let Err(e) = server.await {
        panic!("Mock server failed: {}", e);
    }
}

// An executor that can spawn !Send futures.
#[derive(Clone, Copy, Debug)]
struct LocalExec;

impl<F> hyper::rt::Executor<F> for LocalExec
where
    F: std::future::Future + 'static, // not requiring `Send`
{
    fn execute(&self, fut: F) {
        // This will spawn into the currently running `LocalSet`.
        tokio::task::spawn_local(fut);
    }
}
