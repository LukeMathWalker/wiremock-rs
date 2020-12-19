use crate::{Request, ResponseTemplate};

/// Anything that implements `Responder` can be used to reply to an incoming request when a
/// [`Mock`] is activated.
///
/// The simplest `Responder` is `ResponseTemplate`: no matter the request, it will
/// always return itself.
///
/// You can use `Responder`, though, to implement more sophisticated logic.
/// For example, to propagate a request header back in the response:
///
/// ```rust
/// use http_types::headers::HeaderName;
/// use wiremock::{Match, MockServer, Mock, Request, ResponseTemplate, Responder};
/// use wiremock::matchers::path;
/// use std::convert::TryInto;
/// use std::str::FromStr;
///
/// /// Responds using the specified `ResponseTemplate`, but it dynamically populates the
/// /// `X-Correlation-Id` header from the request data.
/// pub struct CorrelationIdResponder(pub ResponseTemplate);
///
/// impl Responder for CorrelationIdResponder {
///     fn respond(&self, request: &Request) -> ResponseTemplate {
///         let mut response_template = self.0.clone();
///         let header_name = HeaderName::from_str("X-Correlation-Id").unwrap();
///         if let Some(correlation_id) = request.headers.get(&header_name) {
///             response_template = response_template.insert_header(
///                 header_name,
///                 correlation_id.last().to_owned()
///             );
///         }
///         response_template
///     }
/// }
///
/// #[async_std::main]
/// async fn main() {
///     // Arrange
///     let mock_server = MockServer::start().await;
///     let correlation_id = "1241-1245-1548-4567";
///
///     Mock::given(path("/hello"))
///         .respond_with(CorrelationIdResponder(ResponseTemplate::new(200)))
///         .mount(&mock_server)
///         .await;
///     
///     let response = surf::get(format!("{}/hello", &mock_server.uri()))
///         .header("X-Correlation-Id", correlation_id)
///         .await
///         .unwrap();
///     assert_eq!(response.status(), 200);
///     assert_eq!(response.header("X-Correlation-Id").unwrap().as_str(), correlation_id);
/// }
/// ```
///
/// [`Mock`]: struct.Mock.html
/// [`Request`]: struct.Request.html
pub trait Responder: Send + Sync {
    fn respond(&self, request: &Request) -> ResponseTemplate;
}

impl Responder for ResponseTemplate {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        self.clone()
    }
}
