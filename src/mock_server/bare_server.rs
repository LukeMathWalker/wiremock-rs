use crate::mock::Mock;
use crate::mock_server::hyper::run_server;
use crate::mock_set::MockSet;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::RwLock;
use tokio::task::LocalSet;

/// An HTTP web-server running in the background to behave as one of your dependencies using `Mock`s
/// for testing purposes.
///
/// `BareMockServer` is the actual mock server behind the publicly-exposed `MockServer`, which
/// is instead a thin facade over a `BareMockServer` retrieved from a pool - see `get_pooled_server`
/// for more details.
pub(crate) struct BareMockServer {
    mock_set: Arc<RwLock<MockSet>>,
    server_address: SocketAddr,
    // When `_shutdown_trigger` gets dropped the listening server terminates gracefully.
    _shutdown_trigger: tokio::sync::oneshot::Sender<()>,
}

impl BareMockServer {
    /// Start a new instance of a `BareMockServer`.
    ///
    /// Each instance of `BareMockServer` is fully isolated: `start` takes care of finding a random
    /// port available on your local machine, binding it to a `TcpListener` and then
    /// assign it to the new `BareMockServer`.
    pub(crate) async fn start() -> Self {
        let (shutdown_trigger, shutdown_receiver) = tokio::sync::oneshot::channel();
        let mock_set = Arc::new(RwLock::new(MockSet::new()));
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to find a free port!");
        let server_address = listener
            .local_addr()
            .expect("Failed to get server address.");

        let server_mock_set = mock_set.clone();
        std::thread::spawn(move || {
            let server_future = run_server(listener, server_mock_set, shutdown_receiver);

            let mut runtime = tokio::runtime::Builder::new()
                .enable_all()
                .basic_scheduler()
                .build()
                .expect("Cannot build local tokio runtime");

            LocalSet::new().block_on(&mut runtime, server_future)
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
        self.mock_set
            .write()
            .expect("Poisoned lock!")
            .register(mock);
    }

    /// Drop all mounted `Mock`s from an instance of `BareMockServer`.
    ///
    /// It *must* be called if you plan to reuse a `BareMockServer` instance (i.e. in our
    /// `MockServerPoolManager`).
    pub(crate) async fn reset(&self) {
        self.mock_set.write().expect("Poisoned lock!").reset();
    }

    /// Verify that all mounted `Mock`s on this instance of `BareMockServer` have satisfied
    /// their expectations on their number of invocations.
    pub(crate) fn verify(&self) -> bool {
        self.mock_set.read().expect("Poisoned lock!").verify()
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
}
