//! HTTP server implementation for microhttp-rs.
//!
//! This module provides a simple, efficient HTTP server implementation
//! that leverages Rust's concurrency features and the microhttp-rs parser.

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use crate::parser::{Error as ParserError, HttpRequest, Method};

/// HTTP status codes with their standard reason phrases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
}

impl StatusCode {
    /// Get the reason phrase for this status code.
    pub fn reason_phrase(&self) -> &'static str {
        match self {
            StatusCode::Ok => "OK",
            StatusCode::Created => "Created",
            StatusCode::Accepted => "Accepted",
            StatusCode::NoContent => "No Content",
            StatusCode::BadRequest => "Bad Request",
            StatusCode::Unauthorized => "Unauthorized",
            StatusCode::Forbidden => "Forbidden",
            StatusCode::NotFound => "Not Found",
            StatusCode::MethodNotAllowed => "Method Not Allowed",
            StatusCode::InternalServerError => "Internal Server Error",
            StatusCode::NotImplemented => "Not Implemented",
            StatusCode::BadGateway => "Bad Gateway",
            StatusCode::ServiceUnavailable => "Service Unavailable",
        }
    }
}

/// Represents an HTTP response.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// The HTTP status code
    pub status: StatusCode,
    /// The HTTP headers
    pub headers: HashMap<String, String>,
    /// The response body
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Create a new HTTP response with the given status code.
    pub fn new(status: StatusCode) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Server".to_string(), "microhttp-rs".to_string());

        Self {
            status,
            headers,
            body: Vec::new(),
        }
    }

    /// Set the response body with a string.
    pub fn with_body_string(mut self, body: impl Into<String>) -> Self {
        let body_string = body.into();
        self.body = body_string.into_bytes();
        let content_length = self.body.len().to_string();
        self.with_header("Content-Length", content_length)
    }

    /// Set the response body with bytes.
    pub fn with_body_bytes(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        let content_length = self.body.len().to_string();
        self.with_header("Content-Length", content_length)
    }

    /// Add or replace a header.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Set the content type.
    pub fn with_content_type(self, content_type: impl Into<String>) -> Self {
        self.with_header("Content-Type", content_type)
    }

    /// Convert the response to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Status line
        let status_line = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status as u16,
            self.status.reason_phrase()
        );
        result.extend_from_slice(status_line.as_bytes());

        // Headers
        for (name, value) in &self.headers {
            let header_line = format!("{}: {}\r\n", name, value);
            result.extend_from_slice(header_line.as_bytes());
        }

        // Empty line separating headers from body
        result.extend_from_slice(b"\r\n");

        // Body
        result.extend_from_slice(&self.body);

        result
    }
}

/// Errors that can occur during server operation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error parsing the HTTP request.
    #[error("Request parsing error: {0}")]
    ParseError(#[from] ParserError),

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// No handler found for the requested path.
    #[error("No handler found for path: {0}")]
    NotFound(String),

    /// Method not allowed for the requested path.
    #[error("Method {0} not allowed for path: {1}")]
    MethodNotAllowed(Method, String),

    /// Internal server error.
    #[error("Internal server error: {0}")]
    InternalError(String),
}

/// Type alias for a handler function.
pub type HandlerFn = Arc<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = Result<HttpResponse, Error>> + Send>> + Send + Sync>;

/// A route in the HTTP server.
#[derive(Clone)]
pub struct Route {
    /// The path pattern to match.
    pub path: String,
    /// The HTTP methods this route handles.
    pub methods: Vec<Method>,
    /// The handler function.
    pub handler: HandlerFn,
}

/// HTTP server configuration.
#[derive(Clone)]
pub struct ServerConfig {
    /// The address to bind to.
    pub addr: SocketAddr,
    /// The maximum number of concurrent connections.
    pub max_connections: usize,
    /// The read buffer size.
    pub read_buffer_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:8080".parse().unwrap(),
            max_connections: 1024,
            read_buffer_size: 8192,
        }
    }
}

/// An HTTP server.
pub struct HttpServer {
    /// The server configuration.
    pub config: ServerConfig,
    /// The routes.
    pub routes: Arc<RwLock<Vec<Route>>>,
}

impl HttpServer {
    /// Create a new HTTP server with the given configuration.
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            routes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a route to the server.
    pub async fn add_route<F, Fut>(&self, path: impl Into<String>, methods: Vec<Method>, handler: F)
    where
        F: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<HttpResponse, Error>> + Send + 'static,
    {
        let path = path.into();
        let handler = Arc::new(move |req: HttpRequest| -> Pin<Box<dyn Future<Output = Result<HttpResponse, Error>> + Send>> {
            Box::pin(handler(req))
        });

        let route = Route {
            path,
            methods,
            handler,
        };

        self.routes.write().await.push(route);
    }

