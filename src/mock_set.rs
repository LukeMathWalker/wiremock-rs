use crate::active_mock::ActiveMock;
use crate::{Mock, Request};
use futures_timer::Delay;
use http_types::{Response, StatusCode};
use log::debug;
use std::time::Duration;

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
        let mut response: Option<Response> = None;
        let mut delay: Option<Duration> = None;
        for mock in &mut self.mocks {
            if mock.matches(&request) {
                response = Some(mock.response());
                delay = mock.delay().to_owned();
                break;
            }
        }
        if let Some(response) = response {
            if let Some(delay) = delay {
                Delay::new(delay).await;
            }
            response
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
