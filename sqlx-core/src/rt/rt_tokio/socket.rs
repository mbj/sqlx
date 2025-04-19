use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

use crate::io::ReadBuf;
use crate::net::Socket;

/// Adapter that bridges tokio's [`AsyncRead`] + [`AsyncWrite`] to sqlx's [`Socket`] trait.
///
/// This allows using any tokio-compatible async stream (e.g. `tokio_rustls::TlsStream<TcpStream>`)
/// as a transport for sqlx connections.
///
/// Internally buffers reads because sqlx's [`Socket`] trait separates readiness polling
/// (`poll_read_ready`) from data reading (`try_read`), while [`AsyncRead`] combines both.
pub struct TokioAsyncSocket<S> {
    inner: S,
    read_buf: Vec<u8>,
}

impl<S> TokioAsyncSocket<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            read_buf: Vec::new(),
        }
    }
}

impl<S> Socket for TokioAsyncSocket<S>
where
    S: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    fn try_read(&mut self, buf: &mut dyn ReadBuf) -> io::Result<usize> {
        if self.read_buf.is_empty() {
            return Err(io::ErrorKind::WouldBlock.into());
        }

        let to_copy = self.read_buf.len();
        buf.put_slice(&self.read_buf);
        self.read_buf.clear();
        Ok(to_copy)
    }

    fn try_write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut cx = Context::from_waker(std::task::Waker::noop());
        match Pin::new(&mut self.inner).poll_write(&mut cx, buf) {
            Poll::Ready(result) => result,
            Poll::Pending => Err(io::ErrorKind::WouldBlock.into()),
        }
    }

    fn poll_read_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if !self.read_buf.is_empty() {
            return Poll::Ready(Ok(()));
        }

        let mut tmp = [0u8; 8192];
        let mut read_buf = tokio::io::ReadBuf::new(&mut tmp);

        match Pin::new(&mut self.inner).poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                let filled = read_buf.filled();
                if filled.is_empty() {
                    // EOF
                    Poll::Ready(Ok(()))
                } else {
                    self.read_buf.extend_from_slice(filled);
                    Poll::Ready(Ok(()))
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_write_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

impl Socket for TcpStream {
    fn try_read(&mut self, mut buf: &mut dyn ReadBuf) -> io::Result<usize> {
        // Requires `&mut impl BufMut`
        self.try_read_buf(&mut buf)
    }

    fn try_write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (*self).try_write(buf)
    }

    fn poll_read_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (*self).poll_read_ready(cx)
    }

    fn poll_write_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (*self).poll_write_ready(cx)
    }

    fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(self).poll_shutdown(cx)
    }
}

#[cfg(unix)]
impl Socket for tokio::net::UnixStream {
    fn try_read(&mut self, mut buf: &mut dyn ReadBuf) -> io::Result<usize> {
        self.try_read_buf(&mut buf)
    }

    fn try_write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (*self).try_write(buf)
    }

    fn poll_read_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (*self).poll_read_ready(cx)
    }

    fn poll_write_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (*self).poll_write_ready(cx)
    }

    fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(self).poll_shutdown(cx)
    }
}
