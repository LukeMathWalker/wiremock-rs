//! A collection of different matching strategies provided out-of-the-box by `wiremock`.
//!
//! If the set of matchers provided out-of-the-box is not enough for your specific testing needs
//! you can implement your own thanks to the [`Match`] trait.
//!
//! Furthermore, `Fn` closures that take an immutable [`Request`] reference as input and return a boolean
//! as input automatically implement [`Match`] and can be used where a matcher is expected.
//!
//! Check [`Match`]'s documentation for examples.
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
///     assert_eq!(status, 200);
/// }
/// ```
pub struct MethodExactMatcher(Method);

/// Shorthand for [`MethodExactMatcher::new`].
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
/// Match all incoming requests, regardless of their method, path, headers or body.  
///
/// You can use it to verify that a request has been fired towards the server, without making
/// any other assertion about it.
///
/// ### Example:
/// ```rust
/// use wiremock::{MockServer, Mock, ResponseTemplate};
/// use wiremock::matchers::any;
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///
///     let response = ResponseTemplate::new(200);
///     // Respond with a `200 OK` to all requests hitting
///     // the mock server
///     let mock = Mock::given(any()).respond_with(response);
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
///     assert_eq!(status, 200);
/// }
/// ```
pub struct AnyMatcher;

/// Shorthand for [`AnyMatcher::new`].
pub fn any() -> AnyMatcher {
    AnyMatcher
}

impl Match for AnyMatcher {
    fn matches(&self, _request: &Request) -> bool {
        true
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
///     assert_eq!(status, 200);
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
///     assert_eq!(status, 200);
/// }
/// ```
pub struct PathExactMatcher(String);

/// Shorthand for [`PathExactMatcher::new`].
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
///     assert_eq!(status, 200);
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
///     assert_eq!(status, 200);
/// }
/// ```
pub struct PathRegexMatcher(Regex);

/// Shorthand for [`PathRegexMatcher::new`].
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
///         .header("custom", "header")
///         .await
///         .unwrap()
///         .status();
///     
///     // Assert
///     assert_eq!(status, 200);
/// }
/// ```
pub struct HeaderExactMatcher(HeaderName, HeaderValue);

/// Shorthand for [`HeaderExactMatcher::new`].
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
            Some(values) => {
                for value in values {
                    if value == &self.1 {
                        return true;
                    }
                }
                false
            }
        }
    }
}

#[derive(Debug)]
/// Match **exactly** the header name of a request. It checks that the
/// header is present but does not validate the value.
///
/// ### Example:
/// ```rust
/// use wiremock::{MockServer, Mock, ResponseTemplate};
/// use wiremock::matchers::header;
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     use wiremock::matchers::header_exists;
///     let mock_server = MockServer::start().await;
///
///     Mock::given(header_exists("custom"))
///         .respond_with(ResponseTemplate::new(200))
///         .mount(&mock_server)
///         .await;
///
///     // Act
///     let status = surf::get(&mock_server.uri())
///         .header("custom", "header")
///         .await
///         .unwrap()
///         .status();
///
///     // Assert
///     assert_eq!(status, 200);
/// }
/// ```
pub struct HeaderExistsMatcher(HeaderName);

/// Shorthand for [`HeaderExistsMatcher::new`].
pub fn header_exists<K>(key: K) -> HeaderExistsMatcher
where
    K: TryInto<HeaderName>,
    <K as TryInto<HeaderName>>::Error: std::fmt::Debug,
{
    HeaderExistsMatcher::new(key)
}

impl HeaderExistsMatcher {
    pub fn new<K>(key: K) -> Self
    where
        K: TryInto<HeaderName>,
        <K as TryInto<HeaderName>>::Error: std::fmt::Debug,
    {
        let key = key.try_into().expect("Failed to convert to header name.");
        Self(key)
    }
}

impl Match for HeaderExistsMatcher {
    fn matches(&self, request: &Request) -> bool {
        request.headers.get(&self.0).is_some()
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
///         .body("hello world!")
///         .await
///         .unwrap()
///         .status();
///
///     // Assert
///     assert_eq!(status, 200);
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
///         .body(expected_body)
///         .await
///         .unwrap()
///         .status();
///
///     // Assert
///     assert_eq!(status, 200);
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

/// Shorthand for [`BodyExactMatcher::json`].
pub fn body_json<T>(body: T) -> BodyExactMatcher
where
    T: Serialize,
{
    BodyExactMatcher::json(body)
}

/// Shorthand for [`BodyExactMatcher::string`].
pub fn body_string<T>(body: T) -> BodyExactMatcher
where
    T: Into<String>,
{
    BodyExactMatcher::string(body)
}

/// Shorthand for [`BodyExactMatcher::bytes`].
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
///         .body("this is a hello world example!")
///         .await
///         .unwrap()
///         .status();
///
///     // Assert
///     assert_eq!(status, 200);
/// }
/// ```
pub struct BodyContainsMatcher(Vec<u8>);

impl BodyContainsMatcher {
    /// Specify the part of the body that should be matched as a string.
    pub fn string<T: Into<String>>(body: T) -> Self {
        Self(body.into().as_bytes().into())
    }
}

/// Shorthand for [`BodyContainsMatcher::string`].
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
///     assert_eq!(status, 200);
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

/// Shorthand for [`QueryParamExactMatcher::new`].
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

/// Match the json structure of the body of a request.
///
/// ### Example:
/// ```rust
/// use wiremock::{MockServer, Mock, ResponseTemplate};
/// use wiremock::matchers::body_json_structure;
/// use serde_json::json;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Deserialize, Serialize)]
/// struct Greeting {
///     hello: String,
/// }
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///
///     Mock::given(body_json_structure::<Greeting>)
///         .respond_with(ResponseTemplate::new(200))
///         .mount(&mock_server)
///         .await;
///
///     // both json objects have the same structrue and thus succeed
///     let success_cases = vec![
///         json!({"hello": "world!"}),
///         json!({"hello": "everyone!"}),
///     ];
///     for case in success_cases.into_iter() {
///         let status = surf::post(&mock_server.uri())
///             .body(case)
///             .await
///             .unwrap()
///             .status();
///
///         // Assert
///         assert_eq!(status, 200);
///     }
///
///     // this json object has a different structure, and thus does not match
///     let failure_case = json!({"world": "hello!"});
///     let status = surf::post(&mock_server.uri())
///         .body(failure_case)
///         .await
///         .unwrap()
///         .status();
///
///     // Assert
///     assert_eq!(status, 404  );
/// }
/// ```
pub fn body_json_structure<T>(request: &Request) -> bool
where
    for<'de> T: serde::de::Deserialize<'de>,
{
    serde_json::from_slice::<T>(&request.body).is_ok()
}
