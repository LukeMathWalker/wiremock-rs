use crate::{Match, Mock, Request};
use http_types::Response;
use std::time::Duration;

// Given the behaviour specification as a `Mock`, keep track of runtime information concerning
// this mock - e.g. how many times it matched on a incoming request.
pub(crate) struct ActiveMock {
    specification: Mock,
    n_matched_requests: u64,
}

impl ActiveMock {
    pub(crate) fn new(specification: Mock) -> Self {
        Self {
            specification,
            n_matched_requests: 0,
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
    /// Returns true if expectations have been satisfied, false otherwise.
    pub(crate) fn verify(&self) -> bool {
        self.specification
            .expectation
            .contains(self.n_matched_requests)
    }

    pub(crate) fn response(&self) -> Response {
        self.specification.response()
    }

    pub(crate) fn delay(&self) -> &Option<Duration> {
        &self.specification.delay()
    }
}
