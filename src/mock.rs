use crate::response_template::ResponseTemplate;
use crate::{MockServer, Request};
use http_types::Response;
use std::fmt::{Debug, Formatter};
use std::ops::{
    Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};
use std::time::Duration;

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
///         .up_to_n_times(1)
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
    // Maximum number of times (inclusive) we should return a response from this Mock on
    // matching requests.
    // If `None`, there is no cap and we will respond to all incoming matching requests.
    // If `Some(max_n_matches)`, when `max_n_matches` matching incoming requests have been processed,
    // `self.matches` should start returning `false`, regardless of the incoming request.
    pub(crate) max_n_matches: Option<u64>,
    pub(crate) expectation: Times,
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

    /// Specify an upper limit to the number of times you would like this `Mock` to respond to
    /// incoming requests that satisfy the conditions imposed by your [`matchers`].
    ///
    /// ### Example:
    ///
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
    ///         // Default behaviour will have this Mock responding to any incoming request
    ///         // that satisfied our matcher (e.g. being a GET request).
    ///         // We can opt out of the default behaviour by setting a cap on the number of
    ///         // matching requests this Mock should respond to.
    ///         //
    ///         // In this case, once one matching request has been received, the mock will stop
    ///         // matching additional requests and you will receive a 404 if no other mock
    ///         // matches on those requests.
    ///         .up_to_n_times(1)
    ///         .mount(&mock_server)
    ///         .await;
    ///     
    ///     // Act
    ///
    ///     // The first request matches, as expected.
    ///     let status = surf::get(&mock_server.uri())
    ///         .await
    ///         .unwrap()
    ///         .status();
    ///     assert_eq!(status.as_u16(), 200);
    ///
    ///     // The second request does NOT match given our `up_to_n_times(1)` setting.
    ///     let status = surf::get(&mock_server.uri())
    ///         .await
    ///         .unwrap()
    ///         .status();
    ///     assert_eq!(status.as_u16(), 404);
    /// }
    /// ```
    ///
    /// [`matchers`]: matchers/index.html
    pub fn up_to_n_times(mut self, n: u64) -> Mock {
        assert!(n > 0, "n must be strictly greater than 0!");
        self.max_n_matches = Some(n);
        self
    }

    /// Set an expectation on the number of times this `Mock` should match in the current
    /// test case.
    /// Expectations are verified when the [`MockServer`] is shutting down: if the expectation
    /// is not satisfied, the [`MockServer`] will panic.
    ///
    /// By default, no expectation is set for `Mock`s.
    ///
    /// ### When is this useful?
    ///
    /// `expect` can turn out handy when you'd like to verify that a certain side-effect has
    /// (or has not!) taken place.
    ///
    /// For example:
    /// - check that a 3rd party notification API (e.g. email service) is called when an event
    ///   in your application is supposed to trigger a notification;
    /// - check that a 3rd party API is NOT called when the response of a call is expected
    ///   to be retrieved from a cache (`.expect(0)`).
    ///
    /// This technique is also called [spying](https://martinfowler.com/bliki/TestDouble.html).
    ///
    /// ### Example:
    ///
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
    ///         .up_to_n_times(2)
    ///         // We expect the mock to be called at least once.
    ///         // If that does not happen, the `MockServer` will panic on shutdown,
    ///         // causing the whole test to fail.
    ///         .expect(1..)
    ///         .mount(&mock_server)
    ///         .await;
    ///     
    ///     // Act
    ///     let status = surf::get(&mock_server.uri())
    ///         .await
    ///         .unwrap()
    ///         .status();
    ///     assert_eq!(status.as_u16(), 200);
    ///
    ///     // Assert
    ///     // We made at least one matching request, the expectation is satisfied.
    ///     // The `MockServer` will shutdown peacefully, without panicking.
    /// }
    /// ```
    ///
    /// [`MockServer`]: struct.MockServer.html
    pub fn expect<T: Into<Times>>(mut self, r: T) -> Mock {
        let range = r.into();
        self.expectation = range;
        self
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

    /// Build an instance of `http_types::Response` from the [`ResponseTemplate`] associated
    /// with a `Mock`.
    ///
    /// [`ResponseTemplate`]: struct.ResponseTemplate.html
    pub(crate) fn delay(&self) -> &Option<Duration> {
        self.response.delay()
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
            max_n_matches: None,
            expectation: Times(TimesEnum::Unbounded(RangeFull)),
        }
    }
}

