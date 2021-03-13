use wiremock::matchers::{header, headers, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[async_std::test]
async fn should_match_simple_request_header() {
    // Arrange
    let mock_server = MockServer::start().await;
    let mock = Mock::given(method("GET"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_match = surf::get(mock_server.uri())
        .header("content-type", "application/json")
        .await
        .unwrap();
    // Assert
    assert_eq!(should_match.status(), 200);
}

#[async_std::test]
async fn should_not_match_simple_request_header_upon_wrong_key() {
    // Arrange
    let mock_server = MockServer::start().await;
    let mock = Mock::given(method("GET"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_fail_wrong_key = surf::get(mock_server.uri())
        .header("accept", "application/json")
        .await
        .unwrap();
    // Assert
    assert_eq!(should_fail_wrong_key.status(), 404);
}

#[async_std::test]
async fn should_not_match_simple_request_header_upon_wrong_value() {
    // Arrange
    let mock_server = MockServer::start().await;
    let mock = Mock::given(method("GET"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_fail_wrong_value = surf::get(mock_server.uri())
        .header("content-type", "application/xml")
        .await
        .unwrap();
    // Assert
    assert_eq!(should_fail_wrong_value.status(), 404);
}

#[async_std::test]
async fn should_match_multi_request_header() {
    // Arrange
    let mock_server = MockServer::start().await;
    let header_matcher = headers("cache-control", vec!["no-cache", "no-store"]);
    let mock = Mock::given(method("GET"))
        .and(header_matcher)
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_match = surf::get(mock_server.uri())
        .header("cache-control", "no-cache, no-store")
        .await
        .unwrap();

    // Assert
    assert_eq!(should_match.status(), 200);
}

#[async_std::test]
async fn should_not_match_multi_request_header_upon_wrong_values() {
    // Arrange
    let mock_server = MockServer::start().await;
    let header_matcher = headers("cache-control", vec!["no-cache", "no-store"]);
    let mock = Mock::given(method("GET"))
        .and(header_matcher)
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_fail_wrong_values = surf::get(mock_server.uri())
        .header("cache-control", "no-cache, no-transform")
        .await
        .unwrap();

    // Assert
    assert_eq!(should_fail_wrong_values.status(), 404);
}

#[async_std::test]
async fn should_not_match_multi_request_header_upon_incomplete_values() {
    // Arrange
    let mock_server = MockServer::start().await;
    let header_matcher = headers("cache-control", vec!["no-cache", "no-store"]);
    let mock = Mock::given(method("GET"))
        .and(header_matcher)
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_fail_incomplete_values = surf::get(mock_server.uri())
        .header("cache-control", "no-cache")
        .await
        .unwrap();

    // Assert
    assert_eq!(should_fail_incomplete_values.status(), 404);
}
