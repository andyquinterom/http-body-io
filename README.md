# http-body-io

This is a very simple crate that implements IO traits for HTTP bodies. It is
intended to be used with [tokio](https://tokio.rs) and the
[http-body](https://docs.rs/http-body/latest/http_body/) crate. This allows
you to use this crate with [hyper](https://hyper.rs) and frameworks like
[axum](https://docs.rs/axum/latest/axum/).

## Example

This is an example with axum.

```toml
[dependencies]
axum = "0.7.4"
http-body-io = "0.2"
tokio = { version = "1", features = ["full"] }
```

```rust
use axum::{body::Body, http::StatusCode, routing::get, Router};
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root));

    // run our app with hyper, listening globally on port 3333
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3333").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a body reader
async fn root() -> (StatusCode, Body) {
    let buffer_size = 10.try_into().expect("10 is a valid buffer size");

    // We create a channel for the body with a buffer of 1
    // item.
    let (body_writer, body_reader) = http_body_io::channel(buffer_size);

    // Spawn a task to write the body
    tokio::spawn(async move {
        // Create a buffer for the writer, this is not necessary
        // but it can improve performance
        let mut body_writer = tokio::io::BufWriter::new(body_writer);

        // Write some data to the body 100 times
        for _ in 0..100 {
            body_writer.write_all(b"Hello World\n").await.unwrap();
        }

        // Flush the writer to finish the body
        body_writer.flush().await.unwrap();
    });

    // Return the body reader with a status code
    // The write operation will be finished in the background
    (StatusCode::OK, Body::new(body_reader))
}
```
