//! A collection of different matching strategies provided out-of-the-box by `wiremock`.
//!
//! If the set of matchers provided out-of-the-box is not enough for your specific testing needs
//! you can implement your own thanks to the [`Match`] trait.
//!
//! Furthermore, `Fn` closures that take an immutable [`Request`] reference as input and return a boolean
//! as input automatically implement [`Match`] and can be used where a matcher is expected.
//!
//! Check [`Match`]'s documentation for examples.
//!
//! [`Match`]: ../trait.Match.html
//! [`Request`]: ../struct.Request.html
use crate::{Match, Request};
use http_types::headers::{HeaderName, HeaderValue};
use http_types::Method;
use log::debug;
use regex::Regex;
use serde::Serialize;
use std::convert::TryInto;
use std::str;

/// Implement the `Match` trait for all closures, out of the box,
/// if their signature is compatible.
impl<F> Match for F
where
    F: Fn(&Request) -> bool,
    F: Send + Sync,
{
    fn matches(&self, request: &Request) -> bool {
        // Just call the closure itself!
        self(request)
    }
}

#[derive(Debug)]
/// Match **exactly** the method of a request.
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
///     let mock = Mock::given(method("GET")).respond_with(response);
///
///     mock_server.register(mock).await;
///     
///     // Act
///     let status = surf::get(&mock_server.uri())
///         .await
///         .unwrap()
///         .status();
///     
///     // Assert
///     assert_eq!(status.as_u16(), 200);
/// }
/// ```
pub struct MethodExactMatcher(Method);

/// Shorthand for [`MethodExactMatcher::new`](struct.MethodExactMatcher.html).
pub fn method<T>(method: T) -> MethodExactMatcher
where
    T: TryInto<Method>,
    <T as TryInto<Method>>::Error: std::fmt::Debug,
{
    MethodExactMatcher::new(method)
}

impl MethodExactMatcher {
    pub fn new<T>(method: T) -> Self
    where
        T: TryInto<Method>,
        <T as TryInto<Method>>::Error: std::fmt::Debug,
    {
        let method = method
            .try_into()
            .expect("Failed to convert to HTTP method.");
        Self(method)
    }
}

impl Match for MethodExactMatcher {
    fn matches(&self, request: &Request) -> bool {
        request.method == self.0
    }
}

#[derive(Debug)]
/// Match **exactly** the path of a request.
///
/// ### Example:
/// ```rust
/// use wiremock::{MockServer, Mock, ResponseTemplate};
/// use wiremock::matchers::path;
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///
///     let response = ResponseTemplate::new(200).set_body_string("world");
///     let mock = Mock::given(path("/hello")).respond_with(response);
///
///     mock_server.register(mock).await;
///     
///     // Act
///     let status = surf::get(format!("{}/hello", &mock_server.uri()))
///         .await
///         .unwrap()
///         .status();
///     
///     // Assert
///     assert_eq!(status.as_u16(), 200);
/// }
/// ```
///
/// ### Example:
///
/// The path matcher ignores query parameters:
///
/// ```rust
/// use wiremock::{MockServer, Mock, ResponseTemplate};
/// use wiremock::matchers::path;
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///
///     let response = ResponseTemplate::new(200).set_body_string("world");
///     let mock = Mock::given(path("/hello")).respond_with(response);
///
///     mock_server.register(mock).await;
///     
///     // Act
///     let status = surf::get(format!("{}/hello?a_parameter=some_value", &mock_server.uri()))
///         .await
///         .unwrap()
///         .status();
///     
///     // Assert
///     assert_eq!(status.as_u16(), 200);
/// }
/// ```
pub struct PathExactMatcher(String);

/// Shorthand for [`PathExactMatcher::new`](struct.PathExactMatcher.html).
pub fn path<T>(path: T) -> PathExactMatcher
where
    T: Into<String>,
{
    PathExactMatcher::new(path)
}

impl PathExactMatcher {
    pub fn new<T: Into<String>>(path: T) -> Self {
        let path = path.into();

        // Prepend "/" to the path if missing.
        if path.starts_with('/') {
            Self(path)
        } else {
            Self(format!("/{}", path))
        }
    }
}

impl Match for PathExactMatcher {
    fn matches(&self, request: &Request) -> bool {
        request.url.path() == self.0
    }
}