/// Specify how many times we expect a [`Mock`] to match via [`expect`].
/// It is used to set expectations on the usage of a [`Mock`] in a test case.
///
/// You can either specify an exact value, e.g.
/// ```rust
/// use wiremock::Times;
///
/// let times: Times = 10.into();
/// ```
/// or a range
/// ```rust
/// use wiremock::Times;
///
/// // Between 10 and 15 (not included) times
/// let times: Times = (10..15).into();
/// // Between 10 and 15 (included) times
/// let times: Times = (10..=15).into();
/// // At least 10 times
/// let times: Times = (10..).into();
/// // Strictly less than 15 times
/// let times: Times = (..15).into();
/// // Strictly less than 16 times
/// let times: Times = (..=15).into();
/// ```
///
/// [`Mock`]: struct.Mock.html
/// [`expect`]: struct.Mock.html#method.expect
#[derive(Debug)]
pub struct Times(TimesEnum);

impl Times {
    pub(crate) fn contains(&self, n_calls: u64) -> bool {
        match &self.0 {
            TimesEnum::Exact(e) => e == &n_calls,
            TimesEnum::Unbounded(r) => r.contains(&n_calls),
            TimesEnum::Range(r) => r.contains(&n_calls),
            TimesEnum::RangeFrom(r) => r.contains(&n_calls),
            TimesEnum::RangeTo(r) => r.contains(&n_calls),
            TimesEnum::RangeToInclusive(r) => r.contains(&n_calls),
            TimesEnum::RangeInclusive(r) => r.contains(&n_calls),
        }
    }
}

// Implementation notes: this has gone through a couple of iterations before landing to
// what you see now.
//
// The original draft had Times itself as an enum with two variants (Exact and Range), with
// the Range variant generic over `R: RangeBounds<u64>`.
//
// We switched to a generic struct wrapper around a private `R: RangeBounds<u64>` when we realised
// that you would have had to specify a range type when creating the Exact variant
// (e.g. as you do for `Option` when creating a `None` variant).
//
// We achieved the same functionality with a struct wrapper, but exact values had to converted
// to ranges with a single element (e.g. 15 -> 15..16).
// Not the most expressive representation, but we would have lived with it.
//
// We changed once again when we started to update our `MockActor`: we are storing all `Mock`s
// in a vector. Being generic over `R`, the range type leaked into the overall `Mock` (and `ActiveMock`)
// type, thus making those generic as well over `R`.
// To store them in a vector all mocks would have had to use the same range internally, which is
// obviously an unreasonable restrictions.
// At the same time, we can't have a Box<dyn RangeBounds<u64>> because `contains` is a generic
// method hence the requirements for object safety are not satisfied.
//
// Thus we ended up creating this master enum that wraps all range variants with the addition
// of the Exact variant.
// If you can do better, please submit a PR.
// We keep them enum private to the crate to allow for future refactoring.
#[derive(Debug)]
pub(crate) enum TimesEnum {
    Exact(u64),
    Unbounded(RangeFull),
    Range(Range<u64>),
    RangeFrom(RangeFrom<u64>),
    RangeTo(RangeTo<u64>),
    RangeToInclusive(RangeToInclusive<u64>),
    RangeInclusive(RangeInclusive<u64>),
}

impl From<u64> for Times {
    fn from(x: u64) -> Self {
        Times(TimesEnum::Exact(x))
    }
}

// A quick macro to help easing the implementation pain.
macro_rules! impl_from_for_range {
    ($type_name:ident) => {
        impl From<$type_name<u64>> for Times {
            fn from(r: $type_name<u64>) -> Self {
                Times(TimesEnum::$type_name(r))
            }
        }
    };
}

impl_from_for_range!(Range);
impl_from_for_range!(RangeTo);
impl_from_for_range!(RangeFrom);
impl_from_for_range!(RangeInclusive);
impl_from_for_range!(RangeToInclusive);
