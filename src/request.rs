use std::str::FromStr;
use std::{collections::HashMap, fmt};

use futures::AsyncReadExt;
use http_types::headers::{HeaderName, HeaderValue, HeaderValues};
use http_types::{Method, Url};

/// An incoming request to an instance of [`MockServer`].
///
/// Each matcher gets an immutable reference to a `Request` instance in the [`matches`] method
/// defined in the [`Match`] trait.
///
/// [`MockServer`]: crate::MockServer
/// [`matches`]: crate::Match::matches
/// [`Match`]: crate::Match
///
/// ### Implementation notes:
/// We can't use `http_types::Request` directly in our `Match::matches` signature:
/// it requires having mutable access to the request to extract the body (which gets
/// consumed when read!).
/// It would also require `matches` to be async, which is cumbersome due to the lack of async traits.
///
/// We introduce our `Request` type to perform this extraction once when the request
/// arrives in the mock serve, store the result and pass an immutable reference to it
/// to all our matchers.
#[derive(Debug, Clone)]
pub struct Request {
    pub url: Url,
    pub method: Method,
    pub headers: HashMap<HeaderName, HeaderValues>,
    pub body: Vec<u8>,
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{} {}", self.method, self.url)?;
        for (name, values) in &self.headers {
            let values = values
                .iter()
                .map(|value| format!("{}", value))
                .collect::<Vec<_>>();
            let values = values.join(",");
            writeln!(f, "{}: {}", name, values)?;
        }
        writeln!(f, "{}", String::from_utf8_lossy(&self.body))
    }
}

impl Request {
    pub async fn from(mut request: http_types::Request) -> Request {
        let method = request.method();
        let url = request.url().to_owned();

        let mut headers = HashMap::new();
        for (header_name, header_values) in &request {
            headers.insert(header_name.to_owned(), header_values.to_owned());
        }

        let mut body: Vec<u8> = vec![];
        request
            .take_body()
            .into_reader()
            .read_to_end(&mut body)
            .await
            .expect("Failed to read body");

        Self {
            url,
            method,
            headers,
            body,
        }
    }

    pub(crate) async fn from_hyper(request: hyper::Request<hyper::Body>) -> Request {
        let (parts, body) = request.into_parts();
        let method = parts.method.into();
        let url = match parts.uri.authority() {
            Some(_) => parts.uri.to_string(),
            None => format!("http://localhost{}", parts.uri),
        }
        .parse()
        .unwrap();

        let mut headers = HashMap::new();
        for (name, value) in parts.headers {
            if let Some(name) = name {
                let name = name.as_str().as_bytes().to_owned();
                let name = HeaderName::from_bytes(name).unwrap();
                let value = value.as_bytes().to_owned();
                let value = HeaderValue::from_bytes(value).unwrap();
                let value_parts = value.as_str().split(',');
                let value_parts = value_parts
                    .map(|it| it.trim())
                    .filter_map(|it| HeaderValue::from_str(it).ok());
                headers.insert(name, value_parts.collect());
            }
        }

        let body = hyper::body::to_bytes(body)
            .await
            .expect("Failed to read request body.")
            .to_vec();

        Self {
            url,
            method,
            headers,
            body,
        }
    }
}
