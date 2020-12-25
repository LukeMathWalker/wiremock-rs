use crate::{active_mock::ActiveMock, mock::Expectation};

#[derive(Clone)]
pub(crate) struct Verification {
    /// What users specified
    pub(crate) expectation: Expectation,
    /// Actual number of received requests that matched the specifications
    pub(crate) n_matched_requests: u64,
}

impl From<&ActiveMock> for Verification {
    fn from(mock: &ActiveMock) -> Self {
        Self {
            expectation: mock.specification().expectation.clone(),
            n_matched_requests: mock.n_matched_requests(),
        }
    }
}

impl Verification {
    pub(crate) fn error_message(&self) -> String {
        format!(
            "{}. Expected range of matching incoming requests: {:?}, actual: {}",
            self.expectation.error_message, self.expectation.range, self.n_matched_requests
        )
    }
}

pub(crate) enum VerificationOutcome {
    Correct,
    Incorrect(Vec<Verification>),
}