    /// Start the server and listen for incoming connections.
    pub async fn start(&self) -> Result<(), Error> {
        let listener = TcpListener::bind(&self.config.addr).await?;
        println!("Server listening on http://{}", self.config.addr);

        loop {
            let (mut socket, addr) = listener.accept().await?;
            println!("Connection from: {}", addr);

            let routes = self.routes.clone();
            let read_buffer_size = self.config.read_buffer_size;

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(&mut socket, routes, read_buffer_size).await {
                    eprintln!("Error handling connection: {}", e);
                }
            });
        }
    }

    /// Handle a single connection.
    pub async fn handle_connection(
        socket: &mut (impl AsyncRead + AsyncWrite + Unpin),
        routes: Arc<RwLock<Vec<Route>>>,
        read_buffer_size: usize,
    ) -> Result<(), Error> {
        let mut buf = vec![0; read_buffer_size];

        // Read data from the socket
        let n = socket.read(&mut buf).await?;
        if n == 0 {
            return Ok(());  // Connection closed
        }

        // Parse the HTTP request
        let request = match crate::parse_request(&buf[..n]) {
            Ok(req) => req,
            Err(e) => {
                let response = HttpResponse::new(StatusCode::BadRequest)
                    .with_content_type("text/plain")
                    .with_body_string(format!("Error parsing request: {}", e));
                socket.write_all(&response.to_bytes()).await?;
                return Err(Error::ParseError(e));
            }
        };

        // Find a matching route
        let routes_guard = routes.read().await;
        let matching_routes: Vec<&Route> = routes_guard
            .iter()
            .filter(|route| route.path == request.path)
            .collect();

        if matching_routes.is_empty() {
            let response = HttpResponse::new(StatusCode::NotFound)
                .with_content_type("text/plain")
                .with_body_string(format!("Not found: {}", request.path));
            socket.write_all(&response.to_bytes()).await?;
            return Err(Error::NotFound(request.path));
        }

        // Find a route that matches the method
        let route = matching_routes
            .iter()
            .find(|route| route.methods.contains(&request.method));

        match route {
            Some(route) => {
                // Call the handler
                let response = match (route.handler)(request).await {
                    Ok(resp) => resp,
                    Err(e) => {
                        let response = HttpResponse::new(StatusCode::InternalServerError)
                            .with_content_type("text/plain")
                            .with_body_string(format!("Internal server error: {}", e));
                        socket.write_all(&response.to_bytes()).await?;
                        return Err(e);
                    }
                };

                // Send the response
                socket.write_all(&response.to_bytes()).await?;
            }
            None => {
                // Method not allowed
                let allowed_methods: Vec<String> = matching_routes
                    .iter()
                    .flat_map(|route| route.methods.iter().map(|m| m.to_string()))
                    .collect();

                let response = HttpResponse::new(StatusCode::MethodNotAllowed)
                    .with_header("Allow", allowed_methods.join(", "))
                    .with_content_type("text/plain")
                    .with_body_string(format!(
                        "Method {} not allowed for path: {}. Allowed methods: {}",
                        request.method,
                        request.path,
                        allowed_methods.join(", ")
                    ));

                socket.write_all(&response.to_bytes()).await?;
                return Err(Error::MethodNotAllowed(request.method, request.path));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_code_reason_phrase() {
        assert_eq!(StatusCode::Ok.reason_phrase(), "OK");
        assert_eq!(StatusCode::NotFound.reason_phrase(), "Not Found");
        assert_eq!(StatusCode::InternalServerError.reason_phrase(), "Internal Server Error");
    }

    #[test]
    fn test_http_response_creation() {
        let response = HttpResponse::new(StatusCode::Ok);
        assert_eq!(response.status, StatusCode::Ok);
        assert_eq!(response.headers.get("Server"), Some(&"microhttp-rs".to_string()));
        assert!(response.body.is_empty());
    }

    #[test]
    fn test_http_response_with_body_string() {
        let body = "Hello, world!";
        let response = HttpResponse::new(StatusCode::Ok)
            .with_body_string(body);

        assert_eq!(response.body, body.as_bytes());
        assert_eq!(
            response.headers.get("Content-Length"),
            Some(&body.len().to_string())
        );
    }

    #[test]
    fn test_http_response_with_body_bytes() {
        let body = b"Binary data";
        let response = HttpResponse::new(StatusCode::Ok)
            .with_body_bytes(body.to_vec());

        assert_eq!(response.body, body);
        assert_eq!(
            response.headers.get("Content-Length"),
            Some(&body.len().to_string())
        );
    }

    #[test]
    fn test_http_response_with_header() {
        let response = HttpResponse::new(StatusCode::Ok)
            .with_header("X-Custom", "Value");

        assert_eq!(
            response.headers.get("X-Custom"),
            Some(&"Value".to_string())
        );
    }

    #[test]
    fn test_http_response_with_content_type() {
        let response = HttpResponse::new(StatusCode::Ok)
            .with_content_type("application/json");

        assert_eq!(
            response.headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
    }

    #[test]
    fn test_http_response_to_bytes() {
        let response = HttpResponse::new(StatusCode::Ok)
            .with_content_type("text/plain")
            .with_body_string("Hello, world!");

        let bytes = response.to_bytes();
        let response_str = String::from_utf8_lossy(&bytes);

        assert!(response_str.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response_str.contains("Content-Type: text/plain\r\n"));
        assert!(response_str.contains("Content-Length: 13\r\n"));
        assert!(response_str.contains("Server: microhttp-rs\r\n"));
        assert!(response_str.ends_with("\r\n\r\nHello, world!"));
    }

    // Mock tests for HttpServer will be added in a separate test module
}
