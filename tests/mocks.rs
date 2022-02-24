use http_types::StatusCode;
use serde::Serialize;
use serde_json::json;
use std::net::TcpStream;
use wiremock::matchers::{body_json, body_partial_json, method, PathExactMatcher};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[async_std::test]
async fn new_starts_the_server() {
    // Act
    let mock_server = MockServer::start().await;

    // Assert
    assert!(TcpStream::connect(&mock_server.address()).is_ok())
}

#[async_std::test]
async fn returns_404_if_nothing_matches() {
    // Arrange - no mocks mounted
    let mock_server = MockServer::start().await;

    // Act
    let status = surf::get(&mock_server.uri()).await.unwrap().status();

    // Assert
    assert_eq!(status, 404);
}

#[async_std::test]
#[should_panic]
async fn panics_if_the_expectation_is_not_satisfied() {
    // Arrange
    let mock_server = MockServer::start().await;
    let response = ResponseTemplate::new(200);
    Mock::given(method("GET"))
        .respond_with(response)
        .expect(1..)
        .named("panics_if_the_expectation_is_not_satisfied expectation failed")
        .mount(&mock_server)
        .await;

    // Act - we never call the mock
}

#[async_std::test]
#[should_panic(expected = "Verifications failed:
- Mock #0.
\tExpected range of matching incoming requests: 1 <= x
\tNumber of matched incoming requests: 0

The server did not receive any request.")]
async fn no_received_request_line_is_printed_in_the_panic_message_if_expectations_are_not_verified()
{
    // Arrange
    let mock_server = MockServer::start().await;
    let response = ResponseTemplate::new(200);
    Mock::given(method("GET"))
        .respond_with(response)
        .expect(1..)
        .mount(&mock_server)
        .await;

    // Act - we never call the mock
}

#[async_std::test]
#[should_panic(expected = "Verifications failed:
- Mock #0.
\tExpected range of matching incoming requests: 1 <= x
\tNumber of matched incoming requests: 0

Received requests:
- Request #1
\tGET http://localhost/")]
async fn received_request_are_printed_as_panic_message_if_expectations_are_not_verified() {
    // Arrange
    let mock_server = MockServer::start().await;
    let response = ResponseTemplate::new(200);
    Mock::given(method("POST"))
        .respond_with(response)
        .expect(1..)
        .mount(&mock_server)
        .await;

    // Act - we sent a request that does not match (GET)
    surf::get(&mock_server.uri()).await.unwrap();

    // Assert - verified on drop
}

#[async_std::test]
#[should_panic]
async fn panic_during_expectation_does_not_crash() {
    // Arrange
    let mock_server = MockServer::start().await;
    let response = ResponseTemplate::new(200);
    Mock::given(method("GET"))
        .respond_with(response)
        .expect(1..)
        .named("panic_during_expectation_does_not_crash expectation failed")
        .mount(&mock_server)
        .await;

    // Act - start a panic
    panic!("forced panic")
}

#[async_std::test]
async fn simple_route_mock() {
    // Arrange
    let mock_server = MockServer::start().await;
    let response = ResponseTemplate::new(200).set_body_bytes("world");
    let mock = Mock::given(method("GET"))
        .and(PathExactMatcher::new("hello"))
        .respond_with(response);
    mock_server.register(mock).await;

    // Act
    let mut response = surf::get(format!("{}/hello", &mock_server.uri()))
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), 200);
    assert_eq!(response.body_string().await.unwrap(), "world");
}

#[async_std::test]
async fn two_route_mocks() {
    // Arrange
    let mock_server = MockServer::start().await;

    // First
    let response = ResponseTemplate::new(200).set_body_bytes("aaa");
    Mock::given(method("GET"))
        .and(PathExactMatcher::new("first"))
        .respond_with(response)
        .named("/first")
        .mount(&mock_server)
        .await;

    // Second
    let response = ResponseTemplate::new(200).set_body_bytes("bbb");
    Mock::given(method("GET"))
        .and(PathExactMatcher::new("second"))
        .respond_with(response)
        .named("/second")
        .mount(&mock_server)
        .await;

    // Act
    let mut first_response = surf::get(format!("{}/first", &mock_server.uri()))
        .await
        .unwrap();
    let mut second_response = surf::get(format!("{}/second", &mock_server.uri()))
        .await
        .unwrap();

    // Assert
    assert_eq!(first_response.status(), 200);
    assert_eq!(second_response.status(), 200);

    assert_eq!(first_response.body_string().await.unwrap(), "aaa");
    assert_eq!(second_response.body_string().await.unwrap(), "bbb");
}

#[async_std::test]
async fn body_json_matches_independent_of_key_ordering() {
    #[derive(Serialize)]
    struct X {
        b: u8,
        a: u8,
    }

    // Arrange
    let expected_body = json!({ "a": 1, "b": 2 });
    let body = serde_json::to_string(&X { a: 1, b: 2 }).unwrap();

    let mock_server = MockServer::start().await;
    let response = ResponseTemplate::new(200);
    let mock = Mock::given(method("POST"))
        .and(body_json(expected_body))
        .respond_with(response);
    mock_server.register(mock).await;

    // Act
    let response = surf::post(mock_server.uri()).body(body).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::Ok);
}

#[async_std::test]
async fn body_json_partial_matches_a_part_of_response_json() {
    // Arrange
    let expected_body = json!({ "a": 1, "c": { "e": 2 } });
    let body = json!({ "a": 1, "b": 2, "c": { "d": 1, "e": 2 } });

    let mock_server = MockServer::start().await;
    let response = ResponseTemplate::new(200);
    let mock = Mock::given(method("POST"))
        .and(body_partial_json(expected_body))
        .respond_with(response);
    mock_server.register(mock).await;

    // Act
    let response = surf::post(mock_server.uri()).body(body).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::Ok);
}
