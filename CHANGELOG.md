# Changelog

# Next

# `0.5.4`

- Fixes:
    - Handle URI authority properly in the mock server (by [@Tuetuopay])

# `0.5.3`

- Miscellaneous:
    - Remove `textwrap` from the dependency tree (by [@apiraino])

# `0.5.2`

- Features:
    - Support multi-valued headers (by [@beltram])
- Miscellaneous:
    - Improve README (by [@apiraino])

# `0.5.1`

- Features:
    - Capture the port in `hyper`'s `Request` into `wiremock::Request::url`  (by [@beltram])

# `0.5.0`

- Breaking changes:
    - Removed `MockServer::start_on`.  
      Use `MockServer::builder` and `MockServerBuilder::listener` to configure your `MockServer` to start on a pre-determined port (by [@LukeMathWalker]).
    - `MockServer::verify` is now asynchronous (by [@LukeMathWalker]).
- Features:
    - Added request recording to `MockServer`, enabled by default.  
      The recorded requests are used to display more meaningful error messages when assertions are not satisfied and can be retrieved using `MockServer::received_requests` ([by @LukeMathWalker]).
    - Added `MockServerBuilder` to customise the configuration of a `MockServer`.  
      It can be used to bind a `MockServer` to an existing `TcpListener` as well as disabling request recording (by [@LukeMathWalker]).
    - Added `matchers::body_json_schema` to verify the structure of the body attached to an incoming request (by [@Michael-J-Ward]).

[@Michael-J-Ward]: https://github.com/Michael-J-Ward
[@LukeMathWalker]: https://github.com/LukeMathWalker
[@beltram]: https://github.com/beltram
[@apiraino]: https://github.com/apiraino
[@Tuetuopay]: https://github.com/Tuetuopay

