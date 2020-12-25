use crate::mock::Expectation;

/// A report returned by an `ActiveMock` detailing what the user expectations were and
/// how many calls were actually received since the mock was mounted on the server.
#[derive(Clone)]
pub(crate) struct VerificationReport {
    /// What users specified
    pub(crate) expectation: Expectation,
    /// Actual number of received requests that matched the specification
    pub(crate) n_matched_requests: u64,
}

impl VerificationReport {
    pub(crate) fn error_message(&self) -> String {
        format!(
            "{}. Expected range of matching incoming requests: {:?}, actual: {}",
            self.expectation.error_message, self.expectation.range, self.n_matched_requests
        )
    }

    pub(crate) fn is_satisfied(&self) -> bool {
        self.expectation.range.contains(self.n_matched_requests)
    }
}

pub(crate) enum VerificationOutcome {
    /// The expectations set on all active mocks were satisfied.
    Success,
    /// The expectations set for one or more of the active mocks were not satisfied.
    /// All failed expectations are returned.
    Failure(Vec<VerificationReport>),
}
