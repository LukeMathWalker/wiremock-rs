//! All bits and pieces concerning the HTTP mock server are in this module.
//!
//! `bare_server::BareMockServer` is the "front-end" to drive behaviour for the `hyper` HTTP
//! server running in the background, defined in the `hyper` sub-module.
//!
//! `bare_server::BareMockServer` is not exposed directly: crate users only get to interact with
//! `pooled_server::MockServer`, which is nothing more than a wrapper around a `BareMockServer`
//! retrieved from an object pool - see the `pool` submodule for more details on our pooling
//! strategy.
mod bare_server;
mod hyper;
mod pool;
mod pooled_server;

pub use pooled_server::MockServer;
