//! Raw interface to invoker
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tracing::instrument;
/// Means for communicating with invoker
struct Pipes {
    stdin: tokio::io::BufWriter<tokio::process::ChildStdin>,
    stdout: tokio::io::BufReader<tokio::process::ChildStdout>,
}

struct CoherenceWrapper(tokio::sync::OwnedMutexGuard<Pipes>);

impl hyper::client::connect::Connection for CoherenceWrapper {
    fn connected(&self) -> hyper::client::connect::Connected {
        hyper::client::connect::Connected::new()
    }
}

impl tokio::io::AsyncRead for CoherenceWrapper {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        Pin::new(&mut self.0.stdout).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for CoherenceWrapper {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.0.stdin).poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.0.stdin).poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.0.stdin).poll_shutdown(cx)
    }
}

#[derive(Clone)]
struct Connector {
    pipes: Arc<tokio::sync::Mutex<Pipes>>,
}

impl hyper::service::Service<hyper::Uri> for Connector {
    type Error = std::convert::Infallible;
    type Future = futures_util::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Response = CoherenceWrapper;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _uri: hyper::Uri) -> Self::Future {
        let pipes = self.pipes.clone();
        Box::pin(async move { Ok(CoherenceWrapper(pipes.lock_owned().await)) })
    }
}

/// Takes child's input and output and abstracts it as a channel.
/// Should be spawned on background task.
#[instrument(skip(stdin, stdout, tx, rx))]
pub(crate) async fn serve(
    stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
    rx: async_channel::Receiver<hyper::Request<hyper::Body>>,
    tx: async_channel::Sender<hyper::Response<hyper::Body>>,
) {
    let stdin = tokio::io::BufWriter::new(stdin);
    let stdout = tokio::io::BufReader::new(stdout);
    let pipes = Arc::new(tokio::sync::Mutex::new(Pipes { stdin, stdout }));
    let connector = Connector { pipes };
    let client = hyper::client::Client::builder().build::<_, hyper::Body>(connector);
    while let Ok(req) = rx.recv().await {
        let res = match client.request(req).await {
            Ok(response) => response,
            Err(error) => hyper::Response::builder()
                .status(418 /* why? because */)
                .body(format!("{:#}", error).into())
                .unwrap(),
        };
        tx.send(res).await.ok();
    }
    tracing::info!("Sender dropped, exiting");
}
