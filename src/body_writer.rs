use core::{
    future::Future,
    pin::{pin, Pin},
    task::{Context, Poll},
};

use bytes::Bytes;
use tokio::sync::mpsc::error::TrySendError;

/// A writer for the body of an HTTP request or response.
///
/// This writer can be used to write the body of an HTTP request or response
/// using the [`std::io::Write`] or [`tokio::io::AsyncWrite`] traits.
///
/// In order for the reader to stop reading, the writer must be dropped.
pub struct BodyWriter {
    pub(crate) sender: tokio::sync::mpsc::Sender<Bytes>,
}

impl std::fmt::Debug for BodyWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BodyWriter").finish()
    }
}

impl std::io::Write for BodyWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut bytes = Bytes::copy_from_slice(buf);
        loop {
            match self.sender.try_send(bytes) {
                Ok(()) => return Ok(buf.len()),
                Err(TrySendError::Full(bytes_ret)) => {
                    bytes = bytes_ret;
                    std::thread::yield_now();
                }
                Err(TrySendError::Closed(_)) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "BodyWriter closed",
                    ));
                }
            }
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl tokio::io::AsyncWrite for BodyWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let mut this = pin!(self.sender.send(Bytes::copy_from_slice(buf)));
        match this.as_mut().poll(cx) {
            Poll::Pending => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Ok(())) => Poll::Ready(Ok(buf.len())),
            Poll::Ready(Err(_)) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "BodyWriter closed",
            ))),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}
