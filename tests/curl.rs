use curl::easy::{Easy2, Handler, WriteError};
use wiremock::{
    matchers::{body_bytes, method, PathExactMatcher},
    Mock, MockServer, ResponseTemplate,
};

struct Collector(Vec<u8>);

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}
#[tokio::test]
async fn test_curl_put() {
    // Arrange
    let mock_server = MockServer::start().await;

    let response = ResponseTemplate::new(200);
    Mock::given(method("PUT"))
        .and(PathExactMatcher::new("/node"))
        .and(body_bytes("file in bytes".as_bytes()))
        .respond_with(response.set_body_string("Reply"))
        .mount(&mock_server)
        .await;

    let mut easy2 = Easy2::new(Collector(Vec::new()));
    let url = format!("{}/node", mock_server.uri());
    easy2.url(url.as_str()).unwrap();
    easy2.put(true).unwrap();
    easy2.custom_request("PUT").unwrap();

    let body = "file in bytes".as_bytes();
    easy2.post_field_size(body.len() as u64).unwrap();
    easy2.post_fields_copy(body).unwrap();

    let _ = easy2.perform().unwrap();

    assert_eq!(easy2.response_code().unwrap(), 200);
}
