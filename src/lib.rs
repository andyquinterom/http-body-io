use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use tokio::sync::mpsc::{error::TrySendError, Receiver};

pub struct BodyIoError;

pub fn channel() -> (BodyReader, BodyWriter) {
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    (BodyReader { receiver: rx }, BodyWriter { sender: tx })
}

impl std::fmt::Display for BodyIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BodyIoError")
    }
}

impl std::fmt::Debug for BodyIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BodyIoError")
    }
}

impl std::error::Error for BodyIoError {}

pub struct BodyReader {
    receiver: Receiver<Bytes>,
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
            Poll::Pending => Poll::Pending,
        }
    }
}

impl tokio::io::AsyncRead for BodyReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let mut this = Box::pin(self.receiver.recv());
        match this.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(bytes)) => {
                buf.put_slice(&bytes);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => Poll::Ready(Ok(())),
        }
    }
}

pub struct BodyWriter {
    sender: tokio::sync::mpsc::Sender<Bytes>,
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
        let mut this = Box::pin(self.sender.send(Bytes::copy_from_slice(buf)));
        match this.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_body() {
        use std::io::Write;
        let (_reader, mut writer) = channel();
        writer.write_all(b"Hello, ").unwrap();
    }

    #[tokio::test]
    async fn test_async_body() {
        use futures::StreamExt;

        use tokio::io::AsyncWriteExt;

        let (reader, mut writer) = channel();
        writer.write_all(b"Hello, ").await.unwrap();
        drop(writer);

        let mut stream = http_body_util::BodyStream::new(reader);

        let mut body = Vec::new();
        while let Some(Ok(bytes)) = stream.next().await {
            if let Some(bytes) = bytes.data_ref() {
                body.extend_from_slice(bytes);
            }
        }

        assert_eq!(body, b"Hello, ");
    }

    #[tokio::test]
    async fn test_async_body_sync_write() {
        use futures::StreamExt;

        let (reader, mut writer) = channel();

        let writer_thread = std::thread::spawn(move || {
            use std::io::Write;
            writer.write_all(b"Hello, ").unwrap();
        });

        let mut stream = http_body_util::BodyStream::new(reader);

        let mut body = Vec::new();
        while let Some(Ok(bytes)) = stream.next().await {
            if let Some(bytes) = bytes.data_ref() {
                body.extend_from_slice(bytes);
            }
        }

        assert_eq!(body, b"Hello, ");

        writer_thread.join().unwrap();
    }
}
