use reqwest::Client;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// regression tests for https://github.com/LukeMathWalker/wiremock-rs/issues/7
// running both tests will _sometimes_ trigger a hang if the runtimes arent separated correctly

#[tokio::test]
async fn hello_reqwest() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    println!("sending reqwest request");
    let resp = Client::new().get(&mock_server.uri()).send().await.unwrap();
    println!("reqwest request done");

    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn hello_surf() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    println!("sending surf request");
    let status = surf::get(&mock_server.uri()).await.unwrap().status();
    println!("surf request done");

    assert_eq!(status, 200);
}

#[actix_rt::test]
async fn hello_reqwest_actix() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    println!("sending reqwest request");
    let resp = Client::new().get(&mock_server.uri()).send().await.unwrap();
    println!("reqwest request done");

    assert_eq!(resp.status(), 200);
}
