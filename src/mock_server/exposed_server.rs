use crate::mock_server::bare_server::BareMockServer;
use crate::mock_server::pool::get_pooled_mock_server;
use crate::mock_server::MockServerBuilder;
use crate::{mock::Mock, verification::VerificationOutcome, Request};
use deadpool::managed::Object;
use log::debug;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::ops::Deref;

/// An HTTP web-server running in the background to behave as one of your dependencies using [`Mock`]s
/// for testing purposes.
///
/// Each instance of `MockServer` is fully isolated: [`MockServer::start`] takes care of finding a random port
/// available on your local machine which is assigned to the new `MockServer`.
///
/// You can use [`MockServer::builder`] if you need to specify custom configuration - e.g.
/// run on a specific port or disable request recording.
///
/// ## Best practices
///
/// You should use one instance of `MockServer` for each REST API that your application interacts
/// with and needs mocking for testing purposes.
///
/// To ensure full isolation and no cross-test interference, `MockServer`s shouldn't be
/// shared between tests. Instead, `MockServer`s should be created in the test where they are used.
///
/// You can register as many [`Mock`]s as your scenario requires on a `MockServer`.
pub struct MockServer(InnerServer);

/// `MockServer` is either a wrapper around a `BareMockServer` retrieved from an
/// object pool or a wrapper around an exclusive `BareMockServer`.
/// We use the pool when the user does not care about the port the mock server listens to, while
/// we provision a dedicated one if they specify their own `TcpListener` with `start_on`.
///
/// `InnerServer` implements `Deref<Target=BareMockServer>`, so we never actually have to match
/// on `InnerServer` in `MockServer` - the compiler does all the boring heavy-lifting for us.
pub(super) enum InnerServer {
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
    pub(super) fn new(server: InnerServer) -> Self {
        Self(server)
    }

    /// You can use `MockServer::builder` if you need to specify custom configuration - e.g.
    /// run on a specific port or disable request recording.
    ///
    /// If this is not your case, use [`MockServer::start`].
    pub fn builder() -> MockServerBuilder {
        MockServerBuilder::new()
    }

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

    /// Drop all mounted [`Mock`]s from an instance of [`MockServer`].
    /// It also deletes all recorded requests.
    ///
    /// ### Example
    ///
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
    ///
    /// ### Example (Recorded requests are reset)
    ///
    /// ```rust
    /// use wiremock::{MockServer, Mock, ResponseTemplate};
    /// use wiremock::matchers::method;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // Arrange
    ///     let mock_server = MockServer::start().await;
    ///
    ///     // Act
    ///     surf::get(&mock_server.uri()).await.unwrap();
    ///
    ///     // We have recorded the incoming request
    ///     let received_requests = mock_server.received_requests().await.unwrap();
    ///     assert!(!received_requests.is_empty());
    ///
    ///     // Reset the server
    ///     mock_server.reset().await;
    ///
    ///     // All received requests have been forgotten after the call to `.reset`
    ///     let received_requests = mock_server.received_requests().await.unwrap();
    ///     assert!(received_requests.is_empty())
    /// }
    /// ```
    pub async fn reset(&self) {
        self.0.reset().await;
    }

    /// Verify that all mounted `Mock`s on this instance of `MockServer` have satisfied
    /// their expectations on their number of invocations. Panics otherwise.
    pub async fn verify(&self) {
        debug!("Verify mock expectations.");
        if let VerificationOutcome::Failure(failed_verifications) = self.0.verify().await {
            let received_requests_message = if let Some(received_requests) =
                self.0.received_requests().await
            {
                if received_requests.is_empty() {
                    "The server did not receive any request.".into()
                } else {
                    format!(
                        "Received requests:\n{}",
                        received_requests
                            .into_iter()
                            .enumerate()
                            .map(|(index, request)| {
                                format!(
                                    "- Request #{}\n{}",
                                    index + 1,
                                    textwrap::indent(&format!("{}", request), "\t")
                                )
                            })
                            .collect::<String>()
                    )
                }
            } else {
                "Enable request recording on the mock server to get the list of incoming requests as part of the panic message.".into()
            };
            let verifications_errors: String = failed_verifications
                .iter()
                .map(|m| format!("- {}\n", m.error_message()))
                .collect();
            let error_message = format!(
                "Verifications failed:\n{}\n{}",
                verifications_errors, received_requests_message
            );
            if std::thread::panicking() {
                debug!("{}", &error_message);
            } else {
                panic!("{}", &error_message);
            }
        }
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

    /// Return a vector with all the requests received by the `MockServer` since it started.
    /// If no request has been served, it returns an empty vector.
    ///
    /// If request recording has been disabled using [`MockServerBuilder::disable_request_recording`],
    /// it returns `None`.
    ///
    /// ### Example:
    ///
    /// ```rust
    /// use wiremock::MockServer;
    /// use http_types::Method;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // Arrange
    ///     let mock_server = MockServer::start().await;
    ///
    ///     // Act
    ///     surf::get(&mock_server.uri()).await.unwrap();
    ///
    ///     // Assert
    ///     let received_requests = mock_server.received_requests().await.unwrap();
    ///     assert_eq!(received_requests.len(), 1);
    ///
    ///     let received_request = &received_requests[0];
    ///     assert_eq!(received_request.method, Method::Get);
    ///     assert_eq!(received_request.url.path(), "/");
    ///     assert!(received_request.body.is_empty());
    /// }
    /// ```
    ///
    /// ### Example (No request served):
    ///
    /// ```rust
    /// use wiremock::MockServer;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // Arrange
    ///     let mock_server = MockServer::start().await;
    ///
    ///     // Assert
    ///     let received_requests = mock_server.received_requests().await.unwrap();
    ///     assert_eq!(received_requests.len(), 0);
    /// }
    /// ```
    ///
    /// ### Example (Request recording disabled):
    ///
    /// ```rust
    /// use wiremock::MockServer;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // Arrange
    ///     let mock_server = MockServer::builder().disable_request_recording().start().await;
    ///
    ///     // Assert
    ///     let received_requests = mock_server.received_requests().await;
    ///     assert!(received_requests.is_none());
    /// }
    /// ```
    pub async fn received_requests(&self) -> Option<Vec<Request>> {
        self.0.received_requests().await
    }
}

impl Drop for MockServer {
    // Clean up when the `MockServer` instance goes out of scope.
    fn drop(&mut self) {
        futures::executor::block_on(self.verify())
        // The sender half of the channel, `shutdown_trigger`, gets dropped here
        // Triggering the graceful shutdown of the server itself.
    }
}
