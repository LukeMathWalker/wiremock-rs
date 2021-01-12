use crate::{
    active_mock::ActiveMock,
    verification::{VerificationOutcome, VerificationReport},
};
use crate::{Mock, Request, ResponseTemplate};
use futures_timer::Delay;
use http_types::{Response, StatusCode};
use log::debug;

/// The collection of mocks used by a `MockServer` instance to match against
/// incoming requests.
///
/// New mocks are added to `ActiveMockSet` every time [`MockServer::register`](crate::MockServer::register) or
/// [`Mock::mount`](crate::Mock::mount) are called.
pub(crate) struct ActiveMockSet {
    mocks: Vec<ActiveMock>,
}

impl ActiveMockSet {
    /// Create a new instance of MockSet.
    pub(crate) fn new() -> ActiveMockSet {
        ActiveMockSet { mocks: vec![] }
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
        let n_registered_mocks = self.mocks.len();
        let active_mock = ActiveMock::new(mock, n_registered_mocks);
        self.mocks.push(active_mock);
    }

    pub(crate) fn reset(&mut self) {
        self.mocks = vec![];
    }

    pub(crate) fn verify(&self) -> VerificationOutcome {
        let failed_verifications: Vec<VerificationReport> = self
            .mocks
            .iter()
            .map(ActiveMock::verify)
            .filter(|verification_report| !verification_report.is_satisfied())
            .collect();
        if failed_verifications.is_empty() {
            VerificationOutcome::Success
        } else {
            VerificationOutcome::Failure(failed_verifications)
        }
    }
}
