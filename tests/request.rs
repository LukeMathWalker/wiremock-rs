use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use http_types::mime;
use wiremock::{matchers::any, Mock, MockServer, Request, ResponseTemplate};

#[async_std::test]
async fn reuqest_form_data_body() {
    // Arrange
    let form_data: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));
    let mock_server = MockServer::start().await;
    let form_data_clone = form_data.clone();
    Mock::given(any())
        .respond_with(move |request: &Request| {
            let form_data = request.body_form::<HashMap<String, String>>().unwrap();
            *form_data_clone.write().unwrap() = form_data;
            ResponseTemplate::new(200)
        })
        .mount(&mock_server)
        .await;

    // Act
    let _ = surf::post(&mock_server.uri())
        .content_type(mime::FORM)
        .body_string(r#"foo=bar&foo2="h%25l""#.to_string())
        .await
        .unwrap()
        .status();

    // Assert
    let result = form_data.read().unwrap().clone();
    let expected = HashMap::from([
        ("foo".to_string(), "bar".to_string()),
        ("foo2".to_string(), "\"h%l\"".to_string()),
    ]);
    assert_eq!(result, expected);
}
