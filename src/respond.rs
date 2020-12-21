use crate::{Request, ResponseTemplate};

/// Anything that implements `Respond` can be used to reply to an incoming request when a
/// [`Mock`] is activated.
///
/// ## Fixed responses
///
/// The simplest `Respond` is [`ResponseTemplate`]: no matter the request, it will
/// always return itself.
///
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
///     let response = surf::get(&mock_server.uri())
///         .await
///         .unwrap();
///
///     // Assert
///     assert_eq!(response.status(), 200);
///     assert_eq!(response.header("X-Correlation-ID").unwrap().as_str(), correlation_id);
/// }
/// ```
///
/// ## Dynamic responses
///
/// You can use `Respond`, though, to implement responses that depend on the data in
/// the request matched by a [`Mock`].  
/// You could, for example, propagate back a request header in the response:
///
/// ```rust
/// use http_types::headers::HeaderName;
/// use wiremock::{Match, MockServer, Mock, Request, ResponseTemplate, Respond};
/// use wiremock::matchers::path;
/// use std::convert::TryInto;
/// use std::str::FromStr;
///
/// /// Responds using the specified `ResponseTemplate`, but it dynamically populates the
/// /// `X-Correlation-Id` header from the request data.
/// pub struct CorrelationIdResponder(pub ResponseTemplate);
///
/// impl Respond for CorrelationIdResponder {
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
/// [`Mock`]: crate::Mock
/// [`ResponseTemplate`]: crate::ResponseTemplate
pub trait Respond: Send + Sync {
    /// Given a reference to a [`Request`] return a [`ResponseTemplate`] that will be used
    /// by the [`MockServer`] as blueprint for the response returned to the client.
    ///
    /// [`Request`]: crate::Request
    /// [`MockServer`]: crate::MockServer
    /// [`ResponseTemplate`]: crate::ResponseTemplate
    fn respond(&self, request: &Request) -> ResponseTemplate;
}

impl Respond for ResponseTemplate {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        self.clone()
    }
}
