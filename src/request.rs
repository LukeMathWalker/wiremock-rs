use async_std::prelude::*;
use http_types::headers::{HeaderName, HeaderValue};
use http_types::{Method, Url};
use std::{collections::HashMap, fmt};

/// An incoming request to an instance of [`MockServer`].
///
/// Each matcher gets an immutable reference to a `Request` instance in the [`matches`] method
/// defined in the [`Match`] trait.
///
/// [`MockServer`]: struct.MockServer.html
/// [`matches`]: trait.Match.html
/// [`Match`]: trait.Match.html
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
#[derive(Debug)]
pub struct Request {
    pub url: Url,
    pub method: Method,
    pub headers: HashMap<HeaderName, Vec<HeaderValue>>,
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
}
