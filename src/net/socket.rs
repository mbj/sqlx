use std::future::Future;
use std::io;
use std::path::Path;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

pub use buffered::{BufferedSocket, WriteBuffer};
use bytes::BufMut;

use crate::io::ReadBuf;

mod buffered;

pub trait Socket: Send + Sync + Unpin + 'static {
    fn try_read(&mut self, buf: &mut dyn ReadBuf) -> io::Result<usize>;

    fn try_write(&mut self, buf: &[u8]) -> io::Result<usize>;

    fn poll_read_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>>;

    fn poll_write_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>>;

    fn poll_flush(&mut self, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // `flush()` is a no-op for TCP/UDS
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>>;

    fn read<'a, B: ReadBuf>(&'a mut self, buf: &'a mut B) -> Read<'a, Self, B>
    where
        Self: Sized,
    {
        Read { socket: self, buf }
    }

    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Write<'a, Self>
    where
        Self: Sized,
    {
        Write { socket: self, buf }
    }

    fn flush(&mut self) -> Flush<'_, Self>
    where
        Self: Sized,
    {
        Flush { socket: self }
    }

    fn shutdown(&mut self) -> Shutdown<'_, Self>
    where
        Self: Sized,
    {
        Shutdown { socket: self }
    }
}

pub struct Read<'a, S: ?Sized, B> {
    socket: &'a mut S,
    buf: &'a mut B,
}

impl<S: ?Sized, B> Future for Read<'_, S, B>
where
    S: Socket,
    B: ReadBuf,
{
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;

        while this.buf.has_remaining_mut() {
            match this.socket.try_read(&mut *this.buf) {
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    ready!(this.socket.poll_read_ready(cx))?;
                }
                ready => return Poll::Ready(ready),
            }
        }

        Poll::Ready(Ok(0))
    }
}

pub struct Write<'a, S: ?Sized> {
    socket: &'a mut S,
    buf: &'a [u8],
}

impl<S: ?Sized> Future for Write<'_, S>
where
    S: Socket,
{
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;

        while !this.buf.is_empty() {
            match this.socket.try_write(this.buf) {
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    ready!(this.socket.poll_write_ready(cx))?;
                }
                ready => return Poll::Ready(ready),
            }
        }

        Poll::Ready(Ok(0))
    }
}

pub struct Flush<'a, S: ?Sized> {
    socket: &'a mut S,
}

impl<S: Socket + ?Sized> Future for Flush<'_, S> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.socket.poll_flush(cx)
    }
}

pub struct Shutdown<'a, S: ?Sized> {
    socket: &'a mut S,
}

impl<S: ?Sized> Future for Shutdown<'_, S>
where
    S: Socket,
{
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.socket.poll_shutdown(cx)
    }
}

pub trait WithSocket {
    type Output;

    fn with_socket<S: Socket>(self, socket: S) -> impl Future<Output = Self::Output> + Send;
}

pub struct SocketIntoBox;

impl WithSocket for SocketIntoBox {
    type Output = Box<dyn Socket>;

    async fn with_socket<S: Socket>(self, socket: S) -> Self::Output {
        Box::new(socket)
    }
}

impl<S: Socket + ?Sized> Socket for Box<S> {
    fn try_read(&mut self, buf: &mut dyn ReadBuf) -> io::Result<usize> {
        (**self).try_read(buf)
    }

    fn try_write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (**self).try_write(buf)
    }

    fn poll_read_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (**self).poll_read_ready(cx)
    }

    fn poll_write_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (**self).poll_write_ready(cx)
    }

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (**self).poll_flush(cx)
    }

    fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (**self).poll_shutdown(cx)
    }
}

pub async fn connect_tcp<Ws: WithSocket>(
    host: &str,
    port: u16,
    with_socket: Ws,
) -> crate::Result<Ws::Output> {
    Ok(with_socket
        .with_socket(tokio::net::TcpStream::connect((host, port)).await?)
        .await)
}

/// Connect a Unix Domain Socket at the given path.
///
/// Returns an error if Unix Domain Sockets are not supported on this platform.
pub async fn connect_uds<P: AsRef<Path>, Ws: WithSocket>(
    path: P,
    with_socket: Ws,
) -> crate::Result<Ws::Output> {
    #[cfg(unix)]
    {
        use tokio::net::UnixStream;

        let stream = UnixStream::connect(path).await?;

        Ok(with_socket.with_socket(stream).await)
    }

    #[cfg(not(unix))]
    {
        drop((path, with_socket));

        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Unix domain sockets are not supported on this platform",
        )
        .into())
    }
}

impl Socket for tokio::net::TcpStream {
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
        <Self as tokio::io::AsyncWrite>::poll_shutdown(Pin::new(self), cx)
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
        <Self as tokio::io::AsyncWrite>::poll_shutdown(Pin::new(self), cx)
    }
}
