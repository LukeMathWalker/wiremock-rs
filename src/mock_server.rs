use crate::mock::Mock;
use crate::mock_set::MockSet;
use crate::server::run_server;
use log::debug;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::RwLock;
use tokio::task::LocalSet;

/// An HTTP web-server running in the background to behave as one of your dependencies using `Mock`s for testing purposes.
///
/// Each instance of `MockServer` is fully isolated: `start` takes care of finding a random port
/// available on your local machine which is assigned to the new `MockServer`.
///
/// ## Best practices
///
/// You should use one instance of `MockServer` for each REST API that your application interacts
/// with and needs mocking for testing purposes.
///
/// You should use one instance of `MockServer` for each test, to ensure full isolation and
/// no cross-test interference.
///
/// You can register as many `Mock`s as your scenario requires on a `MockServer`.
pub struct MockServer {
    mock_set: Arc<RwLock<MockSet>>,
    server_address: SocketAddr,
    // Used to trigger server shutdown on Drop
    _shutdown_trigger: tokio::sync::oneshot::Sender<()>,
}

impl MockServer {
    /// Start a new instance of a `MockServer`.
    ///
    /// Each instance of `MockServer` is fully isolated: `start` takes care of finding a random port
    /// available on your local machine which is assigned to the new `MockServer`.
    ///
    /// You should use one instance of `MockServer` for each REST API that your application interacts
    /// with and needs mocking for testing purposes.
    ///
    /// ### Example:
    /// ```rust
    /// use wiremock::{MockServer, Mock, ResponseTemplate};
    /// use wiremock::matchers::method;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // Arrange
    ///     let mock_server_one = MockServer::start().await;
    ///     let mock_server_two = MockServer::start().await;
    ///
    ///     assert!(mock_server_one.address() != mock_server_two.address());
    ///
    ///     let mock = Mock::given(method("GET")).respond_with(ResponseTemplate::new(200));
    ///     // Registering the mock with the first mock server - it's now effective!
    ///     // But it *won't* be used by the second mock server!
    ///     mock_server_one.register(mock).await;
    ///
    ///     // Act
    ///
    ///     let status = surf::get(&mock_server_one.uri())
    ///         .await
    ///         .unwrap()
    ///         .status();
    ///     assert_eq!(status, 200);
    ///
    ///     // This would have matched our mock, but we haven't registered it for `mock_server_two`!
    ///     // Hence it returns a 404, the default response when no mocks matched on the mock server.
    ///     let status = surf::get(&mock_server_two.uri())
    ///         .await
    ///         .unwrap()
    ///         .status();
    ///     assert_eq!(status, 404);
    /// }
    /// ```
    pub async fn start() -> Self {
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

    /// Register a `Mock` on an instance of `MockServer`.
    ///
    /// Be careful! `Mock`s are not effective until they are `mount`ed or `register`ed on a `MockServer`.
    ///
    /// `register` is an asynchronous method, make sure to `.await` it!
    ///
    /// ### Example:
    /// ```rust
    /// use wiremock::{MockServer, Mock, ResponseTemplate};
    /// use wiremock::matchers::method;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // Arrange
    ///     let mock_server = MockServer::start().await;
    ///
    ///     let response = ResponseTemplate::new(200);
    ///
    ///     let mock = Mock::given(method("GET")).respond_with(response.clone());
    ///     // Registering the mock with the mock server - it's now effective!
    ///     mock_server.register(mock).await;
    ///
    ///     // We won't register this mock instead.
    ///     let unregistered_mock = Mock::given(method("GET")).respond_with(response);
    ///     
    ///     // Act
    ///     let status = surf::get(&mock_server.uri())
    ///         .await
    ///         .unwrap()
    ///         .status();
    ///     assert_eq!(status, 200);
    ///
    ///     // This would have matched `unregistered_mock`, but we haven't registered it!
    ///     // Hence it returns a 404, the default response when no mocks matched on the mock server.
    ///     let status = surf::post(&mock_server.uri())
    ///         .await
    ///         .unwrap()
    ///         .status();
    ///     assert_eq!(status, 404);
    /// }
    /// ```
    pub async fn register(&self, mock: Mock) {
        self.mock_set
            .write()
            .expect("Poisoned lock!")
            .register(mock);
    }

    /// Drop all mounted `Mock`s from an instance of `MockServer`.
    ///
    /// ### Example:
    /// ```rust
    /// use wiremock::{MockServer, Mock, ResponseTemplate};
    /// use wiremock::matchers::method;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // Arrange
    ///     let mock_server = MockServer::start().await;
    ///
    ///     let response = ResponseTemplate::new(200);
    ///     Mock::given(method("GET")).respond_with(response).mount(&mock_server).await;
    ///     
    ///     // Act
    ///     let status = surf::get(&mock_server.uri())
    ///         .await
    ///         .unwrap()
    ///         .status();
    ///     assert_eq!(status, 200);
    ///
    ///     // Reset the server
    ///     mock_server.reset().await;
    ///
    ///     // This would have matched our mock, but we have dropped it resetting the server!
    ///     let status = surf::post(&mock_server.uri())
    ///         .await
    ///         .unwrap()
    ///         .status();
    ///     assert_eq!(status, 404);
    /// }
    /// ```
    pub async fn reset(&self) {
        self.mock_set.write().expect("Poisoned lock!").reset();
    }

    /// Verify that all mounted `Mock`s on this instance of `MockServer` have satisfied
    /// their expectations on their number of invocations.
    fn verify(&self) -> bool {
        self.mock_set.read().expect("Poisoned lock!").verify()
    }

    /// Return the base uri of this running instance of `MockServer`, e.g. `http://127.0.0.1:4372`.
    ///
    /// Use this method to compose uris when interacting with this instance of `MockServer` via
    /// an HTTP client.
    ///
    /// ### Example:
    /// ```rust
    /// use wiremock::MockServer;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // Arrange - no mocks mounted
    ///
    ///     let mock_server = MockServer::start().await;
    ///     // Act
    ///     let uri = format!("{}/health_check", &mock_server.uri());
    ///     let status = surf::get(uri).await.unwrap().status();
    ///
    ///     // Assert - default response
    ///     assert_eq!(status, 404);
    /// }
    /// ```
    pub fn uri(&self) -> String {
        format!("http://{}", self.server_address)
    }

    /// Return the socket address of this running instance of `MockServer`, e.g. `127.0.0.1:4372`.
    ///
    /// Use this method to interact with the `MockServer` using `TcpStream`s.
    ///
    /// ### Example:
    /// ```rust
    /// use wiremock::MockServer;
    /// use std::net::TcpStream;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // Act - the server is started
    ///     let mock_server = MockServer::start().await;
    ///
    ///     // Assert - we can connect to it
    ///     assert!(TcpStream::connect(mock_server.address()).is_ok());
    /// }
    /// ```
    pub fn address(&self) -> &SocketAddr {
        &self.server_address
    }
}

impl Drop for MockServer {
    // Clean up when the `MockServer` instance goes out of scope.
    fn drop(&mut self) {
        debug!("Verify mock expectations.");
        if !self.verify() {
            if std::thread::panicking() {
                debug!("Verification failed: mock expectations have not been satisfied.");
            } else {
                panic!("Verification failed: mock expectations have not been satisfied.");
            }
        }
        // The sender half of the channel, `shutdown_trigger`, gets dropped here
        // Triggering the graceful shutdown of the server itself.
    }
}
