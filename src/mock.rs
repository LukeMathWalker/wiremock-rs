use crate::response_template::ResponseTemplate;
use crate::{MockServer, Request};
use http_types::Response;
use std::fmt::{Debug, Formatter};

/// Anything that implements `Match` can be used to constrain when a [`Mock`] is activated.
///
/// `Match` is the only trait in the whole `wiremock` crate and can be used to extend
/// the set of matchers provided out-of-the-box to cater to your specific testing needs:
/// ```rust
/// use wiremock::{Match, MockServer, Mock, Request, ResponseTemplate};
/// use wiremock::matchers::HeaderExactMatcher;
/// use std::convert::TryInto;
///
/// // Check that a header with the specified name exists and its value has an odd length.
/// pub struct OddHeaderMatcher(http_types::headers::HeaderName);
///
/// impl Match for OddHeaderMatcher {
///     fn matches(&self, request: &Request) -> bool {
///         match request.headers.get(&self.0) {
///             // We are ignoring multi-valued headers for simplicity
///             Some(values) => values[0].as_str().len() % 2 == 1,
///             None => false
///         }
///     }
/// }
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///
///     Mock::given(OddHeaderMatcher("custom".try_into().unwrap()))
///         .respond_with(ResponseTemplate::new(200))
///         .mount(&mock_server)
///         .await;
///     
///     // Even length
///     let status = surf::get(&mock_server.uri())
///         .set_header("custom", "even")
///         .await
///         .unwrap()
///         .status();
///     assert_eq!(status.as_u16(), 404);
///
///     // Odd length
///     let status = surf::get(&mock_server.uri())
///         .set_header("custom", "odd")
///         .await
///         .unwrap()
///         .status();
///     assert_eq!(status.as_u16(), 200);
/// }
/// ```
///
/// Anonymous functions that take a reference to a [`Request`] as input and return a boolean
/// as output automatically implement the `Match` trait.
///
/// The previous example could be rewritten as follows:
/// ```rust
/// use wiremock::{Match, MockServer, Mock, Request, ResponseTemplate};
/// use wiremock::matchers::HeaderExactMatcher;
/// use std::convert::TryInto;
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///     
///     let header_name: http_types::headers::HeaderName = "custom".try_into().unwrap();
///     // Check that a header with the specified name exists and its value has an odd length.
///     let matcher = move |request: &Request| {
///         match request.headers.get(&header_name) {
///             Some(values) => values[0].as_str().len() % 2 == 1,
///             None => false
///         }
///     };
///
///     Mock::given(matcher)
///         .respond_with(ResponseTemplate::new(200))
///         .mount(&mock_server)
///         .await;
///     
///     // Even length
///     let status = surf::get(&mock_server.uri())
///         .set_header("custom", "even")
///         .await
///         .unwrap()
///         .status();
///     assert_eq!(status.as_u16(), 404);
///
///     // Odd length
///     let status = surf::get(&mock_server.uri())
///         .set_header("custom", "odd")
///         .await
///         .unwrap()
///         .status();
///     assert_eq!(status.as_u16(), 200);
/// }
/// ```
///
/// [`Mock`]: struct.Mock.html
/// [`Request`]: struct.Request.html
pub trait Match: Send + Sync {
    /// Given a reference to a `Request`, determine if it should match or not given
    /// a specific criterion.
    fn matches(&self, request: &Request) -> bool;
}

/// Wrapper around a `Match` trait object.
///
/// We need the wrapper to provide a (fake) implementation of `Debug`,
/// thus allowing us to pass this struct around as a `bastion` message.
/// This is because Rust's closures do not implement `Debug`.
///
/// We wouldn't need this if `bastion` didn't require `Debug` as a trait bound for its Message trait
/// or if Rust automatically implemented `Debug` for closures.
pub(crate) struct Matcher(Box<dyn Match>);

impl Match for Matcher {
    fn matches(&self, request: &Request) -> bool {
        self.0.matches(request)
    }
}

impl Debug for Matcher {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        // Dummy `Debug` implementation to allow us to pass `Matcher` as a message in `bastion`.
        // It's needed because closures do not implement `Debug` and we really want to enable
        // closures as matchers from an API perspective.
        // Might re-think this in the future.
        Ok(())
    }
}

