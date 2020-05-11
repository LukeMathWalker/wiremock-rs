use crate::mock::Mock;
use crate::mock_actor::MockActor;
use crate::server_actor::ServerActor;
use async_std::net::TcpStream;
use bastion::{run, Bastion};
use log::debug;
use std::net::SocketAddr;
use std::time::Duration;

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
    server_actor: ServerActor,
    mock_actor: MockActor,
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
    ///     assert_eq!(status.as_u16(), 200);
    ///
    ///     // This would have matched our mock, but we haven't registered it for `mock_server_two`!
    ///     // Hence it returns a 404, the default response when no mocks matched on the mock server.
    ///     let status = surf::get(&mock_server_two.uri())
    ///         .await
    ///         .unwrap()
    ///         .status();
    ///     assert_eq!(status.as_u16(), 404);
    /// }
    /// ```
    pub async fn start() -> Self {
        // Should I put this behind a lazy_static to call them only once?
        Bastion::init();
        Bastion::start();

        let mock_actor = MockActor::start();

        // Start our mock server
        let server_actor = ServerActor::start(mock_actor.clone()).await;

        let mock_server = Self {
            server_actor,
            mock_actor,
        };

        // Wait (up to 2 second) for the actor to start listening on the specified socket
        for _ in 0..40 {
            if TcpStream::connect(mock_server.address()).await.is_ok() {
                break;
            }
            // Sleep between retries
            async_std::task::sleep(Duration::from_millis(50)).await;
        }

        mock_server
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
    ///     assert_eq!(status.as_u16(), 200);
    ///
    ///     // This would have matched `unregistered_mock`, but we haven't registered it!
    ///     // Hence it returns a 404, the default response when no mocks matched on the mock server.
    ///     let status = surf::post(&mock_server.uri())
    ///         .await
    ///         .unwrap()
    ///         .status();
    ///     assert_eq!(status.as_u16(), 404);
    /// }
    /// ```
    pub async fn register(&self, mock: Mock) {
        self.mock_actor.register(mock).await;
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
    ///     assert_eq!(status.as_u16(), 200);
    ///
    ///     // Reset the server
    ///     mock_server.reset().await;
    ///
    ///     // This would have matched our mock, but we have dropped it resetting the server!
    ///     let status = surf::post(&mock_server.uri())
    ///         .await
    ///         .unwrap()
    ///         .status();
    ///     assert_eq!(status.as_u16(), 404);
    /// }
    /// ```
    pub async fn reset(&self) {
        self.mock_actor.reset().await;
    }

    /// Verify that all mounted `Mock`s on this instance of `MockServer` have satisfied
    /// their expectations on their number of invocations.
    async fn verify(&self) -> bool {
        self.mock_actor.verify().await
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
    ///     assert_eq!(status.as_u16(), 404);
    /// }
    /// ```
    pub fn uri(&self) -> String {
        format!("http://{}", self.server_actor.address)
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
        &self.server_actor.address
    }
}

impl Drop for MockServer {
    // Clean up when the `MockServer` instance goes out of scope.
    fn drop(&mut self) {
        debug!("Verify mock expectations.");
        if !run!(self.verify()) {
            panic!("Verification failed: mock expectations have not been satisfied.");
        }
        debug!("Killing server actor.");
        self.server_actor.actor_ref.kill().unwrap();
        debug!("Killed server actor.");
        debug!("Killing mock actor.");
        self.mock_actor.actor_ref.kill().unwrap();
        debug!("Killed mock actor.");
    }
}
