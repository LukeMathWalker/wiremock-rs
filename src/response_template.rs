use futures::stream::{self, BoxStream};
use futures::{Stream, StreamExt};
use http_types::headers::{HeaderName, HeaderValue};
use http_types::{Response, StatusCode};
use serde::Serialize;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::future;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

/// The blueprint for the response returned by a [`MockServer`] when a [`Mock`] matches on an incoming request.
///
/// [`Mock`]: crate::Mock
/// [`MockServer`]: crate::MockServer
#[derive(Clone)]
pub struct ResponseTemplate {
    mime: Option<http_types::Mime>,
    status_code: StatusCode,
    headers: HashMap<HeaderName, Vec<HeaderValue>>,
    body_fn: Option<Arc<BodyFn>>,
    delay: Option<Duration>,
}

pub(crate) type BodyFn =
    dyn Fn() -> (BodyStream, Option<u64>) + Send + Sync + RefUnwindSafe + UnwindSafe;
pub(crate) type BodyStream =
    BoxStream<'static, Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>>;

impl std::fmt::Debug for ResponseTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResponseTemplate")
            .field("mime", &self.mime)
            .field("status_code", &self.status_code)
            .field("headers", &self.headers)
            .field("body_fn", &self.body_fn.as_ref().map(|_| "[BodyFn]"))
            .field("delay", &self.delay)
            .finish()
    }
}

// `wiremock` is a crate meant for testing - failures are most likely not handled/temporary mistakes.
// Hence we prefer to panic and provide an easier API than to use `Result`s thus pushing
// the burden of "correctness" (and conversions) on the user.
//
// All methods try to accept the widest possible set of inputs and then perform the fallible conversion
// internally, bailing if the fallible conversion fails.
//
// Same principle applies to allocation/cloning, freely used where convenient.
impl ResponseTemplate {
    /// Start building a `ResponseTemplate` specifying the status code of the response.
    pub fn new<S>(s: S) -> Self
    where
        S: TryInto<StatusCode>,
        <S as TryInto<StatusCode>>::Error: std::fmt::Debug,
    {
        let status_code = s.try_into().expect("Failed to convert into status code.");
        Self {
            status_code,
            headers: HashMap::new(),
            mime: None,
            body_fn: None,
            delay: None,
        }
    }

    /// Append a header `value` to list of headers with `key` as header name.
    ///
    /// Unlike `insert_header`, this function will not override the contents of a header:
    /// - if there are no header values with `key` as header name, it will insert one;
    /// - if there are already some values with `key` as header name, it will append to the
    ///   existing list.
    pub fn append_header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        <K as TryInto<HeaderName>>::Error: std::fmt::Debug,
        V: TryInto<HeaderValue>,
        <V as TryInto<HeaderValue>>::Error: std::fmt::Debug,
    {
        let key = key.try_into().expect("Failed to convert into header name.");
        let value = value
            .try_into()
            .expect("Failed to convert into header value.");
        match self.headers.get_mut(&key) {
            Some(headers) => {
                headers.push(value);
            }
            None => {
                self.headers.insert(key, vec![value]);
            }
        }
        self
    }

    /// Insert a header `value` with `key` as header name.
    ///
    /// This function will override the contents of a header:
    /// - if there are no header values with `key` as header name, it will insert one;
    /// - if there are already some values with `key` as header name, it will drop them and
    ///   start a new list of header values, containing only `value`.
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
    ///     let correlation_id = "1311db4f-fe65-4cb2-b514-1bb47f781aa7";
    ///     let template = ResponseTemplate::new(200).insert_header(
    ///         "X-Correlation-ID",
    ///         correlation_id
    ///     );
    ///     Mock::given(method("GET"))
    ///         .respond_with(template)
    ///         .mount(&mock_server)
    ///         .await;
    ///
    ///     // Act
    ///     let res = surf::get(&mock_server.uri())
    ///         .await
    ///         .unwrap();
    ///
    ///     // Assert
    ///     assert_eq!(res.header("X-Correlation-ID").unwrap().as_str(), correlation_id);
    /// }
    /// ```
    pub fn insert_header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        <K as TryInto<HeaderName>>::Error: std::fmt::Debug,
        V: TryInto<HeaderValue>,
        <V as TryInto<HeaderValue>>::Error: std::fmt::Debug,
    {
        let key = key.try_into().expect("Failed to convert into header name.");
        let value = value
            .try_into()
            .expect("Failed to convert into header value.");
        self.headers.insert(key, vec![value]);
        self
    }

    /// Set the response body with bytes.
    ///
    /// It sets "Content-Type" to "application/octet-stream".
    ///
    /// To set a body with bytes but a different "Content-Type"
    /// [`set_body_raw`](#method.set_body_raw) can be used.
    pub fn set_body_bytes<B>(mut self, body: B) -> Self
    where
        B: TryInto<Vec<u8>>,
        <B as TryInto<Vec<u8>>>::Error: std::fmt::Debug,
    {
        let body = body.try_into().expect("Failed to convert into body.");
        self.body_fn = Some(wrap_body_in_arc_fn(body));
        self
    }

