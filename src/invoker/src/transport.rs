use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite};

struct StdioClient {
    stdin: tokio::io::Stdin,
    stdout: tokio::io::Stdout,
}

impl AsyncRead for StdioClient {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.stdin).poll_read(cx, buf)
    }
}

impl AsyncWrite for StdioClient {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.stdout).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stdout).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stdout).poll_shutdown(cx)
    }
}

pub(crate) struct IncomingStdio(Option<StdioClient>);

impl hyper::server::accept::Accept for IncomingStdio {
    type Conn = StdioClient;
    type Error = std::convert::Infallible;

    fn poll_accept(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        let this = Pin::into_inner(self);
        Poll::Ready(this.0.take().map(Ok))
    }
}

impl IncomingStdio {
    pub(crate) fn new() -> Self {
        IncomingStdio(Some(StdioClient {
            stdin: tokio::io::stdin(),
            stdout: tokio::io::stdout(),
        }))
    }
}
