use crate::{active_mock::ActiveMock, mock::Expectation};

/// A report returned by an `ActiveMock` detailing what the user expectations were and
/// how many calls were actually received since the mock was mounted on the server.
#[derive(Clone)]
pub(crate) struct VerificationReport {
    /// What users specified
    pub(crate) expectation: Expectation,
    /// Actual number of received requests that matched the specification
    pub(crate) n_matched_requests: u64,
}

impl From<&ActiveMock> for VerificationReport {
    fn from(mock: &ActiveMock) -> Self {
        Self {
            expectation: mock.specification().expectation.clone(),
            n_matched_requests: mock.n_matched_requests(),
        }
    }
}

impl VerificationReport {
    pub(crate) fn error_message(&self) -> String {
        format!(
            "{}. Expected range of matching incoming requests: {:?}, actual: {}",
            self.expectation.error_message, self.expectation.range, self.n_matched_requests
        )
    }
}

pub(crate) enum VerificationOutcome {
    /// All verifications were successful
    Correct,
    /// Failed verifications
    Incorrect(Vec<VerificationReport>),
}
