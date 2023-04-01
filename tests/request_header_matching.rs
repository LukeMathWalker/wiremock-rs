use hyper::HeaderMap;
use wiremock::matchers::{basic_auth, bearer_token, header, header_regex, headers, method};
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

#[tokio::test]
async fn should_match_multi_request_header_x() {
    // Arrange
    let mock_server = MockServer::start().await;
    let header_matcher = headers("cache-control", vec!["no-cache", "no-store"]);
    let mock = Mock::given(method("GET"))
        .and(header_matcher)
        .respond_with(ResponseTemplate::new(200))
        .expect(1);
    mock_server.register(mock).await;

    // Act
    let mut header_map = HeaderMap::new();
    header_map.append("cache-control", "no-cache".parse().unwrap());
    header_map.append("cache-control", "no-store".parse().unwrap());
    let should_match = reqwest::Client::new()
        .get(mock_server.uri())
        .headers(header_map)
        .send()
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

#[async_std::test]
async fn should_match_regex_single_header_value() {
    // Arrange
    let mock_server = MockServer::start().await;
    let mock = Mock::given(method("GET"))
        .and(header_regex("cache-control", r"no-(cache|store)"))
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_match = surf::get(mock_server.uri())
        .header("cache-control", "no-cache")
        .await
        .unwrap();

    // Assert
    assert_eq!(should_match.status(), 200);
}

#[async_std::test]
async fn should_match_regex_multiple_header_values() {
    // Arrange
    let mock_server = MockServer::start().await;
    let mock = Mock::given(method("GET"))
        .and(header_regex("cache-control", r"no-(cache|store)"))
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_match = surf::get(mock_server.uri())
        .header("cache-control", "no-cache")
        .header("cache-control", "no-store")
        .await
        .unwrap();

    // Assert
    assert_eq!(should_match.status(), 200);
}

#[async_std::test]
async fn should_not_match_regex_with_wrong_header_value() {
    // Arrange
    let mock_server = MockServer::start().await;
    let mock = Mock::given(method("GET"))
        .and(header_regex("cache-control", r"no-(cache|store)"))
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_fail_wrong_value = surf::get(mock_server.uri())
        .header("cache-control", "no-junk")
        .await
        .unwrap();

    // Assert
    assert_eq!(should_fail_wrong_value.status(), 404);
}

#[async_std::test]
async fn should_not_match_regex_with_at_least_one_wrong_header_value() {
    // Arrange
    let mock_server = MockServer::start().await;
    let mock = Mock::given(method("GET"))
        .and(header_regex("cache-control", r"no-(cache|store)"))
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_fail_wrong_value = surf::get(mock_server.uri())
        .header("cache-control", "no-cache")
        .header("cache-control", "no-junk")
        .await
        .unwrap();

    // Assert
    assert_eq!(should_fail_wrong_value.status(), 404);
}

#[async_std::test]
async fn should_not_match_regex_with_no_values_for_header() {
    // Arrange
    let mock_server = MockServer::start().await;
    let mock = Mock::given(method("GET"))
        .and(header_regex("cache-control", r"no-(cache|store)"))
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_fail_wrong_value = surf::get(mock_server.uri()).await.unwrap();

    // Assert
    assert_eq!(should_fail_wrong_value.status(), 404);
}

#[async_std::test]
async fn should_match_basic_auth_header() {
    // Arrange
    let mock_server = MockServer::start().await;
    let mock = Mock::given(method("GET"))
        .and(basic_auth("Aladdin", "open sesame"))
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_match = surf::get(mock_server.uri())
        .header("Authorization", "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==")
        .await
        .unwrap();
    // Assert
    assert_eq!(should_match.status(), 200);
}

#[async_std::test]
async fn should_not_match_bad_basic_auth_header() {
    // Arrange
    let mock_server = MockServer::start().await;
    let mock = Mock::given(method("GET"))
        .and(basic_auth("Aladdin", "close sesame"))
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_not_match = surf::get(mock_server.uri())
        .header("Authorization", "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==")
        .await
        .unwrap();
    // Assert
    assert_eq!(should_not_match.status(), 404);
}

#[async_std::test]
async fn should_match_bearer_token_header() {
    // Arrange
    let mock_server = MockServer::start().await;
    let mock = Mock::given(method("GET"))
        .and(bearer_token("delightful"))
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_match = surf::get(mock_server.uri())
        .header("Authorization", "Bearer delightful")
        .await
        .unwrap();
    // Assert
    assert_eq!(should_match.status(), 200);
}

#[async_std::test]
async fn should_not_match_bearer_token_header() {
    // Arrange
    let mock_server = MockServer::start().await;
    let mock = Mock::given(method("GET"))
        .and(bearer_token("expired"))
        .respond_with(ResponseTemplate::new(200));
    mock_server.register(mock).await;

    // Act
    let should_not_match = surf::get(mock_server.uri())
        .header("Authorization", "Bearer delightful")
        .await
        .unwrap();
    // Assert
    assert_eq!(should_not_match.status(), 404);
}
