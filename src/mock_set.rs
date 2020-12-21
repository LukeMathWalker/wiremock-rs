use crate::active_mock::ActiveMock;
use crate::{Mock, Request, ResponseTemplate};
use futures_timer::Delay;
use http_types::{Response, StatusCode};
use log::debug;

pub(crate) struct MockSet {
    mocks: Vec<ActiveMock>,
}

impl MockSet {
    /// Create a new instance of MockSet.
    pub(crate) fn new() -> MockSet {
        MockSet { mocks: vec![] }
    }

    pub(crate) async fn handle_request(&mut self, request: Request) -> Response {
        debug!("Handling request.");
        let mut response_template: Option<ResponseTemplate> = None;
        for mock in &mut self.mocks {
            if mock.matches(&request) {
                response_template = Some(mock.response_template(&request));
                break;
            }
        }
        if let Some(response_template) = response_template {
            if let Some(delay) = response_template.delay() {
                Delay::new(delay.to_owned()).await;
            }
            response_template.generate_response()
        } else {
            debug!("Got unexpected request:\n{}", request);
            Response::new(StatusCode::NotFound)
        }
    }

    pub(crate) fn register(&mut self, mock: Mock) {
        self.mocks.push(ActiveMock::new(mock));
    }

    pub(crate) fn reset(&mut self) {
        self.mocks = vec![];
    }

    pub(crate) fn verify(&self) -> bool {
        self.mocks.iter().all(|m| m.verify())
    }
}
