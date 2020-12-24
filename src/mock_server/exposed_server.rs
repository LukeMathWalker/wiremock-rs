use crate::mock::Mock;
use crate::mock_server::bare_server::BareMockServer;
use crate::mock_server::pool::get_pooled_mock_server;
use deadpool::managed::Object;
use log::debug;
use std::convert::Infallible;
use std::net::{SocketAddr, TcpListener};
use std::ops::Deref;

/// An HTTP web-server running in the background to behave as one of your dependencies using `Mock`s
/// for testing purposes.
///
/// Each instance of `MockServer` is fully isolated: `start` takes care of finding a random port
/// available on your local machine which is assigned to the new `MockServer`.
///
/// ## Best practices
///
/// You should use one instance of `MockServer` for each REST API that your application interacts
/// with and needs mocking for testing purposes.
///
/// To ensure full isolation and no cross-test interference, `MockServer`s shouldn't be
/// shared between tests. Instead, `MockServer`s should be created in the test where they are used.
///
/// You can register as many `Mock`s as your scenario requires on a `MockServer`.
pub struct MockServer(InnerServer);

/// `MockServer` is either a wrapper around a `BareMockServer` retrieved from an
/// object pool or a wrapper around an exclusive `BareMockServer`.
/// We use the pool when the user does not care about the port the mock server listens to, while
/// we provision a dedicated one if they specify their own `TcpListener` with `start_on`.
///
/// `InnerServer` implements `Deref<Target=BareMockServer>`, so we never actually have to match
/// on `InnerServer` in `MockServer` - the compiler does all the boring heavy-lifting for us.
enum InnerServer {
    Bare(BareMockServer),
    Pooled(Object<BareMockServer, Infallible>),
}

impl Deref for InnerServer {
    type Target = BareMockServer;

    fn deref(&self) -> &Self::Target {
        match self {
            InnerServer::Bare(b) => b,
            InnerServer::Pooled(p) => p.deref(),
        }
    }
}

impl MockServer {
    /// Start a new instance of a `MockServer` listening on a random port.
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
        Self(InnerServer::Pooled(get_pooled_mock_server().await))
    }

    /// Start a new instance of a `MockServer` listening on the
    /// [`TcpListener`](std::net::TcpListener) passed as argument.
    ///
    /// ### Example:
    /// ```rust
    /// use wiremock::MockServer;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // Arrange
    ///     let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    ///     let expected_server_address = listener
    ///         .local_addr()
    ///         .expect("Failed to get server address.");
    ///     let mock_server = MockServer::start_on(listener).await;
    ///
    ///     assert_eq!(&expected_server_address, mock_server.address());
    /// }
    /// ```
    pub async fn start_on(listener: TcpListener) -> Self {
        Self(InnerServer::Bare(BareMockServer::start_on(listener).await))
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
        self.0.register(mock).await
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
        self.0.reset().await;
    }

    /// Verify that all mounted `Mock`s on this instance of `MockServer` have satisfied
    /// their expectations on their number of invocations.
    fn verify(&self) -> bool {
        self.0.verify()
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
        self.0.uri()
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
        self.0.address()
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
