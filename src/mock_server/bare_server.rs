use crate::mock_server::hyper::run_server;
use crate::mock_set::MockId;
use crate::mock_set::MountedMockSet;
use crate::{mock::Mock, verification::VerificationOutcome, Request};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::LocalSet;

/// An HTTP web-server running in the background to behave as one of your dependencies using `Mock`s
/// for testing purposes.
///
/// `BareMockServer` is the actual mock server behind the publicly-exposed `MockServer`, which
/// is instead a thin facade over a `BareMockServer` retrieved from a pool - see `get_pooled_server`
/// for more details.
pub(crate) struct BareMockServer {
    state: Arc<RwLock<MockServerState>>,
    server_address: SocketAddr,
    // When `_shutdown_trigger` gets dropped the listening server terminates gracefully.
    _shutdown_trigger: tokio::sync::oneshot::Sender<()>,
}

/// The elements of [`BareMockServer`] that are affected by each incoming request.
/// By bundling them together, we can expose a unified `handle_request` that ensures
/// they are kept in sync without having to leak logic across multiple corners of the `wiremock`'s codebase.
pub(super) struct MockServerState {
    mock_set: MountedMockSet,
    received_requests: Option<Vec<Request>>,
}

impl MockServerState {
    pub(super) async fn handle_request(
        &mut self,
        request: Request,
    ) -> (http_types::Response, Option<futures_timer::Delay>) {
        // If request recording is enabled, record the incoming request
        // by adding it to the `received_requests` stack
        if let Some(received_requests) = &mut self.received_requests {
            received_requests.push(request.clone());
        }
        self.mock_set.handle_request(request).await
    }
}

impl BareMockServer {
    /// Start a new instance of a `BareMockServer` listening on the specified
    /// [`TcpListener`](std::net::TcpListener).
    pub(super) async fn start(listener: TcpListener, request_recording: RequestRecording) -> Self {
        let (shutdown_trigger, shutdown_receiver) = tokio::sync::oneshot::channel();
        let received_requests = match request_recording {
            RequestRecording::Enabled => Some(Vec::new()),
            RequestRecording::Disabled => None,
        };
        let state = Arc::new(RwLock::new(MockServerState {
            mock_set: MountedMockSet::new(),
            received_requests,
        }));
        let server_address = listener
            .local_addr()
            .expect("Failed to get server address.");

        let server_state = state.clone();
        std::thread::spawn(move || {
            let server_future = run_server(listener, server_state, shutdown_receiver);

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Cannot build local tokio runtime");

            LocalSet::new().block_on(&runtime, server_future)
        });
        for _ in 0..40 {
            if TcpStream::connect_timeout(&server_address, std::time::Duration::from_millis(25))
                .is_ok()
            {
                break;
            }
            futures_timer::Delay::new(std::time::Duration::from_millis(25)).await;
        }

        Self {
            state,
            server_address,
            _shutdown_trigger: shutdown_trigger,
        }
    }

    /// Register a `Mock` on an instance of `BareMockServer`.
    ///
    /// Be careful! `Mock`s are not effective until they are `mount`ed or `register`ed on a
    /// `BareMockServer`.
    pub(crate) async fn register(&self, mock: Mock) {
        self.state.write().await.mock_set.register(mock);
    }

    /// Register a **scoped** `Mock` on an instance of `MockServer`.
    ///
    /// When using `register`, your `Mock`s will be active until the `MockServer` is shut down.  
    /// When using `register_as_scoped`, your `Mock`s will be active as long as the returned `MockGuard` is not dropped.
    /// When the returned `MockGuard` is dropped, `MockServer` will verify that the expectations set on the scoped `Mock` were
    /// verified - if not, it will panic.
    pub async fn register_as_scoped(&self, mock: Mock) -> MockGuard {
        let mock_id = self.state.write().await.mock_set.register(mock);
        MockGuard {
            mock_id,
            server_state: self.state.clone(),
        }
    }

    /// Drop all mounted `Mock`s from an instance of `BareMockServer`.
    /// Delete all recorded requests.
    ///
    /// It *must* be called if you plan to reuse a `BareMockServer` instance (i.e. in our
    /// `MockServerPoolManager`).
    pub(crate) async fn reset(&self) {
        let mut state = self.state.write().await;
        state.mock_set.reset();
        if let Some(received_requests) = &mut state.received_requests {
            received_requests.clear();
        }
    }

    /// Verify that all mounted `Mock`s on this instance of `BareMockServer` have satisfied
    /// their expectations on their number of invocations.
    pub(crate) async fn verify(&self) -> VerificationOutcome {
        let mock_set = &self.state.read().await.mock_set;
        mock_set.verify_all()
    }

    /// Return the base uri of this running instance of `BareMockServer`, e.g. `http://127.0.0.1:4372`.
    ///
    /// Use this method to compose uris when interacting with this instance of `BareMockServer` via
    /// an HTTP client.
    pub(crate) fn uri(&self) -> String {
        format!("http://{}", self.server_address)
    }

    /// Return the socket address of this running instance of `BareMockServer`, e.g. `127.0.0.1:4372`.
    ///
    /// Use this method to interact with the `BareMockServer` using `TcpStream`s.
    pub(crate) fn address(&self) -> &SocketAddr {
        &self.server_address
    }

    /// Return a vector with all the requests received by the `BareMockServer` since it started.  
    /// If no request has been served, it returns an empty vector.
    ///
    /// If request recording was disabled, it returns `None`.
    pub(crate) async fn received_requests(&self) -> Option<Vec<Request>> {
        let state = self.state.read().await;
        state.received_requests.to_owned()
    }
}

pub(super) enum RequestRecording {
    Enabled,
    Disabled,
}

/// You get a `MockGuard` when registering a **scoped** [`Mock`] using [`MockServer::register_as_scoped`](crate::MockServer::register_as_scoped)
/// or [`Mock::mount_as_scoped`].
///
/// When the [`MockGuard`] is dropped, the [`MockServer`](crate::MockServer) verifies that the expectations set on the
/// scoped [`Mock`] were verified - if not, it will panic.
///
/// # Limitations
///
/// When expectations of a scoped [`Mock`] are not verified, it will trigger a panic - just like a normal [`Mock`].
/// Due to [limitations](https://internals.rust-lang.org/t/should-drop-glue-use-track-caller/13682) in Rust's `Drop` trait,
/// the panic message will not include the filename and the line location
/// where the corresponding `MockGuard` was dropped - it will point into `wiremock`'s source code.
///
/// This can be an issue when you are using more than one scoped [`Mock`] in a single test - which of them panicked?
/// To improve your debugging experience it is strongly recommended to use [`Mock::named`] to assign a unique
/// identifier to your scoped [`Mock`]s, which will in turn be referenced in the panic message if their expectations are
/// not met.
#[must_use = "All *_scoped methods return a `MockGuard`.
This guard MUST be bound to a variable (e.g. _mock_guard), \
otherwise the mock will immediately be unmounted (and its expectations checked).
Check `wiremock`'s documentation on scoped mocks for more details."]
pub struct MockGuard {
    mock_id: MockId,
    server_state: Arc<RwLock<MockServerState>>,
}

impl MockGuard {
    pub async fn received_requests(&self) -> Vec<crate::Request> {
        let state = self.server_state.read().await;
        let (mounted_mock, _) = &state.mock_set[self.mock_id];
        mounted_mock.received_requests()
    }
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