/// Given a set of matchers, a `Mock` instructs an instance of [`MockServer`] to return a pre-determined response if the matching conditions are satisfied.
///
/// `Mock`s have to be mounted (or registered) with a [`MockServer`] to become effective.
///
/// ### Example (using [`register`]):
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
///
/// ### Example (using [`mount`]):
///
/// If you prefer a fluent style, you can use the [`mount`] method on the `Mock` itself
/// instead of [`register`].
/// ```rust
/// use wiremock::{MockServer, Mock, ResponseTemplate};
/// use wiremock::matchers::method;
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///
///     Mock::given(method("GET"))
///         .respond_with(ResponseTemplate::new(200))
///         // Mounting the mock on the mock server - it's now effective!
///         .mount(&mock_server)
///         .await;
///     
///     // Act
///     let status = surf::get(&mock_server.uri())
///         .await
///         .unwrap()
///         .status();
///     assert_eq!(status.as_u16(), 200);
/// }
/// ```
///
/// Both `register` and `mount` are asynchronous methods - don't forget to `.await` them!
///
/// [`MockServer`]: struct.MockServer.html
/// [`register`]: struct.MockServer.html#method.register
/// [`mount`]: #method.mount
#[derive(Debug)]
pub struct Mock {
    pub(crate) matchers: Vec<Matcher>,
    pub(crate) response: ResponseTemplate,
}

/// A fluent builder to construct a [`Mock`] instance given matchers and a [`ResponseTemplate`].
///
/// [`Mock`]: struct.Mock.html
/// [`ResponseTemplate`]: struct.ResponseTemplate.html
#[derive(Debug)]
pub struct MockBuilder {
    pub(crate) matchers: Vec<Matcher>,
}

impl Mock {
    /// Start building a `Mock` specifying the first matcher.
    ///
    /// It returns an instance of [`MockBuilder`].
    ///
    /// [`MockBuilder`]: struct.MockBuilder.html
    pub fn given<M: 'static + Match>(matcher: M) -> MockBuilder {
        MockBuilder {
            matchers: vec![Matcher(Box::new(matcher))],
        }
    }

    /// Mount a `Mock` on an instance of [`MockServer`].
    ///
    /// Be careful! `Mock`s are not effective until they are [`mount`]ed or [`register`]ed on a [`MockServer`].
    ///
    /// [`mount`] is an asynchronous method, make sure to `.await` it!
    ///
    /// [`MockServer`]: struct.MockServer.html
    /// [`register`]: struct.MockServer.html#method.register
    /// [`mount`]: #method.mount
    pub async fn mount(self, server: &MockServer) {
        server.register(self).await;
    }

    /// Build an instance of `http_types::Response` from the [`ResponseTemplate`] associated
    /// with a `Mock`.
    ///
    /// [`ResponseTemplate`]: struct.ResponseTemplate.html
    pub fn response(&self) -> Response {
        self.response.generate_response()
    }
}

impl Match for Mock {
    fn matches(&self, request: &Request) -> bool {
        self.matchers.iter().all(|matcher| matcher.matches(request))
    }
}

impl MockBuilder {
    /// Add another request matcher to the mock you are building.
    ///
    /// **All** specified [`matchers`] must match for the overall [`Mock`] to match an incoming request.
    ///
    /// [`matchers`]: matchers/index.html
    /// [`Mock`]: struct.Mock.html
    pub fn and<M: Match + 'static>(mut self, matcher: M) -> Self {
        self.matchers.push(Matcher(Box::new(matcher)));
        self
    }

    /// Establish what [`ResponseTemplate`] should be used to generate a response when an incoming
    /// request matches.
    ///
    /// `respond_with` finalises the `MockBuilder` and returns you a [`Mock`] instance, ready to
    /// be [`register`]ed or [`mount`]ed on a [`MockServer`]!
    ///
    /// [`Mock`]: struct.Mock.html
    /// [`MockServer`]: struct.MockServer.html
    /// [`ResponseTemplate`]: struct.ResponseTemplate.html
    /// [`register`]: struct.MockServer.html#method.register
    /// [`mount`]: #method.mount
    pub fn respond_with(self, template: ResponseTemplate) -> Mock {
        Mock {
            matchers: self.matchers,
            response: template,
        }
    }
}
