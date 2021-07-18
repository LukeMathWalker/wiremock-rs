use crate::mock_server::hyper::run_server;
use crate::mock_set::ActiveMockSet;
use crate::{mock::Mock, verification::VerificationOutcome, MockGuard, Request};
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

pub(crate) struct MockServerState {
    pub(crate) mock_set: ActiveMockSet,
    pub(crate) received_requests: Option<Vec<Request>>,
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
            mock_set: ActiveMockSet::new(),
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
    /// When using `register_scoped`, your `Mock`s will be active as long as the returned `MockGuard` is not dropped.
    /// When the returned `MockGuard` is dropped, `MockServer` will verify that the expectations set on the scoped `Mock` were
    /// verified - if not, it will panic.
    pub async fn register_scoped(&self, mock: Mock) -> MockGuard {
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
        if let Some(received_requests) = &state.received_requests {
            Some(received_requests.clone())
        } else {
            None
        }
    }
}

pub(super) enum RequestRecording {
    Enabled,
    Disabled,
}