#[derive(Debug)]
/// Match the path of a request against a regular expression.
///
/// ### Example:
/// ```rust
/// use wiremock::{MockServer, Mock, ResponseTemplate};
/// use wiremock::matchers::path_regex;
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///
///     let response = ResponseTemplate::new(200).set_body_string("world");
///     let mock = Mock::given(path_regex(r"^/hello/\d{3}$")).respond_with(response);
///
///     mock_server.register(mock).await;
///
///     // Act
///     let status = surf::get(format!("{}/hello/123", &mock_server.uri()))
///         .await
///         .unwrap()
///         .status();
///
///     // Assert
///     assert_eq!(status.as_u16(), 200);
/// }
/// ```
///
/// ### Example:
/// ```rust
/// use wiremock::{MockServer, Mock, ResponseTemplate};
/// use wiremock::matchers::path_regex;
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///
///     let response = ResponseTemplate::new(200).set_body_string("world");
///     let mock = Mock::given(path_regex(r"^/users/[a-z0-9-~_]{1,}/posts$")).respond_with(response);
///
///     mock_server.register(mock).await;
///
///     // Act
///     let status = surf::get(format!("{}/users/da2854ea-b70f-46e7-babc-2846eff4d33c/posts", &mock_server.uri()))
///         .await
///         .unwrap()
///         .status();
///
///     // Assert
///     assert_eq!(status.as_u16(), 200);
/// }
/// ```
pub struct PathRegexMatcher(Regex);

/// Shorthand for [`PathRegexMatcher::new`](struct.PathRegexMatcher.html).
pub fn path_regex<T>(path: T) -> PathRegexMatcher
where
    T: Into<String>,
{
    PathRegexMatcher::new(path)
}

impl PathRegexMatcher {
    pub fn new<T: Into<String>>(path: T) -> Self {
        let path = path.into();

        Self(Regex::new(&path).expect("Failed to create regex for path matcher"))
    }
}

impl Match for PathRegexMatcher {
    fn matches(&self, request: &Request) -> bool {
        self.0.is_match(request.url.path())
    }
}

#[derive(Debug)]
/// Match **exactly** the header of a request.
///
/// ### Example:
/// ```rust
/// use wiremock::{MockServer, Mock, ResponseTemplate};
/// use wiremock::matchers::header;
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///
///     Mock::given(header("custom", "header"))
///         .respond_with(ResponseTemplate::new(200))
///         .mount(&mock_server)
///         .await;
///     
///     // Act
///     let status = surf::get(&mock_server.uri())
///         .set_header("custom", "header")
///         .await
///         .unwrap()
///         .status();
///     
///     // Assert
///     assert_eq!(status.as_u16(), 200);
/// }
/// ```
pub struct HeaderExactMatcher(HeaderName, HeaderValue);

/// Shorthand for [`HeaderExactMatcher::new`](struct.HeaderExactMatcher.html).
pub fn header<K, V>(key: K, value: V) -> HeaderExactMatcher
where
    K: TryInto<HeaderName>,
    <K as TryInto<HeaderName>>::Error: std::fmt::Debug,
    V: TryInto<HeaderValue>,
    <V as TryInto<HeaderValue>>::Error: std::fmt::Debug,
{
    HeaderExactMatcher::new(key, value)
}

impl HeaderExactMatcher {
    pub fn new<K, V>(key: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        <K as TryInto<HeaderName>>::Error: std::fmt::Debug,
        V: TryInto<HeaderValue>,
        <V as TryInto<HeaderValue>>::Error: std::fmt::Debug,
    {
        let key = key.try_into().expect("Failed to convert to header name.");
        let value = value
            .try_into()
            .expect("Failed to convert to header value.");
        Self(key, value)
    }
}

impl Match for HeaderExactMatcher {
    fn matches(&self, request: &Request) -> bool {
        match request.headers.get(&self.0) {
            None => false,
            Some(values) => values.contains(&self.1),
        }
    }
}

#[derive(Debug)]
/// Match **exactly** the body of a request.
///
/// ### Example (string):
/// ```rust
/// use wiremock::{MockServer, Mock, ResponseTemplate};
/// use wiremock::matchers::body_string;
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///
///     Mock::given(body_string("hello world!"))
///         .respond_with(ResponseTemplate::new(200))
///         .mount(&mock_server)
///         .await;
///
///     // Act
///     let status = surf::post(&mock_server.uri())
///         .body_string("hello world!".into())
///         .await
///         .unwrap()
///         .status();
///
///     // Assert
///     assert_eq!(status.as_u16(), 200);
/// }
/// ```
///
/// ### Example (json):
/// ```rust
/// use wiremock::{MockServer, Mock, ResponseTemplate};
/// use wiremock::matchers::body_json;
/// use serde_json::json;
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///
///     let expected_body = json!({
///         "hello": "world!"
///     });
///     Mock::given(body_json(&expected_body))
///         .respond_with(ResponseTemplate::new(200))
///         .mount(&mock_server)
///         .await;
///
///     // Act
///     let status = surf::post(&mock_server.uri())
///         .body_json(&expected_body)
///         .unwrap()
///         .await
///         .unwrap()
///         .status();
///
///     // Assert
///     assert_eq!(status.as_u16(), 200);
/// }
/// ```
pub struct BodyExactMatcher(Vec<u8>);

impl BodyExactMatcher {
    /// Specify the expected body as a string.
    pub fn string<T: Into<String>>(body: T) -> Self {
        let body = body.into();
        Self(body.as_bytes().into())
    }

