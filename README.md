# microhttp-rs

A minimal HTTP parser library written in Rust.

## Features

- Parse HTTP requests from byte slices
- Support for common HTTP methods (GET, POST, PUT, DELETE, HEAD, OPTIONS, PATCH)
- Support for HTTP versions 1.0, 1.1, and 2.0
- Proper error handling with descriptive error messages

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
microhttp-rs = "0.1.0"
```

### Example: Parsing an HTTP request

```rust
use microhttp_rs::{parse_request, HttpRequest, Method, HttpVersion, Error};

fn main() {
    let request_bytes = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";

    match parse_request(request_bytes) {
        Ok(request) => {
            println!("Method: {}", request.method);
            println!("Path: {}", request.path);
            println!("Version: {}", request.version);
            println!("Headers: {:?}", request.headers);
        }
        Err(err) => {
            println!("Error parsing request: {}", err);
        }
    }
}
```

### Example: Simple HTTP Server

The library includes a simple HTTP server example that demonstrates how to use the library:

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use microhttp_rs::{parse_request, Error};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buf = [0; 1024];
            let n = socket.read(&mut buf).await.unwrap_or(0);

            let response = match parse_request(&buf[..n]) {
                Ok(_request) => {
                    "HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\nHello world!"
                }
                Err(err) => {
                    // Handle error...
                    "HTTP/1.1 400 Bad Request\r\n\r\nInvalid request"
                }
            };

            let _ = socket.write_all(response.as_bytes()).await;
        });
    }
}
```

## Examples

The library includes several examples in the `examples` directory:

### Simple Parser

A basic example showing how to parse HTTP requests:

```bash
cargo run --example simple_parser
```

### Simple HTTP Server

A complete HTTP server example:

```bash
cargo run --example simple_server
```

Then visit http://localhost:8080 in your browser or use curl:

```bash
curl http://localhost:8080/hello
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
