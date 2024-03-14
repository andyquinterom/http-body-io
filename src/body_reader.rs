use super::BodyIoError;

#[allow(unused_imports)]
use crate::BodyWriter;

use core::{
    future::Future,
    pin::{pin, Pin},
    task::{Context, Poll},
};

use bytes::Bytes;
use tokio::sync::mpsc::Receiver;

/// A reader for the body of an HTTP request or response.
///
/// This reader implements the [`http_body::Body`] trait and is used for
/// web servers to access the data being sent by the [`BodyWriter`].
pub struct BodyReader {
    pub(crate) receiver: Receiver<Bytes>,
}

impl std::fmt::Debug for BodyReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BodyReader").finish()
    }
}

impl http_body::Body for BodyReader {
    type Data = Bytes;
    type Error = BodyIoError;
    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        match this.receiver.poll_recv(cx) {
            Poll::Ready(Some(bytes)) => {
                let frame = http_body::Frame::data(bytes);
                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

impl tokio::io::AsyncRead for BodyReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let mut this = pin!(self.receiver.recv());
        match this.as_mut().poll(cx) {
            Poll::Pending => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(bytes)) => {
                buf.put_slice(&bytes);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => Poll::Ready(Ok(())),
        }
    }
}