    /// Set the response body with a stream of bytes.
    ///
    /// It sets "Content-Type" to "application/octet-stream".
    ///
    /// If the `length` is not set, "Transfer-Encoding" is set to "chunked".
    ///
    /// To set a body with a stream of bytes but a different "Content-Type"
    /// [`set_body_stream_raw`](#method.set_body_stream_raw) can be used.
    pub fn set_body_bytes_stream<F, B, T>(mut self, body_stream_fn: F, length: Option<u64>) -> Self
    where
        F: Fn() -> B + Send + Sync + UnwindSafe + RefUnwindSafe + 'static,
        B: Stream<Item = T> + Send + Sync + 'static,
        T: TryInto<Vec<u8>>,
        <T as TryInto<Vec<u8>>>::Error: std::error::Error + Send + Sync + 'static,
    {
        self.body_fn = Some(Arc::new(move || {
            let stream = Box::pin(body_stream_fn().map(|item| {
                item.try_into().map_err(|err| {
                    Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>
                })
            }));
            (stream, length)
        }));
        self
    }

    /// Set the response body from a JSON-serializable value.
    ///
    /// It sets "Content-Type" to "application/json".
    pub fn set_body_json<B: Serialize>(mut self, body: B) -> Self {
        let body = serde_json::to_vec(&body).expect("Failed to convert into body.");

        self.body_fn = Some(wrap_body_in_arc_fn(body));
        self.mime = Some(
            http_types::Mime::from_str("application/json")
                .expect("Failed to convert into Mime header"),
        );
        self
    }

    /// Set the response body to a string.
    ///
    /// It sets "Content-Type" to "text/plain".
    pub fn set_body_string<T>(mut self, body: T) -> Self
    where
        T: TryInto<String>,
        <T as TryInto<String>>::Error: std::fmt::Debug,
    {
        let body = body.try_into().expect("Failed to convert into body.");

        self.body_fn = Some(wrap_body_in_arc_fn(body.into_bytes()));
        self.mime = Some(
            http_types::Mime::from_str("text/plain").expect("Failed to convert into Mime header"),
        );
        self
    }

    /// Set the response body to a stream of strings.
    ///
    /// It sets "Content-Type" to "text/plain".
    ///
    /// If the `length` is not set, "Transfer-Encoding" is set to "chunked".
    #[must_use]
    pub fn set_body_string_stream<F, B, T>(mut self, body_stream_fn: F, length: Option<u64>) -> Self
    where
        F: Fn() -> B + Send + Sync + UnwindSafe + RefUnwindSafe + 'static,
        B: Stream<Item = T> + Send + Sync + 'static,
        T: TryInto<String>,
        <T as TryInto<String>>::Error: std::error::Error + Send + Sync + 'static,
    {
        self.body_fn = Some(Arc::new(move || {
            let stream = Box::pin(body_stream_fn().map(|item| {
                item.try_into().map(String::into_bytes).map_err(|err| {
                    Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>
                })
            }));
            (stream, length)
        }));
        self.mime = Some(
            http_types::Mime::from_str("text/plain").expect("Failed to convert into Mime header"),
        );
        self
    }

    /// Set a raw response body. The mime type needs to be set because the
    /// raw body could be of any type.
    ///
    /// ### Example:
    /// ```rust
    /// use surf::http::mime;
    /// use wiremock::{MockServer, Mock, ResponseTemplate};
    /// use wiremock::matchers::method;
    ///
    /// mod external {
    ///     // This could be a method of a struct that is
    ///     // implemented in another crate and the struct
    ///     // does not implement Serialize.
    ///     pub fn body() -> Vec<u8>{
    ///         r#"{"hello": "world"}"#.as_bytes().to_owned()
    ///     }
    /// }
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // Arrange
    ///     let mock_server = MockServer::start().await;
    ///     let template = ResponseTemplate::new(200).set_body_raw(
    ///         external::body(),
    ///         "application/json"
    ///     );
    ///     Mock::given(method("GET"))
    ///         .respond_with(template)
    ///         .mount(&mock_server)
    ///         .await;
    ///
    ///     // Act
    ///     let mut res = surf::get(&mock_server.uri())
    ///         .await
    ///         .unwrap();
    ///     let body = res.body_string()
    ///         .await
    ///         .unwrap();
    ///
    ///     // Assert
    ///     assert_eq!(body, r#"{"hello": "world"}"#);
    ///     assert_eq!(res.content_type(), Some(mime::JSON));
    /// }
    /// ```
    pub fn set_body_raw<B>(mut self, body: B, mime: &str) -> Self
    where
        B: TryInto<Vec<u8>>,
        <B as TryInto<Vec<u8>>>::Error: std::fmt::Debug,
    {
        let body = body.try_into().expect("Failed to convert into body.");
        self.body_fn = Some(wrap_body_in_arc_fn(body));
        self.mime =
            Some(http_types::Mime::from_str(mime).expect("Failed to convert into Mime header"));
        self
    }

