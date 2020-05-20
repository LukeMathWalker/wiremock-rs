use http_types::headers::{HeaderName, HeaderValue};
use http_types::{Response, StatusCode};
use serde::Serialize;
use std::collections::HashMap;
use std::convert::TryInto;
use std::str::FromStr;

/// The blueprint for the response returned by a [`MockServer`] when a [`Mock`] matches on an incoming request.
///
/// [`Mock`]: struct.Mock.html
/// [`MockServer`]: struct.MockServer.html
#[derive(Clone, Debug)]
pub struct ResponseTemplate {
    mime: Option<http_types::Mime>,
    status_code: StatusCode,
    headers: HashMap<HeaderName, Vec<HeaderValue>>,
    body: Option<Vec<u8>>,
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
            body: None,
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
    #[deprecated(since = "0.1.1", note = "Please use set_body_bytes instead.")]
    pub fn set_body<B>(mut self, body: B) -> Self
    where
        B: TryInto<Vec<u8>>,
        <B as TryInto<Vec<u8>>>::Error: std::fmt::Debug,
    {
        let body = body.try_into().expect("Failed to convert into body.");
        self.body = Some(body);
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
        self.body = Some(body);
        self
    }

    /// Set the response body from a JSON-serializable value.
    ///
    /// It sets "Content-Type" to "application/json".
    pub fn set_body_json<B: Serialize>(mut self, body: B) -> Self {
        let body = serde_json::to_vec(&body).expect("Failed to convert into body.");

        self.body = Some(body);
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

        self.body = Some(body.into_bytes());
        self.mime = Some(
            http_types::Mime::from_str("text/plain").expect("Failed to convert into Mime header"),
        );
        self
    }

    /// Set a raw response body. The mime type needs to be set because the
    /// raw body could be of any type.
    pub fn set_body_raw<B>(mut self, body: B, mime: &str) -> Self
    where
        B: TryInto<Vec<u8>>,
        <B as TryInto<Vec<u8>>>::Error: std::fmt::Debug,
    {
        let body = body.try_into().expect("Failed to convert into body.");
        self.body = Some(body);
        self.mime = Some(
            http_types::Mime::from_str(mime).expect("Failed to convert into Mime header"),
        );
        self
    }

    /// Generate a response from the template.
    pub(crate) fn generate_response(&self) -> Response {
        let mut response = Response::new(self.status_code);

        // Add headers
        for (header_name, header_values) in &self.headers {
            response
                .insert_header(header_name.clone(), header_values.as_slice())
                .unwrap();
        }

        // Add body, if specified
        if let Some(body) = &self.body {
            response.set_body(body.clone());
        }

        // Set content-type, if needed
        if let Some(mime) = &self.mime {
            response.set_content_type(mime.to_owned());
        }

        response
    }
}
