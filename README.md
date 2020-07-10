<h1 align="center">wiremock</h1>
<div align="center">
 <strong>
   HTTP mocking to test Rust applications.
 </strong>
</div>

<br />

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/wiremock">
    <img src="https://img.shields.io/crates/v/wiremock.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/wiremock">
    <img src="https://img.shields.io/crates/d/wiremock.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs.rs docs -->
  <a href="https://docs.rs/wiremock">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
  <!-- CI -->
  <a href="https://github.com/LukeMathWalker/wiremock-rs">
    <img src="https://circleci.com/gh/LukeMathWalker/wiremock-rs.svg?style=shield" alt="CircleCI badge" />
  </a>
</div>
<br/>

`wiremock` provides HTTP mocking to perform black-box testing of Rust applications that
interact with third-party APIs.

It provides mocking of HTTP responses using request matching and response templating.

*The name `wiremock` is a reference to [`WireMock.Net`](https://github.com/WireMock-Net/WireMock.Net), a .NET port of the original [`Wiremock`](http://wiremock.org/) from Java.*

<div align="center">
  <a style="display: inline" href="https://docs.rs/wiremock">Documentation</a>
  <span style="display: inline"> - </span>
  <a style="display: inline" href="https://crates.io/crates/wiremock">Crates.io</a>
</div>

# Table of Contents
1. [Getting started](#getting-started)
2. [Matchers](#matchers)
3. [Spying](#spying)
4. [Test isolation](#test-isolation)
5. [Runtime compatibility](#runtime-compatibility)
6. [Prior art](#prior-art)
7. [Future evolution](#future-evolution)
8. [License](#license)

## Getting started
```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[async_std::main]
async fn main() {
    // Start a background HTTP server on a random local port
    let mock_server = MockServer::start().await;

    // Arrange the behaviour of the MockServer adding a Mock:
    // when it receives a GET request on '/hello' it will respond with a 200.
    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200))
        // Mounting the mock on the mock server - it's now effective!
        .mount(&mock_server)
        .await;

    // If we probe the MockServer using any HTTP client it behaves as expected.
    let status = surf::get(format!("{}/hello", &mock_server.uri()))
        .await
        .unwrap()
        .status();
    assert_eq!(status.as_u16(), 200);

    // If the request doesn't match any `Mock` mounted on our `MockServer` a 404 is returned.
    let status = surf::get(format!("{}/missing", &mock_server.uri()))
        .await
        .unwrap()
        .status();
    assert_eq!(status.as_u16(), 404);
}
```

## Matchers

`wiremock` provides a set of matching strategies out of the box - check the [`matchers`] module
for a complete list.

You can define your own matchers using the [`Match`] trait, as well as using `Fn` closures.
Check [`Match`]'s documentation for more details and examples.

## Spying

`wiremock` empowers you to set expectations on the number of invocations to your [`Mock`]s -
check the [`expect`] method for more details.

Expectations can be used to verify that a side-effect has (or has not) taken place!

Expectations are automatically verified during the shutdown of each [`MockServer`] instance,
at the end of your test. A failed verification will trigger a panic.
By default, no expectations are set on your [`Mock`]s.

## Test isolation

Each instance of [`MockServer`] is fully isolated: [`start`] takes care of finding a random port
available on your local machine which is assigned to the new [`MockServer`].

You should use one instance of [`MockServer`] for each test, to ensure full isolation and
no cross-test interference.

When a [`MockServer`] instance goes out of scope (e.g. the test finishes), the corresponding
HTTP server running in the background is shut down to free up the port it was using.

## Runtime compatibility

`wiremock` can be used (and it is tested to work) with both [`async_std`] and [`tokio`] as
futures runtimes.
If you encounter any compatibility bug, please open an issue on our [GitHub repository].

## Prior art

[`mockito`] and [`httpmock`] provide HTTP mocking for Rust.

Check the table below to see how `wiremock` compares to them across the following dimensions:
- Test execution strategy (do tests have to be executed sequentially or can they be executed in parallel?);
- How many APIs can I mock in a test?
- Out-of-the-box request matchers;
- Extensible request matching (i.e. you can define your own matchers);
- Sync/Async API;
- Spying (e.g. verify that a mock has/hasn't been called in a test);
- Standalone mode (i.e. can I launch an HTTP mock server outside of a test suite?).

|           | Test execution strategy | How many APIs can I mock? | Out-of-the-box request matchers | Extensible request maching | API   | Spying | Standalone mode |
|-----------|-------------------------|---------------------------|---------------------------------|----------------------------|-------|----------|-----------------|
| mockito   | ❌ Sequential             | ❌ 1                        | ✔                           | ❌                        | Sync  | ✔     | ❌              |
| httpmock | ❌ Sequential             | ❌ 1                        | ✔                           | ❌                        | Sync  | ✔     | ✔              |
| wiremock  | ✔ Parallel ️              | ✔ Unbounded                | ✔                           | ✔                       | Async | ✔      | ❌              |


## Future evolution

More request matchers can be added to those provided out-of-the-box to handle common usecases.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.


[`MockServer`]: https://docs.rs/wiremock/0.2.3/wiremock/struct.MockServer.html
[`Mock`]: https://docs.rs/wiremock/0.2.3/wiremock/struct.Mock.html
[`Match`]: https://docs.rs/wiremock/0.2.3/wiremock/trait.Match.html
[`start`]: https://docs.rs/wiremock/0.2.3/wiremock/struct.MockServer.html#method.start
[`expect`]: https://docs.rs/wiremock/0.2.3/wiremock/struct.Mock.html#method.expect
[`matchers`]: https://docs.rs/wiremock/0.2.3/wiremock/matchers/index.html
[GitHub repository]: https://github.com/LukeMathWalker/wiremock-rs
[`mockito`]: https://docs.rs/mockito/
[`httpmock`]: https://docs.rs/httpmock/
[`async_std`]: https://docs.rs/async-std/
[`tokio`]: https://docs.rs/tokio/