    /// Set a raw response body using a stream of data. The mime type needs to be set because the
    /// raw body could be of any type.
    ///
    /// If the `length` is not set, "Transfer-Encoding" is set to "chunked".
    ///
    /// It the `mime` parameter is `None`, the "Content-Type" header is set to
    /// "application/octet-stream
    ///
    /// ### Example:
    /// ```rust
    /// use surf::http::mime;
    /// use wiremock::matchers::method;
    /// use wiremock::{Mock, MockServer, ResponseTemplate};
    ///
    /// mod external {
    ///     use futures::{future, stream, Stream, StreamExt};
    ///
    ///     use std::convert::Infallible;
    ///
    ///     // This could be a method of a struct that is implemented in another crate and a stream
    ///     // of strings is returned.
    ///     pub fn body() -> impl Stream<Item = Result<&'static str, Infallible>> {
    ///         stream::once(future::ok(r#"{"hello": "#))
    ///             .chain(stream::once(future::ok(r#""world"}"#)))
    ///     }
    /// }
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // Arrange
    ///     let mock_server = MockServer::start().await;
    ///     let template =
    ///         ResponseTemplate::new(200).set_body_raw_stream(external::body, Some(18), Some("application/json"));
    ///     Mock::given(method("GET"))
    ///         .respond_with(template)
    ///         .mount(&mock_server)
    ///         .await;
    ///
    ///     // Act
    ///     let mut res = surf::get(&mock_server.uri()).await.unwrap();
    ///     let body = res.body_string().await.unwrap();
    ///
    ///     // Assert
    ///     assert_eq!(body, r#"{"hello": "world"}"#);
    ///     assert_eq!(res.content_type(), Some(mime::JSON));
    /// }
    /// ```
    #[must_use]
    pub fn set_body_raw_stream<F, B, T, E>(
        mut self,
        body_stream_fn: F,
        length: Option<u64>,
        mime: Option<&str>,
    ) -> Self
    where
        F: Fn() -> B + Send + Sync + UnwindSafe + RefUnwindSafe + 'static,
        B: Stream<Item = Result<T, E>> + Send + Sync + 'static,
        T: TryInto<Vec<u8>>,
        <T as TryInto<Vec<u8>>>::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
        E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        self.body_fn = Some(Arc::new(move || {
            let stream = Box::pin(body_stream_fn().map(|item| {
                item.map_err(Into::into)
                    .and_then(|item| item.try_into().map_err(Into::into))
            }));
            (stream, length)
        }));
        self.mime = mime.map(|mime| {
            http_types::Mime::from_str(mime).expect("Failed to convert into Mime header")
        });
        self
    }

    /// By default the [`MockServer`] tries to fulfill incoming requests as fast as possible.
    ///
    /// You can use `set_delay` to introduce an artificial delay to simulate the behaviour of
    /// a real server with a non-negligible latency.
    ///
    /// In particular, you can use it to test the behaviour of your timeout policies.
    ///
    /// ### Example:
    /// ```rust
    /// use isahc::config::Configurable;
    /// use wiremock::{MockServer, Mock, ResponseTemplate};
    /// use wiremock::matchers::method;
    /// use std::time::Duration;
    /// use async_std::prelude::FutureExt;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // Arrange
    ///     let mock_server = MockServer::start().await;
    ///     let delay = Duration::from_secs(1);
    ///     let template = ResponseTemplate::new(200).set_delay(delay);
    ///     Mock::given(method("GET"))
    ///         .respond_with(template)
    ///         .mount(&mock_server)
    ///         .await;
    ///
    ///     // Act
    ///     let mut res = async_std::future::timeout(
    ///         // Shorter than the response delay!
    ///         delay / 3,
    ///         surf::get(&mock_server.uri())
    ///     )
    ///     .await;
    ///
    ///     // Assert - Timeout error!
    ///     assert!(res.is_err());
    /// }
    /// ```
    ///
    /// [`MockServer`]: crate::mock_server::MockServer
    pub fn set_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);

        self
    }

    /// Generate a response from the template.
    pub(crate) fn generate_response(&self) -> (Response, Option<BodyStream>, Option<u64>) {
        let mut response = Response::new(self.status_code);

        // Add headers
        for (header_name, header_values) in &self.headers {
            response.insert_header(header_name.clone(), header_values.as_slice());
        }

        // Set content-type, if needed
        if let Some(mime) = &self.mime {
            response.set_content_type(mime.to_owned());
        }

        // Get body stream, if specified
        // TODO: use Option::unzip, bumping MSRV to 1.66
        let (body, length) = match self.body_fn.as_deref().map(|f| f()) {
            Some((body, length)) => (Some(body), length),
            None => (None, None),
        };

        (response, body, length)
    }

    /// Retrieve the response delay.
    pub(crate) fn delay(&self) -> &Option<Duration> {
        &self.delay
    }
}

#[inline]
fn wrap_body_in_arc_fn(body: Vec<u8>) -> Arc<BodyFn> {
    Arc::new(move || {
        let length = body.len();
        let stream = Box::pin(stream::once(future::ready(Ok(body.clone()))));
        (
            stream,
            Some(u64::try_from(length).expect("Length of body is too big")),
        )
    })
}