    /// Specify the expected body as a vector of bytes.
    pub fn bytes<T: Into<Vec<u8>>>(body: T) -> Self {
        let body = body.into();
        Self(body)
    }

    /// Specify something JSON-serializable as the expected body.
    pub fn json<T: Serialize>(body: T) -> Self {
        let body = serde_json::to_vec(&body).expect("Failed to serialise body");
        Self(body)
    }
}

/// Shorthand for [`BodyExactMatcher::json`](struct.BodyExactMatcher.html).
pub fn body_json<T>(body: T) -> BodyExactMatcher
where
    T: Serialize,
{
    BodyExactMatcher::json(body)
}

/// Shorthand for [`BodyExactMatcher::string`](struct.BodyExactMatcher.html).
pub fn body_string<T>(body: T) -> BodyExactMatcher
where
    T: Into<String>,
{
    BodyExactMatcher::string(body)
}

/// Shorthand for [`BodyExactMatcher::bytes`](struct.BodyExactMatcher.html).
pub fn body_bytes<T>(body: T) -> BodyExactMatcher
where
    T: Into<Vec<u8>>,
{
    BodyExactMatcher::bytes(body)
}

impl Match for BodyExactMatcher {
    fn matches(&self, request: &Request) -> bool {
        request.body == self.0
    }
}

#[derive(Debug)]
/// Match part of the body of a request.
///
/// ### Example (string):
/// ```rust
/// use wiremock::{MockServer, Mock, ResponseTemplate};
/// use wiremock::matchers::body_string_contains;
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///
///     Mock::given(body_string_contains("hello world"))
///         .respond_with(ResponseTemplate::new(200))
///         .mount(&mock_server)
///         .await;
///
///     // Act
///     let status = surf::post(&mock_server.uri())
///         .body_string("this is a hello world example!".into())
///         .await
///         .unwrap()
///         .status();
///
///     // Assert
///     assert_eq!(status.as_u16(), 200);
/// }
/// ```
pub struct BodyContainsMatcher(Vec<u8>);

impl BodyContainsMatcher {
    /// Specify the part of the body that should be matched as a string.
    pub fn string<T: Into<String>>(body: T) -> Self {
        Self(body.into().as_bytes().into())
    }
}

/// Shorthand for [`BodyContainsMatcher::string`](struct.BodyContainsMatcher.html).
pub fn body_string_contains<T>(body: T) -> BodyContainsMatcher
where
    T: Into<String>,
{
    BodyContainsMatcher::string(body)
}

impl Match for BodyContainsMatcher {
    fn matches(&self, request: &Request) -> bool {
        let body = match str::from_utf8(&request.body) {
            Ok(body) => body.to_string(),
            Err(err) => {
                debug!("can't convert body from byte slice to string: {}", err);
                return false;
            }
        };

        let part = match str::from_utf8(&self.0) {
            Ok(part) => part,
            Err(err) => {
                debug!(
                    "can't convert expected part from byte slice to string: {}",
                    err
                );
                return false;
            }
        };

        body.contains(part)
    }
}

#[derive(Debug)]
/// Match **exactly** the query parameter of a request.
///
/// ### Example:
/// ```rust
/// use wiremock::{MockServer, Mock, ResponseTemplate};
/// use wiremock::matchers::query_param;
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///
///     Mock::given(query_param("hello", "world"))
///         .respond_with(ResponseTemplate::new(200))
///         .mount(&mock_server)
///         .await;
///
///     // Act
///     let status = surf::get(format!("{}?hello=world", &mock_server.uri()))
///         .await
///         .unwrap()
///         .status();
///
///     // Assert
///     assert_eq!(status.as_u16(), 200);
/// }
/// ```
pub struct QueryParamExactMatcher(String, String);

impl QueryParamExactMatcher {
    /// Specify the expected value for a query parameter.
    pub fn new<K: Into<String>, V: Into<String>>(key: K, value: V) -> Self {
        let key = key.into();
        let value = value.into();
        Self(key, value)
    }
}

/// Shorthand for [`QueryParamExactMatcher::new`](struct.QueryParamExactMatcher.html).
pub fn query_param<K, V>(key: K, value: V) -> QueryParamExactMatcher
where
    K: Into<String>,
    V: Into<String>,
{
    QueryParamExactMatcher::new(key, value)
}

impl Match for QueryParamExactMatcher {
    fn matches(&self, request: &Request) -> bool {
        request
            .url
            .query_pairs()
            .any(|q| q.0 == self.0.as_str() && q.1 == self.1.as_str())
    }
}
