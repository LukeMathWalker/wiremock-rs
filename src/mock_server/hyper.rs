use crate::mock_server::bare_server::MockServerState;
use futures::future::{BoxFuture, FutureExt as _};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper_server::accept::Accept;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

/// Work around a lifetime error where, for some reason,
/// `Box<dyn std::error::Error + Send + Sync + 'static>` can't be converted to a
/// `Box<dyn std::error::Error + Send + Sync>`
pub(super) struct ErrorLifetimeCast(Box<dyn std::error::Error + Send + Sync + 'static>);

impl From<ErrorLifetimeCast> for Box<dyn std::error::Error + Send + Sync> {
    fn from(value: ErrorLifetimeCast) -> Self {
        value.0
    }
}

#[derive(Clone)]
pub(super) struct HyperRequestHandler {
    server_state: Arc<RwLock<MockServerState>>,
}

impl hyper::service::Service<hyper::Request<hyper::body::Incoming>> for HyperRequestHandler {
    type Response = hyper::Response<Full<Bytes>>;
    type Error = ErrorLifetimeCast;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn call(&self, request: hyper::Request<hyper::body::Incoming>) -> Self::Future {
        let server_state = self.server_state.clone();
        async move {
            let wiremock_request = crate::Request::from_hyper(request).await;
            let (response, delay) = server_state
                .write()
                .await
                .handle_request(wiremock_request)
                .await
                .map_err(ErrorLifetimeCast)?;

            // We do not wait for the delay within the handler otherwise we would be
            // holding on to the write-side of the `RwLock` on `mock_set`.
            // Holding on the lock while waiting prevents us from handling other requests until
            // we have waited the whole duration specified in the delay.
            // In particular, we cannot perform even perform read-only operation -
            // e.g. check that mock assumptions have been verified.
            // Using long delays in tests without handling the delay as we are doing here
            // caused tests to hang (see https://github.com/seanmonstar/reqwest/issues/1147)
            if let Some(delay) = delay {
                delay.await;
            }

            Ok::<_, ErrorLifetimeCast>(response)
        }
        .boxed()
    }
}

/// The actual HTTP server responding to incoming requests according to the specified mocks.
pub(super) async fn run_server<A>(
    listener: std::net::TcpListener,
    server_state: Arc<RwLock<MockServerState>>,
    mut shutdown_signal: tokio::sync::watch::Receiver<()>,
    acceptor: A,
) where
    A: Accept<tokio::net::TcpStream, HyperRequestHandler> + Send + Clone + 'static,
    <A as Accept<tokio::net::TcpStream, HyperRequestHandler>>::Future: Send,
    <A as Accept<tokio::net::TcpStream, HyperRequestHandler>>::Stream:
        Unpin + Send  + AsyncWrite + AsyncRead + 'static,
    <A as Accept<tokio::net::TcpStream, HyperRequestHandler>>::Service:
        hyper::service::Service<http::Request<hyper::body::Incoming>, Response = http::Response<Full<Bytes>>>
    + Send,
    <<A as Accept<tokio::net::TcpStream, HyperRequestHandler>>::Service
     as hyper::service::Service<http::Request<hyper::body::Incoming>>>::Error:
        Send + Sync  + Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    <<A as Accept<tokio::net::TcpStream, HyperRequestHandler>>::Service
     as hyper::service::Service<http::Request<hyper::body::Incoming>>>::Future: Send + 'static,
{
    listener
        .set_nonblocking(true)
        .expect("Cannot set non-blocking mode on TcpListener");
    let listener = TcpListener::from_std(listener).expect("Cannot upgrade TcpListener");

    let request_handler = HyperRequestHandler {
        server_state: server_state.clone(),
    };

    loop {
        let (stream, _) = tokio::select! { biased;
            accepted = listener.accept() => {
                match accepted {
                    Ok(accepted) => accepted,
                    Err(_) => break,
                }
            },
            _ = shutdown_signal.changed() => {
                log::info!("Mock server shutting down");
                break;
            }
        };
        let request_handler = request_handler.clone();
        let mut shutdown_signal = shutdown_signal.clone();
        let acceptor = acceptor.clone();
        tokio::task::spawn(async move {
            let accept = acceptor.accept(stream, request_handler).await;
            let (stream, request_service) = match accept {
                Ok((stream, service)) => (stream, service),
                Err(e) => {
                    log::error!("Failed to accept connection: {}", e);
                    return;
                }
            };

            let io = TokioIo::new(stream);

            let http_server =
                hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
            let conn = http_server.serve_connection_with_upgrades(io, request_service);
            tokio::pin!(conn);

            loop {
                tokio::select! {
                    _ = conn.as_mut() => break,
                    _ = shutdown_signal.changed() => conn.as_mut().graceful_shutdown(),
                }
            }
        });
    }
}
