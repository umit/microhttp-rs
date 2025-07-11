//! A simple HTTP server example demonstrating how to use the microhttp-rs library.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use microhttp_rs::{parse_request, ParserError};
use log::{info, error, debug};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize the logger
    env_logger::init();
    // Bind to localhost:8082
    let listener = TcpListener::bind("127.0.0.1:8082").await?;
    info!("Server listening on http://127.0.0.1:8082");

    loop {
        // Accept incoming connections
        let (mut socket, addr) = listener.accept().await?;
        info!("Connection from: {addr}");

        // Spawn a new task for each connection
        tokio::spawn(async move {
            let mut buf = [0; 1024];

            // Read data from the socket
            match socket.read(&mut buf).await {
                Ok(n) if n > 0 => {
                    debug!("Received {n} bytes");

                    // Parse the HTTP request
                    let response = match parse_request(&buf[..n]) {
                        Ok(request) => {
                            debug!("Parsed request: {} {} {}", request.method, request.path, request.version);

                            // Generate a simple response
                            let body = format!(
                                "Hello! You requested {} with method {} and HTTP version {}",
                                request.path, request.method, request.version
                            );

                            format!(
                                "HTTP/1.1 200 OK\r\n\
                                Content-Type: text/plain\r\n\
                                Content-Length: {}\r\n\
                                \r\n\
                                {}",
                                body.len(),
                                body
                            )
                        },
                        Err(err) => {
                            error!("Error parsing request: {err}");

                            // Generate an error response
                            let error_message = match err {
                                ParserError::InvalidPath => "Invalid HTTP path".to_string(),
                                ParserError::MissingHeader(header) => format!("Required header is missing: {header}"),
                                ParserError::InvalidHeaderFormat => "Invalid header format".to_string(),
                                ParserError::InvalidMethod(method) => format!("Invalid HTTP method: {method}"),
                                ParserError::InvalidVersion(version) => format!("Invalid HTTP version: {version}"),
                                ParserError::MalformedRequestLine(line) => format!("Malformed request line: {line}"),
                                ParserError::EmptyRequest => "Empty request".to_string(),
                                ParserError::JsonError(e) => format!("JSON parsing error: {e}"),
                            };

                            format!(
                                "HTTP/1.1 400 Bad Request\r\n\
                                Content-Type: text/plain\r\n\
                                Content-Length: {}\r\n\
                                \r\n\
                                {}",
                                error_message.len(),
                                error_message
                            )
                        }
                    };

                    // Write the response back to the client
                    if let Err(e) = socket.write_all(response.as_bytes()).await {
                        error!("Error writing response: {e}");
                    }
                },
                Ok(_) => {
                    info!("Client closed connection");
                },
                Err(e) => {
                    error!("Error reading from socket: {e}");
                }
            }
        });
    }
}
