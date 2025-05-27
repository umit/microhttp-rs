//! A simple HTTP server example demonstrating how to use the microhttp-rs library.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use microhttp_rs::{parse_request, Error};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Bind to localhost:8081
    let listener = TcpListener::bind("127.0.0.1:8081").await?;
    println!("Server listening on http://127.0.0.1:8081");

    loop {
        // Accept incoming connections
        let (mut socket, addr) = listener.accept().await?;
        println!("Connection from: {}", addr);

        // Spawn a new task for each connection
        tokio::spawn(async move {
            let mut buf = [0; 1024];

            // Read data from the socket
            match socket.read(&mut buf).await {
                Ok(n) if n > 0 => {
                    println!("Received {} bytes", n);

                    // Parse the HTTP request
                    let response = match parse_request(&buf[..n]) {
                        Ok(request) => {
                            println!("Parsed request: {} {} {}", request.method, request.path, request.version);

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
                            println!("Error parsing request: {}", err);

                            // Generate an error response
                            let error_message = match err {
                                Error::InvalidPath => "Invalid HTTP path".to_string(),
                                Error::MissingHeader(header) => format!("Required header is missing: {}", header),
                                Error::InvalidHeaderFormat => "Invalid header format".to_string(),
                                Error::InvalidMethod(method) => format!("Invalid HTTP method: {}", method),
                                Error::InvalidVersion(version) => format!("Invalid HTTP version: {}", version),
                                Error::MalformedRequestLine(line) => format!("Malformed request line: {}", line),
                                Error::EmptyRequest => "Empty request".to_string(),
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
                        println!("Error writing response: {}", e);
                    }
                },
                Ok(_) => {
                    println!("Client closed connection");
                },
                Err(e) => {
                    println!("Error reading from socket: {}", e);
                }
            }
        });
    }
}
