use crate::mock_server::hyper::run_server;
use crate::mock_set::ActiveMockSet;
use crate::{mock::Mock, verification::VerificationOutcome, Request};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::LocalSet;

/// An HTTP web-server running in the background to behave as one of your dependencies using `Mock`s
/// for testing purposes.
///
/// `BareMockServer` is the actual mock server behind the publicly-exposed `MockServer`, which
/// is instead a thin facade over a `BareMockServer` retrieved from a pool - see `get_pooled_server`
/// for more details.
pub(crate) struct BareMockServer {
    mock_set: Arc<RwLock<ActiveMockSet>>,
    received_requests: Option<Arc<Mutex<Vec<Request>>>>,
    server_address: SocketAddr,
    // When `_shutdown_trigger` gets dropped the listening server terminates gracefully.
    _shutdown_trigger: tokio::sync::oneshot::Sender<()>,
}

impl BareMockServer {
    /// Start a new instance of a `BareMockServer` listening on the specified
    /// [`TcpListener`](std::net::TcpListener).
    pub(super) async fn start(listener: TcpListener, request_recording: RequestRecording) -> Self {
        let (shutdown_trigger, shutdown_receiver) = tokio::sync::oneshot::channel();
        let mock_set = Arc::new(RwLock::new(ActiveMockSet::new()));
        let received_requests = match request_recording {
            RequestRecording::Enabled => Some(Arc::new(Mutex::new(Vec::new()))),
            RequestRecording::Disabled => None,
        };
        let server_address = listener
            .local_addr()
            .expect("Failed to get server address.");

        let server_mock_set = mock_set.clone();
        let server_received_requests = received_requests.clone();
        std::thread::spawn(move || {
            let server_future = run_server(
                listener,
                server_mock_set,
                server_received_requests,
                shutdown_receiver,
            );

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
            mock_set,
            received_requests,
            server_address,
            _shutdown_trigger: shutdown_trigger,
        }
    }

    /// Register a `Mock` on an instance of `BareMockServer`.
    ///
    /// Be careful! `Mock`s are not effective until they are `mount`ed or `register`ed on a
    /// `BareMockServer`.
    ///
    /// `register` is an asynchronous method, make sure to `.await` it!
    pub(crate) async fn register(&self, mock: Mock) {
        self.mock_set.write().await.register(mock);
    }

    /// Drop all mounted `Mock`s from an instance of `BareMockServer`.
    ///
    /// It *must* be called if you plan to reuse a `BareMockServer` instance (i.e. in our
    /// `MockServerPoolManager`).
    pub(crate) async fn reset(&self) {
        self.mock_set.write().await.reset();
    }

    /// Verify that all mounted `Mock`s on this instance of `BareMockServer` have satisfied
    /// their expectations on their number of invocations.
    pub(crate) async fn verify(&self) -> VerificationOutcome {
        let mock_set = self.mock_set.read().await;
        mock_set.verify()
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
        if let Some(received_requests) = &self.received_requests {
            Some(received_requests.lock().await.clone())
        } else {
            None
        }
    }
}

pub(super) enum RequestRecording {
    Enabled,
    Disabled,
}
