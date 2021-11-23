use crate::{verification::VerificationReport, Match, Mock, Request, ResponseTemplate};

/// Given the behaviour specification as a [`Mock`](crate::Mock), keep track of runtime information
/// concerning this mock - e.g. how many times it matched on a incoming request.
pub(crate) struct MountedMock {
    pub(crate) specification: Mock,
    n_matched_requests: u64,
    /// The position occupied by this mock within the parent [`MountedMockSet`](crate::mock_set::MountedMockSet)
    /// collection of `MountedMock`s.
    ///
    /// E.g. `0` if this is the first mock that we try to match against an incoming request, `1`
    /// if it is the second, etc.
    position_in_set: usize,
}

impl MountedMock {
    pub(crate) fn new(specification: Mock, position_in_set: usize) -> Self {
        Self {
            specification,
            n_matched_requests: 0,
            position_in_set,
        }
    }

    /// This is NOT the same of `matches` from the `Match` trait!
    /// Key difference: we are talking a mutable reference to `self` in order to capture
    /// additional information (e.g. how many requests we matched so far) or change behaviour
    /// after a certain threshold has been crossed (e.g. start returning `false` for all requests
    /// once enough requests have been matched according to `max_n_matches`).
    pub(crate) fn matches(&mut self, request: &Request) -> bool {
        if Some(self.n_matched_requests) == self.specification.max_n_matches {
            // Skip the actual check if we are already at our maximum of matched requests.
            false
        } else {
            let matched = self
                .specification
                .matchers
                .iter()
                .all(|matcher| matcher.matches(request));

            if matched {
                // Increase match count
                self.n_matched_requests += 1;
            }

            matched
        }
    }

    /// Verify if this mock has verified the expectations set at creation time
    /// over the number of invocations.
    pub(crate) fn verify(&self) -> VerificationReport {
        VerificationReport {
            mock_name: self.specification.name.clone(),
            n_matched_requests: self.n_matched_requests,
            expectation_range: self.specification.expectation_range.clone(),
            position_in_set: self.position_in_set,
        }
    }

    pub(crate) fn response_template(&self, request: &Request) -> ResponseTemplate {
        self.specification.response_template(request)
    }
}
