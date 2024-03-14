mod body_writer;
pub use body_writer::BodyWriter;

mod body_reader;
pub use body_reader::BodyReader;

pub struct BodyIoError;

pub fn channel(bufsize: usize) -> (BodyReader, BodyWriter) {
    let (tx, rx) = tokio::sync::mpsc::channel(bufsize);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_body() {
        use std::io::Write;
        let (_reader, mut writer) = channel(10);
        writer.write_all(b"Hello, ").unwrap();
    }

    #[tokio::test]
    async fn test_async_body() {
        use futures::StreamExt;

        use tokio::io::AsyncWriteExt;

        let (reader, mut writer) = channel(10);
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

        let (reader, mut writer) = channel(10);

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
